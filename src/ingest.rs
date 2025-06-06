use crate::traversal;
use crate::visitor;
use pgn_reader::BufferedReader;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::thread;
use zstd::stream::read::Decoder;
use crate::config::IngestConfig;

pub fn ingest(cfg: IngestConfig) {
    let san_tree_shared =
        Arc::new(Mutex::new(visitor::SanTree::new(
            &cfg.db_path,
            cfg.threshold_writes,
        )));
    let mut handles = vec![];

    for pgn_path in cfg.pgn_files {
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
        let mut visitor = visitor::MyVisitor::new(
            Arc::clone(&san_tree_shared),
            cfg.min_rating,
            cfg.min_ply_count,
            cfg.required_words.clone(),
            cfg.forbidden_words.clone(),
        );
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
