use rocksdb::MergeOperands;
use crate::game_stats::GameWins;

/// Merge-operator for `RocksDB` values that store `GameWins`
/// (12-byte {black, white, draw} counters packed big-endian).
#[must_use] pub fn wins_merge_op(
    _key: &[u8],
    existing: Option<&[u8]>,
    operands: &MergeOperands,
) -> Option<Vec<u8>> {
    // start from the current value (if any) …
    let mut total = existing
        .map(GameWins::from_bytes)
        .unwrap_or_default();          // … or all-zero counters

    // …then fold every 12-byte delta into it
    for op in operands {
        total = total.combine(&GameWins::from_bytes(op));  // same combine() used elsewhere :contentReference[oaicite:0]{index=0}
    }

    Some(total.to_bytes())
}
