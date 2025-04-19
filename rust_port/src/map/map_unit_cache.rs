use std::collections::HashSet;

/// Cache for frequently accessed unit properties to avoid recalculating them
#[derive(Default)]
pub struct MapUnitCache {
    // Movement related
    pub can_pass_through_impassable_tiles: bool,
    pub can_enter_ice_tiles: bool,
    pub can_enter_ocean_tiles: bool,
    pub double_movement_in_coast: bool,
    pub double_movement_in_forest: bool,
    pub double_movement_in_jungle: bool,
    pub double_movement_in_snow: bool,
    pub double_movement_in_tundra: bool,
    pub ignore_zone_of_control: bool,
    pub ignore_terrain_cost: bool,
    pub rough_terrain_penalty: f32,
    pub embarked_movement_bonus: f32,

    // Combat related
    pub bonus_against_cities: i32,
    pub bonus_when_flanking: i32,
    pub heal_when_destroy_unit: i32,
    pub interceptor_strength_bonus: i32,
    pub no_defensive_bonuses: bool,
    pub additional_attack_per_turn: i32,

    // Visibility related
    pub sight_distance: i32,
    pub extended_sight_distance: i32,
    pub can_see_over_obstacles: bool,

    // Special abilities
    pub can_found_cities: bool,
    pub can_capture_cities: bool,
    pub can_carry_air_units: bool,
    pub can_move_after_attacking: bool,
    pub can_pillage_improvements: bool,
    pub can_paradrop: bool,
    pub can_enter_foreign_tiles: bool,
    pub must_set_up_to_range_attack: bool,
    pub cannot_capture_units: bool,

    // Unit type specific
    pub is_ranged: bool,
    pub is_nuclear_weapon: bool,
    pub is_great_person: bool,
    pub is_air_unit: bool,
    pub is_water_unit: bool,
    pub is_religious_unit: bool,

    // Other
    pub maintenance_cost: i32,
    pub resource_consumption: i32,
    pub uniques_cache: HashSet<String>,
}

impl MapUnitCache {
    pub fn new() -> Self {
        Self {
            can_pass_through_impassable_tiles: false,
            can_enter_ice_tiles: false,
            can_enter_ocean_tiles: false,
            double_movement_in_coast: false,
            double_movement_in_forest: false,
            double_movement_in_jungle: false,
            double_movement_in_snow: false,
            double_movement_in_tundra: false,
            ignore_zone_of_control: false,
            ignore_terrain_cost: false,
            rough_terrain_penalty: 0.0,
            embarked_movement_bonus: 0.0,

            bonus_against_cities: 0,
            bonus_when_flanking: 0,
            heal_when_destroy_unit: 0,
            interceptor_strength_bonus: 0,
            no_defensive_bonuses: false,
            additional_attack_per_turn: 0,

            sight_distance: 2,
            extended_sight_distance: 0,
            can_see_over_obstacles: false,

            can_found_cities: false,
            can_capture_cities: false,
            can_carry_air_units: false,
            can_move_after_attacking: false,
            can_pillage_improvements: false,
            can_paradrop: false,
            can_enter_foreign_tiles: false,
            must_set_up_to_range_attack: false,
            cannot_capture_units: false,

            is_ranged: false,
            is_nuclear_weapon: false,
            is_great_person: false,
            is_air_unit: false,
            is_water_unit: false,
            is_religious_unit: false,

            maintenance_cost: 0,
            resource_consumption: 0,
            uniques_cache: HashSet::new(),
        }
    }

    /// Updates the cache based on the unit's current state and uniques
    pub fn update_uniques(&mut self) {
        // This will be implemented to update all cached values based on unit uniques
        // For now it's a placeholder that will be filled in when we implement the unique system
    }
}