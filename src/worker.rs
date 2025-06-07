use crate::{chess_db, game_stats::GameWins, GameSummary};
use crossbeam_channel::Receiver;
use rocksdb::{WriteBatch, WriteOptions, DB};
use shakmaty::{Chess, Color, Move, Position};
use std::collections::HashMap;
use std::sync::Arc;

/// Per‑thread aggregation map.
pub struct StatsCache {
    map: HashMap<Vec<u8>, GameWins>,
    flush_threshold: usize,
}

impl StatsCache {
    #[must_use] pub fn new(flush_threshold: usize) -> Self { Self { map: HashMap::new(), flush_threshold } }

    #[inline] fn bump(&mut self, key: Vec<u8>, wins: &GameWins) { let e = self.map.entry(key).or_default(); *e = e.combine(wins); }
    #[inline] fn should_flush(&self) -> bool { self.map.len() >= self.flush_threshold }

    pub fn flush_to_db(&mut self, db: &DB) {
        if self.map.is_empty() { return; }
        let mut batch = WriteBatch::default();
        for (k, v) in self.map.drain() { batch.merge(&k, v.to_bytes()); }
        let mut opts = WriteOptions::default();
        opts.disable_wal(true);
        db.write_opt(batch, &opts).expect("rocksdb write failed");
    }
}

/// Entry point: called from `ingest` for each Rayon worker thread.
pub fn run(rx: &Receiver<GameSummary>, db: &Arc<DB>, flush_threshold: usize) {
    let mut cache = StatsCache::new(flush_threshold);
    while let Ok(game) = rx.recv() {
        process_game(&game, &mut cache);
        if cache.should_flush() { cache.flush_to_db(db); }
    }
    cache.flush_to_db(db); // final flush
}

fn process_game(game: &GameSummary, cache: &mut StatsCache) {
    let mut board = Chess::new();
    let wins = winner_to_wins(game.winner);
    for san_plus in &game.sans {
        accumulate_position(&board, &wins, cache);
        let Ok(mv) = san_plus.san.to_move(&board) else { return };
        accumulate_position_move(&board, &mv, &wins, cache);
        board.play_unchecked(&mv);
    }
    accumulate_position(&board, &wins, cache); // final position
}

#[inline]
fn accumulate_position(pos: &Chess, wins: &GameWins, cache: &mut StatsCache) {
    let keyable = chess_db::pos_to_keyable(pos);
    cache.bump(chess_db::pos_to_key(&keyable), wins);
}

#[inline]
fn accumulate_position_move(pos: &Chess, mv: &Move, wins: &GameWins, cache: &mut StatsCache) {
    let keyable = chess_db::pos_to_keyable(pos);
    cache.bump(chess_db::pos_move_to_key(&keyable, mv), wins);
}

#[inline]
fn winner_to_wins(winner: Option<Color>) -> GameWins {
    match winner {
        Some(Color::White) => GameWins { white: 1, ..Default::default() },
        Some(Color::Black) => GameWins { black: 1, ..Default::default() },
        None               => GameWins { draws:  1, ..Default::default() },
    }
}
