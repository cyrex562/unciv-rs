use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::time::Duration;

/// Serializes a Duration to a string in ISO 8601 format
pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Convert Duration to ISO 8601 string format (e.g., "PT1H30M")
    let secs = duration.as_secs();
    let nanos = duration.subsec_nanos();

    let mut result = String::from("PT");

    if secs >= 3600 {
        let hours = secs / 3600;
        result.push_str(&format!("{}H", hours));
    }

    if secs % 3600 >= 60 {
        let minutes = (secs % 3600) / 60;
        result.push_str(&format!("{}M", minutes));
    }

    let seconds = secs % 60;
    if seconds > 0 || nanos > 0 {
        let mut sec_str = format!("{}S", seconds);
        if nanos > 0 {
            sec_str = format!("{}.{:09}S", seconds, nanos);
        }
        result.push_str(&sec_str);
    }

    serializer.serialize_str(&result)
}

/// Deserializes a string in ISO 8601 format to a Duration
pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    // Parse ISO 8601 duration format (e.g., "PT1H30M")
    if !s.starts_with("PT") {
        return Err(serde::de::Error::custom("Duration must start with 'PT'"));
    }

    let mut secs = 0u64;
    let mut nanos = 0u32;

    let mut chars = s[2..].chars().peekable();

    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() || c == '.' {
            let mut num_str = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() || c == '.' {
                    num_str.push(chars.next().unwrap());
                } else {
                    break;
                }
            }

            let num: f64 = num_str.parse().map_err(serde::de::Error::custom)?;

            if let Some(unit) = chars.next() {
                match unit {
                    'H' => secs += (num * 3600.0) as u64,
                    'M' => secs += (num * 60.0) as u64,
                    'S' => {
                        let whole_secs = num.floor() as u64;
                        let fractional_secs = ((num - num.floor()) * 1_000_000_000.0) as u32;
                        secs += whole_secs;
                        nanos = fractional_secs;
                    },
                    _ => return Err(serde::de::Error::custom(format!("Invalid unit: {}", unit))),
                }
            }
        } else {
            chars.next();
        }
    }

    Ok(Duration::new(secs, nanos))
}

/// A wrapper for Duration that implements Serialize and Deserialize
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SerializableDuration(pub Duration);

impl Serialize for SerializableDuration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize(&self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for SerializableDuration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize(deserializer).map(SerializableDuration)
    }
}

impl From<Duration> for SerializableDuration {
    fn from(duration: Duration) -> Self {
        SerializableDuration(duration)
    }
}

impl From<SerializableDuration> for Duration {
    fn from(serializable: SerializableDuration) -> Self {
        serializable.0
    }
}