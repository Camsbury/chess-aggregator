use btoi;
use pgn_reader::{
    Color,
    Outcome,
    RawHeader,
    SanPlus,
    Skip,
    Visitor,
};
use radix_trie::{Trie};
use sysinfo::{System, SystemExt};
use crate::GameStats;

const MIN_RATING: u32 = 2000;
const MIN_PLY_COUNT: u32 = 7;

pub struct MyVisitor { // 'a lifetime
    // db: &'a DB,
    // batch: &'a mut WriteBatch,
    pub san_tree: Trie<String, GameStats>,
    pub san_string: String,
    pub winner: Option<Color>,
    pub sys: System,
    pub skip_game: bool,
    pub ply_count: u32,
}

impl MyVisitor { // this one had _ lifetime
    pub fn new<'a>(
        // db: &'a DB,
        // batch: &'a mut WriteBatch,
    ) -> MyVisitor { // this one had a lifetime
        MyVisitor {
            // db,
            // batch,
            san_tree: Trie::default(),
            san_string: String::new(),
            winner: None,
            sys: System::new_all(),
            skip_game: false,
            ply_count: 0,
        }
    }
}

impl Visitor for MyVisitor { // '_ lifetime
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
                    Ok(rating) => if rating < MIN_RATING {
                        self.skip_game = true;
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
        // self.batch.put([1], [1]);
        // if self.batch.size_in_bytes() > 200 * 1024 * 1024 {
        //     self.db.write(std::mem::take(self.batch)).ok().unwrap();
        // }
    }

    fn begin_variation(&mut self) -> Skip {
        Skip(true) // stay in the mainline
    }

    fn end_game(&mut self) -> Self::Result {
        if self.ply_count > MIN_PLY_COUNT {
            let s = std::mem::take(&mut self.san_string);
            self.sys.refresh_memory();
            // self.sys.available_memory()
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
        }
    }
}
