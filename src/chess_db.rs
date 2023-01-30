use std::convert::TryInto;
use game_stats::{GameWins, GameStats};
use rocksdb::{DB, WriteBatch};
use shakmaty::{fen::Epd, EnPassantMode, Chess, Move};

const PS: &str = "position_stats";
const PMC: &str = "position_move_count";


fn pos_to_fen(
    pos: Chess
) -> String {
    Epd::from_position(pos, EnPassantMode::Legal).to_string()
}

pub fn get_pos_stats(
    db: &DB,
    pos: &Chess,
) -> Option<GameStats> {
    Some(GameStats::new())
}

pub fn get_pos_wins(
    db: &DB,
    pos: &Chess,
) -> Option<GameWins> {
    let fen = pos_to_fen(pos.clone());
    if let Ok(Some(bytes)) = db.get(PS.to_owned() + &fen) {
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
    let fen = pos_to_fen(pos.clone());
    if let Some(db_stats) = get_pos_wins(db, &pos) {
        batch.put(PS.to_owned() + &fen, db_stats.combine(&game_stats).to_bytes())
    } else {
        batch.put(PS.to_owned() + &fen, game_stats.to_bytes())
    }
}

pub fn get_pos_move(
    db: &DB,
    pos: &Chess,
    chess_move: Move,
) -> Option<u32> {
    let fen = pos_to_fen(pos.clone());
    if let Ok(Some(bytes)) = db.get(PMC.to_owned() + &fen + &chess_move.to_string()) {
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
    let fen = pos_to_fen(pos.clone());
    if let Some(db_count) = get_pos_move(db, &pos, chess_move.clone()) {
        batch.put(PMC.to_owned() + &fen + &chess_move.to_string(), (db_count + count).to_be_bytes());
    } else {
        batch.put(PMC.to_owned() + &fen + &chess_move.to_string(), count.to_be_bytes());
    }
}

