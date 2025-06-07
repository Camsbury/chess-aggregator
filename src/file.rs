use std::{fs, hash::{Hash, Hasher}, collections::hash_map::DefaultHasher};

pub fn id(path: &str) -> anyhow::Result<[u8; 8]> {
    let meta = fs::metadata(path)?;
    let mut h = DefaultHasher::new();
    path.hash(&mut h);                   // absolute path
    meta.len().hash(&mut h);             // file size
    meta.modified()?.duration_since(std::time::UNIX_EPOCH)?
                  .as_secs()
                  .hash(&mut h);         // mtime, 1 s resolution
    Ok(h.finish().to_be_bytes())
}
