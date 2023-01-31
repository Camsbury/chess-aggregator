use std::convert::TryInto;
use std::collections::HashMap;
use serde::{Serialize};

#[derive(Debug, Copy, Clone, Default, Serialize)]
pub struct GameWins {
    pub black: u32,
    pub white: u32,
    pub draw:  u32,
}

impl GameWins {
    pub fn new() -> GameWins {
        GameWins {
            black: 0,
            white: 0,
            draw: 0,
        }
    }

    pub fn to_bytes(self) -> Vec<u8> {
        vec![
            self.black.to_be_bytes(),
            self.white.to_be_bytes(),
            self.draw.to_be_bytes(),
        ].iter().flat_map(|s| s.iter().copied()).collect()
    }

        // TODO: much unsafety - means erroneous DB data
    pub fn from_bytes(bytes: Vec<u8>) -> GameWins {
        GameWins {
            black: u32::from_be_bytes(bytes[..4].try_into().unwrap()),
            white: u32::from_be_bytes(bytes[4..8].try_into().unwrap()),
            draw:  u32::from_be_bytes(bytes[8..12].try_into().unwrap()),
        }
    }

    pub fn combine(self, other: &GameWins) -> GameWins {
        GameWins {
            black: self.black + other.black,
            white: self.white + other.white,
            draw: self.draw + other.draw,
        }
    }

    pub fn total(&self) -> u32 {
        self.black + self.white + self.draw
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct GameStats {
    pub game_wins: GameWins,
    pub game_moves: HashMap<String, GameWins>,
}

impl GameStats {
    pub fn new() -> GameStats {
        GameStats {
            game_wins: GameWins::new(),
            game_moves: HashMap::new(),
        }
    }
}
