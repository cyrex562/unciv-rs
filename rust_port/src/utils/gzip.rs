// Source: orig_src/core/src/com/unciv/ui/screens/savescreens/Gzip.kt
// Ported to Rust

use std::io::{self, Read, Write};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

/// Utility for compressing and decompressing strings using gzip
pub struct Gzip;

impl Gzip {
    /// Compress a string using gzip and encode it in base64
    pub fn zip(input: &str) -> io::Result<String> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(input.as_bytes())?;
        let compressed = encoder.finish()?;
        Ok(BASE64.encode(compressed))
    }

    /// Decode a base64 string and decompress it using gzip
    pub fn unzip(input: &str) -> io::Result<String> {
        let decoded = BASE64.decode(input)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let mut decoder = GzDecoder::new(&decoded[..]);
        let mut decompressed = String::new();
        decoder.read_to_string(&mut decompressed)?;
        Ok(decompressed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zip_unzip() {
        let original = "Test string for compression";
        let compressed = Gzip::zip(original).unwrap();
        let decompressed = Gzip::unzip(&compressed).unwrap();
        assert_eq!(original, decompressed);
    }
}