use crate::game_stats::{GameStats, GameWins};
use rocksdb::{WriteBatch, DB};
use shakmaty::{fen::Fen, uci::Uci, CastlingMode, EnPassantMode, Chess, Move};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

const PS: &[u8] = "position_stats".as_bytes();
const PMC: &[u8] = "position_move_count".as_bytes();

pub fn pos_to_fen(pos: &Chess) -> String {
    Fen::from_position(pos.clone(), EnPassantMode::Legal).to_string()
}

pub fn pos_to_keyable(pos: &Chess) -> Vec<u8> {
    let mut hasher = DefaultHasher::new();
    Fen::from_position(pos.clone(), EnPassantMode::Legal).hash(&mut hasher);
    let hash = hasher.finish();
    hash.to_be_bytes().to_vec()
}

pub fn pos_to_key(keyable: &[u8]) -> Vec<u8> {
    let mut ret = PS.to_owned();
    ret.append(&mut keyable.to_owned());
    ret
}

pub fn pos_to_prefix(keyable: &[u8]) -> Vec<u8> {
    let mut ret = PMC.to_owned();
    ret.append(&mut keyable.to_owned());
    ret
}

pub fn pos_move_to_key(keyable: &[u8], chess_move: &Move) -> Vec<u8> {
    let mut ret = pos_to_prefix(keyable);
    ret.append(&mut chess_move.to_uci(CastlingMode::Standard).to_string().as_bytes().to_vec());
    ret
}

fn key_to_uci(key: &[u8], prefix: &[u8]) -> Uci {
    Uci::from_ascii(&key[prefix.len()..]).expect("Failed to parse UCI from key")
}

fn is_valid_prefix(key: &[u8], prefix: &[u8]) -> bool {
    prefix == key.get(..prefix.len()).expect("Prefix is longer than key!!")
}

pub struct ChessDB<'a> {
    db: &'a DB,
    cache: HashMap<Vec<u8>, GameWins>,
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
        let prefix_iter = self.db.prefix_iterator(&prefix);
        let mut game_moves = HashMap::new();
        for item in prefix_iter {
            let (key, value) = item.expect("Prefix iter error in rocks db?");
            // NOTE: stopping iter on mismatched prefix, not sure how to bound it otherwise
            if !is_valid_prefix(&key, &prefix) {
                break;
            }
            let m = key_to_uci(&key, &prefix)
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

    pub fn get_pos_wins(&mut self, keyable: &[u8]) -> Option<GameWins> {
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

    pub fn update_pos_wins(&mut self, keyable: &[u8], game_wins: GameWins) {
        let key = pos_to_key(keyable);
        if let Some(db_stats) = Self::get_pos_wins(self, keyable) {
            self.cache.insert(key, db_stats.combine(&game_wins));
        } else {
            self.cache.insert(key, game_wins);
        }
    }

    pub fn get_pos_move_wins(
        &mut self,
        keyable: &[u8],
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
        keyable: &[u8],
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
        println!("Iterating the cache");
        for (k, v) in self.cache.drain() {
            batch.put(k, v.to_bytes());
        }
        println!("Writing to DB");
        self.db
            .write(batch)
            .expect("Failed to write batch to rocks.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shakmaty::{san::San, Position};
    #[test]
    fn prefix_of_key_test() {
        let mut board = Chess::new();
        let e4 = "e4".parse::<San>().unwrap().to_move(&board).unwrap();
        board.play_unchecked(&e4);
        let e5 = "e5".parse::<San>().unwrap().to_move(&board).unwrap();

        let keyable = pos_to_keyable(&board);
        let key = pos_move_to_key(&keyable, &e5);

        let fen_str = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1";
        let fen: Fen = fen_str.parse().expect("invalid FEN!");
        let pos: Chess = fen.into_position(CastlingMode::Standard).expect("Not a parseable FEN?!");
        let keyable2 = pos_to_keyable(&pos);
        let prefix = pos_to_prefix(&keyable2);

        assert_eq!(prefix, &key[..prefix.len()]);
    }
}
