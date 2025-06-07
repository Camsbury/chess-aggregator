//! ingest.rs – single‑reader / multi‑worker orchestration
//!
//! Opens the `RocksDB` database using the *`db_path`* from `config::Ingest`, so callers
//! no longer need to pass a `DB` handle explicitly.  The rest of the pipeline
//! (reader thread + Rayon worker pool) is unchanged.

use anyhow::{Context, Result as AnyResult};
use crate::GameSummary;
use crate::config;
use crate::extractor::Extractor;
use crate::merge::wins_merge_op;
use crate::worker;
use crate::chess_db::FS;
use crate::file;
use crossbeam_channel::Sender;
use crossbeam_channel as chan;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use num_cpus;
use rayon::ThreadPoolBuilder;
use rocksdb::{Options, DB};
use std::{fs, io};
use std::sync::Arc;
use chrono;

/// Top‑level ingestion entry‑point with a single‑sender lifetime fix.
///
/// The *only* `Sender` is moved into the reader thread so that it is
/// dropped automatically when the reader finishes, letting every worker
/// see `Err(Disconnected)` and exit cleanly.  No explicit `drop(tx)` is
/// required in the outer scope.
pub fn ingest(cfg: &config::Ingest) -> anyhow::Result<()> {
    // 0) Open (or create) RocksDB once.
    let mut db_opts = Options::default();
    db_opts.create_if_missing(true);
    db_opts.set_merge_operator_associative("add_wins", wins_merge_op);
    let db = Arc::new(DB::open(&db_opts, &cfg.db_path)?);

    // 1) Determine worker‑thread count from the system.
    let n_threads = num_cpus::get().max(1);

    // 2) Build a Rayon pool with that many threads.
    let pool = ThreadPoolBuilder::new()
        .num_threads(n_threads)
        .build()
        .context("build thread‑pool")?;

    // 3) Bounded channel provides back‑pressure. Capacity scales with cache_size.
    let channel_cap = std::cmp::max(4096, cfg.cache_size / 16);
    let (tx, rx) = chan::bounded::<GameSummary>(channel_cap);

    // 4) Spawn worker tasks inside the pool. Each has its own write‑cache.
    pool.scope(|s| {
        for _ in 0..n_threads {
            let rx = rx.clone();
            let db = db.clone();
            let flush_threshold = cfg.cache_size;
            s.spawn(move |_| {
                worker::run(&rx, &db, flush_threshold);
            });
        }

        // 5) Reader thread: owns *the* Sender and drops it when done.
        let reader_db = db.clone();
        let reader_handle = std::thread::spawn({
            let tx = tx;           // move, do not clone – guarantees closure
            let cfg = cfg.clone();
            move || {
                if let Err(err) = run_reader(&cfg, &reader_db, tx) {
                    eprintln!("[ingest] reader error: {err}");
                }
            }
        });

        // Wait for the reader to finish; the pool will wait for workers.
        reader_handle.join().expect("reader thread panicked");
    });

    Ok(())
}

/// Runs inside the *reader* thread.
///
/// * `cfg.pgn_files` – list of archives (plain PGN, `.gz`, `.zst`, …).
/// * `tx`            – bounded channel feeding parsed games to workers.
pub fn run_reader(
    cfg: &config::Ingest,
    db: &DB,
    tx: Sender<GameSummary>,
) -> AnyResult<()> {
    // 1️⃣  Total compressed bytes across all input archives ---------------
    let total_bytes: u64 = cfg
        .pgn_files
        .iter()
        .try_fold(0u64, |acc, p| {
            let len = fs::metadata(p)
                .with_context(|| format!("stat {p:?}"))?
                .len();
            Ok::<u64, anyhow::Error>(acc + len)
        })?;

    // 2️⃣  Progress bars --------------------------------------------------
    let mp = MultiProgress::new();
    let overall = mp.add(ProgressBar::new(total_bytes));
    overall.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} {bytes:>12}/{total_bytes:12} {wide_bar} {eta} {msg}",
        )?
        .progress_chars("▏▎▍▌▋▊▉█"),
    );

    // 3️⃣  Process each archive ------------------------------------------
    for path in &cfg.pgn_files {
         // ① build the RocksDB key
        let mut file_key = Vec::from(FS);
        file_key.extend_from_slice(&file::id(path)?);

        // ② skip if we already saw it
        if db.get(&file_key)?.is_some() {
            eprintln!("Skipping already-ingested {path}");
            continue;
        }


        let short = std::path::Path::new(path)
            .file_name()
            .map_or_else(|| path.into(), |os| os.to_string_lossy());

        overall.set_message(short.to_string());

        let file_len = fs::metadata(path)
            .with_context(|| format!("stat {path:?}"))?
            .len();

        // Per‑file bar
        let bar = mp.add(ProgressBar::new(file_len));
        bar.set_style(
            ProgressStyle::with_template(
                "{spinner:.cyan} {bytes:>10}/{total_bytes:10} {wide_bar}",
            )?
            .progress_chars("•░▒▓█"),
        );

        // Open & wrap
        let file = fs::File::open(path)
            .with_context(|| format!("open {path:?}"))?;
        let file = bar.wrap_read(file);      // ticks file bar
        let file = overall.wrap_read(file);  // ticks global bar
        let reader = io::BufReader::new(file);

        // Optional decompression
        let decoder: Box<dyn io::Read + Send> = match std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
        {
            Some("zst" | "zstd") => Box::new(zstd::stream::read::Decoder::new(reader)?),
            Some("gz")           => Box::new(flate2::read::GzDecoder::new(reader)),
            _                    => Box::new(reader),
        };

        // Parse and send games
        let mut br  = pgn_reader::BufferedReader::new(decoder);
        let mut vis = Extractor::new(&tx, cfg);
        br.read_all(&mut vis)
            .with_context(|| format!("parse {path:?}"))?;
        bar.finish_and_clear();
        db.put(&file_key, chrono::Utc::now().timestamp().to_be_bytes())?;
    }
    overall.finish_and_clear();                     // leave the bar at “done”
    mp.println("stream closed — workers finishing payloads…")?;

    // 4️⃣  Done – drop the *owned* sender so the channel closes -----------
    drop(tx);
    Ok(())
}
