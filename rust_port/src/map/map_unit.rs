use std::collections::HashMap;
use crate::utils::Vector2;
use crate::constants::Constants;
use crate::logic::civilization::Civilization;
use crate::logic::city::City;
use crate::map::tile::Tile;
use crate::models::ruleset::Ruleset;
use crate::models::ruleset::unit::{BaseUnit, UnitType};
use crate::models::ruleset::unique::{UniqueType, UniqueMap, StateForConditionals};
use crate::models::UnitActionType;
use crate::models::ruleset::tile::TileImprovement;
use crate::ui::components::UnitMovementMemoryType;
use crate::map::mapunit::movement::UnitMovement;

/// Represents a unit's movement memory - used for movement arrow overlay
#[derive(Clone)]
pub struct UnitMovementMemory {
    pub position: Vector2,
    pub movement_type: UnitMovementMemoryType,
}

impl UnitMovementMemory {
    pub fn new(position: Vector2, movement_type: UnitMovementMemoryType) -> Self {
        Self {
            position: Vector2::new(position.x, position.y),
            movement_type,
        }
    }
}

/// Represents a unit's status effect
pub struct UnitStatus {
    pub name: String,
    pub turns_left: i32,
    #[allow(dead_code)]
    uniques: Vec<Unique>,
}

impl UnitStatus {
    pub fn new(name: String, turns: i32) -> Self {
        Self {
            name,
            turns_left: turns,
            uniques: Vec::new(),
        }
    }

    pub fn set_transients(&mut self, unit: &MapUnit) {
        self.uniques = unit.civ.game_info.ruleset.unit_promotions
            .get(&self.name)
            .map(|promotion| promotion.unique_objects.clone())
            .unwrap_or_default();
    }
}

/// The immutable properties and mutable game state of an individual unit present on the map
pub struct MapUnit {
    // Persisted fields
    pub owner: String,
    pub original_owner: Option<String>,
    pub name: String,
    pub instance_name: Option<String>,
    pub current_movement: f32,
    pub health: i32,
    pub id: i32,
    pub action: Option<String>,
    pub automated: bool,
    pub escorting: bool,
    pub automated_road_connection_destination: Option<Vector2>,
    pub automated_road_connection_path: Option<Vec<Vector2>>,
    pub attacks_this_turn: i32,
    pub promotions: UnitPromotions,
    pub due: bool,
    pub is_transported: bool,
    pub turns_fortified: i32,
    pub ability_to_times_used: HashMap<String, i32>,
    pub religion: Option<String>,
    pub religious_strength_lost: i32,
    pub movement_memories: Vec<UnitMovementMemory>,
    pub most_recent_move_type: UnitMovementMemoryType,
    pub attacks_since_turn_start: Vec<Vector2>,
    pub status_map: HashMap<String, UnitStatus>,

    // Transient fields
    pub civ: Civilization,
    pub base_unit: BaseUnit,
    pub current_tile: Tile,
    temp_uniques_map: UniqueMap,
    non_unit_uniques_map: UniqueMap,
    pub movement: UnitMovement,
    pub is_destroyed: bool,
    pub cache: MapUnitCache,
    pub viewable_tiles: HashSet<Tile>,
}

impl MapUnit {
    pub fn new() -> Self {
        Self {
            owner: String::new(),
            original_owner: None,
            name: String::new(),
            instance_name: None,
            current_movement: 0.0,
            health: 100,
            id: Constants::NO_ID,
            action: None,
            automated: false,
            escorting: false,
            automated_road_connection_destination: None,
            automated_road_connection_path: None,
            attacks_this_turn: 0,
            promotions: UnitPromotions::new(),
            due: true,
            is_transported: false,
            turns_fortified: 0,
            ability_to_times_used: HashMap::new(),
            religion: None,
            religious_strength_lost: 0,
            movement_memories: Vec::new(),
            most_recent_move_type: UnitMovementMemoryType::UnitMoved,
            attacks_since_turn_start: Vec::new(),
            status_map: HashMap::new(),
            civ: Civilization::new(),
            base_unit: BaseUnit::new(),
            current_tile: Tile::new(),
            temp_uniques_map: UniqueMap::new(),
            non_unit_uniques_map: UniqueMap::new(),
            movement: UnitMovement::new(),
            is_destroyed: false,
            cache: MapUnitCache::new(),
            viewable_tiles: HashSet::new(),
        }
    }

    /// Gets the unit's display name for UI
    pub fn display_name(&self) -> String {
        let base_name = match &self.instance_name {
            Some(instance) => format!("{} ({})", instance, self.name),
            None => format!("[{}]", self.name),
        };

        match &self.religion {
            Some(religion) => format!("{} ({})", base_name, self.get_religion_display_name()),
            None => base_name,
        }
    }

    /// Gets a shorter display name for the unit
    pub fn short_display_name(&self) -> String {
        match &self.instance_name {
            Some(instance) => format!("[{}]", instance),
            None => format!("[{}]", self.name),
        }
    }

    /// Gets the unit's movement string for display
    pub fn get_movement_string(&self) -> String {
        format!("{:.1}/{}", self.current_movement, self.get_max_movement())
    }

    /// Gets the unit's current tile
    pub fn get_tile(&self) -> &Tile {
        &self.current_tile
    }

    /// Gets the closest city to the unit
    pub fn get_closest_city(&self) -> Option<&City> {
        self.civ.cities.iter()
            .min_by_key(|city| city.get_center_tile().aerial_distance_to(&self.current_tile))
    }

    /// Checks if the unit is military
    pub fn is_military(&self) -> bool {
        self.base_unit.is_military
    }

    /// Checks if the unit is civilian
    pub fn is_civilian(&self) -> bool {
        self.base_unit.is_civilian()
    }

    /// Checks if the unit is fortified
    pub fn is_fortified(&self) -> bool {
        self.status_map.contains_key("Fortified")
    }

    /// Gets the number of fortification turns
    pub fn get_fortification_turns(&self) -> i32 {
        if !(self.is_fortified() || self.is_guarding()) {
            return 0;
        }
        self.turns_fortified
    }

    /// Checks if the unit is sleeping
    pub fn is_sleeping(&self) -> bool {
        self.action.as_ref().map_or(false, |a| a.starts_with(UnitActionType::Sleep.value()))
    }

    /// Checks if the unit is moving
    pub fn is_moving(&self) -> bool {
        self.action.as_ref().map_or(false, |a| a.starts_with("moveTo"))
    }

    /// Gets the unit's movement destination if moving
    pub fn get_movement_destination(&self) -> Option<&Tile> {
        self.action.as_ref().and_then(|action| {
            let coords: Vec<&str> = action.replace("moveTo ", "").split(',').collect();
            if coords.len() != 2 {
                return None;
            }
            let x = coords[0].parse::<f32>().ok()?;
            let y = coords[1].parse::<f32>().ok()?;
            Some(self.current_tile.tile_map.get_tile_at_position(&Vector2::new(x, y)))
        })
    }

    /// Returns true if the unit can move through the given tile
    pub fn can_pass_through(&self, tile: &Tile) -> bool {
        // Units can always pass through tiles they can enter
        if self.can_enter(tile) {
            return true;
        }

        // Check cache for special movement abilities
        if self.cache.can_pass_through_impassable_tiles {
            return true;
        }

        false
    }

    /// Returns true if the unit can enter the given tile
    pub fn can_enter(&self, tile: &Tile) -> bool {
        // Units cannot enter tiles owned by other civilizations unless they have permission
        if !self.cache.can_enter_foreign_tiles && tile.owner.is_some() && tile.owner != Some(self.owner.clone()) {
            return false;
        }

        // Check terrain-specific movement restrictions
        match tile.base_terrain.as_str() {
            "Ocean" => self.cache.can_enter_ocean_tiles,
            "Ice" => self.cache.can_enter_ice_tiles,
            _ => true,
        }
    }

    /// Returns the movement cost for entering the given tile
    pub fn get_movement_cost(&self, from_tile: &Tile, to_tile: &Tile) -> f32 {
        // If unit ignores terrain cost, movement cost is always 1
        if self.cache.ignore_terrain_cost {
            return 1.0;
        }

        let mut cost = to_tile.movement_cost;

        // Apply terrain-specific movement modifiers
        if to_tile.is_coastal() && self.cache.double_movement_in_coast {
            cost *= 0.5;
        }
        if to_tile.has_forest() && self.cache.double_movement_in_forest {
            cost *= 0.5;
        }
        if to_tile.has_jungle() && self.cache.double_movement_in_jungle {
            cost *= 0.5;
        }
        if to_tile.is_snow() && self.cache.double_movement_in_snow {
            cost *= 0.5;
        }
        if to_tile.is_tundra() && self.cache.double_movement_in_tundra {
            cost *= 0.5;
        }

        // Apply rough terrain penalty if applicable
        if to_tile.is_rough() {
            cost += self.cache.rough_terrain_penalty;
        }

        // Apply embarked movement bonus if applicable
        if self.is_embarked() && self.cache.embarked_movement_bonus > 0.0 {
            cost -= self.cache.embarked_movement_bonus;
        }

        // Movement cost cannot be less than 0.1
        cost.max(0.1)
    }

    /// Returns true if the unit can move to the given tile with its remaining movement
    pub fn can_move_to(&self, from_tile: &Tile, to_tile: &Tile) -> bool {
        if !self.can_enter(to_tile) {
            return false;
        }

        let cost = self.get_movement_cost(from_tile, to_tile);
        self.current_movement >= cost
    }

    /// Returns true if the unit is currently embarked (on water)
    pub fn is_embarked(&self) -> bool {
        self.status_map.contains_key("Embarked")
    }

    /// Returns true if the unit can embark (move onto water tiles)
    pub fn can_embark(&self) -> bool {
        self.uniques.contains("Can embark")
    }

    /// Returns true if the unit can move after performing an attack
    pub fn can_move_after_attacking(&self) -> bool {
        self.cache.can_move_after_attacking
    }

    /// Returns true if the unit ignores zone of control
    pub fn ignores_zone_of_control(&self) -> bool {
        self.cache.ignore_zone_of_control
    }

    /// Returns true if the unit can paradrop
    pub fn can_paradrop(&self) -> bool {
        self.cache.can_paradrop
    }

    /// Returns the unit's sight range
    pub fn get_sight_distance(&self) -> i32 {
        self.cache.sight_distance + self.cache.extended_sight_distance
    }

    /// Returns true if the unit can see over obstacles (like hills)
    pub fn can_see_over_obstacles(&self) -> bool {
        self.cache.can_see_over_obstacles
    }

    /// Updates the unit's unique abilities
    pub fn update_uniques(&mut self) {
        let other_unique_sources = self.promotions.get_promotions()
            .iter()
            .flat_map(|p| p.unique_objects.clone())
            .chain(self.status_map.values().flat_map(|s| s.uniques.clone()));

        let unique_sources = self.base_unit.ruleset_unique_objects
            .iter()
            .chain(other_unique_sources);

        self.temp_uniques_map = UniqueMap::from_sequence(unique_sources.clone());
        self.non_unit_uniques_map = UniqueMap::from_sequence(other_unique_sources);
        self.cache.update_uniques();
    }

    /// Gets all unique abilities for the unit
    pub fn get_uniques(&self) -> impl Iterator<Item = &Unique> {
        self.temp_uniques_map.get_all_uniques()
    }

    /// Gets matching unique abilities of a specific type
    pub fn get_matching_uniques(
        &self,
        unique_type: UniqueType,
        state_for_conditionals: &StateForConditionals,
        check_civ_info_uniques: bool,
    ) -> impl Iterator<Item = &Unique> {
        let unit_uniques = self.temp_uniques_map.get_matching_uniques(
            unique_type,
            state_for_conditionals,
        );

        if check_civ_info_uniques {
            unit_uniques.chain(
                self.civ.get_matching_uniques(unique_type, state_for_conditionals)
            )
        } else {
            unit_uniques
        }
    }

    /// Returns the unit's base combat strength
    pub fn get_base_strength(&self) -> f32 {
        self.cache.base_strength
    }

    /// Returns the unit's base ranged strength
    pub fn get_base_ranged_strength(&self) -> f32 {
        self.cache.base_ranged_strength
    }

    /// Returns true if the unit can perform melee attacks
    pub fn can_melee(&self) -> bool {
        self.cache.can_melee
    }

    /// Returns true if the unit can perform ranged attacks
    pub fn can_ranged(&self) -> bool {
        self.cache.can_ranged
    }

    /// Returns the unit's range for ranged attacks
    pub fn get_range(&self) -> i32 {
        self.cache.range
    }

    /// Returns true if the unit can attack the target unit
    pub fn can_attack(&self, target: &MapUnit) -> bool {
        // Cannot attack own units
        if self.owner == target.owner {
            return false;
        }

        // Check if we have any attack capability
        if !self.can_melee() && !self.can_ranged() {
            return false;
        }

        // Check if target is valid based on attack type
        if self.can_ranged() {
            // For ranged units, check if target is within range
            // Range calculation would be done elsewhere based on tile positions
            true
        } else {
            // For melee units, check if target is adjacent
            // Adjacent check would be done elsewhere based on tile positions
            true
        }
    }

    /// Returns the unit's current health
    pub fn get_health(&self) -> i32 {
        self.health
    }

    /// Returns true if the unit is at full health
    pub fn is_at_full_health(&self) -> bool {
        self.health >= 100
    }

    /// Returns true if the unit can heal
    pub fn can_heal(&self) -> bool {
        !self.cache.cannot_heal && self.health < 100
    }

    /// Returns the amount of healing the unit receives per turn
    pub fn get_healing_rate(&self) -> i32 {
        if !self.can_heal() {
            return 0;
        }

        let mut healing = if self.is_fortified() {
            20 // Base healing when fortified
        } else {
            10 // Base healing when not fortified
        };

        // Apply healing modifiers from status effects
        if self.status_map.contains_key("HealingBonus") {
            healing += 5;
        }

        healing
    }

    /// Returns true if the unit can fortify
    pub fn can_fortify(&self) -> bool {
        self.cache.can_fortify
    }

    /// Returns true if the unit is garrisoned in a city
    pub fn is_garrisoned(&self) -> bool {
        self.status_map.contains_key("Garrisoned")
    }

    /// Returns true if the unit can garrison in cities
    pub fn can_garrison(&self) -> bool {
        self.cache.can_garrison
    }

    /// Returns the unit's experience level
    pub fn get_experience_level(&self) -> i32 {
        self.experience_level
    }

    /// Returns true if the unit can gain experience
    pub fn can_gain_experience(&self) -> bool {
        self.cache.can_gain_experience
    }

    /// Returns the unit's current experience points
    pub fn get_experience(&self) -> i32 {
        self.experience
    }

    /// Returns the experience points needed for the next level
    pub fn get_experience_for_next_level(&self) -> i32 {
        10 * (self.experience_level + 1) // Simple formula, adjust as needed
    }

    // More methods will be implemented here
}