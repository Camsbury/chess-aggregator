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

pub mod chess_db;
pub mod game_stats;
pub mod ingest;
pub mod server;
pub mod traversal;
pub mod visitor;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        panic!("Usage: {} ingest or {} serve", args[0], args[0]);
    }

    if args[1] == "ingest" {
        if args.len() != 4 {
            panic!("Usage: {} ingest <db_path> <file_paths_file>", args[0]);
        }
        let db_path = &args[2];
        let filename = &args[3];
        ingest::ingest(filename, db_path);
    } else if args[1] == "serve" {
        if args.len() != 3 {
            panic!("Usage: {} serve <db_path>", args[0]);
        }
        let db_path = args[2].to_string();
        server::serve(db_path).unwrap();
    } else {
        panic!("Usage: {} ingest or {} serve", args[0], args[0]);
    }
}
