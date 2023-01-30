extern crate btoi;
extern crate nibble_vec;
extern crate pgn_reader;
extern crate radix_trie;
extern crate rocksdb;
extern crate shakmaty;
extern crate sysinfo;
extern crate zstd;

pub mod visitor;
pub mod traversal;
pub mod game_stats;

use pgn_reader::BufferedReader;
use shakmaty::{EnPassantMode, fen::Fen};
// use rocksdb::{Options, DB, WriteBatch};
use std::fs::File;
use std::io::{BufRead, BufReader};
use zstd::stream::read::Decoder;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Usage: {} <db_path> <file_paths_file>", args[0]);
        std::process::exit(1);
    }
    let filename = &args[2];
    let file = match File::open(filename) {
        Ok(file) => file,
        Err(err) => {
            panic!("Failed to open file listing .pgn.zst files: {:?}", err);
        }
    };

    // let db_path = &args[1];
    // let mut db_opts = Options::default();
    // db_opts.create_if_missing(true);
    // let db = DB::open(&db_opts, db_path).unwrap();

    let reader = BufReader::new(file);
    for line in reader.lines() {
        let compressed_pgn_file = line.unwrap();
        println!("Processing file: {}", compressed_pgn_file);
        let file = match File::open(&compressed_pgn_file) {
            Ok(file) => file,
            Err(err) => {
                println!("Failed to open .pgn.zst file: {:?}", err);
                continue;
            }
        };
        let decoder = Decoder::new(file).unwrap();
        let mut buffered = BufferedReader::new(decoder);
        // let mut batch = WriteBatch::default();
        let mut visitor = visitor::MyVisitor::new(
            // &db,
            // &mut batch,
        );
        if let Err(err) = buffered.read_all(&mut visitor) {
            panic!("Failed to read games: {:?}", err);
        }
        let pos_stats = traversal::extract_stats(visitor.san_tree);
        // for (k, v) in pos_stats.iter().take(5) {
        //     println!("Key: {}", Fen::from_position(k.clone(), EnPassantMode::Always));
        //     dbg!("Val: {}", v);
        // }
    }
}

