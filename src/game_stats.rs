use std::convert::TryInto;

#[derive(Debug, Copy, Clone)]
pub struct GameStats {
    pub black: u32,
    pub white: u32,
    pub draw:  u32,
}

impl GameStats {
    pub fn new() -> GameStats {
        GameStats {
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
    pub fn from_bytes(bytes: Vec<u8>) -> GameStats {
        GameStats {
            black: u32::from_be_bytes(bytes[..4].try_into().unwrap()),
            white: u32::from_be_bytes(bytes[4..8].try_into().unwrap()),
            draw:  u32::from_be_bytes(bytes[8..12].try_into().unwrap()),
        }
    }

    pub fn combine(self, other: &GameStats) -> GameStats {
        GameStats {
            black: self.black + other.black,
            white: self.white + other.white,
            draw: self.draw + other.draw,
        }
    }

    pub fn total(&self) -> u32 {
        self.black + self.white + self.draw
    }
}

impl Default for GameStats {
    fn default() -> Self {
        Self::new()
    }
}
