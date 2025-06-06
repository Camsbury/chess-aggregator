use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use chess_aggregator::ingest;
use chess_aggregator::server;
use chess_aggregator::config;

/// Command‑line entry point. Replaces manual `std::env::args()` handling
/// with `clap` – easier to extend and gives free `--help`.
#[derive(Parser)]
#[command(name = "chess-aggregator", version, about = "Bulk‑ingest Lichess PGNs and serve aggregated stats")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Ingest PGN data according to a JSON configuration file
    Ingest {
        /// Path to the JSON file
        #[arg(value_name = "CONFIG.json")]
        config: PathBuf,
    },
    /// Launch the HTTP API backed by an existing RocksDB database
    Serve {
        /// Path to the JSON file
        #[arg(value_name = "CONFIG.json")]
        config: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Ingest { config } => {
            let bytes = fs::read(&config)
                .with_context(|| format!("reading {:?}", config))?;
            let cfg: config::Ingest = serde_json::from_slice(&bytes)
                .context("parsing JSON config")?;
            ingest::ingest(&cfg)?;
        }
        Command::Serve { config } => {
            let bytes = fs::read(&config)
                .with_context(|| format!("reading {:?}", config))?;
            let cfg: config::Server = serde_json::from_slice(&bytes)
                .context("parsing JSON config")?;
            server::serve(cfg)?;
        }
    }

    Ok(())
}
