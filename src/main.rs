extern crate chess;
extern crate pgn_reader;
extern crate rocksdb;
extern crate shakmaty;
extern crate sysinfo;
extern crate zstd;

use pgn_reader::{BufferedReader, SanPlus, Skip, Visitor};
use rocksdb::{Options, DB, WriteBatch};
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
                println!("Failed to open file: {}", err);
                continue;
            }
        };
        let decoder = Decoder::new(file).unwrap();
        let mut buffered = BufferedReader::new(decoder);
        let mut batch = WriteBatch::default();
        let mut visitor = MyVisitor::new(&db, &mut batch);
        if let Err(err) = buffered.read_all(&mut visitor) {
            println!("Failed to read games: {}", err);
            std::process::exit(1);
        }
        println!("{} positions!", visitor.count);
        db.write(batch).ok().unwrap();
    }

}

struct MyVisitor<'a> {
    db: &'a DB,
    batch: &'a mut WriteBatch,
    count: u64,
}

impl MyVisitor<'_> {
    fn new<'a>(
        db: &'a DB,
        batch: &'a mut WriteBatch,
    ) -> MyVisitor<'a> {
        MyVisitor {
            db,
            batch,
            count: 0,
        }
    }
}

impl Visitor for MyVisitor<'_> {
    type Result = ();

    fn san(&mut self, _san_plus: SanPlus) {
        self.count += 1;
        self.batch.put([1], [1]);
        if self.batch.size_in_bytes() > 200 * 1024 * 1024 {
            self.db.write(std::mem::take(self.batch)).ok().unwrap();
        }
    }

    fn begin_variation(&mut self) -> Skip {
        Skip(true) // stay in the mainline
    }

    fn end_game(&mut self) -> Self::Result {
    }
}
