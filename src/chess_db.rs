use crate::game_stats::{GameStats, GameWins};
use rocksdb::{WriteBatch, DB};
use shakmaty::{fen::Fen, uci::Uci, CastlingMode, Chess, EnPassantMode, Move};
use std::collections::HashMap;

const PS: &str = "position_stats";
const PMC: &str = "position_move_count";

pub fn pos_to_keyable(pos: &Chess) -> String {
    Fen::from_position(pos.clone(), EnPassantMode::Legal).to_string()
}

pub fn pos_to_key(keyable: &String) -> String {
    PS.to_owned() + keyable
}

fn pos_to_prefix(keyable: &String) -> String {
    PMC.to_owned() + keyable
}

pub fn pos_move_to_key(keyable: &String, chess_move: &Move) -> String {
    PMC.to_owned() + keyable + &chess_move.to_uci(CastlingMode::Standard).to_string()
}

fn key_to_uci(key: Vec<u8>, prefix: &str) -> Uci {
    let key_string =
        String::from_utf8(key).expect("Key isn't decoding to UTF-8 correctly");
    let move_string: String = key_string
        .chars()
        .into_iter()
        .skip(prefix.chars().count())
        .collect();
    Uci::from_ascii(move_string.as_bytes())
        .expect("Failed to parse UCI from key")
}

fn is_valid_prefix(key: &[u8], prefix: &str) -> bool {
    let key_string = String::from_utf8(key.to_owned())
        .expect("Key isn't decoding to UTF-8 correctly");
    key_string.starts_with(prefix)
}

pub struct ChessDB<'a> {
    db: &'a DB,
    cache: HashMap<String, GameWins>,
}

impl ChessDB<'_> {
    pub fn new(db: &DB) -> ChessDB {
        ChessDB {
            db,
            cache: HashMap::new(),
        }
    }

    // TODO: include cache maybe?
    pub fn get_pos_stats(&mut self, pos: &Chess) -> Option<GameStats> {
        let keyable = pos_to_keyable(pos);
        let prefix = pos_to_prefix(&keyable);
        let prefix_clone = prefix.clone();
        let prefix_iter = self.db.prefix_iterator(prefix);
        let mut game_moves = HashMap::new();
        for item in prefix_iter {
            let (key, value) = item.expect("Prefix iter error in rocks db?");
            let key_clone = key.clone().into_vec();
            // NOTE: stopping iter on mismatched prefix, not sure how to bound it otherwise
            if !is_valid_prefix(&key_clone, &prefix_clone) {
                break;
            }
            let m = key_to_uci(key_clone, &prefix_clone)
                .to_move(pos)
                .expect("The move is invalid uci for the position!");
            let game_wins = GameWins::from_bytes(value.to_vec());
            let uci = m.to_uci(CastlingMode::Standard).to_string();
            game_moves.insert(uci, game_wins);
        }

        Self::get_pos_wins(self, &keyable).map(|game_wins| GameStats {
            game_wins,
            game_moves,
        })
    }

    pub fn get_pos_wins(&mut self, keyable: &String) -> Option<GameWins> {
        let key = pos_to_key(keyable);
        match self.cache.get(&key) {
            None => {
                let game_wins = GameWins::from_bytes(
                    self.db.get(&key).ok()??.to_vec(),
                );
                self.cache.insert(key, game_wins);
                Some(game_wins)
            }
            game_wins => game_wins.copied(),
        }
    }

    pub fn update_pos_wins(&mut self, keyable: &String, game_wins: GameWins) {
        let key = pos_to_key(keyable);
        if let Some(db_stats) = Self::get_pos_wins(self, keyable) {
            self.cache.insert(key, db_stats.combine(&game_wins));
        } else {
            self.cache.insert(key, game_wins);
        }
    }

    pub fn get_pos_move_wins(
        &mut self,
        keyable: &String,
        chess_move: Move,
    ) -> Option<GameWins> {
        let key = pos_move_to_key(keyable, &chess_move);
        match self.cache.get(&key) {
            None => {
                let game_wins = GameWins::from_bytes(
                    self.db.get(pos_to_key(keyable)).ok()??.to_vec(),
                );
                self.cache.insert(key, game_wins);
                Some(game_wins)
            }
            game_wins => game_wins.copied(),
        }
    }

    pub fn update_pos_move_wins(
        &mut self,
        keyable: &String,
        chess_move: Move, // TODO: check that to_string is reasonable
        game_wins: GameWins,
    ) {
        let key = pos_move_to_key(keyable, &chess_move);
        if let Some(db_wins) = Self::get_pos_move_wins(self, keyable, chess_move) {
            self.cache.insert(key, db_wins.combine(&game_wins));
        } else {
            self.cache.insert(key, game_wins);
        }
    }

    pub fn flush(&mut self) {
        println!("Creating batch");
        let mut batch = WriteBatch::default();
        let cache = std::mem::take(&mut self.cache);
        println!("Iterating the cache");
        for (k, v) in cache {
            batch.put(k, v.to_bytes());
        }
        println!("Writing to DB");
        self.db
            .write(batch)
            .expect("Failed to write batch to rocks.");
    }
}
