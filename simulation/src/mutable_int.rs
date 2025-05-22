use std::fmt;
use crate::models::translations::tr;

/// A wrapper for a mutable integer value
pub struct MutableInt {
    /// The underlying integer value
    value: i32,
}

impl MutableInt {
    /// Create a new MutableInt with the given value
    ///
    /// # Parameters
    ///
    /// * `value` - The initial value
    ///
    /// # Returns
    ///
    /// A new MutableInt
    pub fn new(value: i32) -> Self {
        Self { value }
    }

    /// Create a new MutableInt with a default value of 0
    ///
    /// # Returns
    ///
    /// A new MutableInt with value 0
    pub fn default() -> Self {
        Self { value: 0 }
    }

    /// Increment the value by 1
    pub fn inc(&mut self) {
        self.value += 1;
    }

    /// Get the current value
    ///
    /// # Returns
    ///
    /// The current value
    pub fn get(&self) -> i32 {
        self.value
    }

    /// Set the value to a new value
    ///
    /// # Parameters
    ///
    /// * `new_value` - The new value
    pub fn set(&mut self, new_value: i32) {
        self.value = new_value;
    }

    /// Add a value to the current value
    ///
    /// # Parameters
    ///
    /// * `addend` - The value to add
    pub fn add(&mut self, addend: i32) {
        self.value += addend;
    }
}

impl Default for MutableInt {
    fn default() -> Self {
        Self::default()
    }
}

impl fmt::Display for MutableInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", tr(&self.value.to_string()))
    }
}

impl fmt::Debug for MutableInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MutableInt({})", self.value)
    }
}

impl Clone for MutableInt {
    fn clone(&self) -> Self {
        Self { value: self.value }
    }
}

impl Copy for MutableInt {}

impl PartialEq for MutableInt {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for MutableInt {}