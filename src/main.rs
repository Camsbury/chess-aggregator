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
pub mod chess_db;

use shakmaty::{Chess, san::San};

use pgn_reader::BufferedReader;
use rocksdb::{Options, DB};
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

    let db_path = &args[1];
    let mut db_opts = Options::default();
    db_opts.create_if_missing(true);
    let db = DB::open(&db_opts, db_path).unwrap();

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
        let mut visitor = visitor::MyVisitor::new(&db);
        if let Err(err) = buffered.read_all(&mut visitor) {
            panic!("Failed to read games: {:?}", err);
        }
        traversal::extract_stats(
            &db,
            &mut visitor.san_tree,
        );
        let starting_pos = Chess::new();
        if let Some(stats) = chess_db::get_pos_wins(
            &db,
            &starting_pos,
        ) {
            dbg!("Starting stats: {}", &stats);
        }
        if let Some(count) = chess_db::get_pos_move(
            &db,
            &starting_pos,
            San::from_ascii("e4".as_bytes()).unwrap().to_move(&starting_pos).unwrap(),
        ) {
            println!("e4 played {count} times!");
        }
    }
}

