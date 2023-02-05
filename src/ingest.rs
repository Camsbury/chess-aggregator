use crate::traversal;
use crate::visitor;
use pgn_reader::BufferedReader;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use std::thread;
use zstd::stream::read::Decoder;

pub fn ingest(filename: &str, db_path: &str) {
    let san_tree_shared = Arc::new(Mutex::new(visitor::SanTree::new(db_path)));
    let mut handles = vec![];

    let file = match File::open(filename) {
        Ok(file) => file,
        Err(err) => {
            panic!("Failed to open file listing .pgn.zst files: {:?}", err);
        }
    };
    let reader = BufReader::new(file);
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
        let mut visitor = visitor::MyVisitor::new(Arc::clone(&san_tree_shared));
        let handle = thread::spawn(move || {
            if let Err(err) = buffered.read_all(&mut visitor) {
                panic!("Failed to read games: {:?}", err);
            }
        });
        handles.push(handle);
    }
    for handle in handles {
        handle.join().unwrap();
    }

    let mut san_tree = san_tree_shared.lock().unwrap();
    println!("Built trie from games");
    println!("Write count: {}", san_tree.write_count);
    traversal::extract_stats(&mut san_tree);
}
