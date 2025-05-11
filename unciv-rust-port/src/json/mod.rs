use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use serde::{Serialize, Deserialize};
use serde_json;

/// A thread-safe JSON serializer/deserializer for Unciv
pub struct UncivJson {
    /// Whether to ignore deprecated fields during deserialization
    pub ignore_deprecated: bool,
    /// Whether to ignore unknown fields during deserialization
    pub ignore_unknown_fields: bool,
}

impl Default for UncivJson {
    fn default() -> Self {
        UncivJson {
            ignore_deprecated: true,
            ignore_unknown_fields: true,
        }
    }
}

impl UncivJson {
    /// Create a new UncivJson instance with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to ignore deprecated fields
    pub fn set_ignore_deprecated(mut self, ignore: bool) -> Self {
        self.ignore_deprecated = ignore;
        self
    }

    /// Set whether to ignore unknown fields
    pub fn set_ignore_unknown_fields(mut self, ignore: bool) -> Self {
        self.ignore_unknown_fields = ignore;
        self
    }

    /// Serialize a value to a JSON string
    pub fn to_string<T: Serialize>(&self, value: &T) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(value)
    }

    /// Serialize a value to a JSON string with minimal whitespace
    pub fn to_string_minimal<T: Serialize>(&self, value: &T) -> Result<String, serde_json::Error> {
        serde_json::to_string(value)
    }

    /// Deserialize a JSON string into a value
    pub fn from_str<'a, T: Deserialize<'a>>(&self, s: &'a str) -> Result<T, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// Load a JSON file from a path
    pub fn from_file<T: for<'de> Deserialize<'de>>(&self, path: &Path) -> io::Result<T> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        serde_json::from_str(&contents)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData,
                format!("Could not parse json of file {}: {}", path.display(), e)))
    }

    /// Load a JSON file from a path with a specific encoding
    pub fn from_file_with_encoding<T: for<'de> Deserialize<'de>>(
        &self,
        path: &Path,
        _encoding: &str
    ) -> io::Result<T> {
        let mut file = File::open(path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        // In a real implementation, we would use a proper encoding library
        // For now, we'll just use UTF-8
        let contents = String::from_utf8(contents)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData,
                format!("Invalid UTF-8 in file {}: {}", path.display(), e)))?;

        serde_json::from_str(&contents)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData,
                format!("Could not parse json of file {}: {}", path.display(), e)))
    }
}

/// Create a new UncivJson instance with default settings
pub fn json() -> UncivJson {
    UncivJson::new()
}

/// Load a JSON file from a path
pub fn from_json_file<T: for<'de> Deserialize<'de>>(path: &Path) -> io::Result<T> {
    json().from_file(path)
}

/// Load a JSON file from a path with a specific encoding
pub fn from_json_file_with_encoding<T: for<'de> Deserialize<'de>>(
    path: &Path,
    encoding: &str
) -> io::Result<T> {
    json().from_file_with_encoding(path, encoding)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::serializers::SerializableDuration;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        name: String,
        duration: SerializableDuration,
    }

    #[test]
    fn test_serialization() {
        let json = json();
        let value = TestStruct {
            name: "test".to_string(),
            duration: SerializableDuration(Duration::from_secs(3600)),
        };

        let serialized = json.to_string(&value).unwrap();
        assert!(serialized.contains("\"name\":\"test\""));
        assert!(serialized.contains("\"duration\":\"PT1H\""));

        let deserialized: TestStruct = json.from_str(&serialized).unwrap();
        assert_eq!(deserialized.name, "test");
        assert_eq!(deserialized.duration.0, Duration::from_secs(3600));
    }
}