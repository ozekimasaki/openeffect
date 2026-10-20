use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

pub fn temp_dir(name: &str) -> PathBuf {
    let n = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("openeffect-{name}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}
