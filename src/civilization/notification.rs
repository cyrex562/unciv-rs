use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::models::ruleset::Ruleset;
use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::components::image_getter::ImageGetter;
use crate::ui::components::table::Table;
use crate::ui::components::actor::Actor;

/// Category - UI grouping, within a Category the most recent Notification will be shown on top
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NotificationCategory {
    // These names are displayed, so remember to add a translation template
    // - if there's no other source for one.
    General,
    Trade,
    Diplomacy,
    Production,
    Units,
    War,
    Religion,
    Espionage,
    Cities,
}

impl NotificationCategory {
    pub fn safe_value_of(name: &str) -> Option<Self> {
        match name {
            "General" => Some(Self::General),
            "Trade" => Some(Self::Trade),
            "Diplomacy" => Some(Self::Diplomacy),
            "Production" => Some(Self::Production),
            "Units" => Some(Self::Units),
            "War" => Some(Self::War),
            "Religion" => Some(Self::Religion),
            "Espionage" => Some(Self::Espionage),
            "Cities" => Some(Self::Cities),
            _ => None,
        }
    }
}

/// Actions that can be performed when clicking a notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationAction {
    LocationAction { location: (i32, i32) },
    CivilopediaAction { link: String },
    CityAction { city: (i32, i32) },
}

impl NotificationAction {
    pub fn execute(&self, world_screen: &mut WorldScreen) {
        match self {
            Self::LocationAction { location } => {
                // TODO: Implement location action
            }
            Self::CivilopediaAction { link } => {
                // TODO: Implement civilopedia action
            }
            Self::CityAction { city } => {
                // TODO: Implement city action
            }
        }
    }
}

/// Represents a game notification with text, icons, and actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// Category - UI grouping, within a Category the most recent Notification will be shown on top
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<NotificationCategory>,

    /// The notification text, untranslated - will be translated on the fly
    #[serde(skip_serializing_if = "String::is_empty")]
    text: String,

    /// Icons to be shown
    #[serde(skip_serializing_if = "Vec::is_empty")]
    icons: Vec<String>,

    /// Actions on clicking a Notification - will be activated round-robin style
    #[serde(skip_serializing_if = "Vec::is_empty")]
    actions: Vec<NotificationAction>,

    /// For round-robin activation in execute
    #[serde(skip)]
    index: usize,
}

impl Notification {
    pub fn new(
        text: String,
        notification_icons: &[String],
        actions: Option<&[NotificationAction]>,
        category: Option<NotificationCategory>,
    ) -> Self {
        Self {
            category: category.unwrap_or(NotificationCategory::General),
            text,
            icons: notification_icons.to_vec(),
            actions: actions.map_or_else(Vec::new, |a| a.to_vec()),
            index: 0,
        }
    }

    pub fn add_notification_icons_to(&self, table: &mut Table, ruleset: &Ruleset, icon_size: f32) {
        if self.icons.is_empty() {
            return;
        }

        for icon in self.icons.iter().rev() {
            let image: Actor = if ruleset.technologies.contains_key(icon) {
                ImageGetter::get_tech_icon_portrait(icon, icon_size)
            } else if ruleset.nations.contains_key(icon) {
                ImageGetter::get_nation_portrait(ruleset.nations.get(icon).unwrap(), icon_size)
            } else if ruleset.units.contains_key(icon) {
                ImageGetter::get_unit_icon(ruleset.units.get(icon).unwrap())
            } else {
                ImageGetter::get_image(icon)
            };
            table.add(image).size(icon_size).pad_right(5.0);
        }
    }

    pub fn execute(&mut self, world_screen: &mut WorldScreen) {
        if self.actions.is_empty() {
            return;
        }
        self.actions[self.index].execute(world_screen);
        self.index = (self.index + 1) % self.actions.len(); // cycle through actions
    }

    pub fn reset_execute_round_robin(&mut self) {
        self.index = 0;
    }

    // Getters
    pub fn category(&self) -> NotificationCategory {
        self.category.unwrap_or(NotificationCategory::General)
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn icons(&self) -> &[String] {
        &self.icons
    }

    pub fn actions(&self) -> &[NotificationAction] {
        &self.actions
    }
}

impl Default for Notification {
    fn default() -> Self {
        Self {
            category: Some(NotificationCategory::General),
            text: String::new(),
            icons: Vec::new(),
            actions: Vec::new(),
            index: 0,
        }
    }
}