extern crate chess;
extern crate pgn_reader;
extern crate rocksdb;
extern crate shakmaty;
extern crate sysinfo;
extern crate zstd;

use pgn_reader::{BufferedReader, Skip, Visitor, SanPlus};
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
            println!("Failed to open file: {}", err);
            std::process::exit(1);
        }
    };

    let reader = BufReader::new(file);
    for line in reader.lines() {
        let compressed_pgn_file = line.unwrap();
        println!("Processing file: {}", compressed_pgn_file);
        let file = match File::open(&compressed_pgn_file) {
            Ok(file) => file,
            Err(err) => {
                println!("Failed to open file: {}", err);
                continue;
            }
        };
        let decoder = Decoder::new(file).unwrap();
        let mut buffered = BufferedReader::new(decoder);
        let mut visitor = MyVisitor::new();
        if let Err(err) = buffered.read_all(&mut visitor) {
            println!("Failed to read games: {}", err);
            std::process::exit(1);
        }
        println!("Count: {}", visitor.game_count)
    }

}

struct MyVisitor {
    game_count: u64,
}

impl MyVisitor {
    fn new() -> MyVisitor {
        MyVisitor {
            game_count: 0,
        }
    }
}

impl Visitor for MyVisitor {
    type Result = ();

    fn begin_game(&mut self) {
    }

    fn san(&mut self, _san_plus: SanPlus) {
        self.game_count += 1;
    }

    fn begin_variation(&mut self) -> Skip {
        Skip(true) // stay in the mainline
    }

    fn end_game(&mut self) -> Self::Result {
        // self.game_count += 1;
    }}

