use std::error::Error;
use std::fmt;
use std::iter::Take;
use std::slice::Iter;

/// A trait for errors that can be shown to the user.
///
/// This trait provides a way to get a localized error message that can be displayed to the user.
pub trait UncivShowableError: Error {
    /// Returns a localized version of the error message.
    ///
    /// This should be implemented to return a translated version of the error message.
    fn get_localized_message(&self) -> String;
}

/// An error wrapper marking an error as suitable to be shown to the user.
///
/// # Arguments
///
/// * `error_text` - The untranslated error message.
///   Use `get_localized_message()` to get the translated message.
///   Usual formatting (`[] or {}`) applies, as does the need to include the text in templates.properties.
pub struct UncivShowableException {
    error_text: String,
    cause: Option<Box<dyn Error + Send + Sync>>,
}

impl UncivShowableException {
    /// Creates a new `UncivShowableException` with the given error text and optional cause.
    pub fn new(error_text: String, cause: Option<Box<dyn Error + Send + Sync>>) -> Self {
        UncivShowableException { error_text, cause }
    }
}

impl fmt::Display for UncivShowableException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error_text)
    }
}

impl Error for UncivShowableException {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause.as_ref().map(|e| e.as_ref() as &dyn Error)
    }
}

impl UncivShowableError for UncivShowableException {
    fn get_localized_message(&self) -> String {
        // In a real implementation, this would use a translation system
        // For now, we'll just return the error text
        self.error_text.clone()
    }
}

/// An error indicating a game or map cannot be loaded because mods are missing.
///
/// # Arguments
///
/// * `missing_mods` - An iterator of strings representing the missing mods.
///   The error message will only include up to the first five missing mods.
pub struct MissingModsException {
    missing_mods: Vec<String>,
}

impl MissingModsException {
    /// Creates a new `MissingModsException` with the given missing mods.
    pub fn new<I>(missing_mods: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mods: Vec<String> = missing_mods.into_iter().collect();
        MissingModsException { missing_mods: mods }
    }

    /// Returns an iterator over the missing mods.
    pub fn missing_mods(&self) -> &[String] {
        &self.missing_mods
    }

    /// Returns a shortened list of missing mods (up to 5).
    fn shorten(&self) -> String {
        self.missing_mods.iter()
            .take(5)
            .cloned()
            .collect::<Vec<String>>()
            .join(", ")
    }
}

impl fmt::Display for MissingModsException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Missing mods: [{}]", self.shorten())
    }
}

impl Error for MissingModsException {}

impl UncivShowableError for MissingModsException {
    fn get_localized_message(&self) -> String {
        // In a real implementation, this would use a translation system
        // For now, we'll just return the error message
        format!("Missing mods: [{}]", self.shorten())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unciv_showable_exception() {
        let exception = UncivShowableException::new(
            "Test error".to_string(),
            None,
        );
        assert_eq!(exception.to_string(), "Test error");
        assert_eq!(exception.get_localized_message(), "Test error");
    }

    #[test]
    fn test_missing_mods_exception() {
        let missing_mods = vec!["mod1".to_string(), "mod2".to_string(), "mod3".to_string()];
        let exception = MissingModsException::new(missing_mods);
        assert_eq!(exception.to_string(), "Missing mods: [mod1, mod2, mod3]");
        assert_eq!(exception.get_localized_message(), "Missing mods: [mod1, mod2, mod3]");
    }

    #[test]
    fn test_missing_mods_exception_with_many_mods() {
        let missing_mods = vec![
            "mod1".to_string(), "mod2".to_string(), "mod3".to_string(),
            "mod4".to_string(), "mod5".to_string(), "mod6".to_string(),
        ];
        let exception = MissingModsException::new(missing_mods);
        assert_eq!(exception.to_string(), "Missing mods: [mod1, mod2, mod3, mod4, mod5]");
        assert_eq!(exception.get_localized_message(), "Missing mods: [mod1, mod2, mod3, mod4, mod5]");
    }
}