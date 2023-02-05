use crate::traversal;
use crate::visitor;
use pgn_reader::BufferedReader;
use rocksdb::{Options, DB};
use std::fs::File;
use std::io::{BufRead, BufReader};
use zstd::stream::read::Decoder;

pub fn ingest(filename: &str, db_path: &str) {
    let file = match File::open(filename) {
        Ok(file) => file,
        Err(err) => {
            panic!("Failed to open file listing .pgn.zst files: {:?}", err);
        }
    };

    let mut db_opts = Options::default();
    db_opts.create_if_missing(true);
    let db = DB::open(&db_opts, db_path).unwrap();

    let reader = BufReader::new(file);
    let mut visitor = visitor::MyVisitor::new(&db);
    for line in reader.lines() {
        let pgn_path = line.expect("Line didn't parse?!");
        if pgn_path.is_empty() {
            continue;
        }
        println!("Processing file: {}", pgn_path);
        let file = match File::open(&pgn_path) {
            Ok(file) => file,
            Err(err) => {
                println!("Failed to open .pgn.zst file: {:?}", err);
                continue;
            }
        };
        let decoder = Decoder::new(file).unwrap();
        let mut buffered = BufferedReader::new(decoder);
        if let Err(err) = buffered.read_all(&mut visitor) {
            panic!("Failed to read games: {:?}", err);
        }
    }
    println!("Built trie from games");
    println!("Write count: {}", visitor.write_count);
    traversal::extract_stats(&db, &mut visitor.san_tree);
}
