use std::iter::IntoIterator;

use crate::ui::screens::civilopediascreen::formatted_line::FormattedLine;
use crate::ui::screens::civilopediascreen::i_civilopedia_text::ICivilopediaText;
use crate::models::ruleset::Ruleset;

/// Storage struct for instantiation of the simplest form containing only the lines collection
pub struct SimpleCivilopediaText {
    /// The formatted lines to display
    pub civilopedia_text: Vec<FormattedLine>,
}

impl SimpleCivilopediaText {
    /// Create a new SimpleCivilopediaText with the given formatted lines
    pub fn new(civilopedia_text: Vec<FormattedLine>) -> Self {
        Self { civilopedia_text }
    }

    /// Create a new SimpleCivilopediaText from a sequence of strings
    pub fn from_strings(strings: impl IntoIterator<Item = String>) -> Self {
        Self {
            civilopedia_text: strings
                .into_iter()
                .map(|s| FormattedLine::new().with_text(s))
                .collect(),
        }
    }

    /// Create a new SimpleCivilopediaText from a sequence of formatted lines and strings
    pub fn from_lines_and_strings(
        first: impl IntoIterator<Item = FormattedLine>,
        strings: impl IntoIterator<Item = String>,
    ) -> Self {
        let mut lines: Vec<FormattedLine> = first.into_iter().collect();
        lines.extend(
            strings
                .into_iter()
                .map(|s| FormattedLine::new().with_text(s)),
        );
        Self { civilopedia_text: lines }
    }
}

impl ICivilopediaText for SimpleCivilopediaText {
    fn civilopedia_text(&self) -> Vec<FormattedLine> {
        self.civilopedia_text.clone()
    }

    fn make_link(&self) -> String {
        String::new()
    }
}