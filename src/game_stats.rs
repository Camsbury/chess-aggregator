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
