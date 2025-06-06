extern crate actix_web;
extern crate btoi;
extern crate nibble_vec;
extern crate pgn_reader;
extern crate radix_trie;
extern crate rocksdb;
extern crate serde;
extern crate shakmaty;
extern crate sysinfo;
extern crate zstd;

// pub mod chess_db;
// pub mod game_stats;
// pub mod ingest;
// pub mod server;
// pub mod traversal;
// pub mod visitor;

// fn main() {
//     let args: Vec<String> = std::env::args().collect();

//     if args.len() < 2 {
//         panic!("Usage: {} ingest or {} serve", args[0], args[0]);
//     }

//     if args[1] == "ingest" {
//         if args.len() != 4 {
//             panic!("Usage: {} ingest <db_path> <file_paths_file>", args[0]);
//         }
//         let db_path = &args[2];
//         let filename = &args[3];
//         ingest::ingest(filename, db_path);
//     } else if args[1] == "serve" {
//         if args.len() != 3 {
//             panic!("Usage: {} serve <db_path>", args[0]);
//         }
//         let db_path = args[2].to_string();
//         server::serve(db_path).unwrap();
//     } else {
//         panic!("Usage: {} ingest or {} serve", args[0], args[0]);
//     }
// }

use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

pub mod config;
pub mod chess_db;
pub mod game_stats;
pub mod ingest;
pub mod server;
pub mod traversal;
pub mod visitor;

use crate::config::IngestConfig;

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
        /// Path to RocksDB directory
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Ingest { config } => {
            let bytes = fs::read(&config)
                .with_context(|| format!("reading {:?}", config))?;
            let cfg: IngestConfig = serde_json::from_slice(&bytes)
                .context("parsing JSON config")?;
            ingest::ingest(cfg);
        }
        Command::Serve { db_path } => {
            server::serve(db_path.to_string_lossy().into())?;
        }
    }

    Ok(())
}
