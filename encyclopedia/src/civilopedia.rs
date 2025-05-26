use crate::models::ruleset::Ruleset;
use crate::ui::screens::civilopediascreen::formatted_line::FormattedLine;

/// Civilopedia component for game objects
#[derive(Clone, Debug, Default)]
pub struct Civilopedia {
    /// The entry filename or link for this object
    pub entry: String,
}

impl Civilopedia {
    pub fn new(entry: String) -> Self {
        Self { entry }
    }

    pub fn make_link(&self) -> String {
        format!("docs/civilopedia/{}", self.entry)
    }

    pub fn get_civilopedia_text_lines(&self, _ruleset: &Ruleset) -> Vec<FormattedLine> {
        // In a real implementation, this would load and parse the file
        vec![FormattedLine::new(format!("See Civilopedia entry: {}", self.entry), 0, 0)]
    }
}
