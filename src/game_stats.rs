use serde::Serialize;
use std::collections::HashMap;
use std::convert::TryInto;

#[derive(Debug, Copy, Clone, Default, Serialize)]
pub struct GameWins {
    pub black: u32,
    pub white: u32,
    pub draws: u32,
}

impl GameWins {
    #[must_use] pub const fn new() -> Self {
        Self {
            black: 0,
            white: 0,
            draws: 0,
        }
    }

    #[must_use] pub fn to_bytes(self) -> Vec<u8> {
        [
            self.black.to_be_bytes(),
            self.white.to_be_bytes(),
            self.draws.to_be_bytes(),
        ]
        .iter()
        .flat_map(|s| s.iter().copied())
        .collect()
    }

    #[must_use] pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            black: u32::from_be_bytes(bytes[..4].try_into().unwrap()),
            white: u32::from_be_bytes(bytes[4..8].try_into().unwrap()),
            draws: u32::from_be_bytes(bytes[8..12].try_into().unwrap()),
        }
    }

    #[must_use] pub const fn combine(self, other: &Self) -> Self {
        Self {
            black: self.black + other.black,
            white: self.white + other.white,
            draws: self.draws + other.draws,
        }
    }

    #[must_use] pub const fn total(&self) -> u32 {
        self.black + self.white + self.draws
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct GameStats {
    pub game_wins: GameWins,
    pub game_moves: HashMap<String, GameWins>,
}

impl GameStats {
    #[must_use] pub fn new() -> Self {
        Self {
            game_wins: GameWins::new(),
            game_moves: HashMap::new(),
        }
    }
}
