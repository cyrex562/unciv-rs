use std::collections::HashMap;
use std::fmt;

use crate::models::ruleset::{
    Ruleset, RulesetObject, unique::{Unique, UniqueType, UniqueTarget, StateForConditionals},
};
use crate::models::civilization::Civilization;
use crate::models::map_unit::MapUnit;
use crate::models::ui::KeyCharAndCode;
use crate::models::civilopedia::ICivilopediaText;

/// Represents a presentation style for an event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presentation {
    /// Does not display a popup, choice chosen randomly
    None,
    /// Shows an alert popup
    Alert,
    /// Shows a floating notification
    Floating,
}

/// Represents a game event that can occur
pub struct Event {
    /// Base ruleset object that this event extends
    base: RulesetObject,

    /// The name of the event
    name: String,

    /// The uniques associated with this event
    uniques: Vec<String>,

    /// The presentation style of the event
    presentation: Presentation,

    /// The text description of the event
    text: String,

    /// The choices available for this event
    choices: Vec<EventChoice>,

    /// The ruleset this event belongs to
    ruleset: Option<Ruleset>,
}

impl Event {
    /// Creates a new empty event
    pub fn new() -> Self {
        Self {
            base: RulesetObject::new(),
            name: String::new(),
            uniques: Vec::new(),
            presentation: Presentation::Alert,
            text: String::new(),
            choices: Vec::new(),
            ruleset: None,
        }
    }

    /// Gets the matching choices for this event based on the current state
    /// Returns None when no choice passes the condition tests
    /// Returns an empty list when the event has no choices and conditions are fulfilled
    pub fn get_matching_choices(&self, state_for_conditionals: &StateForConditionals) -> Option<Vec<&EventChoice>> {
        if !self.is_available(state_for_conditionals) {
            return None;
        }

        if self.choices.is_empty() {
            return Some(Vec::new());
        }

        let matching_choices: Vec<&EventChoice> = self.choices.iter()
            .filter(|choice| choice.matches_conditions(state_for_conditionals))
            .collect();

        if matching_choices.is_empty() {
            None
        } else {
            Some(matching_choices)
        }
    }

    /// Checks if this event is available based on the current state
    pub fn is_available(&self, state_for_conditionals: &StateForConditionals) -> bool {
        // Check that all OnlyAvailable uniques have their conditions met
        let only_available_uniques = self.get_matching_uniques(UniqueType::OnlyAvailable, &StateForConditionals::ignore_conditionals());
        if only_available_uniques.iter().any(|unique| !unique.conditionals_apply(state_for_conditionals)) {
            return false;
        }

        // Check that there are no Unavailable uniques that apply
        let unavailable_uniques = self.get_matching_uniques(UniqueType::Unavailable, state_for_conditionals);
        unavailable_uniques.is_empty()
    }
}

impl RulesetObject for Event {
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
        UniqueTarget::Event
    }

    fn make_link(&self) -> String {
        format!("Event/{}", self.name)
    }

    fn get_civilopedia_text_lines(&self, ruleset: &Ruleset) -> Vec<String> {
        vec![self.text.clone()]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Represents a choice that can be made for an event
pub struct EventChoice {
    /// Base ruleset object that this choice extends
    base: RulesetObject,

    /// The name of the choice
    name: String,

    /// The uniques associated with this choice
    uniques: Vec<String>,

    /// The text description of the choice
    text: String,

    /// The keyboard shortcut for this choice
    key_shortcut: String,

    /// The ruleset this choice belongs to
    ruleset: Option<Ruleset>,
}

impl EventChoice {
    /// Creates a new empty event choice
    pub fn new() -> Self {
        Self {
            base: RulesetObject::new(),
            name: String::new(),
            uniques: Vec::new(),
            text: String::new(),
            key_shortcut: String::new(),
            ruleset: None,
        }
    }

    /// Checks if this choice matches the conditions for the current state
    pub fn matches_conditions(&self, state_for_conditionals: &StateForConditionals) -> bool {
        // Check that there are no Unavailable uniques that apply
        if self.has_unique(UniqueType::Unavailable, state_for_conditionals) {
            return false;
        }

        // Check that all OnlyAvailable uniques have their conditions met
        let only_available_uniques = self.get_matching_uniques(UniqueType::OnlyAvailable, &StateForConditionals::ignore_conditionals());
        if only_available_uniques.iter().any(|unique| !unique.conditionals_apply(state_for_conditionals)) {
            return false;
        }

        true
    }

    /// Triggers this choice for a civilization and optional unit
    pub fn trigger_choice(&self, civ: &mut Civilization, unit: Option<&MapUnit>) -> bool {
        let mut success = false;
        let state_for_conditionals = StateForConditionals::new(civ, unit);

        // Get all triggerable uniques
        let trigger_uniques: Vec<&Unique> = self.unique_objects().iter()
            .filter(|unique| unique.is_triggerable())
            .collect();

        // Trigger each unique
        for unique in trigger_uniques.iter().flat_map(|unique| unique.get_multiplied(&state_for_conditionals)) {
            if unique.trigger_unique(civ, unit) {
                success = true;
            }
        }

        success
    }
}

impl RulesetObject for EventChoice {
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
        UniqueTarget::EventChoice
    }

    fn make_link(&self) -> String {
        String::new()
    }

    fn get_civilopedia_text_lines(&self, ruleset: &Ruleset) -> Vec<String> {
        vec![self.text.clone()]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ICivilopediaText for EventChoice {
    fn get_civilopedia_text_lines(&self, ruleset: &Ruleset) -> Vec<String> {
        self.get_civilopedia_text_lines(ruleset)
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl fmt::Display for EventChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::new();
        assert_eq!(event.name, "");
        assert_eq!(event.text, "");
        assert_eq!(event.presentation, Presentation::Alert);
        assert!(event.choices.is_empty());
    }

    #[test]
    fn test_event_choice_creation() {
        let choice = EventChoice::new();
        assert_eq!(choice.name, "");
        assert_eq!(choice.text, "");
        assert_eq!(choice.key_shortcut, "");
    }

    #[test]
    fn test_event_availability() {
        let event = Event::new();
        let state = StateForConditionals::empty_state();
        assert!(event.is_available(&state));
    }

    #[test]
    fn test_event_choice_matching() {
        let choice = EventChoice::new();
        let state = StateForConditionals::empty_state();
        assert!(choice.matches_conditions(&state));
    }
}