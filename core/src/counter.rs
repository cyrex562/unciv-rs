use std::collections::HashMap;
use std::fmt;
use serde::{Serialize, Deserialize};

/// A specialized map that stores non-zero integers.
/// - All mutating methods will remove keys when their value is zeroed
/// - Getting a nonexistent key returns 0
/// - Implements serialization for compact format
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Counter<K: Eq + std::hash::Hash + Clone + fmt::Display> {
    counts: HashMap<K, i32>,
}

impl<K: Eq + std::hash::Hash + Clone + fmt::Display> Counter<K> {
    /// Creates a new empty Counter
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }

    /// Creates a Counter from an existing map
    pub fn from_map(map: HashMap<K, i32>) -> Self {
        let mut counter = Self::new();
        for (key, value) in map {
            if value != 0 {
                counter.counts.insert(key, value);
            }
        }
        counter
    }

    /// Gets the count for a key, returning 0 if the key doesn't exist
    pub fn get(&self, key: &K) -> i32 {
        *self.counts.get(key).unwrap_or(&0)
    }

    /// Sets the count for a key, removing it if the value is 0
    pub fn set(&mut self, key: K, value: i32) {
        if value == 0 {
            self.counts.remove(&key);
        } else {
            self.counts.insert(key, value);
        }
    }

    /// Adds a value to the count for a key
    pub fn add(&mut self, key: K, value: i32) {
        let current = self.get(&key);
        self.set(key, current + value);
    }

    /// Adds all counts from another Counter
    pub fn add_counter(&mut self, other: &Counter<K>) {
        for (key, value) in &other.counts {
            self.add(key.clone(), *value);
        }
    }

    /// Removes all counts from another Counter
    pub fn remove_counter(&mut self, other: &Counter<K>) {
        for (key, value) in &other.counts {
            self.add(key.clone(), -value);
        }
    }

    /// Multiplies all counts by a factor
    pub fn multiply(&self, amount: i32) -> Counter<K> {
        let mut new_counter = Counter::new();
        for (key, value) in &self.counts {
            new_counter.set(key.clone(), value * amount);
        }
        new_counter
    }

    /// Returns the sum of all values
    pub fn sum_values(&self) -> i32 {
        self.counts.values().sum()
    }

    /// Returns true if the counter is empty
    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }

    /// Returns the number of entries in the counter
    pub fn len(&self) -> usize {
        self.counts.len()
    }

    /// Returns a reference to the underlying HashMap
    pub fn as_map(&self) -> &HashMap<K, i32> {
        &self.counts
    }
}

// Implement standard operators
impl<K: Eq + std::hash::Hash + Clone + fmt::Display> std::ops::Add for Counter<K> {
    type Output = Counter<K>;

    fn add(self, other: Counter<K>) -> Counter<K> {
        let mut result = self.clone();
        result.add_counter(&other);
        result
    }
}

impl<K: Eq + std::hash::Hash + Clone + fmt::Display> std::ops::AddAssign for Counter<K> {
    fn add_assign(&mut self, other: Counter<K>) {
        self.add_counter(&other);
    }
}

impl<K: Eq + std::hash::Hash + Clone + fmt::Display> std::ops::Sub for Counter<K> {
    type Output = Counter<K>;

    fn sub(self, other: Counter<K>) -> Counter<K> {
        let mut result = self.clone();
        result.remove_counter(&other);
        result
    }
}

impl<K: Eq + std::hash::Hash + Clone + fmt::Display> std::ops::SubAssign for Counter<K> {
    fn sub_assign(&mut self, other: Counter<K>) {
        self.remove_counter(&other);
    }
}

impl<K: Eq + std::hash::Hash + Clone + fmt::Display> std::ops::Mul<i32> for Counter<K> {
    type Output = Counter<K>;

    fn mul(self, amount: i32) -> Counter<K> {
        self.multiply(amount)
    }
}

// Special case for String keys
impl Counter<String> {
    /// Creates a zero Counter that cannot be modified
    pub fn zero() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }
}

// Implement Display for debugging
impl<K: Eq + std::hash::Hash + Clone + fmt::Display> fmt::Display for Counter<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Counter {{")?;
        for (key, value) in &self.counts {
            write!(f, " {}: {}", key, value)?;
        }
        write!(f, " }}")
    }
}

// Implement Debug
impl<K: Eq + std::hash::Hash + Clone + fmt::Display> fmt::Debug for Counter<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Counter {{")?;
        for (key, value) in &self.counts {
            write!(f, " {}: {}", key, value)?;
        }
        write!(f, " }}")
    }
}