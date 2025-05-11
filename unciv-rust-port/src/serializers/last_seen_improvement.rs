use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use nalgebra::Vector2;
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::de::{self, Visitor, MapAccess};
use std::fmt;
use std::ops::{Deref, DerefMut};

/// A wrapper for Vector2 that implements proper serialization to/from string format "(x,y)"
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SerializableVector2(pub Vector2<f32>);

impl Hash for SerializableVector2 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Convert to string format for consistent hashing
        self.to_pretty_string().hash(state);
    }
}

impl Eq for SerializableVector2 {}

impl SerializableVector2 {
    pub fn to_pretty_string(&self) -> String {
        format!("({},{})", self.0.x, self.0.y)
    }

    pub fn from_pretty_string(s: &str) -> Option<Self> {
        let s = s.trim_start_matches('(').trim_end_matches(')');
        let mut parts = s.split(',');
        let x = parts.next()?.parse().ok()?;
        let y = parts.next()?.parse().ok()?;
        Some(SerializableVector2(Vector2::new(x, y)))
    }
}

impl Serialize for SerializableVector2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_pretty_string())
    }
}

impl<'de> Deserialize<'de> for SerializableVector2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Vector2Visitor;

        impl<'de> Visitor<'de> for Vector2Visitor {
            type Value = SerializableVector2;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string in format \"(x,y)\"")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                SerializableVector2::from_pretty_string(value)
                    .ok_or_else(|| de::Error::custom("invalid Vector2 format"))
            }
        }

        deserializer.deserialize_str(Vector2Visitor)
    }
}

/// A HashMap wrapper that handles Vector2 keys with special serialization
#[derive(Debug, Clone, Default)]
pub struct LastSeenImprovement(HashMap<SerializableVector2, String>);

impl LastSeenImprovement {
    pub fn new() -> Self {
        LastSeenImprovement(HashMap::new())
    }
}

impl Deref for LastSeenImprovement {
    type Target = HashMap<SerializableVector2, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LastSeenImprovement {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Serialize for LastSeenImprovement {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(&k.to_pretty_string(), v)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for LastSeenImprovement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LastSeenImprovementVisitor;

        impl<'de> Visitor<'de> for LastSeenImprovementVisitor {
            type Value = LastSeenImprovement;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map")
            }

            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut map = LastSeenImprovement::new();

                // Check for old format
                if let Some((key, value)) = access.next_entry::<String, serde_json::Value>()? {
                    if key == "class" && value.as_str() == Some("com.unciv.json.HashMapVector2") {
                        // Handle old format
                        while let Some((_, entry)) = access.next_entry::<String, Vec<serde_json::Value>>()? {
                            if entry.len() == 2 {
                                if let (Some(key_str), Some(value)) = (entry[0].as_str(), entry[1].as_str()) {
                                    if let Some(key) = SerializableVector2::from_pretty_string(key_str) {
                                        map.insert(key, value.to_string());
                                    }
                                }
                            }
                        }
                        return Ok(map);
                    }

                    // Not old format, handle first entry
                    if let Some(key) = SerializableVector2::from_pretty_string(&key) {
                        if let Some(value) = value.as_str() {
                            map.insert(key, value.to_string());
                        }
                    }
                }

                // Handle remaining entries
                while let Some((key, value)) = access.next_entry::<String, String>()? {
                    if let Some(key) = SerializableVector2::from_pretty_string(&key) {
                        map.insert(key, value);
                    }
                }

                Ok(map)
            }
        }

        deserializer.deserialize_map(LastSeenImprovementVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_serialization() {
        let mut map = LastSeenImprovement::new();
        map.insert(
            SerializableVector2(Vector2::new(1.0, 2.0)),
            "test".to_string(),
        );

        let serialized = serde_json::to_string(&map).unwrap();
        assert_eq!(serialized, r#"{"(1,2)":"test"}"#);
    }

    #[test]
    fn test_deserialization() {
        let json = r#"{"(1,2)":"test"}"#;
        let map: LastSeenImprovement = serde_json::from_str(json).unwrap();
        assert_eq!(
            map.get(&SerializableVector2(Vector2::new(1.0, 2.0))).unwrap(),
            "test"
        );
    }

    #[test]
    fn test_old_format_deserialization() {
        let json = json!({
            "class": "com.unciv.json.HashMapVector2",
            "entries": [["(1,2)", "test"]]
        });
        let map: LastSeenImprovement = serde_json::from_value(json).unwrap();
        assert_eq!(
            map.get(&SerializableVector2(Vector2::new(1.0, 2.0))).unwrap(),
            "test"
        );
    }
}