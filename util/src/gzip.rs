// Source: orig_src/core/src/com/unciv/ui/screens/savescreens/Gzip.kt
// Ported to Rust

use std::io::{self, Read, Write};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

/// Utility for compressing and decompressing data using GZIP
pub struct Gzip;

impl Gzip {
    /// Compresses a string and encodes it as base64
    pub fn zip(data: &str) -> String {
        let compressed = Self::compress(data);
        Self::encode(&compressed)
    }

    /// Decodes a base64 string and decompresses it
    pub fn unzip(data: &str) -> String {
        let decoded = Self::decode(data);
        Self::decompress(&decoded)
    }

    /// Compresses a string into a byte array
    fn compress(data: &str) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data.as_bytes()).unwrap();
        encoder.finish().unwrap()
    }

    /// Decompresses a byte array into a string
    fn decompress(compressed: &[u8]) -> String {
        let mut decoder = GzDecoder::new(compressed);
        let mut result = String::new();
        decoder.read_to_string(&mut result).unwrap();
        result
    }

    /// Encodes a byte array as base64
    fn encode(bytes: &[u8]) -> String {
        BASE64.encode(bytes)
    }

    /// Decodes a base64 string into a byte array
    fn decode(base64_str: &str) -> Vec<u8> {
        BASE64.decode(base64_str).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zip_unzip() {
        let original = "Test string for compression";
        let compressed = Gzip::zip(original);
        let decompressed = Gzip::unzip(&compressed);
        assert_eq!(original, decompressed);
    }
}