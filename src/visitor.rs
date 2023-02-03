use crate::game_stats::GameWins;
use crate::traversal;
use btoi;
use pgn_reader::{Color, Outcome, RawHeader, SanPlus, Skip, Visitor};
use radix_trie::Trie;
use rocksdb::DB;
use sysinfo::{System, SystemExt};

const MIN_RATING: u32 = 1800;
const MIN_PLY_COUNT: u32 = 7;
const MIN_CLEANUP_MEMORY: u64 = 5 * 1024 * 1024 * 1024;

pub struct MyVisitor<'a> {
    db: &'a DB,
    pub san_tree: Trie<String, GameWins>,
    pub san_string: String,
    pub winner: Option<Color>,
    pub sys: System,
    pub skip_game: bool,
    pub ply_count: u32,
}

impl MyVisitor<'_> {
    pub fn new(db: &DB) -> MyVisitor {
        MyVisitor {
            db,
            san_tree: Trie::default(),
            san_string: String::new(),
            winner: None,
            sys: System::new_all(),
            skip_game: false,
            ply_count: 0,
        }
    }
}

impl Visitor for MyVisitor<'_> {
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
        if self.ply_count > MIN_PLY_COUNT {
            match self.winner {
                Some(Color::White) => self.san_tree.map_with_default(
                    s,
                    |x| x.white += 1,
                    GameWins {
                        black: 0,
                        white: 1,
                        draw: 0,
                    },
                ),
                Some(Color::Black) => self.san_tree.map_with_default(
                    s,
                    |x| x.black += 1,
                    GameWins {
                        black: 1,
                        white: 0,
                        draw: 0,
                    },
                ),
                None => self.san_tree.map_with_default(
                    s,
                    |x| x.draw += 1,
                    GameWins {
                        black: 0,
                        white: 0,
                        draw: 1,
                    },
                ),
            }
            self.sys.refresh_memory();
            if self.sys.available_memory() < MIN_CLEANUP_MEMORY {
                traversal::extract_stats(self.db, &mut self.san_tree);
            }
        }
    }
}
