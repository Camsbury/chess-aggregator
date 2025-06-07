extern crate actix_web;
extern crate btoi;
extern crate nibble_vec;
extern crate pgn_reader;
extern crate radix_trie;
extern crate rocksdb;
extern crate serde;
extern crate shakmaty;
extern crate sysinfo;
extern crate zstd;

pub mod config;
pub mod chess_db;
pub mod game_stats;
pub mod ingest;
pub mod server;
pub mod extractor;
pub mod worker;
pub mod merge;
pub mod file;

use shakmaty::{Color, san::SanPlus};


/// Chess game data to ingest
#[derive(Debug)]
pub struct GameSummary {
    pub winner: Option<Color>,
    pub sans: Vec<SanPlus>,
}
