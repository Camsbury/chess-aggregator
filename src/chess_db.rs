use std::collections::HashMap;
use std::convert::TryInto;
use game_stats::{GameWins, GameStats};
use rocksdb::{DB, WriteBatch};
use shakmaty::{uci::Uci, fen::Epd, CastlingMode, EnPassantMode, Chess, Move};

const PS: &str = "position_stats";
const PMC: &str = "position_move_count";


fn pos_to_fen(
    pos: Chess
) -> String {
    Epd::from_position(pos, EnPassantMode::Legal).to_string()
}

fn pos_to_key(
    pos: Chess,
) -> String {
    let fen = pos_to_fen(pos);
    PS.to_owned() + &fen
}

fn pos_to_prefix(pos: Chess) -> String {
    let fen = pos_to_fen(pos);
    PMC.to_owned() + &fen
}

fn pos_move_to_key(
    pos: Chess,
    chess_move: Move,
) -> String {
    (pos_to_prefix(pos)) + &chess_move.to_uci(CastlingMode::Standard).to_string()
}

fn key_to_uci(
    key: Vec<u8>,
    prefix: &str,
) -> Uci {
    let key_string = String::from_utf8(key).expect("Key isn't decoding to UTF-8 correctly");
    let move_string: String = key_string.chars().into_iter().skip(prefix.chars().count()).collect();
    Uci::from_ascii(move_string.as_bytes()).expect("Failed to parse UCI from key")
}

fn is_valid_prefix(
    key: &[u8],
    prefix: &str,
) -> bool {
    let key_string = String::from_utf8(key.to_owned()).expect("Key isn't decoding to UTF-8 correctly");
    key_string.starts_with(prefix)
}

pub fn get_pos_stats(
    db: &DB,
    pos: &Chess,
) -> Option<GameStats> {
    let prefix = pos_to_prefix(pos.clone());
    let prefix_clone = prefix.clone();
    let prefix_iter = db.prefix_iterator(prefix);
    let mut game_moves = HashMap::new();
    for item in prefix_iter {
        let (key, value) = item.expect("Prefix iter error in rocks db?");
        let key_clone = key.clone().into_vec();
        // NOTE: stopping iter on mismatched prefix, not sure how to bound it otherwise
        if !is_valid_prefix(&key_clone, &prefix_clone) {
            break;
        }
        let m = key_to_uci(
            key_clone,
            &prefix_clone,
        ).to_move(pos).expect("The move is invalid uci for the position!");
        let count = u32::from_be_bytes(value[..4].try_into().expect("The count is not encoded correctly for this move!!"));
        game_moves.insert(m, count);
    }

    get_pos_wins(db, pos).map(
        |game_wins| GameStats {
            game_wins,
            game_moves,
        }
    )
}

pub fn get_pos_wins(
    db: &DB,
    pos: &Chess,
) -> Option<GameWins> {
    if let Ok(Some(bytes)) = db.get(pos_to_key(pos.clone())) {
        Some(GameWins::from_bytes(bytes))
    } else {
        None
    }
}

pub fn update_pos_wins(
    db: &DB,
    batch: &mut WriteBatch,
    pos: Chess,
    game_stats: GameWins,
) {
    let key = pos_to_key(pos.clone());
    if let Some(db_stats) = get_pos_wins(db, &pos) {
        batch.put(key, db_stats.combine(&game_stats).to_bytes())
    } else {
        batch.put(key, game_stats.to_bytes())
    }
}

pub fn get_pos_move(
    db: &DB,
    pos: &Chess,
    chess_move: Move,
) -> Option<u32> {
    let key = pos_move_to_key(pos.clone(), chess_move);
    if let Ok(Some(bytes)) = db.get(key) {
        // TODO: much unsafety - means erroneous DB data
        Some(u32::from_be_bytes(bytes[..4].try_into().unwrap()))
    } else {
        None
    }

}

pub fn update_pos_move(
    db: &DB,
    batch: &mut WriteBatch,
    pos: Chess,
    chess_move: Move, // TODO: check that to_string is reasonable
    count: u32,
) {
    let key = pos_move_to_key(pos.clone(), chess_move.clone());
    if let Some(db_count) = get_pos_move(db, &pos, chess_move) {
        batch.put(key, (db_count + count).to_be_bytes());
    } else {
        batch.put(key, count.to_be_bytes());
    }
}

