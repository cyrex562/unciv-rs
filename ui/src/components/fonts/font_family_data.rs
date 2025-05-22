use std::fmt;
use crate::models::translations::tr;

/// Font family data with local and invariant names
///
/// If save in `GameSettings` need use invariant_family.
/// If show to user need use local_name.
/// If save local_name in `GameSettings` may generate garbled characters by encoding.
#[derive(Debug, Clone)]
pub struct FontFamilyData {
    /// The localized name of the font family
    pub local_name: String,
    /// The invariant name of the font family (used for saving in settings)
    pub invariant_name: String,
    /// Optional file path to the font file
    pub file_path: Option<String>,
}

impl FontFamilyData {
    /// Creates a new FontFamilyData instance
    pub fn new(local_name: String, invariant_name: Option<String>, file_path: Option<String>) -> Self {
        Self {
            local_name: local_name.clone(),
            invariant_name: invariant_name.unwrap_or(local_name),
            file_path,
        }
    }

    /// Creates a default FontFamilyData instance
    pub fn default() -> Self {
        Self::new(
            "Default Font".to_string(),
            Some("Default".to_string()),
            None,
        )
    }
}

impl Default for FontFamilyData {
    fn default() -> Self {
        Self::default()
    }
}

impl PartialEq for FontFamilyData {
    /// Implement equality such that _only_ the invariant_name field is compared
    fn eq(&self, other: &Self) -> bool {
        self.invariant_name == other.invariant_name
    }
}

impl Eq for FontFamilyData {}

impl std::hash::Hash for FontFamilyData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.invariant_name.hash(state);
    }
}

impl fmt::Display for FontFamilyData {
    /// For SelectBox usage
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", tr(&self.local_name))
    }
}