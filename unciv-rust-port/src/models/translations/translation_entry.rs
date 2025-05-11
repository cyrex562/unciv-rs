use std::collections::HashMap;

/// One 'translatable' string
///
/// # Properties
/// * `entry`: Original translatable string as defined in the game,
///            including [placeholders] or {subsentences}
/// * `translations`: Map of language codes to translations
///
/// # See also
/// * `Translations`
#[derive(Debug, Clone)]
pub struct TranslationEntry {
    /// Original translatable string
    pub entry: String,
    /// Map of language codes to translations
    translations: HashMap<String, String>,
}

impl TranslationEntry {
    /// Create a new TranslationEntry with the given original string
    pub fn new(entry: String) -> Self {
        Self {
            entry,
            translations: HashMap::new(),
        }
    }

    /// Get a translation for a specific language
    pub fn get(&self, language: &str) -> Option<&String> {
        self.translations.get(language)
    }

    /// Add or update a translation for a specific language
    pub fn insert(&mut self, language: String, translation: String) -> Option<String> {
        self.translations.insert(language, translation)
    }

    /// Remove a translation for a specific language
    pub fn remove(&mut self, language: &str) -> Option<String> {
        self.translations.remove(language)
    }

    /// Check if a translation exists for a specific language
    pub fn contains_key(&self, language: &str) -> bool {
        self.translations.contains_key(language)
    }

    /// Get all language codes that have translations
    pub fn languages(&self) -> impl Iterator<Item = &String> {
        self.translations.keys()
    }

    /// Get all translations
    pub fn translations(&self) -> impl Iterator<Item = (&String, &String)> {
        self.translations.iter().map(|(k, v)| (k, v))
    }

    /// Get the number of translations
    pub fn len(&self) -> usize {
        self.translations.len()
    }

    /// Check if there are any translations
    pub fn is_empty(&self) -> bool {
        self.translations.is_empty()
    }

    /// Clear all translations
    pub fn clear(&mut self) {
        self.translations.clear();
    }
}

// Implement HashMap-like methods for backward compatibility
impl TranslationEntry {
    /// Get a mutable reference to a translation
    pub fn get_mut(&mut self, language: &str) -> Option<&mut String> {
        self.translations.get_mut(language)
    }

    /// Get a translation or insert a default value
    pub fn entry(&mut self, language: String) -> std::collections::hash_map::Entry<String, String> {
        self.translations.entry(language)
    }

    /// Get all language codes
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.translations.keys()
    }

    /// Get all translations as mutable references
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&String, &mut String)> {
        self.translations.iter_mut().map(|(k, v)| (k, v))
    }

    /// Get all translations
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.translations.iter().map(|(k, v)| (k, v))
    }

    /// Get all translations as owned values
    pub fn into_iter(self) -> impl Iterator<Item = (String, String)> {
        self.translations.into_iter()
    }
}