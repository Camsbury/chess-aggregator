use crate::game_stats::{GameStats, GameWins};
use rocksdb::{WriteBatch, DB};
use shakmaty::{
    uci::Uci,
    CastlingMode,
    Chess,
    EnPassantMode,
    Move,
    zobrist::{Zobrist64, ZobristHash},
};
use std::collections::HashMap;

//position stats
const PS: &[u8] = b"ps";
//position move stats
const PMS: &[u8] = b"pms";
//file ingestion stats
pub const FS: &[u8] = b"fs";

#[must_use]
pub fn pos_to_keyable(pos: &Chess) -> Vec<u8> {
    // hash that ignores half-move and full-move counters
    let h: u64 = pos.zobrist_hash::<Zobrist64>(EnPassantMode::Legal).into();
    // eight-byte prefix
    h.to_be_bytes().into()
}

#[must_use] pub fn pos_to_key(keyable: &[u8]) -> Vec<u8> {
    let mut ret = PS.to_owned();
    ret.append(&mut keyable.to_owned());
    ret
}

#[must_use] pub fn pos_to_prefix(keyable: &[u8]) -> Vec<u8> {
    let mut ret = PMS.to_owned();
    ret.append(&mut keyable.to_owned());
    ret
}

#[must_use] pub fn pos_move_to_key(keyable: &[u8], chess_move: &Move) -> Vec<u8> {
    let mut ret = pos_to_prefix(keyable);
    ret.append(
        &mut chess_move
            .to_uci(CastlingMode::Standard)
            .to_string()
            .as_bytes()
            .to_vec(),
    );
    ret
}

fn key_to_uci(key: &[u8], prefix: &[u8]) -> Uci {
    Uci::from_ascii(&key[prefix.len()..]).expect("Failed to parse UCI from key")
}

fn is_valid_prefix(key: &[u8], prefix: &[u8]) -> bool {
    prefix
        == key
            .get(..prefix.len())
            .expect("Prefix is longer than key!!")
}

pub struct ChessDB<'a> {
    db: &'a DB,
    cache: HashMap<Vec<u8>, GameWins>,
}

impl ChessDB<'_> {
    #[must_use] pub fn new(db: &DB) -> ChessDB {
        ChessDB {
            db,
            cache: HashMap::new(),
        }
    }

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
            let game_wins = GameWins::from_bytes(&value);
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
                let game_wins =
                    GameWins::from_bytes(&self.db.get(&key).ok()??);
                self.cache.insert(key, game_wins);
                Some(game_wins)
            }
            game_wins => game_wins.copied(),
        }
    }

    pub fn flush(&mut self) {
        println!("Cache size at flush: {}", self.cache.len());
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
        println!("Written to DB");
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

        let fen_str =
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1";
        let fen: Fen = fen_str.parse().expect("invalid FEN!");
        let pos: Chess = fen
            .into_position(CastlingMode::Standard)
            .expect("Not a parseable FEN?!");
        let keyable2 = pos_to_keyable(&pos);
        let prefix = pos_to_prefix(&keyable2);

        assert_eq!(prefix, &key[..prefix.len()]);
    }
}
