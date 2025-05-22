use crate::city::City;
use crate::civilization::transients::CivInfoTransientCache;
use crate::civilization::{Civilization, MapUnitAction, NotificationCategory};
use crate::map::mapunit::MapUnit;
use crate::map::tile::Tile;
use crate::models::ruleset::unique::{UniqueTriggerActivation, UniqueType};
use crate::models::ruleset::unit::BaseUnit;
use crate::utils::UncivGame;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

/// Make sure all MapUnits have the starting promotions that they're supposed to.
pub fn guarantee_unit_promotions(game_info: &mut GameInfo) {
    for tile in game_info.tile_map.values() {
        for unit in tile.get_units() {
            for starting_promo in &unit.base_unit.promotions {
                unit.promotions.add_promotion(starting_promo, true);
            }
        }
    }
}

/// Convert from Fortify X to Fortify and save off X
pub fn convert_fortify(game_info: &mut GameInfo) {
    let reg = Regex::new(r"^Fortify\s+(\d+)([\w\s]*)").unwrap();

    for civ in &mut game_info.civilizations {
        for unit in civ.units.get_civ_units_mut() {
            if let Some(action) = &unit.action {
                if let Some(caps) = reg.captures(action) {
                    let turns = caps.get(1).unwrap().as_str().parse::<i32>().unwrap();
                    let heal = caps.get(2).unwrap().as_str();

                    unit.turns_fortified = turns;
                    unit.action = Some(format!("Fortify{}", heal));
                }
            }
        }
    }
}

pub fn ensure_unit_ids(game_info: &mut GameInfo) {
    if game_info.last_unit_id == 0 {
        let max_id = game_info
            .tile_map
            .values()
            .flat_map(|tile| tile.get_units())
            .map(|unit| unit.id)
            .max()
            .unwrap_or(0)
            .max(0);

        game_info.last_unit_id = max_id;
    }

    for tile in game_info.tile_map.values_mut() {
        for unit in tile.get_units_mut() {
            if unit.id == Constants::NO_ID {
                game_info.last_unit_id += 1;
                unit.id = game_info.last_unit_id;
            }
        }
    }
}

/// Manages units for a civilization
#[derive(Clone, Serialize, Deserialize)]
pub struct UnitManager {
    #[serde(skip)]
    pub civ_info: Option<Arc<Civilization>>,

    /// All units of the civilization, ordered.
    /// Collection order and next_potentially_due_at determine activation order when using "Next unit".
    ///
    /// When loading a save, this is entirely rebuilt from Tile.*Unit.
    /// * GameInfo.setTransients -> TileMap.setTransients -> Tile.setUnitTransients -> MapUnit.assignOwner -> [add_unit] (the MapUnit overload)
    ///
    /// We never add or remove from here directly, could cause comodification problems.
    /// Instead, we create a copy list with the change, and replace this list.
    /// The other solution, casting toList() every "get", has a performance cost
    #[serde(skip)]
    unit_list: Vec<MapUnit>,

    /// Index in unit_list of the unit that is potentially due and is next up for button "Next unit".
    #[serde(skip)]
    next_potentially_due_at: usize,
}

impl UnitManager {
    pub fn new(civ_info: Arc<Civilization>) -> Self {
        Self {
            civ_info: Some(civ_info),
            unit_list: Vec::new(),
            next_potentially_due_at: 0,
        }
    }

    pub fn clone(&self) -> Self {
        Self {
            civ_info: self.civ_info.clone(),
            unit_list: self.unit_list.clone(),
            next_potentially_due_at: self.next_potentially_due_at,
        }
    }

    pub fn set_transients(&mut self, civ_info: Arc<Civilization>) {
        self.civ_info = Some(civ_info);
    }

    /// Creates a new MapUnit and places it on the map.
    ///
    /// # Arguments
    ///
    /// * `unit_name` - The BaseUnit name to create a MapUnit instance of - auto-mapped to a nation equivalent if one exists
    /// * `city` - The City to place the new unit in or near
    ///
    /// # Returns
    ///
    /// The new unit or None if unsuccessful (invalid unitName, no tile found where it could be placed, or civ has no cities)
    pub fn add_unit(&mut self, unit_name: &str, city: Option<&City>) -> Option<MapUnit> {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");
        let unit = civ_info.game_info.ruleset.units.get(unit_name)?;
        self.add_unit_base(unit, city)
    }

    /// Creates a new MapUnit and places it on the map.
    ///
    /// # Arguments
    ///
    /// * `base_unit` - The BaseUnit to create a MapUnit instance of - auto-mapped to a nation equivalent if one exists
    /// * `city` - The City to place the new unit in or near
    ///
    /// # Returns
    ///
    /// The new unit or None if unsuccessful (no tile found where it could be placed, or civ has no cities)
    pub fn add_unit_base(&mut self, base_unit: &BaseUnit, city: Option<&City>) -> Option<MapUnit> {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");

        if civ_info.cities.is_empty() {
            return None;
        }

        let unit = civ_info.get_equivalent_unit(base_unit);
        let cities_not_in_resistance: Vec<_> = civ_info
            .cities
            .iter()
            .filter(|city| !city.is_in_resistance())
            .collect();

        let city_to_add_to =
            if unit.is_water_unit && (city.is_none() || !city.unwrap().is_coastal()) {
                cities_not_in_resistance
                    .iter()
                    .filter(|city| city.is_coastal())
                    .choose(&mut rand::thread_rng())
                    .or_else(|| {
                        civ_info
                            .cities
                            .iter()
                            .filter(|city| city.is_coastal())
                            .choose(&mut rand::thread_rng())
                    })
            } else if let Some(city) = city {
                Some(city)
            } else {
                cities_not_in_resistance
                    .iter()
                    .choose(&mut rand::thread_rng())
                    .or_else(|| civ_info.cities.iter().choose(&mut rand::thread_rng()))
            }?; // If we got a free water unit with no coastal city to place it in

        let placed_unit = self.place_unit_near_tile(city_to_add_to.location, &unit.name)?;

        if unit.is_great_person {
            civ_info.add_notification(
                format!(
                    "A [{}] has been born in [{}]!",
                    unit.name, city_to_add_to.name
                ),
                MapUnitAction::new(placed_unit.clone()),
                NotificationCategory::General,
                unit.name.clone(),
            );
        }

        if placed_unit.has_unique(UniqueType::ReligiousUnit)
            && civ_info.game_info.is_religion_enabled()
        {
            if !placed_unit.has_unique(UniqueType::TakeReligionOverBirthCity)
                || civ_info
                    .religion_manager
                    .religion
                    .as_ref()
                    .map_or(false, |r| !r.is_major_religion())
            {
                placed_unit.religion = city_to_add_to.religion.get_majority_religion_name();
            }
        }

        Some(placed_unit)
    }

    /// Tries to place a unit into the Tile closest to the given location
    ///
    /// # Arguments
    ///
    /// * `location` - where to try to place the unit
    /// * `unit_name` - name of the BaseUnit to create and place
    ///
    /// # Returns
    ///
    /// created MapUnit or None if no suitable location was found
    pub fn place_unit_near_tile(
        &mut self,
        location: (f32, f32),
        unit_name: &str,
    ) -> Option<MapUnit> {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");
        let unit = civ_info.game_info.ruleset.units.get(unit_name)?;
        self.place_unit_near_tile_base(location, unit, None)
    }

    /// Tries to place a unit into the Tile closest to the given location
    ///
    /// # Arguments
    ///
    /// * `location` - where to try to place the unit
    /// * `base_unit` - BaseUnit to create and place
    /// * `unit_id` - Optional unit ID to assign
    ///
    /// # Returns
    ///
    /// created MapUnit or None if no suitable location was found
    pub fn place_unit_near_tile_base(
        &mut self,
        location: (f32, f32),
        base_unit: &BaseUnit,
        unit_id: Option<i32>,
    ) -> Option<MapUnit> {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");
        let unit = civ_info
            .game_info
            .tile_map
            .place_unit_near_tile(location, base_unit, civ_info, unit_id)?;

        let trigger_notification_text = format!("due to gaining a [{}]", unit.name);

        for unique in unit.get_uniques() {
            if !unique.has_trigger_conditional() && unique.conditionals_apply(&unit.cache.state) {
                UniqueTriggerActivation::trigger_unique(unique, &unit, &trigger_notification_text);
            }
        }

        for unique in civ_info
            .get_triggered_uniques(UniqueType::TriggerUponGainingUnit, &unit.cache.state)
            .filter(|u| unit.matches_filter(&u.params[0]))
        {
            UniqueTriggerActivation::trigger_unique(unique, &unit, &trigger_notification_text);
        }

        if !unit.get_resource_requirements_per_turn().is_empty() {
            civ_info.cache.update_civ_resources();
        }

        for unique in civ_info.get_matching_uniques(
            UniqueType::LandUnitsCrossTerrainAfterUnitGained,
            &unit.cache.state,
        ) {
            if unit.matches_filter(&unique.params[1]) {
                civ_info.pass_through_impassable_unlocked = true; // Update the cached Boolean
                civ_info.passable_impassables.push(unique.params[0].clone()); // Add to list of passable impassables
            }
        }

        if unit.has_unique(UniqueType::ReligiousUnit) && civ_info.game_info.is_religion_enabled() {
            unit.religion = civ_info
                .religion_manager
                .religion
                .as_ref()
                .map(|r| r.name.clone());
        }

        Some(unit)
    }

    /// Gets the number of units in the civilization
    pub fn get_civ_units_size(&self) -> usize {
        self.unit_list.len()
    }

    /// Gets all units of the civilization
    pub fn get_civ_units(&self) -> &[MapUnit] {
        &self.unit_list
    }

    /// Gets all great people of the civilization
    pub fn get_civ_great_people(&self) -> Vec<&MapUnit> {
        self.unit_list
            .iter()
            .filter(|map_unit| map_unit.is_great_person())
            .collect()
    }

    /// Similar to get_civ_units(), but the returned list is rotated so that the
    /// 'next_potentially_due_at' unit is first here.
    fn get_civ_units_starting_at_next_due(&self) -> Vec<&MapUnit> {
        let mut result = Vec::with_capacity(self.unit_list.len());

        // Add units from next_potentially_due_at to the end
        result.extend(self.unit_list[self.next_potentially_due_at..].iter());

        // Add units from the beginning to next_potentially_due_at
        result.extend(self.unit_list[..self.next_potentially_due_at].iter());

        result
    }

    /// Assigns an existing mapUnit to this manager.
    ///
    /// Used during load game via setTransients to regenerate a Civilization's list from the serialized Tile fields.
    ///
    /// # Arguments
    ///
    /// * `map_unit` - The unit to add
    /// * `update_civ_info` - When true, calls update_stats_for_next_turn and possibly update_civ_resources
    pub fn add_unit_map_unit(&mut self, map_unit: MapUnit, update_civ_info: bool) {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");

        // Since we create a new list anyway (otherwise some concurrent modification
        // exception will happen), also rearrange existing units so that
        // 'next_potentially_due_at' becomes 0. This way new units are always last to be due
        // (can be changed as wanted, just have a predictable place).
        let mut new_list = self
            .get_civ_units_starting_at_next_due()
            .iter()
            .cloned()
            .collect::<Vec<_>>();

        new_list.push(map_unit);
        self.unit_list = new_list;
        self.next_potentially_due_at = 0;

        if update_civ_info {
            // Not relevant when updating Tile transients, since some info of the civ itself isn't yet available,
            // and in any case it'll be updated once civ info transients are
            civ_info.update_stats_for_next_turn(); // unit upkeep
            if !map_unit.get_resource_requirements_per_turn().is_empty() {
                civ_info.cache.update_civ_resources();
            }
        }
    }

    /// Removes a unit from the manager
    pub fn remove_unit(&mut self, map_unit: &MapUnit) {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");

        // See comment in add_unit_map_unit().
        let mut new_list = self
            .get_civ_units_starting_at_next_due()
            .iter()
            .cloned()
            .collect::<Vec<_>>();

        new_list.retain(|unit| unit != map_unit);
        self.unit_list = new_list;
        self.next_potentially_due_at = 0;

        civ_info.update_stats_for_next_turn(); // unit upkeep
        if !map_unit.get_resource_requirements_per_turn().is_empty() {
            civ_info.cache.update_civ_resources();
        }
    }

    /// Gets all idle units
    pub fn get_idle_units(&self) -> Vec<&MapUnit> {
        self.unit_list
            .iter()
            .filter(|unit| unit.is_idle())
            .collect()
    }

    /// Gets all due units
    pub fn get_due_units(&self) -> Vec<&MapUnit> {
        self.get_civ_units_starting_at_next_due()
            .iter()
            .filter(|unit| unit.due && unit.is_idle())
            .collect()
    }

    /// Checks if we should go to a due unit
    pub fn should_go_to_due_unit(&self) -> bool {
        UncivGame::current().settings.check_for_due_units && !self.get_due_units().is_empty()
    }

    /// Gets a unit by its ID
    pub fn get_unit_by_id(&self, id: i32) -> Option<&MapUnit> {
        self.unit_list.iter().find(|unit| unit.id == id)
    }

    /// Returns the next due unit, but preferably not 'unit_to_skip': this is returned only if it is the only remaining due unit.
    pub fn cycle_through_due_units(&mut self, unit_to_skip: Option<&MapUnit>) -> Option<&MapUnit> {
        if self.unit_list.is_empty() {
            return None;
        }

        let mut return_at = self.next_potentially_due_at;
        let mut fallback_at = -1;

        loop {
            if self.unit_list[return_at].due && self.unit_list[return_at].is_idle() {
                if unit_to_skip.is_none() || self.unit_list[return_at] != unit_to_skip.unwrap() {
                    self.next_potentially_due_at = (return_at + 1) % self.unit_list.len();
                    return Some(&self.unit_list[return_at]);
                } else {
                    fallback_at = return_at as i32;
                }
            }

            return_at = (return_at + 1) % self.unit_list.len();

            if return_at == self.next_potentially_due_at {
                break;
            }
        }

        if fallback_at >= 0 {
            self.next_potentially_due_at = (fallback_at as usize + 1) % self.unit_list.len();
            return Some(&self.unit_list[fallback_at as usize]);
        } else {
            None
        }
    }
}
