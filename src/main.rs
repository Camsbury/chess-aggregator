extern crate chess;
extern crate pgn_reader;
extern crate rocksdb;
extern crate shakmaty;
extern crate sysinfo;
extern crate zstd;

use pgn_reader::{BufferedReader, Color, Outcome, SanPlus, Skip, Visitor};
use rocksdb::{Options, WriteBatch, DB};
use shakmaty::{fen::Fen, uci::Uci, CastlingMode, Chess, Move, Position};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::collections::HashMap;
use sysinfo::{System, SystemExt};
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
    let mut cache: HashMap<Vec<u8>, u64> = HashMap::new();

    let mut sys = System::new_all();

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
        let mut visitor = MyVisitor::new(&db, &mut cache, &mut sys);
        if let Err(err) = buffered.read_all(&mut visitor) {
            println!("Failed to read games: {}", err);
            std::process::exit(1);
        }
    }
    write_cache(&db, &mut cache);

    ////////////////////////////////////////////////////////////////////////////
    // NOTE: Testing that the db is actually being written to
    let start_fen: Fen =
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
            .parse()
            .ok()
            .unwrap();
    let start_pos: Chess = start_fen
        .into_position(CastlingMode::Standard)
        .ok()
        .unwrap();

    let start_pos_count = fetch_pos_count(&db, &start_pos);

    println!(
        "The starting position has been seen {} times!",
        start_pos_count
    );
    ////////////////////////////////////////////////////////////////////////////

    // NOTE: Destroy the DB for now to keep it fresh every time
    DB::destroy(&db_opts, db_path).ok();
}

struct MyVisitor<'a> {
    board: Chess,
    outcome: Option<Outcome>,
    db: &'a DB,
    cache: &'a mut HashMap<Vec<u8>, u64>,
    sys: &'a mut System,
    game_count: u64,
    move_count: u64,
}

impl MyVisitor<'_> {
    fn new<'a>(
        db: &'a DB,
        cache: &'a mut HashMap<Vec<u8>, u64>,
        sys: &'a mut System,
    ) -> MyVisitor<'a> {
        MyVisitor {
            board: Chess::default(),
            outcome: None,
            db,
            cache,
            sys,
            game_count: 0,
            move_count: 0,
        }
    }
}

impl Visitor for MyVisitor<'_> {
    type Result = ();

    fn begin_game(&mut self) {
        self.board = Chess::default();
        self.outcome = None;
        self.game_count += 1;
        self.move_count = 0;
    }

    fn outcome(&mut self, outcome: Option<Outcome>) {
        self.outcome = outcome;
    }

    fn san(&mut self, san_plus: SanPlus) {
        self.move_count += 1;
        match san_plus.san.to_move(&self.board) {
            Ok(chess_move) => {
                write_pos_info(
                    self.db,
                    self.cache,
                    self.sys,
                    &self.board,
                    self.outcome,
                    Some(&chess_move),
                );

                match std::mem::take(&mut self.board).play(&chess_move) {
                    Ok(new_board) => self.board = new_board,
                    Err(err) => panic!("{}", err),
                }
            }
            Err(err) => {
                println!(
                    "Error: {:?}
                     Position: {}
                     SAN: {}
                     Game Count: {}
                     Move Count: {}",
                    err,
                    self.board.board(),
                    &san_plus.san.to_string(),
                    self.game_count,
                    self.move_count,
                )
            }
        }
    }

    fn begin_variation(&mut self) -> Skip {
        Skip(true) // stay in the mainline
    }

    fn end_game(&mut self) -> Self::Result {
        write_pos_info(
            self.db,
            self.cache,
            self.sys,
            &self.board,
            self.outcome,
            None,
        );
    }
}

fn write_pos_info(
    db: &DB,
    cache: &mut HashMap<Vec<u8>, u64>,
    sys: &mut System,
    position: &Chess,
    outcome: Option<Outcome>,
    chess_move: Option<&Move>,
) {
    let pc_key = create_pc_key(position);
    increment_key(db, cache, sys, pc_key);

    if let Some(Outcome::Decisive { winner: color }) = outcome {
        match color {
            Color::White => {
                let pwc_key = create_pwc_key(position);
                increment_key(db, cache, sys, pwc_key);
            }
            Color::Black => {
                let pbc_key = create_pbc_key(position);
                increment_key(db, cache, sys, pbc_key);
            }
        }
    }
    if let Some(m) = chess_move {
        let puc_key = create_puc_key(position, m);
        increment_key(db, cache, sys, puc_key);
    }
}

fn create_pc_key(position: &Chess) -> Vec<u8> {
    let mut pc_vec = "pos_count".as_bytes().to_vec();
    pc_vec.extend_from_slice(position.board().to_string().as_bytes());
    pc_vec
}

fn create_pwc_key(position: &Chess) -> Vec<u8> {
    let mut pwc_vec = "pos_white_count".as_bytes().to_vec();
    pwc_vec.extend_from_slice(position.board().to_string().as_bytes());
    pwc_vec
}

fn create_pbc_key(position: &Chess) -> Vec<u8> {
    let mut pbc_vec = "pos_black_count".as_bytes().to_vec();
    pbc_vec.extend_from_slice(position.board().to_string().as_bytes());
    pbc_vec
}

fn create_puc_key(position: &Chess, chess_move: &Move) -> Vec<u8> {
    let mut puc_vec = "pos_uci_count".as_bytes().to_vec();
    puc_vec.extend_from_slice(position.board().to_string().as_bytes());
    puc_vec.extend_from_slice(
        Uci::from_standard(chess_move).to_string().as_bytes(),
    );
    puc_vec
}

fn increment_key(
    db: &DB,
    cache: &mut HashMap<Vec<u8>, u64>,
    sys: &mut System,
    key: Vec<u8>,
) {
        match db.get_pinned(&key) {
            Ok(maybe_value) => match maybe_value {
                Some(value) => {
                    let mut bytes = [0u8; 8];
                    bytes.copy_from_slice(value.as_ref());
                    let count = u64::from_be_bytes(bytes);

                    if let Err(err) = db.put(
                        key.as_slice(),
                        (count + 1).to_be_bytes()
                    ) {
                        println!("Failed to write to db: {}", err);
                        std::process::exit(1);
                    }
                }
                None => {
                    if let Err(err) = db.put(
                        key.as_slice(),
                        u64::to_be_bytes(1)
                    ) {
                        println!("Failed to write to db: {}", err);
                        std::process::exit(1);
                    }
                }
            },
            Err(err) => println!("Error: {:?}", err),
        }
}

fn write_cache(db: &DB, cache: &mut HashMap<Vec<u8>, u64>) {
        let mut batch = WriteBatch::default();

        // NOTE: my attempt at freeing memory as I iterate?
        cache.retain(|key, value| {
            batch.put(key.as_slice(),value.to_be_bytes());
            false
        });
        if let Err(err) = db.write(batch) {
            println!("Failed to write batch: {}", err);
            std::process::exit(1);
        }
}

fn fetch_pos_count(db: &DB, pos: &Chess) -> u64 {
    let key = create_pc_key(pos);
    let count: u64;
    match db.get_pinned(key.as_slice()) {
        Ok(maybe_value) => match maybe_value {
            Some(value) => {
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(value.as_ref());
                count = u64::from_be_bytes(bytes);
            }
            None => {
                count = 0;
            }
        },
        Err(err) => {
            println!("Failed to fetch pos count: {}", err);
            std::process::exit(1);
        }
    }
    count
}
