extern crate chess;
extern crate pgn_reader;
extern crate rocksdb;
extern crate sysinfo;
extern crate zstd;

use chess::{Board, ChessMove, Game, Piece, Rank, Square};
use pgn_reader::{BufferedReader, Color, Outcome, SanPlus, Skip, Visitor};
use rocksdb::{Options, WriteBatch, DB};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::str::FromStr;
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
    let mut batch = WriteBatch::default();

    let mut sys = System::new_all();

    // {
    //     let position = Board::from_str(
    //         "2rr2k1/5pp1/p3p2p/P7/RQ1q4/1P4P1/2p2P1P/1N2R1K1 b - - 0 1",
    //     )
    //     .ok()
    //     .unwrap();
    //     match ChessMove::from_san(&position, "c1Q") {
    //         Ok(chess_move) => println!("UCI of c1Q: {}", chess_move),
    //         Err(err) => {
    //             println!("Error: {:?}", err);
    //             std::process::exit(1);
    //         }
    //     }
    // }

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
        let mut visitor = MyVisitor::new(&db, &mut batch, &mut sys);
        if let Err(err) = buffered.read_all(&mut visitor) {
            println!("Failed to read games: {}", err);
            std::process::exit(1);
        }
    }
    if let Err(err) = db.write(batch) {
        println!("Failed to write batch: {}", err);
        std::process::exit(1);
    }

    ////////////////////////////////////////////////////////////////////////////
    // NOTE: Testing that the db is actually being written to
    let start_pos = Board::from_str(
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    )
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
    board: Board,
    outcome: Option<Outcome>,
    db: &'a DB,
    batch: &'a mut WriteBatch,
    sys: &'a mut System,
    game_count: u64,
    move_count: u64,
}

impl MyVisitor<'_> {
    fn new<'a>(
        db: &'a DB,
        batch: &'a mut WriteBatch,
        sys: &'a mut System,
    ) -> MyVisitor<'a> {
        MyVisitor {
            board: Board::default(),
            outcome: None,
            db,
            batch,
            sys,
            game_count: 0,
            move_count: 0,
        }
    }
}

impl Visitor for MyVisitor<'_> {
    type Result = ();

    fn begin_game(&mut self) {
        self.board = Board::default();
        self.outcome = None;
        self.game_count += 1;
        self.move_count = 0;
    }

    fn outcome(&mut self, outcome: Option<Outcome>) {
        self.outcome = outcome;
    }

    fn san(&mut self, san_plus: SanPlus) {
        self.move_count += 1;
        match ChessMove::from_san(
            &self.board,
            &san_plus.san.to_string().replace('=', ""),
        ) {
            Ok(chess_move) => {
                write_pos_info(
                    self.db,
                    self.batch,
                    self.sys,
                    self.board,
                    self.outcome,
                    Some(chess_move),
                );
                let mut result = Board::default();
                self.board.make_move(chess_move, &mut result);
                if let Some(square) = result.en_passant() {
                    result.en_passant = fix_en_passant(square)
                }
                self.board = result
            }
            Err(err) => {
                println!(
                    "Error: {:?}
                     Position: {}
                     SAN: {}
                     Game Count: {}
                     Move Count: {}",
                    err,
                    self.board,
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
            self.batch,
            self.sys,
            self.board,
            self.outcome,
            None,
        );
    }
}

fn write_pos_info(
    db: &DB,
    batch: &mut WriteBatch,
    sys: &mut System,
    position: Board,
    outcome: Option<Outcome>,
    chess_move: Option<ChessMove>,
) {
    let pc_key = create_pc_key(&position);
    increment_key(db, batch, sys, pc_key.as_slice());

    if let Some(Outcome::Decisive { winner: color }) = outcome {
        match color {
            Color::White => {
                let pwc_key = create_pwc_key(&position);
                increment_key(db, batch, sys, pwc_key.as_slice());
            }
            Color::Black => {
                let pbc_key = create_pbc_key(&position);
                increment_key(db, batch, sys, pbc_key.as_slice());
            }
        }
    }
    if let Some(m) = chess_move {
        let puc_key = create_puc_key(&position, &m);
        increment_key(db, batch, sys, puc_key.as_slice());
    }
}

fn create_pc_key(position: &Board) -> Vec<u8> {
    let mut pc_vec = "pos_count".as_bytes().to_vec();
    pc_vec.extend_from_slice(position.get_hash().to_be_bytes().as_slice());
    pc_vec
}

fn create_pwc_key(position: &Board) -> Vec<u8> {
    let mut pwc_vec = "pos_white_count".as_bytes().to_vec();
    pwc_vec.extend_from_slice(position.get_hash().to_be_bytes().as_slice());
    pwc_vec
}

fn create_pbc_key(position: &Board) -> Vec<u8> {
    let mut pbc_vec = "pos_black_count".as_bytes().to_vec();
    pbc_vec.extend_from_slice(position.get_hash().to_be_bytes().as_slice());
    pbc_vec
}

fn create_puc_key(position: &Board, chess_move: &ChessMove) -> Vec<u8> {
    let mut puc_vec = "pos_uci_count".as_bytes().to_vec();
    puc_vec.extend_from_slice(position.get_hash().to_be_bytes().as_slice());
    puc_vec.extend_from_slice(chess_move.to_string().as_bytes());
    puc_vec
}

fn increment_key(
    db: &DB,
    batch: &mut WriteBatch,
    sys: &mut System,
    key: &[u8],
) {
    match db.get_pinned(key) {
        Ok(maybe_value) => match maybe_value {
            Some(value) => {
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(value.as_ref());
                let count = u64::from_be_bytes(bytes);
                batch.put(key, (count + 1).to_be_bytes());
            }
            None => {
                batch.put(key, u64::to_be_bytes(1));
            }
        },
        Err(err) => println!("Error: {:?}", err),
    }
    sys.refresh_memory(); // not sure if needed
    if sys.available_memory() < 5000000000 {
        if let Err(err) = db.write(std::mem::take(batch)) {
            println!("Failed to write batch: {}", err);
            std::process::exit(1);
        }
    }
}

fn fetch_pos_count(db: &DB, pos: &Board) -> u64 {
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

fn fix_en_passant(square: Square) -> Square {
    match square.get_rank() {
        Rank::Fifth => Square::make_square(Rank::Sixth, square.get_file()),
        Rank::Fourth => Square::make_square(Rank::Third, square.get_file()),
        _ => {
            println!("Failed to fix en passant for: {}", square);
            std::process::exit(1);
        }
    }
}
