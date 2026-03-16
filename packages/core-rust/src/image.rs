//! Image loading, encoding, and transmission via the Kitty graphics protocol.
//!
//! Supports loading images from file paths (PNG, JPEG, GIF, WebP) and raw RGBA
//! bytes. Images are encoded as base64, optionally compressed with zlib, and
//! chunked into 4096-byte payloads for transmission.

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

use base64::Engine as _;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use image::GenericImageView as _;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum base64 payload size per chunk (Kitty protocol limit).
const CHUNK_SIZE: usize = 4096;

/// Threshold in bytes above which zlib compression is applied.
const COMPRESSION_THRESHOLD: usize = 4096;

/// Counter for auto-assigning image IDs.
static NEXT_IMAGE_ID: AtomicU32 = AtomicU32::new(1);

// ---------------------------------------------------------------------------
// ImageData
// ---------------------------------------------------------------------------

/// Raw image data ready for encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageData {
    /// RGBA pixel bytes (row-major, 4 bytes per pixel).
    pub rgba: Vec<u8>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl ImageData {
    /// Load an image from a file path (PNG, JPEG, GIF, WebP).
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or decoded.
    pub fn from_file(path: &Path) -> io::Result<Self> {
        let img = image::open(path)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        let (width, height) = img.dimensions();
        let rgba = img.into_rgba8().into_raw();
        Ok(Self {
            rgba,
            width,
            height,
        })
    }

    /// Create image data from raw RGBA bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the byte length does not match `width * height * 4`.
    pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> io::Result<Self> {
        let expected = (width as usize) * (height as usize) * 4;
        if rgba.len() != expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "expected {} bytes for {}x{} RGBA image, got {}",
                    expected,
                    width,
                    height,
                    rgba.len()
                ),
            ));
        }
        Ok(Self {
            rgba,
            width,
            height,
        })
    }
}

// ---------------------------------------------------------------------------
// Encoding
// ---------------------------------------------------------------------------

/// Compress data with zlib.
fn compress_zlib(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    encoder.finish()
}

/// Encode image data as a base64 string, optionally compressing first.
///
/// Returns `(base64_string, was_compressed)`.
fn encode_payload(rgba: &[u8]) -> io::Result<(String, bool)> {
    let engine = base64::engine::general_purpose::STANDARD;
    if rgba.len() > COMPRESSION_THRESHOLD {
        let compressed = compress_zlib(rgba)?;
        // Only use compression if it actually reduces size.
        if compressed.len() < rgba.len() {
            return Ok((engine.encode(&compressed), true));
        }
    }
    Ok((engine.encode(rgba), false))
}

/// Split a base64 payload into chunks of at most [`CHUNK_SIZE`] bytes.
#[must_use]
pub fn chunk_payload(payload: &str) -> Vec<&str> {
    if payload.is_empty() {
        return vec![""];
    }
    let mut chunks = Vec::new();
    let mut offset = 0;
    while offset < payload.len() {
        let end = (offset + CHUNK_SIZE).min(payload.len());
        chunks.push(&payload[offset..end]);
        offset = end;
    }
    chunks
}

// ---------------------------------------------------------------------------
// Kitty graphics protocol sequences
// ---------------------------------------------------------------------------

/// Build the Kitty graphics protocol escape sequence for transmitting an image.
///
/// The image is encoded as base64 (with optional zlib compression) and split
/// into chunks. Each chunk is wrapped in an APC sequence.
///
/// # Errors
///
/// Returns an error if compression fails.
pub fn encode_transmit(data: &ImageData, image_id: u32) -> io::Result<Vec<u8>> {
    let (payload, compressed) = encode_payload(&data.rgba)?;
    let chunks = chunk_payload(&payload);
    // f=32 means RGBA, o=z means zlib compressed
    let compression = if compressed { ",o=z" } else { "" };

    let mut buf = Vec::new();

    for (i, chunk) in chunks.iter().enumerate() {
        let is_last = i == chunks.len() - 1;
        let more = u8::from(!is_last);

        if i == 0 {
            // First chunk carries the header.
            let header = format!(
                "\x1b_Gf=32,s={},v={},i={image_id},m={more},a=t{compression};",
                data.width, data.height,
            );
            buf.extend_from_slice(header.as_bytes());
        } else {
            let header = format!("\x1b_Gm={more};");
            buf.extend_from_slice(header.as_bytes());
        }
        buf.extend_from_slice(chunk.as_bytes());
        buf.extend_from_slice(b"\x1b\\");
    }

    Ok(buf)
}

/// Build a Kitty graphics protocol display/placement command.
///
/// Places an already-transmitted image at the current cursor position.
#[must_use]
pub fn encode_display(image_id: u32, placement_id: Option<u32>) -> Vec<u8> {
    match placement_id {
        Some(pid) => format!("\x1b_Ga=p,i={image_id},p={pid};\x1b\\").into_bytes(),
        None => format!("\x1b_Ga=p,i={image_id};\x1b\\").into_bytes(),
    }
}

// ---------------------------------------------------------------------------
// Deletion commands
// ---------------------------------------------------------------------------

/// Target for image deletion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteTarget {
    /// Delete by image ID.
    ById(u32),
    /// Delete a specific placement of an image.
    ByPlacement { image_id: u32, placement_id: u32 },
    /// Delete all images.
    All,
}

/// Build a Kitty graphics protocol delete command.
#[must_use]
pub fn encode_delete(target: DeleteTarget) -> Vec<u8> {
    match target {
        DeleteTarget::ById(id) => format!("\x1b_Ga=d,d=i,i={id};\x1b\\").into_bytes(),
        DeleteTarget::ByPlacement {
            image_id,
            placement_id,
        } => format!("\x1b_Ga=d,d=i,i={image_id},p={placement_id};\x1b\\").into_bytes(),
        DeleteTarget::All => b"\x1b_Ga=d,d=a;\x1b\\".to_vec(),
    }
}

// ---------------------------------------------------------------------------
// Image cache
// ---------------------------------------------------------------------------

/// Tracks which images have been uploaded to the terminal.
#[derive(Debug, Default)]
pub struct ImageCache {
    /// Maps a cache key (e.g. file path hash or caller-provided key) to the
    /// image ID assigned by the terminal protocol.
    entries: HashMap<String, u32>,
}

impl ImageCache {
    /// Create a new empty image cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate a new unique image ID.
    #[must_use]
    pub fn next_id() -> u32 {
        NEXT_IMAGE_ID.fetch_add(1, Ordering::Relaxed)
    }

    /// Look up a cached image ID by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<u32> {
        self.entries.get(key).copied()
    }

    /// Insert an entry into the cache.
    pub fn insert(&mut self, key: String, image_id: u32) {
        self.entries.insert(key, image_id);
    }

    /// Remove an entry from the cache. Returns the image ID if it was present.
    pub fn remove(&mut self, key: &str) -> Option<u32> {
        self.entries.remove(key)
    }

    /// Clear all entries from the cache.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Returns the number of cached entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ---------------------------------------------------------------------------
// High-level transmit helper
// ---------------------------------------------------------------------------

/// Transmit an image to the terminal, using the cache to avoid re-uploading.
///
/// Returns the image ID and the bytes to write to the terminal.
///
/// # Errors
///
/// Returns an error if encoding fails.
pub fn transmit_image(
    data: &ImageData,
    cache_key: Option<&str>,
    explicit_id: Option<u32>,
    cache: &mut ImageCache,
) -> io::Result<(u32, Vec<u8>)> {
    // Check cache first.
    if let Some(key) = cache_key {
        if let Some(id) = cache.get(key) {
            // Already uploaded — return empty payload (just display it).
            return Ok((id, Vec::new()));
        }
    }

    let image_id = explicit_id.unwrap_or_else(ImageCache::next_id);
    let payload = encode_transmit(data, image_id)?;

    if let Some(key) = cache_key {
        cache.insert(key.to_owned(), image_id);
    }

    Ok((image_id, payload))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- ImageData --

    #[test]
    fn image_data_from_rgba_valid() {
        let rgba = vec![0u8; 4 * 2 * 3]; // 2x3 image
        let data = ImageData::from_rgba(rgba, 2, 3).unwrap();
        assert_eq!(data.width, 2);
        assert_eq!(data.height, 3);
        assert_eq!(data.rgba.len(), 24);
    }

    #[test]
    fn image_data_from_rgba_wrong_size() {
        let rgba = vec![0u8; 10];
        let result = ImageData::from_rgba(rgba, 2, 3);
        assert!(result.is_err());
    }

    #[test]
    fn image_data_from_rgba_zero_dimensions() {
        let rgba = vec![];
        let data = ImageData::from_rgba(rgba, 0, 0).unwrap();
        assert_eq!(data.width, 0);
        assert_eq!(data.height, 0);
        assert!(data.rgba.is_empty());
    }

    // -- Chunking --

    #[test]
    fn chunk_payload_empty() {
        let chunks = chunk_payload("");
        assert_eq!(chunks, vec![""]);
    }

    #[test]
    fn chunk_payload_small() {
        let payload = "ABCD";
        let chunks = chunk_payload(payload);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "ABCD");
    }

    #[test]
    fn chunk_payload_exact_boundary() {
        let payload = "A".repeat(CHUNK_SIZE);
        let chunks = chunk_payload(&payload);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), CHUNK_SIZE);
    }

    #[test]
    fn chunk_payload_multiple_chunks() {
        let payload = "B".repeat(CHUNK_SIZE * 2 + 100);
        let chunks = chunk_payload(&payload);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), CHUNK_SIZE);
        assert_eq!(chunks[1].len(), CHUNK_SIZE);
        assert_eq!(chunks[2].len(), 100);
    }

    #[test]
    fn chunk_payload_reassembly() {
        let payload = "X".repeat(CHUNK_SIZE * 3 + 500);
        let chunks = chunk_payload(&payload);
        let reassembled: String = chunks.into_iter().collect();
        assert_eq!(reassembled, payload);
    }

    // -- Encoding --

    #[test]
    fn encode_payload_small_not_compressed() {
        let data = vec![0u8; 100]; // Below threshold
        let (encoded, compressed) = encode_payload(&data).unwrap();
        assert!(!compressed);
        // Verify it's valid base64
        let engine = base64::engine::general_purpose::STANDARD;
        let decoded = engine.decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn encode_payload_large_compressed() {
        // Highly compressible data (all zeros) above threshold
        let data = vec![0u8; 10000];
        let (encoded, compressed) = encode_payload(&data).unwrap();
        assert!(compressed);
        // Verify we can decode and decompress
        let engine = base64::engine::general_purpose::STANDARD;
        let compressed_bytes = engine.decode(&encoded).unwrap();
        let mut decoder = flate2::read::ZlibDecoder::new(&compressed_bytes[..]);
        let mut decompressed = Vec::new();
        io::Read::read_to_end(&mut decoder, &mut decompressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn encode_payload_incompressible_data() {
        // Random-ish data that doesn't compress well
        let data: Vec<u8> = (0..5000).map(|i| (i * 97 + 13) as u8).collect();
        let (encoded, _compressed) = encode_payload(&data).unwrap();
        let engine = base64::engine::general_purpose::STANDARD;
        // Whether or not it compressed, the base64 should decode to something valid
        let decoded = engine.decode(&encoded).unwrap();
        if _compressed {
            let mut decoder = flate2::read::ZlibDecoder::new(&decoded[..]);
            let mut decompressed = Vec::new();
            io::Read::read_to_end(&mut decoder, &mut decompressed).unwrap();
            assert_eq!(decompressed, data);
        } else {
            assert_eq!(decoded, data);
        }
    }

    // -- Transmit encoding --

    #[test]
    fn encode_transmit_small_image() {
        let data = ImageData::from_rgba(vec![255u8; 16], 2, 2).unwrap();
        let result = encode_transmit(&data, 42).unwrap();
        let text = String::from_utf8(result).unwrap();
        // Should contain APC start
        assert!(text.contains("\x1b_G"));
        // Should contain image id
        assert!(text.contains("i=42"));
        // Should contain dimensions
        assert!(text.contains("s=2"));
        assert!(text.contains("v=2"));
        // Should contain format
        assert!(text.contains("f=32"));
        // Should end with ST
        assert!(text.ends_with("\x1b\\"));
    }

    #[test]
    fn encode_transmit_single_chunk_has_m_0() {
        let data = ImageData::from_rgba(vec![0u8; 16], 2, 2).unwrap();
        let result = encode_transmit(&data, 1).unwrap();
        let text = String::from_utf8(result).unwrap();
        // Single chunk: m=0 (no more data)
        assert!(text.contains("m=0"));
        assert!(!text.contains("m=1"));
    }

    #[test]
    fn encode_transmit_multi_chunk_image() {
        // Use large pseudo-random data that doesn't compress well
        let size = 500;
        let rgba: Vec<u8> = (0..(size * size * 4) as u64)
            .map(|i| {
                let x = i.wrapping_mul(2_654_435_761); // Knuth multiplicative hash
                (x >> 16) as u8
            })
            .collect();
        let data = ImageData::from_rgba(rgba, size, size).unwrap();
        let result = encode_transmit(&data, 99).unwrap();
        let text = String::from_utf8(result).unwrap();
        // Multiple chunks means m=1 should appear
        assert!(text.contains("m=1"), "expected multi-chunk encoding");
        // Count ST terminators to verify multiple chunks
        let chunk_count = text.matches("\x1b\\").count();
        assert!(
            chunk_count > 1,
            "expected multiple chunks, got {chunk_count}"
        );
    }

    #[test]
    fn encode_transmit_contains_valid_base64_payload() {
        let data = ImageData::from_rgba(vec![42u8; 16], 2, 2).unwrap();
        let result = encode_transmit(&data, 7).unwrap();
        let text = String::from_utf8(result).unwrap();
        // Extract payload: between first ; and \x1b
        let after_header = text.split(';').nth(1).unwrap();
        let payload = after_header.split("\x1b").next().unwrap();
        let engine = base64::engine::general_purpose::STANDARD;
        assert!(
            engine.decode(payload).is_ok(),
            "payload should be valid base64"
        );
    }

    // -- Display encoding --

    #[test]
    fn encode_display_without_placement() {
        let result = encode_display(5, None);
        assert_eq!(result, b"\x1b_Ga=p,i=5;\x1b\\");
    }

    #[test]
    fn encode_display_with_placement() {
        let result = encode_display(5, Some(3));
        assert_eq!(result, b"\x1b_Ga=p,i=5,p=3;\x1b\\");
    }

    // -- Deletion --

    #[test]
    fn encode_delete_by_id() {
        let result = encode_delete(DeleteTarget::ById(42));
        assert_eq!(result, b"\x1b_Ga=d,d=i,i=42;\x1b\\");
    }

    #[test]
    fn encode_delete_by_placement() {
        let result = encode_delete(DeleteTarget::ByPlacement {
            image_id: 10,
            placement_id: 2,
        });
        assert_eq!(result, b"\x1b_Ga=d,d=i,i=10,p=2;\x1b\\");
    }

    #[test]
    fn encode_delete_all() {
        let result = encode_delete(DeleteTarget::All);
        assert_eq!(result, b"\x1b_Ga=d,d=a;\x1b\\");
    }

    // -- Image cache --

    #[test]
    fn cache_new_is_empty() {
        let cache = ImageCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn cache_insert_and_get() {
        let mut cache = ImageCache::new();
        cache.insert("test.png".to_owned(), 42);
        assert_eq!(cache.get("test.png"), Some(42));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn cache_get_missing() {
        let cache = ImageCache::new();
        assert_eq!(cache.get("missing"), None);
    }

    #[test]
    fn cache_remove() {
        let mut cache = ImageCache::new();
        cache.insert("img.png".to_owned(), 1);
        assert_eq!(cache.remove("img.png"), Some(1));
        assert!(cache.is_empty());
        assert_eq!(cache.remove("img.png"), None);
    }

    #[test]
    fn cache_clear() {
        let mut cache = ImageCache::new();
        cache.insert("a".to_owned(), 1);
        cache.insert("b".to_owned(), 2);
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn cache_overwrite() {
        let mut cache = ImageCache::new();
        cache.insert("key".to_owned(), 10);
        cache.insert("key".to_owned(), 20);
        assert_eq!(cache.get("key"), Some(20));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn next_id_increments() {
        let id1 = ImageCache::next_id();
        let id2 = ImageCache::next_id();
        assert!(id2 > id1);
    }

    // -- transmit_image --

    #[test]
    fn transmit_image_no_cache() {
        let data = ImageData::from_rgba(vec![0u8; 16], 2, 2).unwrap();
        let mut cache = ImageCache::new();
        let (id, payload) = transmit_image(&data, None, Some(50), &mut cache).unwrap();
        assert_eq!(id, 50);
        assert!(!payload.is_empty());
        // No cache key, so cache should be empty
        assert!(cache.is_empty());
    }

    #[test]
    fn transmit_image_with_cache_key() {
        let data = ImageData::from_rgba(vec![0u8; 16], 2, 2).unwrap();
        let mut cache = ImageCache::new();
        let (id, payload) = transmit_image(&data, Some("my_img"), Some(51), &mut cache).unwrap();
        assert_eq!(id, 51);
        assert!(!payload.is_empty());
        assert_eq!(cache.get("my_img"), Some(51));
    }

    #[test]
    fn transmit_image_cache_hit_returns_empty_payload() {
        let data = ImageData::from_rgba(vec![0u8; 16], 2, 2).unwrap();
        let mut cache = ImageCache::new();
        cache.insert("cached".to_owned(), 100);

        let (id, payload) = transmit_image(&data, Some("cached"), Some(200), &mut cache).unwrap();
        assert_eq!(id, 100); // Returns cached ID, not explicit
        assert!(payload.is_empty()); // No re-upload
    }

    #[test]
    fn transmit_image_auto_id() {
        let data = ImageData::from_rgba(vec![0u8; 16], 2, 2).unwrap();
        let mut cache = ImageCache::new();
        let (id1, _) = transmit_image(&data, None, None, &mut cache).unwrap();
        let (id2, _) = transmit_image(&data, None, None, &mut cache).unwrap();
        assert!(id2 > id1);
    }

    // -- Compression --

    #[test]
    fn compress_zlib_roundtrip() {
        let original = b"Hello, Kitty! ".repeat(100);
        let compressed = compress_zlib(&original).unwrap();
        assert!(compressed.len() < original.len());

        let mut decoder = flate2::read::ZlibDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        io::Read::read_to_end(&mut decoder, &mut decompressed).unwrap();
        assert_eq!(decompressed, original);
    }

    // -- File loading (integration-style) --

    #[test]
    fn image_data_from_file_nonexistent() {
        let result = ImageData::from_file(Path::new("/tmp/nonexistent_kittyui_test.png"));
        assert!(result.is_err());
    }

    // -- Protocol sequence structure --

    #[test]
    fn encode_transmit_apc_structure() {
        let data = ImageData::from_rgba(vec![0u8; 16], 2, 2).unwrap();
        let result = encode_transmit(&data, 1).unwrap();
        // Every APC starts with \x1b_ and ends with \x1b\\
        let text = String::from_utf8(result).unwrap();
        let apc_starts = text.matches("\x1b_G").count();
        let apc_ends = text.matches("\x1b\\").count();
        assert_eq!(apc_starts, apc_ends, "APC start/end count mismatch");
    }

    #[test]
    fn encode_transmit_action_is_transmit() {
        let data = ImageData::from_rgba(vec![0u8; 16], 2, 2).unwrap();
        let result = encode_transmit(&data, 1).unwrap();
        let text = String::from_utf8(result).unwrap();
        assert!(
            text.contains("a=t"),
            "first chunk should have action=transmit"
        );
    }
}
