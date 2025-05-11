use std::any::Any;
use std::collections::HashMap;
use std::fmt;

use crate::constants::Constants;
use crate::models::ruleset::{
    Ruleset, RulesetObject,
    unique::{UniqueTarget, uniques_to_civilopedia_text_lines}
};
use crate::models::translations::tr;
use crate::ui::object_descriptions::FormattedLine;
use crate::ui::screens::civilopedia_screen::FormattedLine as CivilopediaFormattedLine;
use crate::UncivGame;

/// Represents a religious belief in the game.
pub struct Belief {
    /// The name of the belief
    name: String,

    /// The type of the belief
    belief_type: BeliefType,

    /// The uniques associated with this belief
    uniques: Vec<String>,

    /// The civilopedia text for this belief
    civilopedia_text: Option<Vec<CivilopediaFormattedLine>>,

    /// Whether this belief is hidden from the civilopedia
    hidden_from_civilopedia: bool,
}

impl Belief {
    /// Creates a new empty belief
    pub fn new() -> Self {
        Self {
            name: String::new(),
            belief_type: BeliefType::None,
            uniques: Vec::new(),
            civilopedia_text: None,
            hidden_from_civilopedia: false,
        }
    }

    /// Creates a new belief with the given type
    pub fn with_type(belief_type: BeliefType) -> Self {
        Self {
            name: String::new(),
            belief_type,
            uniques: Vec::new(),
            civilopedia_text: None,
            hidden_from_civilopedia: false,
        }
    }

    /// Gets the type of this belief
    pub fn belief_type(&self) -> &BeliefType {
        &self.belief_type
    }

    /// Sets the type of this belief
    pub fn set_belief_type(&mut self, belief_type: BeliefType) {
        self.belief_type = belief_type;
    }

    /// Gets the uniques associated with this belief
    pub fn uniques(&self) -> &[String] {
        &self.uniques
    }

    /// Gets the civilopedia text for this belief
    pub fn civilopedia_text(&self) -> Option<&[CivilopediaFormattedLine]> {
        self.civilopedia_text.as_deref()
    }

    /// Sets the civilopedia text for this belief
    pub fn set_civilopedia_text(&mut self, text: Vec<CivilopediaFormattedLine>) {
        self.civilopedia_text = Some(text);
    }

    /// Gets whether this belief is hidden from the civilopedia
    pub fn is_hidden_from_civilopedia(&self) -> bool {
        self.hidden_from_civilopedia
    }

    /// Sets whether this belief is hidden from the civilopedia
    pub fn set_hidden_from_civilopedia(&mut self, hidden: bool) {
        self.hidden_from_civilopedia = hidden;
    }

    /// Gets beliefs that match the given name in a unique parameter
    fn get_beliefs_matching(name: &str, ruleset: &Ruleset) -> Vec<&Belief> {
        ruleset.beliefs().values()
            .filter(|belief| !belief.is_hidden_from_civilopedia())
            .filter(|belief| {
                belief.unique_objects().iter().any(|unique| {
                    unique.params().iter().any(|param| param == name)
                })
            })
            .collect()
    }

    /// Gets civilopedia text lines for all beliefs referencing a given name in a unique parameter
    pub fn get_civilopedia_text_matching(
        name: &str,
        ruleset: &Ruleset,
        with_see_also: bool
    ) -> Vec<CivilopediaFormattedLine> {
        let matching_beliefs = Self::get_beliefs_matching(name, ruleset);
        if matching_beliefs.is_empty() {
            return Vec::new();
        }

        let mut lines = Vec::new();
        if with_see_also {
            lines.push(CivilopediaFormattedLine::new());
            lines.push(CivilopediaFormattedLine::with_text("{See also}:"));
        }

        for belief in matching_beliefs {
            lines.push(CivilopediaFormattedLine::with_link(
                belief.name().to_string(),
                belief.make_link(),
                1
            ));
        }

        lines
    }

    /// Gets the civilopedia religion entry
    pub fn get_civilopedia_religion_entry(ruleset: &Ruleset) -> Belief {
        let mut belief = Belief::new();
        belief.set_name("Religions".to_string());

        let mut lines = Vec::new();
        lines.push(CivilopediaFormattedLine::with_separator(true));

        // Sort religions by name
        let mut religions: Vec<_> = ruleset.religions().keys().collect();
        religions.sort_by(|a, b| {
            tr(a, true).cmp(&tr(b, true))
        });

        for religion in religions {
            lines.push(CivilopediaFormattedLine::with_icon(
                religion.to_string(),
                format!("Belief/{}", religion)
            ));
        }

        belief.set_civilopedia_text(lines);
        belief
    }
}

impl RulesetObject for Belief {
    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn uniques(&self) -> &[String] {
        &self.uniques
    }

    fn uniques_mut(&mut self) -> &mut Vec<String> {
        &mut self.uniques
    }

    fn get_unique_target(&self) -> UniqueTarget {
        if self.belief_type.is_founder() {
            UniqueTarget::FounderBelief
        } else {
            UniqueTarget::FollowerBelief
        }
    }

    fn make_link(&self) -> String {
        format!("Belief/{}", self.name)
    }

    fn get_civilopedia_text_header(&self) -> CivilopediaFormattedLine {
        let color = if self.belief_type == BeliefType::None {
            "#e34a2b".to_string()
        } else {
            String::new()
        };

        CivilopediaFormattedLine::with_header(
            self.name.to_string(),
            self.make_link(),
            2,
            color
        )
    }

    fn get_sort_group(&self, _ruleset: &Ruleset) -> i32 {
        self.belief_type as i32
    }

    fn get_civilopedia_text_lines(&self, _ruleset: &Ruleset) -> Vec<CivilopediaFormattedLine> {
        self.get_civilopedia_text_lines(false)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Belief {
    /// Gets the civilopedia text lines for this belief
    pub fn get_civilopedia_text_lines(&self, with_header: bool) -> Vec<CivilopediaFormattedLine> {
        let mut text_list = Vec::new();

        if with_header {
            text_list.push(CivilopediaFormattedLine::with_text(
                self.name.to_string(),
                Constants::heading_font_size(),
                true,
                Some(self.make_link())
            ));
            text_list.push(CivilopediaFormattedLine::new());
        }

        if self.belief_type != BeliefType::None {
            text_list.push(CivilopediaFormattedLine::with_text(
                format!("{{Type}}: {{{}}}", self.belief_type),
                self.belief_type.color().to_string(),
                with_header
            ));
        }

        uniques_to_civilopedia_text_lines(&mut text_list, None);

        text_list
    }
}

/// Subtypes of Beliefs - directly deserialized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BeliefType {
    /// No belief type
    None,

    /// Pantheon belief - processed per city
    Pantheon,

    /// Founder belief - processed globally for founding civ only
    Founder,

    /// Follower belief - processed per city
    Follower,

    /// Enhancer belief - processed globally for founding civ only
    Enhancer,

    /// Any belief type
    Any,
}

impl BeliefType {
    /// Gets the color associated with this belief type
    pub fn color(&self) -> &'static str {
        match self {
            BeliefType::None => "",
            BeliefType::Pantheon => "#44c6cc",
            BeliefType::Founder => "#c00000",
            BeliefType::Follower => "#ccaa44",
            BeliefType::Enhancer => "#72cc45",
            BeliefType::Any => "",
        }
    }

    /// Gets whether this belief type is a follower belief
    pub fn is_follower(&self) -> bool {
        matches!(self, BeliefType::Pantheon | BeliefType::Follower)
    }

    /// Gets whether this belief type is a founder belief
    pub fn is_founder(&self) -> bool {
        matches!(self, BeliefType::Founder | BeliefType::Enhancer)
    }
}

impl fmt::Display for BeliefType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BeliefType::None => write!(f, "None"),
            BeliefType::Pantheon => write!(f, "Pantheon"),
            BeliefType::Founder => write!(f, "Founder"),
            BeliefType::Follower => write!(f, "Follower"),
            BeliefType::Enhancer => write!(f, "Enhancer"),
            BeliefType::Any => write!(f, "Any"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_belief_type() {
        assert_eq!(BeliefType::None.color(), "");
        assert_eq!(BeliefType::Pantheon.color(), "#44c6cc");
        assert_eq!(BeliefType::Founder.color(), "#c00000");
        assert_eq!(BeliefType::Follower.color(), "#ccaa44");
        assert_eq!(BeliefType::Enhancer.color(), "#72cc45");
        assert_eq!(BeliefType::Any.color(), "");

        assert!(BeliefType::Pantheon.is_follower());
        assert!(BeliefType::Follower.is_follower());
        assert!(!BeliefType::Founder.is_follower());
        assert!(!BeliefType::Enhancer.is_follower());
        assert!(!BeliefType::None.is_follower());
        assert!(!BeliefType::Any.is_follower());

        assert!(BeliefType::Founder.is_founder());
        assert!(BeliefType::Enhancer.is_founder());
        assert!(!BeliefType::Pantheon.is_founder());
        assert!(!BeliefType::Follower.is_founder());
        assert!(!BeliefType::None.is_founder());
        assert!(!BeliefType::Any.is_founder());
    }

    #[test]
    fn test_belief() {
        let mut belief = Belief::new();
        belief.set_name("Test Belief".to_string());
        belief.set_belief_type(BeliefType::Pantheon);

        assert_eq!(belief.name(), "Test Belief");
        assert_eq!(*belief.belief_type(), BeliefType::Pantheon);
        assert_eq!(belief.make_link(), "Belief/Test Belief");

        let text_lines = belief.get_civilopedia_text_lines(true);
        assert!(!text_lines.is_empty());
        assert_eq!(text_lines[0].text(), "Test Belief");
    }
}