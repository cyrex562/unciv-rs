use serde::{Serialize, Deserialize};
use crate::models::ruleset::{Ruleset, RulesetObject, UniqueTarget};
use crate::ui::screens::civilopediascreen::FormattedLine;

/// Container for json-read "Tutorial" text, potentially decorated.
/// Two types for now - triggered (which can be hidden from Civilopedia via the usual unique) and Civilopedia-only.
/// Triggered ones are displayed in a Popup, the relation is via `name` (the enum name cleaned by dropping leading '_'
/// and replacing other '_' with blanks must match a json entry name exactly).
///
/// Has access to the full power of uniques for:
/// * Easier integration into Civilopedia
/// * HiddenWithoutReligion, HiddenFromCivilopedia work _directly_
/// * Future expansion - other meta tests to display or not are thinkable,
///   e.g. modders may want to hide instructions until you discover the game element?
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tutorial {
    /// The name of the tutorial
    pub name: String,
    /// The uniques associated with this tutorial
    pub uniques: Vec<String>,
    /// These lines will be displayed (when the Tutorial is _triggered_) one after another,
    /// and the Tutorial is marked as completed only once the last line is dismissed with "OK"
    /// TODO migrate to civilopediaText then remove or deprecate?
    pub steps: Option<Vec<String>>,
}

impl Tutorial {
    /// Create a new Tutorial instance
    pub fn new() -> Self {
        Self {
            name: String::new(),
            uniques: Vec::new(),
            steps: None,
        }
    }
}

impl RulesetObject for Tutorial {
    fn get_unique_target(&self) -> UniqueTarget {
        UniqueTarget::Tutorial
    }

    fn make_link(&self) -> String {
        format!("Tutorial/{}", self.name)
    }

    fn get_civilopedia_text_lines(&self, _ruleset: &Ruleset) -> Vec<FormattedLine> {
        self.steps.as_ref()
            .map(|steps| steps.iter().map(|step| FormattedLine::new(step.clone())).collect())
            .unwrap_or_default()
    }
}