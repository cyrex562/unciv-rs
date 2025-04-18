use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::ui::components::input::keyboard_binding::{KeyboardBinding, KeyCharAndCode};

/// Manage user-configurable keyboard bindings
///
/// A primary instance lives in the game settings and is read/write accessible
/// through the `KeyboardBindings[]` syntax.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeyboardBindings {
    /// The map of bindings to key codes
    #[serde(flatten)]
    bindings: HashMap<KeyboardBinding, KeyCharAndCode>,
}

impl KeyboardBindings {
    /// Create a new empty KeyboardBindings
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Add a binding by name and value string
    fn put_by_name(&mut self, name: &str, value: &str) {
        if let Some(binding) = KeyboardBinding::values().into_iter().find(|b| format!("{:?}", b) == name) {
            self.put(binding, value);
        }
    }

    /// Add a binding by KeyboardBinding and value string
    /// An empty value resets the binding to default
    pub fn put(&mut self, binding: KeyboardBinding, value: &str) {
        if value.is_empty() {
            self.bindings.remove(&binding);
        } else {
            if let Some(key) = KeyCharAndCode::parse(value) {
                if key != KeyCharAndCode::UNKNOWN {
                    self.put_key(binding, key);
                }
            }
        }
    }

    /// Add or replace a binding or remove it if value is the default for the binding
    /// Returns the previously bound key if any
    pub fn put_key(&mut self, binding: KeyboardBinding, value: KeyCharAndCode) -> Option<KeyCharAndCode> {
        let result = self.bindings.get(&binding).copied();

        if binding.default_key() == value {
            self.bindings.remove(&binding);
        } else {
            self.bindings.insert(binding, value);
        }

        result
    }

    /// Get a binding, returns the default key for missing entries
    pub fn get(&self, binding: KeyboardBinding) -> KeyCharAndCode {
        self.bindings.get(&binding).copied().unwrap_or_else(|| binding.default_key())
    }

    /// Get all bindings
    pub fn iter(&self) -> impl Iterator<Item = (&KeyboardBinding, &KeyCharAndCode)> {
        self.bindings.iter()
    }

    /// Get all bindings mutably
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&KeyboardBinding, &mut KeyCharAndCode)> {
        self.bindings.iter_mut()
    }

    /// Clear all bindings
    pub fn clear(&mut self) {
        self.bindings.clear();
    }

    /// Remove a binding
    pub fn remove(&mut self, binding: KeyboardBinding) -> Option<KeyCharAndCode> {
        self.bindings.remove(&binding)
    }

    /// Check if a binding exists
    pub fn contains_key(&self, binding: KeyboardBinding) -> bool {
        self.bindings.contains_key(&binding)
    }

    /// Get the number of bindings
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Check if there are no bindings
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

impl std::ops::Index<KeyboardBinding> for KeyboardBindings {
    type Output = KeyCharAndCode;

    fn index(&self, binding: KeyboardBinding) -> &Self::Output {
        // This is a bit tricky since we need to return a reference to a value that might not exist
        // We'll use a static value for the default key
        static DEFAULT_KEY: std::sync::OnceLock<KeyCharAndCode> = std::sync::OnceLock::new();

        if let Some(key) = self.bindings.get(&binding) {
            key
        } else {
            // This is a hack - we're returning a reference to a static value
            // In a real implementation, you might want to handle this differently
            DEFAULT_KEY.get_or_init(|| binding.default_key())
        }
    }
}

impl std::ops::IndexMut<KeyboardBinding> for KeyboardBindings {
    fn index_mut(&mut self, binding: KeyboardBinding) -> &mut Self::Output {
        // This is also tricky - we need to ensure the key exists
        if !self.bindings.contains_key(&binding) {
            self.bindings.insert(binding, binding.default_key());
        }

        self.bindings.get_mut(&binding).unwrap()
    }
}

/// Global access to keyboard bindings
pub struct GlobalKeyboardBindings;

impl GlobalKeyboardBindings {
    /// Get the default keyboard bindings
    pub fn default() -> &'static KeyboardBindings {
        // In a real implementation, this would access the game settings
        // For now, we'll return a static instance
        static DEFAULT: std::sync::OnceLock<KeyboardBindings> = std::sync::OnceLock::new();
        DEFAULT.get_or_init(KeyboardBindings::new)
    }

    /// Get a binding
    pub fn get(binding: KeyboardBinding) -> KeyCharAndCode {
        Self::default()[binding]
    }

    /// Set a binding
    pub fn set(binding: KeyboardBinding, key: KeyCharAndCode) {
        // In a real implementation, this would modify the game settings
        // For now, we'll just log that this was called
        println!("Setting global keyboard binding {:?} to {:?}", binding, key);
    }
}

// Implement serialization/deserialization
impl serde::Serialize for KeyboardBindings {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.bindings.len()))?;

        for (binding, key) in &this.bindings {
            map.serialize_entry(&format!("{:?}", binding), key)?;
        }

        map.end()
    }
}

impl<'de> serde::Deserialize<'de> for KeyboardBindings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map: HashMap<String, KeyCharAndCode> = HashMap::deserialize(deserializer)?;
        let mut bindings = HashMap::new();

        for (name, key) in map {
            if let Some(binding) = KeyboardBinding::values().into_iter().find(|b| format!("{:?}", b) == name) {
                bindings.insert(binding, key);
            }
        }

        Ok(Self { bindings })
    }
}