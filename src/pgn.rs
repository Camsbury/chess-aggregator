extern crate pgn_reader;

use pgn_reader::{SanPlus, Skip, Visitor, Outcome, Color};
use radix_trie::{Trie};

pub struct GameStats {
    pub black: u32,
    pub white: u32,
    pub draw:  u32,
}

pub struct MyVisitor { // 'a lifetime
    // db: &'a DB,
    // batch: &'a mut WriteBatch,
    pub san_tree: Trie<String, GameStats>,
    pub san_string: String,
    pub winner: Option<Color>,
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
    }
}
