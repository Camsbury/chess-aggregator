use crate::game_stats::GameWins;
use crate::traversal;
use btoi;
use pgn_reader::{Color, Outcome, RawHeader, SanPlus, Skip, Visitor};
use radix_trie::Trie;
use rocksdb::{Options, DB};
use std::sync::{Arc, Mutex};

const MIN_RATING: u32 = 1800;
const MIN_PLY_COUNT: u32 = 7;
const THRESHOLD_WRITES: u32 = 2_000_000;

pub struct SanTree {
    pub db: DB,
    pub tree: Trie<String, GameWins>,
    pub write_count: u32,
}

impl SanTree {
    pub fn new(db_path: &str) -> SanTree {
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        let db = DB::open(&db_opts, db_path).unwrap();

        SanTree {
            db,
            tree: Trie::default(),
            write_count: 0,
        }
    }

    pub fn inc_writes(&mut self) {
        self.write_count += 1;
        if self.write_count > THRESHOLD_WRITES {
            self.write_count = 0;
            println!("Trie threshold reached: extracting stats");
            traversal::extract_stats(self);
        }
    }
}

pub struct MyVisitor {
    pub san_tree: Arc<Mutex<SanTree>>,
    pub san_string: String,
    pub winner: Option<Color>,
    pub skip_game: bool,
    pub ply_count: u32,
}

impl MyVisitor {
    pub fn new(san_tree: Arc<Mutex<SanTree>>) -> MyVisitor {
        MyVisitor {
            san_tree,
            san_string: String::new(),
            winner: None,
            skip_game: false,
            ply_count: 0,
        }
    }
}

impl Visitor for MyVisitor {
    // '_ lifetime
    type Result = ();

    fn begin_headers(&mut self) {
        self.skip_game = false;
    }

    fn header(&mut self, key: &[u8], value: RawHeader) {
        if b"WhiteElo" == key || b"BlackElo" == key {
            if value.as_bytes() == b"?" {
                self.skip_game = true;
            } else {
                match btoi::btoi::<u32>(value.as_bytes()) {
                    Ok(rating) => {
                        if rating < MIN_RATING {
                            self.skip_game = true;
                        }
                    }
                    _ => self.skip_game = true,
                }
            }
        }
    }

    fn end_headers(&mut self) -> Skip {
        Skip(self.skip_game)
    }

    fn begin_game(&mut self) -> Self::Result {
        self.ply_count = 0;
        self.san_string = "".to_string();
    }

    fn outcome(&mut self, outcome: Option<Outcome>) {
        if let Some(Outcome::Decisive { winner: color }) = outcome {
            self.winner = Some(color);
        } else {
            self.winner = None;
        }
    }

    fn san(&mut self, san_plus: SanPlus) {
        self.ply_count += 1;
        self.san_string.push_str(&format!(" {}", san_plus.san));
    }

    fn begin_variation(&mut self) -> Skip {
        Skip(true) // stay in the mainline
    }

    fn end_game(&mut self) -> Self::Result {
        let s = std::mem::take(&mut self.san_string);
        let mut san_tree = self.san_tree.lock().unwrap();
        if self.ply_count > MIN_PLY_COUNT {
            match self.winner {
                Some(Color::White) => san_tree.tree.map_with_default(
                    s,
                    |x| x.white += 1,
                    GameWins {
                        black: 0,
                        white: 1,
                        draw: 0,
                    },
                ),
                Some(Color::Black) => san_tree.tree.map_with_default(
                    s,
                    |x| x.black += 1,
                    GameWins {
                        black: 1,
                        white: 0,
                        draw: 0,
                    },
                ),
                None => san_tree.tree.map_with_default(
                    s,
                    |x| x.draw += 1,
                    GameWins {
                        black: 0,
                        white: 0,
                        draw: 1,
                    },
                ),
            }
            san_tree.inc_writes();
        }
    }
}
