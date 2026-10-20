use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use effect_api::Frame32;
use model::{ItemId, Time};

/// 青がディスク、緑が RAM。両方に載っているフレームは両方真になる。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CachePresence {
    pub in_ram: bool,
    pub on_disk: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FrameKey {
    pub composition: ItemId,
    pub frame: i64,
    pub downsample_x: u32,
    pub downsample_y: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacheError {
    pub message: String,
}

struct DiskEntry {
    path: PathBuf,
    bytes: u64,
}

/// ディスクキャッシュ。プロジェクトには保存しない。
///
/// 上限を下げたあとに超過分を消すタイミングは仕様が要確認。
/// この実装は `set_max_bytes` と `store` が戻る時点で使用量を上限以下にする。
/// 可逆圧縮の方式名も要確認なので、ここでは単純な RLE を使う。
pub struct FrameCache {
    enabled: bool,
    max_bytes: u64,
    folder: PathBuf,
    preview_from_disk: bool,
    compress: bool,
    ram_bytes: u64,
    ram: HashMap<FrameKey, Frame32>,
    ram_order: VecDeque<FrameKey>,
    ram_usage: u64,
    disk: HashMap<FrameKey, DiskEntry>,
    disk_order: VecDeque<FrameKey>,
}

impl FrameCache {
    pub fn open(
        folder: impl Into<PathBuf>,
        max_bytes: u64,
        ram_bytes: u64,
        compress: bool,
    ) -> Result<Self, CacheError> {
        let folder = folder.into();
        fs::create_dir_all(&folder).map_err(io_error)?;
        Ok(Self {
            enabled: true,
            max_bytes,
            folder,
            preview_from_disk: true,
            compress,
            ram_bytes,
            ram: HashMap::new(),
            ram_order: VecDeque::new(),
            ram_usage: 0,
            disk: HashMap::new(),
            disk_order: VecDeque::new(),
        })
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn preview_from_disk(&self) -> bool {
        self.preview_from_disk
    }

    pub fn max_bytes(&self) -> u64 {
        self.max_bytes
    }

    pub fn disk_usage(&self) -> u64 {
        self.disk.values().map(|entry| entry.bytes).sum()
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_preview_from_disk(&mut self, enabled: bool) {
        self.preview_from_disk = enabled;
    }

    pub fn set_compress(&mut self, compress: bool) {
        self.compress = compress;
    }

    pub fn set_folder(&mut self, folder: impl Into<PathBuf>) -> Result<(), CacheError> {
        let folder = folder.into();
        fs::create_dir_all(&folder).map_err(io_error)?;
        self.folder = folder;
        Ok(())
    }

    pub fn set_max_bytes(&mut self, max_bytes: u64) -> Result<(), CacheError> {
        self.max_bytes = max_bytes;
        self.enforce_disk_limit()
    }

    pub fn presence(&self, key: &FrameKey) -> CachePresence {
        CachePresence {
            in_ram: self.ram.contains_key(key),
            on_disk: self.disk.contains_key(key),
        }
    }

    pub fn disk_path(&self, key: &FrameKey) -> Option<&Path> {
        self.disk.get(key).map(|entry| entry.path.as_path())
    }

    pub fn store(&mut self, key: FrameKey, frame: &Frame32) -> Result<(), CacheError> {
        self.store_ram(key, frame);
        if !self.enabled {
            return Ok(());
        }
        let payload = encode_frame(frame, self.compress);
        let bytes = payload.len() as u64;
        if bytes > self.max_bytes {
            return Ok(());
        }
        self.remove_disk(key)?;
        while self.disk_usage().saturating_add(bytes) > self.max_bytes {
            let Some(oldest) = self.disk_order.front().copied() else {
                break;
            };
            if oldest == key {
                break;
            }
            self.remove_disk(oldest)?;
        }
        if self.disk_usage().saturating_add(bytes) > self.max_bytes {
            return Ok(());
        }
        let path = self.folder.join(file_name(&key));
        fs::write(&path, &payload).map_err(io_error)?;
        self.disk.insert(key, DiskEntry { path, bytes });
        self.disk_order.push_back(key);
        Ok(())
    }

    pub fn playback(&self, key: &FrameKey) -> Result<Option<Frame32>, CacheError> {
        if let Some(frame) = self.ram.get(key) {
            return Ok(Some(frame.clone()));
        }
        if self.preview_from_disk {
            return self.load_disk(key);
        }
        Ok(None)
    }

    pub fn load_disk(&self, key: &FrameKey) -> Result<Option<Frame32>, CacheError> {
        let Some(entry) = self.disk.get(key) else {
            return Ok(None);
        };
        let bytes = fs::read(&entry.path).map_err(io_error)?;
        decode_frame(&bytes).map(Some)
    }

    pub fn empty_disk(&mut self) -> Result<(), CacheError> {
        let keys: Vec<FrameKey> = self.disk.keys().copied().collect();
        for key in keys {
            self.remove_disk(key)?;
        }
        Ok(())
    }

    fn store_ram(&mut self, key: FrameKey, frame: &Frame32) {
        self.remove_ram(key);
        let bytes = ram_size(frame);
        if bytes > self.ram_bytes {
            return;
        }
        while self.ram_usage.saturating_add(bytes) > self.ram_bytes {
            let Some(oldest) = self.ram_order.front().copied() else {
                break;
            };
            self.remove_ram(oldest);
        }
        if self.ram_usage.saturating_add(bytes) > self.ram_bytes {
            return;
        }
        self.ram.insert(key, frame.clone());
        self.ram_order.push_back(key);
        self.ram_usage = self.ram_usage.saturating_add(bytes);
    }

    fn remove_ram(&mut self, key: FrameKey) {
        if let Some(frame) = self.ram.remove(&key) {
            self.ram_usage = self.ram_usage.saturating_sub(ram_size(&frame));
            self.ram_order.retain(|item| *item != key);
        }
    }

    fn remove_disk(&mut self, key: FrameKey) -> Result<(), CacheError> {
        if let Some(entry) = self.disk.remove(&key) {
            if entry.path.exists() {
                fs::remove_file(&entry.path).map_err(io_error)?;
            }
            self.disk_order.retain(|item| *item != key);
        }
        Ok(())
    }

    fn enforce_disk_limit(&mut self) -> Result<(), CacheError> {
        while self.disk_usage() > self.max_bytes {
            let Some(oldest) = self.disk_order.front().copied() else {
                break;
            };
            self.remove_disk(oldest)?;
        }
        Ok(())
    }
}

pub fn frame_index(time: Time, frame_rate: f64) -> i64 {
    time.to_frames(frame_rate).round() as i64
}

fn ram_size(frame: &Frame32) -> u64 {
    (frame.samples.len() as u64).saturating_mul(4)
}

fn file_name(key: &FrameKey) -> String {
    format!(
        "c{}_f{}_x{}_y{}.oefc",
        key.composition.get(),
        key.frame,
        key.downsample_x,
        key.downsample_y
    )
}

fn io_error(error: std::io::Error) -> CacheError {
    CacheError {
        message: error.to_string(),
    }
}

const MAGIC: &[u8; 4] = b"OEFC";

fn encode_frame(frame: &Frame32, compress: bool) -> Vec<u8> {
    let mut raw = Vec::with_capacity(frame.samples.len() * 4);
    for sample in &frame.samples {
        raw.extend_from_slice(&sample.to_le_bytes());
    }
    let payload = if compress { rle_encode(&raw) } else { raw };
    let mut out = Vec::with_capacity(16 + payload.len());
    out.extend_from_slice(MAGIC);
    out.push(1);
    out.push(u8::from(compress));
    out.extend_from_slice(&frame.width.to_le_bytes());
    out.extend_from_slice(&frame.height.to_le_bytes());
    out.extend_from_slice(&u32::from(frame.channels).to_le_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&payload);
    out
}

fn decode_frame(bytes: &[u8]) -> Result<Frame32, CacheError> {
    if bytes.len() < 22 || &bytes[0..4] != MAGIC {
        return Err(CacheError {
            message: "not a frame cache".to_string(),
        });
    }
    let compressed = bytes[5] == 1;
    let width = u32::from_le_bytes(bytes[6..10].try_into().unwrap());
    let height = u32::from_le_bytes(bytes[10..14].try_into().unwrap());
    let channels = u32::from_le_bytes(bytes[14..18].try_into().unwrap());
    let payload_len = u32::from_le_bytes(bytes[18..22].try_into().unwrap()) as usize;
    let payload = bytes.get(22..22 + payload_len).ok_or_else(|| CacheError {
        message: "truncated frame cache".to_string(),
    })?;
    let raw = if compressed {
        rle_decode(payload)?
    } else {
        payload.to_vec()
    };
    if channels == 0 || raw.len() % 4 != 0 {
        return Err(CacheError {
            message: "bad frame cache".to_string(),
        });
    }
    let mut samples = Vec::with_capacity(raw.len() / 4);
    for chunk in raw.chunks_exact(4) {
        samples.push(f32::from_le_bytes(chunk.try_into().unwrap()));
    }
    let expected = (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(channels as usize);
    if samples.len() != expected || channels > u32::from(u8::MAX) {
        return Err(CacheError {
            message: "frame cache size mismatch".to_string(),
        });
    }
    Ok(Frame32 {
        width,
        height,
        channels: channels as u8,
        samples,
    })
}

fn rle_encode(data: &[u8]) -> Vec<u8> {
    let mut samples = Vec::with_capacity(data.len() / 4);
    for chunk in data.chunks_exact(4) {
        samples.push(f32::from_le_bytes(chunk.try_into().unwrap()));
    }
    let mut out = Vec::new();
    let mut index = 0;
    while index < samples.len() {
        let value = samples[index];
        let mut count = 1u32;
        while (index + count as usize) < samples.len()
            && samples[index + count as usize].to_bits() == value.to_bits()
            && count < u32::MAX
        {
            count += 1;
        }
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&value.to_le_bytes());
        index += count as usize;
    }
    out
}

fn rle_decode(data: &[u8]) -> Result<Vec<u8>, CacheError> {
    if data.len() % 8 != 0 {
        return Err(CacheError {
            message: "bad rle".to_string(),
        });
    }
    let mut out = Vec::new();
    for record in data.chunks_exact(8) {
        let count = u32::from_le_bytes(record[0..4].try_into().unwrap());
        if count == 0 {
            return Err(CacheError {
                message: "bad rle".to_string(),
            });
        }
        let value = record[4..8].to_vec();
        for _ in 0..count {
            out.extend_from_slice(&value);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{frame_index, FrameCache, FrameKey};
    use effect_api::Frame32;
    use model::{ItemId, Time};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "openeffect-gpu-{name}-{}-{nanos}",
            std::process::id()
        ))
    }

    fn solid(value: f32) -> Frame32 {
        Frame32::new(16, 16, 4, value)
    }

    fn key(frame: i64) -> FrameKey {
        FrameKey {
            composition: ItemId::new(3),
            frame,
            downsample_x: 1,
            downsample_y: 1,
        }
    }

    #[test]
    fn frame_index_uses_the_composition_rate() {
        assert_eq!(frame_index(Time::from_seconds(1.0), 24.0), 24);
    }

    #[test]
    fn enabled_cache_marks_disk_and_disabled_cache_does_not_grow() {
        let dir = temp_dir("marks");
        let mut cache = FrameCache::open(&dir, u64::MAX, u64::MAX, false).unwrap();
        assert!(cache.enabled());
        cache.store(key(0), &solid(0.25)).unwrap();
        assert!(cache.presence(&key(0)).on_disk);
        assert!(cache.presence(&key(0)).in_ram);
        let usage = cache.disk_usage();
        assert!(usage > 0);
        cache.set_enabled(false);
        cache.store(key(1), &solid(0.5)).unwrap();
        assert_eq!(cache.disk_usage(), usage);
        assert!(!cache.presence(&key(1)).on_disk);
        assert!(cache.presence(&key(1)).in_ram);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn disk_usage_stays_within_the_limit() {
        let dir = temp_dir("limit");
        let mut cache = FrameCache::open(&dir, u64::MAX, u64::MAX, false).unwrap();
        cache.store(key(0), &solid(0.25)).unwrap();
        let one = cache.disk_usage();
        cache.set_max_bytes(one.saturating_mul(2)).unwrap();
        cache.store(key(1), &solid(0.5)).unwrap();
        assert!(cache.disk_usage() <= cache.max_bytes());
        cache.set_max_bytes(one).unwrap();
        assert!(cache.disk_usage() <= one);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn new_frames_follow_the_chosen_folder() {
        let first = temp_dir("folder-a");
        let second = temp_dir("folder-b");
        let mut cache = FrameCache::open(&first, u64::MAX, u64::MAX, false).unwrap();
        cache.store(key(0), &solid(0.25)).unwrap();
        let old = cache.disk_path(&key(0)).unwrap().to_path_buf();
        assert!(old.starts_with(&first));
        cache.set_folder(&second).unwrap();
        cache.store(key(1), &solid(0.5)).unwrap();
        let new = cache.disk_path(&key(1)).unwrap();
        assert!(new.starts_with(&second));
        assert!(old.exists());
        let _ = fs::remove_dir_all(&first);
        let _ = fs::remove_dir_all(&second);
    }

    #[test]
    fn empty_disk_clears_usage_and_the_blue_mark() {
        let dir = temp_dir("empty");
        let mut cache = FrameCache::open(&dir, u64::MAX, u64::MAX, false).unwrap();
        cache.store(key(0), &solid(0.25)).unwrap();
        let path = cache.disk_path(&key(0)).unwrap().to_path_buf();
        cache.empty_disk().unwrap();
        assert_eq!(cache.disk_usage(), 0);
        assert!(!cache.presence(&key(0)).on_disk);
        assert!(!path.exists());
        assert!(cache.presence(&key(0)).in_ram);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn preview_from_disk_plays_frames_that_do_not_fit_in_ram() {
        let dir = temp_dir("playback");
        let frame = solid(0.75);
        let mut cache = FrameCache::open(&dir, u64::MAX, 0, false).unwrap();
        assert!(cache.preview_from_disk());
        cache.store(key(0), &frame).unwrap();
        assert!(cache.presence(&key(0)).on_disk);
        assert!(!cache.presence(&key(0)).in_ram);
        let played = cache.playback(&key(0)).unwrap().unwrap();
        assert_eq!(played.samples, frame.samples);
        cache.set_preview_from_disk(false);
        assert!(cache.playback(&key(0)).unwrap().is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn lossless_compression_is_smaller_and_matches_pixels() {
        let raw_dir = temp_dir("raw");
        let compressed_dir = temp_dir("rle");
        let frame = solid(0.125);
        let mut raw = FrameCache::open(&raw_dir, u64::MAX, 0, false).unwrap();
        let mut compressed = FrameCache::open(&compressed_dir, u64::MAX, 0, true).unwrap();
        raw.store(key(0), &frame).unwrap();
        compressed.store(key(0), &frame).unwrap();
        assert!(compressed.disk_usage() < raw.disk_usage());
        assert_eq!(
            raw.load_disk(&key(0)).unwrap().unwrap().samples,
            compressed.load_disk(&key(0)).unwrap().unwrap().samples
        );
        let _ = fs::remove_dir_all(&raw_dir);
        let _ = fs::remove_dir_all(&compressed_dir);
    }
}
