use serde::Deserialize;

// Put the helpers right above the struct so the names stay private.
const fn default_min_rating() -> u32   { 0 }
const fn default_cache_size() -> usize { 1_000_000 }

/// Shape of the JSON config expected by the `ingest` sub‑command.
#[derive(Clone, Debug, Deserialize)]
pub struct Ingest {
    /// `RocksDB` path. Created if it does not exist.
    pub db_path: String,
    /// Minimum ply to keep a game.
    pub min_ply_count: u32,
    /// Time Controls allowed
    pub time_controls: Vec<String>,
    /// Minimum Elo for White *and* Black to keep a game.
    #[serde(default = "default_min_rating")]
    pub min_rating: u32,
    /// Max entries kept in the in‑memory `RocksDB` write‑cache.
    #[serde(default = "default_cache_size")]
    pub cache_size: usize,
    /// Location of `.pgn.zst` archives to process.
    pub pgn_dir: String,
}

/// Shape of the JSON config expected by the `ingest` sub‑command.
#[derive(Clone, Debug, Deserialize)]
pub struct Server {
    /// `RocksDB` path. Created if it does not exist.
    pub db_path: String,
}
