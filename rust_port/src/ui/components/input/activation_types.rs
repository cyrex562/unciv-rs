use std::collections::HashSet;

/// Formal encapsulation of the input interaction types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActivationTypes {
    /// Keyboard keystroke activation
    Keystroke,

    /// Single tap activation
    Tap,

    /// Double tap activation
    Doubletap,

    /// Triple tap activation (just to clarify it ends here)
    Tripletap,

    /// Right click activation
    RightClick,

    /// Double right click activation
    DoubleRightClick,

    /// Long press activation
    Longpress,
}

impl ActivationTypes {
    /// Gets the tap count for this activation type
    pub fn tap_count(&self) -> i32 {
        match self {
            ActivationTypes::Keystroke => 0,
            ActivationTypes::Tap => 1,
            ActivationTypes::Doubletap => 2,
            ActivationTypes::Tripletap => 3,
            ActivationTypes::RightClick => 1,
            ActivationTypes::DoubleRightClick => 2,
            ActivationTypes::Longpress => 0,
        }
    }

    /// Gets the button for this activation type
    pub fn button(&self) -> i32 {
        match self {
            ActivationTypes::Keystroke => 0,
            ActivationTypes::Tap => 0,
            ActivationTypes::Doubletap => 0,
            ActivationTypes::Tripletap => 0,
            ActivationTypes::RightClick => 1,
            ActivationTypes::DoubleRightClick => 1,
            ActivationTypes::Longpress => 0,
        }
    }

    /// Returns true if this activation type is a gesture
    pub fn is_gesture(&self) -> bool {
        match self {
            ActivationTypes::Keystroke => false,
            _ => true,
        }
    }

    /// Gets the equivalent activation type for this activation type
    fn equivalent_to(&self) -> Option<ActivationTypes> {
        match self {
            ActivationTypes::Tap => Some(ActivationTypes::Keystroke),
            ActivationTypes::Longpress => Some(ActivationTypes::RightClick),
            _ => None,
        }
    }

    /// Checks whether two ActivationTypes are declared equivalent, e.g. RightClick and Longpress
    pub fn is_equivalent(&self, other: &ActivationTypes) -> bool {
        self == &other.equivalent_to().unwrap_or(*other) ||
        other == &self.equivalent_to().unwrap_or(*self)
    }

    /// Gets all activation types that are equivalent to the given type
    pub fn equivalent_values(type_: ActivationTypes) -> Vec<ActivationTypes> {
        let mut result = Vec::new();

        // Add the type itself
        result.push(type_);

        // Add the equivalent type if it exists
        if let Some(equivalent) = type_.equivalent_to() {
            result.push(equivalent);
        }

        // Add types that have this type as their equivalent
        for value in ActivationTypes::values() {
            if value.equivalent_to() == Some(type_) {
                result.push(value);
            }
        }

        result
    }

    /// Gets all activation types that are gestures
    pub fn gestures() -> Vec<ActivationTypes> {
        ActivationTypes::values()
            .into_iter()
            .filter(|t| t.is_gesture())
            .collect()
    }

    /// Gets all values of the ActivationTypes enum
    pub fn values() -> Vec<ActivationTypes> {
        vec![
            ActivationTypes::Keystroke,
            ActivationTypes::Tap,
            ActivationTypes::Doubletap,
            ActivationTypes::Tripletap,
            ActivationTypes::RightClick,
            ActivationTypes::DoubleRightClick,
            ActivationTypes::Longpress,
        ]
    }
}