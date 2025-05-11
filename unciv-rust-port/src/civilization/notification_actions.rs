use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::constants::NO_ID;
use crate::models::map::{MapUnit, TileMap};
use crate::models::civilization::Civilization;
use crate::ui::screens::{
    cityscreen::CityScreen,
    diplomacyscreen::DiplomacyScreen,
    overviewscreen::{EmpireOverviewCategories, EmpireOverviewScreen, EspionageOverviewScreen},
    pickerscreens::{PolicyPickerScreen, PromotionPickerScreen, TechPickerScreen},
    worldscreen::WorldScreen,
};
use crate::ui::components::maya_calendar::MayaCalendar;
use crate::utils::math::Vector2;

/// Trait for notification actions that can be executed when a user clicks on a notification
pub trait NotificationAction: Send + Sync {
    fn execute(&self, world_screen: &mut WorldScreen);
}

/// A notification action that shows map places
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationAction {
    location: Vector2,
}

impl LocationAction {
    pub fn new(location: Vector2) -> Self {
        Self { location }
    }

    /// Creates a sequence of LocationActions from a sequence of locations
    pub fn from_locations(locations: &[Vector2]) -> Vec<Box<dyn NotificationAction>> {
        locations.iter()
            .map(|&loc| Box::new(Self::new(loc)) as Box<dyn NotificationAction>)
            .collect()
    }
}

impl NotificationAction for LocationAction {
    fn execute(&self, world_screen: &mut WorldScreen) {
        world_screen.map_holder.set_center_position(self.location, false);
    }
}

/// A notification action that shows the tech screen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechAction {
    tech_name: String,
}

impl TechAction {
    pub fn new(tech_name: String) -> Self {
        Self { tech_name }
    }
}

impl NotificationAction for TechAction {
    fn execute(&self, world_screen: &mut WorldScreen) {
        let tech = world_screen.game_info.ruleset.technologies.get(&self.tech_name);
        if let Some(tech) = tech {
            world_screen.game.push_screen(Box::new(TechPickerScreen::new(
                world_screen.selected_civ.clone(),
                tech.clone(),
            )));
        }
    }
}

/// A notification action that enters a city
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CityAction {
    city: Vector2,
}

impl CityAction {
    pub fn new(city: Vector2) -> Self {
        Self { city }
    }

    /// Creates a list of actions for a city (location and city action)
    pub fn with_location(city: &City) -> Vec<Box<dyn NotificationAction>> {
        vec![
            Box::new(LocationAction::new(city.location)),
            Box::new(Self::new(city.location)),
        ]
    }
}

impl NotificationAction for CityAction {
    fn execute(&self, world_screen: &mut WorldScreen) {
        if let Some(city) = world_screen.map_holder.tile_map.get_city_at(self.city) {
            if city.civ == world_screen.viewing_civ {
                world_screen.game.push_screen(Box::new(CityScreen::new(city.clone())));
            }
        }
    }
}

/// A notification action that enters the diplomacy screen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiplomacyAction {
    other_civ_name: String,
    show_trade: bool,
}

impl DiplomacyAction {
    pub fn new(other_civ_name: String, show_trade: bool) -> Self {
        Self {
            other_civ_name,
            show_trade,
        }
    }
}

impl NotificationAction for DiplomacyAction {
    fn execute(&self, world_screen: &mut WorldScreen) {
        let current_civ = world_screen.selected_civ.clone();
        let other_civ = world_screen.game_info.get_civilization(&self.other_civ_name);

        // Check if we can show trade
        let mut show_trade = self.show_trade;

        if show_trade && other_civ == current_civ {
            // Can't trade with yourself
            show_trade = false;
        }

        // Can't trade with city-states
        if show_trade && (other_civ.is_city_state || current_civ.is_city_state) {
            show_trade = false;
        }

        // Can't trade while at war
        if show_trade && current_civ.is_at_war_with(&other_civ) {
            show_trade = false;
        }

        world_screen.game.push_screen(Box::new(DiplomacyScreen::new(
            current_civ,
            other_civ,
            show_trade,
        )));
    }
}

/// A notification action that shows the Maya Long Count popup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MayaLongCountAction;

impl NotificationAction for MayaLongCountAction {
    fn execute(&self, world_screen: &mut WorldScreen) {
        MayaCalendar::open_popup(world_screen, world_screen.selected_civ.clone(), world_screen.game_info.get_year());
    }
}

/// A notification action that shows and selects things on the map
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapUnitAction {
    location: Vector2,
    id: i32,
}

impl MapUnitAction {
    pub fn new(location: Vector2, id: i32) -> Self {
        Self { location, id }
    }

    pub fn from_unit(unit: &MapUnit) -> Self {
        Self::new(unit.current_tile.position, unit.id)
    }

    /// Creates a sequence of MapUnitActions from a sequence of units
    pub fn from_units(units: &[MapUnit]) -> Vec<Box<dyn NotificationAction>> {
        units.iter()
            .map(|unit| Box::new(Self::from_unit(unit)) as Box<dyn NotificationAction>)
            .collect()
    }
}

impl NotificationAction for MapUnitAction {
    fn execute(&self, world_screen: &mut WorldScreen) {
        let select_unit = self.id != NO_ID;
        let unit = if !select_unit {
            world_screen.game_info.tile_map.get_units_at(self.location)
                .iter()
                .find(|u| u.id == self.id)
                .cloned()
        } else {
            None
        };

        world_screen.map_holder.set_center_position(
            self.location,
            select_unit,
            unit,
        );
    }
}

/// A notification action that shows a Civilopedia entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivilopediaAction {
    link: String,
}

impl CivilopediaAction {
    pub fn new(link: String) -> Self {
        Self { link }
    }
}

impl NotificationAction for CivilopediaAction {
    fn execute(&self, world_screen: &mut WorldScreen) {
        world_screen.open_civilopedia(&self.link);
    }
}

/// A notification action that shows the promotion picker for a unit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromoteUnitAction {
    name: String,
    location: Vector2,
}

impl PromoteUnitAction {
    pub fn new(name: String, location: Vector2) -> Self {
        Self { name, location }
    }
}

impl NotificationAction for PromoteUnitAction {
    fn execute(&self, world_screen: &mut WorldScreen) {
        if let Some(tile) = world_screen.game_info.tile_map.get_tile_at(self.location) {
            if let Some(unit) = tile.military_unit.as_ref() {
                if unit.name == self.name && unit.civ == world_screen.selected_civ {
                    world_screen.game.push_screen(Box::new(PromotionPickerScreen::new(unit.clone())));
                }
            }
        }
    }
}

/// A notification action that opens the Empire Overview to a specific page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverviewAction {
    page: EmpireOverviewCategories,
    select: String,
}

impl OverviewAction {
    pub fn new(page: EmpireOverviewCategories, select: String) -> Self {
        Self { page, select }
    }
}

impl NotificationAction for OverviewAction {
    fn execute(&self, world_screen: &mut WorldScreen) {
        world_screen.game.push_screen(Box::new(EmpireOverviewScreen::new(
            world_screen.selected_civ.clone(),
            self.page,
            self.select.clone(),
        )));
    }
}

/// A notification action that opens the policy picker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyAction {
    select: Option<String>,
}

impl PolicyAction {
    pub fn new(select: Option<String>) -> Self {
        Self { select }
    }
}

impl NotificationAction for PolicyAction {
    fn execute(&self, world_screen: &mut WorldScreen) {
        world_screen.game.push_screen(Box::new(PolicyPickerScreen::new(
            world_screen.selected_civ.clone(),
            world_screen.can_change_state,
            self.select.clone(),
        )));
    }
}

/// A notification action that opens the Espionage Overview screen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EspionageAction;

impl EspionageAction {
    /// Creates a sequence of actions with location and espionage
    pub fn with_location(location: Option<Vector2>) -> Vec<Box<dyn NotificationAction>> {
        let mut actions = Vec::new();
        if let Some(loc) = location {
            actions.push(Box::new(LocationAction::new(loc)));
        }
        actions.push(Box::new(Self));
        actions
    }
}

impl NotificationAction for EspionageAction {
    fn execute(&self, world_screen: &mut WorldScreen) {
        world_screen.game.push_screen(Box::new(EspionageOverviewScreen::new(
            world_screen.selected_civ.clone(),
            world_screen,
        )));
    }
}

/// A notification action that opens a URL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkAction {
    url: String,
}

impl LinkAction {
    pub fn new(url: String) -> Self {
        Self { url }
    }
}

impl NotificationAction for LinkAction {
    fn execute(&self, world_screen: &mut WorldScreen) {
        if !self.url.is_empty() {
            // TODO: Implement URL opening
            // In Kotlin: Gdx.net.openURI(url)
        }
    }
}

/// Deserializer for notification actions
pub struct NotificationActionsDeserializer;

impl NotificationActionsDeserializer {
    pub fn deserialize(data: &serde_json::Value) -> Vec<Box<dyn NotificationAction>> {
        let mut actions = Vec::new();

        // This is a simplified implementation
        // In the actual code, we would need to parse the JSON structure
        // and create the appropriate action types

        if let Some(action_obj) = data.as_object() {
            for (key, value) in action_obj {
                match key.as_str() {
                    "LocationAction" => {
                        if let Some(location) = value.get("location") {
                            if let (Some(x), Some(y)) = (
                                location.get("x").and_then(|v| v.as_i64()),
                                location.get("y").and_then(|v| v.as_i64()),
                            ) {
                                actions.push(Box::new(LocationAction::new(Vector2::new(x as f32, y as f32))));
                            }
                        }
                    }
                    "TechAction" => {
                        if let Some(tech_name) = value.get("techName").and_then(|v| v.as_str()) {
                            actions.push(Box::new(TechAction::new(tech_name.to_string())));
                        }
                    }
                    "CityAction" => {
                        if let Some(city) = value.get("city") {
                            if let (Some(x), Some(y)) = (
                                city.get("x").and_then(|v| v.as_i64()),
                                city.get("y").and_then(|v| v.as_i64()),
                            ) {
                                actions.push(Box::new(CityAction::new(Vector2::new(x as f32, y as f32))));
                            }
                        }
                    }
                    "DiplomacyAction" => {
                        if let (Some(other_civ_name), Some(show_trade)) = (
                            value.get("otherCivName").and_then(|v| v.as_str()),
                            value.get("showTrade").and_then(|v| v.as_bool()),
                        ) {
                            actions.push(Box::new(DiplomacyAction::new(
                                other_civ_name.to_string(),
                                show_trade,
                            )));
                        }
                    }
                    "MayaLongCountAction" => {
                        actions.push(Box::new(MayaLongCountAction));
                    }
                    "MapUnitAction" => {
                        if let (Some(location), Some(id)) = (
                            value.get("location").and_then(|loc| {
                                if let (Some(x), Some(y)) = (
                                    loc.get("x").and_then(|v| v.as_i64()),
                                    loc.get("y").and_then(|v| v.as_i64()),
                                ) {
                                    Some(Vector2::new(x as f32, y as f32))
                                } else {
                                    None
                                }
                            }),
                            value.get("id").and_then(|v| v.as_i64()),
                        ) {
                            actions.push(Box::new(MapUnitAction::new(location, id as i32)));
                        }
                    }
                    "CivilopediaAction" => {
                        if let Some(link) = value.get("link").and_then(|v| v.as_str()) {
                            actions.push(Box::new(CivilopediaAction::new(link.to_string())));
                        }
                    }
                    "PromoteUnitAction" => {
                        if let (Some(name), Some(location)) = (
                            value.get("name").and_then(|v| v.as_str()),
                            value.get("location").and_then(|loc| {
                                if let (Some(x), Some(y)) = (
                                    loc.get("x").and_then(|v| v.as_i64()),
                                    loc.get("y").and_then(|v| v.as_i64()),
                                ) {
                                    Some(Vector2::new(x as f32, y as f32))
                                } else {
                                    None
                                }
                            }),
                        ) {
                            actions.push(Box::new(PromoteUnitAction::new(
                                name.to_string(),
                                location,
                            )));
                        }
                    }
                    "OverviewAction" => {
                        if let (Some(page), Some(select)) = (
                            value.get("page").and_then(|v| v.as_str()),
                            value.get("select").and_then(|v| v.as_str()),
                        ) {
                            let page = match page {
                                "Resources" => EmpireOverviewCategories::Resources,
                                "Cities" => EmpireOverviewCategories::Cities,
                                "Units" => EmpireOverviewCategories::Units,
                                "Diplomacy" => EmpireOverviewCategories::Diplomacy,
                                "Religion" => EmpireOverviewCategories::Religion,
                                "Espionage" => EmpireOverviewCategories::Espionage,
                                "Victory" => EmpireOverviewCategories::Victory,
                                _ => EmpireOverviewCategories::Resources,
                            };
                            actions.push(Box::new(OverviewAction::new(
                                page,
                                select.to_string(),
                            )));
                        }
                    }
                    "PolicyAction" => {
                        let select = value.get("select").and_then(|v| v.as_str()).map(|s| s.to_string());
                        actions.push(Box::new(PolicyAction::new(select)));
                    }
                    "EspionageAction" => {
                        actions.push(Box::new(EspionageAction));
                    }
                    "LinkAction" => {
                        if let Some(url) = value.get("url").and_then(|v| v.as_str()) {
                            actions.push(Box::new(LinkAction::new(url.to_string())));
                        }
                    }
                    _ => {}
                }
            }
        }

        actions
    }
}