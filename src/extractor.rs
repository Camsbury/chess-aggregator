use pgn_reader::{Color, Outcome, RawHeader, Skip, Visitor};
use shakmaty::san::SanPlus;
use crossbeam_channel::Sender;
use crate::{GameSummary, config};

/// Visitor that extracts the winner + SAN move list for each game that passes
/// filtering, then sends it to the worker pool.
pub struct Extractor<'a> {
    tx: &'a Sender<GameSummary>,
    winner: Option<Color>,
    sans: Vec<SanPlus>,
    skip_game: bool,
    ply_count: u32,
    // filters
    min_rating: u32,
    min_ply_count: u32,
    time_controls: Vec<String>,
}

impl<'a> Extractor<'a> {
    #[must_use] pub fn new(tx: &'a Sender<GameSummary>, cfg: &config::Ingest) -> Self {
        Self {
            tx,
            winner: None,
            sans: Vec::new(),
            skip_game: false,
            ply_count: 0,
            min_rating: cfg.min_rating,
            min_ply_count: cfg.min_ply_count,
            time_controls: cfg.time_controls.clone(),
        }
    }
}

impl Visitor for Extractor<'_> {
    type Result = ();

    fn begin_headers(&mut self) {
        self.skip_game = false;
    }

    fn header(&mut self, key: &[u8], value: RawHeader) {
        match key {
            b"WhiteElo" | b"BlackElo" => {
                if value.as_bytes() == b"?" {
                    self.skip_game = true;
                } else if let Ok(rating) = btoi::btoi::<u32>(value.as_bytes()) {
                    if rating < self.min_rating {
                        self.skip_game = true;
                    }
                } else {
                    self.skip_game = true;
                }
            }
            b"Event" => {
                if let Ok(ev_raw) = std::str::from_utf8(value.as_bytes()) {
                    let ev_lc = ev_raw.trim_matches(&['\"', '\''][..]).to_ascii_lowercase();
                    let has_time_controls = self.time_controls.iter().any(|w| ev_lc.contains(w));
                    let is_casual  = ev_lc.contains("casual");
                    if !has_time_controls || is_casual { self.skip_game = true; }
                } else {
                    self.skip_game = true;
                }
            }
            _ => {}
        }
    }

    fn end_headers(&mut self) -> Skip { Skip(self.skip_game) }

    fn begin_game(&mut self) { self.ply_count = 0; self.sans.clear(); }

    fn san(&mut self, san_plus: SanPlus) {
        self.ply_count += 1;
        self.sans.push(san_plus);
    }

    fn outcome(&mut self, outcome: Option<Outcome>) {
        self.winner = match outcome {
            Some(Outcome::Decisive { winner }) => Some(winner),
            _ => None,
        };
    }

    fn end_game(&mut self) {
        if !self.skip_game && self.ply_count >= self.min_ply_count {
            let summary = GameSummary { winner: self.winner, sans: std::mem::take(&mut self.sans) };
            let _ = self.tx.send(summary); // ignore error on shutdown
        }
        self.sans.clear();
        self.ply_count = 0;
        self.winner = None;
    }

    fn begin_variation(&mut self) -> Skip { Skip(true) }
}
