use crate::civilization::{Civilization, AlertType, CivilopediaAction, NotificationCategory, PopupAlert};
use crate::models::ruleset::unique::{UniqueType, UniqueTriggerActivation};
use crate::utils::extensions::ToPercent;
use std::sync::Arc;
use serde::{Serialize, Deserialize};

/// Manages golden ages for a civilization
#[derive(Clone, Serialize, Deserialize)]
pub struct GoldenAgeManager {
    /// Reference to the civilization this manager belongs to
    #[serde(skip)]
    pub civ_info: Option<Arc<Civilization>>,

    /// Happiness points stored for the next golden age
    pub stored_happiness: i32,

    /// Number of golden ages experienced so far
    pub number_of_golden_ages: i32,

    /// Turns remaining in the current golden age
    pub turns_left_for_current_golden_age: i32,
}

impl GoldenAgeManager {
    /// Creates a new GoldenAgeManager
    pub fn new() -> Self {
        Self {
            civ_info: None,
            stored_happiness: 0,
            number_of_golden_ages: 0,
            turns_left_for_current_golden_age: 0,
        }
    }

    /// Sets the transient references to the civilization
    pub fn set_transients(&mut self, civ_info: Arc<Civilization>) {
        self.civ_info = Some(civ_info);
    }

    /// Checks if the civilization is currently in a golden age
    pub fn is_golden_age(&self) -> bool {
        self.turns_left_for_current_golden_age > 0
    }

    /// Adds happiness points to the stored happiness
    pub fn add_happiness(&mut self, amount: i32) {
        self.stored_happiness += amount;
    }

    /// Calculates the happiness required for the next golden age
    pub fn happiness_required_for_next_golden_age(&self) -> i32 {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");

        let mut cost = (500.0 + self.number_of_golden_ages as f32 * 250.0);
        cost *= civ_info.cities.len() as f32 / 100.0; // https://forums.civfanatics.com/resources/complete-guide-to-happiness-vanilla.25584/
        cost *= civ_info.game_info.speed.modifier;

        cost as i32
    }

    /// Calculates the length of a golden age based on modifiers
    pub fn calculate_golden_age_length(&self, unmodified_number_of_turns: i32) -> i32 {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");

        let mut turns_to_golden_age = unmodified_number_of_turns as f32;

        for unique in civ_info.get_matching_uniques(UniqueType::GoldenAgeLength) {
            turns_to_golden_age *= unique.params[0].parse::<f32>().unwrap_or(1.0) / 100.0;
        }

        turns_to_golden_age *= civ_info.game_info.speed.golden_age_length_modifier;

        turns_to_golden_age as i32
    }

    /// Enters a golden age with the specified duration
    pub fn enter_golden_age(&mut self, unmodified_number_of_turns: i32) {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");

        self.turns_left_for_current_golden_age += self.calculate_golden_age_length(unmodified_number_of_turns);

        civ_info.add_notification(
            "You have entered a Golden Age!",
            CivilopediaAction::new("Tutorial/Golden Age"),
            NotificationCategory::General,
            "StatIcons/Happiness"
        );

        civ_info.popup_alerts.push(PopupAlert::new(AlertType::GoldenAge, ""));

        for unique in civ_info.get_triggered_uniques(UniqueType::TriggerUponEnteringGoldenAge) {
            UniqueTriggerActivation::trigger_unique(unique, civ_info.clone());
        }

        // Golden Age can happen mid turn with Great Artist effects
        for city in &mut civ_info.cities {
            city.city_stats.update();
        }
    }

    /// Processes end-of-turn actions for golden age management
    pub fn end_turn(&mut self, happiness: i32) {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");

        if !self.is_golden_age() {
            self.stored_happiness = (self.stored_happiness + happiness).max(0);
        }

        if self.is_golden_age() {
            self.turns_left_for_current_golden_age -= 1;
        } else if self.stored_happiness > self.happiness_required_for_next_golden_age() {
            self.stored_happiness -= self.happiness_required_for_next_golden_age();
            self.enter_golden_age(10); // Default duration
            self.number_of_golden_ages += 1;
        }
    }
}