extern crate chess;
extern crate pgn_reader;
extern crate radix_trie;
extern crate rocksdb;
extern crate shakmaty;
extern crate sysinfo;
extern crate zstd;

use pgn_reader::{BufferedReader, SanPlus, Skip, Visitor, Outcome, Color};
use radix_trie::{Trie, TrieCommon};
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
        let mut visitor = MyVisitor::new(
            // &db,
            // &mut batch,
        );
        if let Err(err) = buffered.read_all(&mut visitor) {
            panic!("Failed to read games: {:?}", err);
        }
        println!("{} lines!", visitor.san_tree.len());
        // for (k, v) in visitor.san_tree.iter().take(10) {
        //     println!("Key of: {k}");
        //     println!("Val of: {v}");
        // }
        // println!("Na3 as bytes: {:?}", " Na3 d5 c3".as_bytes());
        // println!("Attempt: {}", String::from_utf8(vec![78, 96, 19, 50, 6]).unwrap());
        // println!("{}", String::from_utf8(vec![78, 97, 51]).unwrap());
        // for (i, child) in visitor.san_tree.children().enumerate() {
        //     println!("Child 1: {i}");
        //     let prefix1 = child.prefix();
        //     let p1s = prefix1.clone().as_bytes().to_vec();
        //     println!("Prefix 1: {}", String::from_utf8(p1s).unwrap());
        //     for (i, child) in child.children().enumerate() {
        //         println!("\tChild 2: {i}");
        //         let prefix2 = prefix1.clone().join(child.prefix());
        //         let p2s = prefix2.as_bytes().to_vec();
        //         println!("\tPrefix 2: {}", String::from_utf8(p2s).unwrap());
        //         for (i, child) in child.children().enumerate() {
        //             println!("\t\tChild 3: {i}");
        //             match child.value() {
        //                 Some(count) => println!("\t\t Count of: {count}"),
        //                 None => println!("No count yet..."),
        //             }
        //             let prefix3 = prefix2.clone().join(child.prefix());
        //             let p3s = prefix3.as_bytes().to_vec();
        //             println!("\t\tPrefix 3 Vec: {:?}", p3s.clone());
        //             println!("\t\tPrefix 3: {}", String::from_utf8(p3s).unwrap());
        //         }
        //     }
        // }
    }
}

struct GameStats {
    black: u32,
    white: u32,
    draw:  u32,
}

struct MyVisitor { // 'a lifetime
    // db: &'a DB,
    // batch: &'a mut WriteBatch,
    san_tree: Trie<String, GameStats>,
    san_string: String,
    winner: Option<Color>,
}

impl MyVisitor { // this one had _ lifetime
    fn new<'a>(
        // db: &'a DB,
        // batch: &'a mut WriteBatch,
    ) -> MyVisitor { // this one had a lifetime
        MyVisitor {
            // db,
            // batch,
            san_tree: Trie::default(),
            san_string: String::new(),
            winner: None,
        }
    }
}

impl Visitor for MyVisitor { // '_ lifetime
    type Result = ();

    fn begin_game(&mut self) -> Self::Result {
    }

    fn outcome(&mut self, outcome: Option<Outcome>) {
        if let Some(Outcome::Decisive { winner: color }) = outcome {
            self.winner = Some(color);
        } else {
            self.winner = None;
        }
    }

    fn san(&mut self, san_plus: SanPlus) {
        self.san_string.push_str(&format!(" {}", san_plus.san));
        // self.batch.put([1], [1]);
        // if self.batch.size_in_bytes() > 200 * 1024 * 1024 {
        //     self.db.write(std::mem::take(self.batch)).ok().unwrap();
        // }
    }

    fn begin_variation(&mut self) -> Skip {
        Skip(true) // stay in the mainline
    }

    fn end_game(&mut self) -> Self::Result {
        let s = std::mem::take(&mut self.san_string);
        match self.winner {
            Some(Color::White) => self.san_tree.map_with_default(
                s,
                |x| x.white += 1,
                GameStats { black: 0, white: 1, draw: 0}
            ),
            Some(Color::Black) => self.san_tree.map_with_default(
                s,
                |x| x.black += 1,
                GameStats { black: 1, white: 0, draw: 0}
            ),
            None => self.san_tree.map_with_default(
                s,
                |x| x.draw += 1,
                GameStats { black: 0, white: 0, draw: 1}
            ),
        }
        // self.san_tree.map_with_default(s, |x| *x += 1, 1)
    }
}
