use serde::Deserialize;

// Put the helpers right above the struct so the names stay private.
fn default_min_rating()      -> u32   { 0 }
fn default_threshold_writes() -> u32 { 60_000_000 }
fn default_cache_size()       -> usize { 1_000_000 }

/// Shape of the JSON config expected by the `ingest` sub‑command.
#[derive(Debug, Deserialize)]
pub struct IngestConfig {
    /// RocksDB path. Created if it does not exist.
    pub db_path: String,
    /// Minimum ply to keep a game.
    pub min_ply_count: u32,
    /// Event words marking a game for inclusion
    pub required_words: Vec<String>,
    /// Event words marking a game for exclusion
    pub forbidden_words: Vec<String>,
    /// Minimum Elo for White *and* Black to keep a game.
    #[serde(default = "default_min_rating")]
    pub min_rating: u32,
    /// Amount of writes to the SANTree allowed before extracting stats
    #[serde(default = "default_threshold_writes")]
    pub threshold_writes: u32,
    /// Max entries kept in the in‑memory RocksDB write‑cache.
    #[serde(default = "default_cache_size")]
    pub cache_size: usize,
    /// List of `.pgn.zst` archives to process.
    pub pgn_files: Vec<String>,
}
