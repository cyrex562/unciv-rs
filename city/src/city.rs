use crate::city::city_constructions::CityConstructions;
use crate::city::city_flags::CityFlags;
use crate::city::city_focus::CityFocus;
use crate::city::city_resources::CityResources;
use crate::city::city_stats::CityStats;
use crate::city::great_person_points_breakdown::GreatPersonPointsBreakdown;
use crate::city::managers::city_conquest_functions::CityConquestFunctions;
use crate::city::managers::city_espionage_manager::CityEspionageManager;
use crate::city::managers::city_expansion_manager::CityExpansionManager;
use crate::city::managers::city_population_manager::CityPopulationManager;
use crate::city::managers::city_religion_manager::CityReligionManager;
use crate::civilization::civilization::Civilization;
use crate::espionage::spy_flee_reason::SpyFleeReason;
use crate::map::tile_map::TileMap;
use crate::map::unit::UnitPromotions;
use crate::map::MapUnit;
use crate::ruleset::ruleset::Ruleset;
use crate::ruleset::tile::tile_resource::TileResource;
use crate::stats::game_resource::GameResource;
use crate::stats::stat::Stat;
use crate::stats::sub_stat::SubStat;
use crate::tile::tile::{RoadStatus, Tile};
use crate::unique::state_for_conditionals::StateForConditionals;
use crate::unique::UniqueType;
use ggez::mint::Vector2;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

/// Represents a city in the game
pub struct City<'a> {
    /// The civilization that owns this city
    pub civ: Option<Arc<Civilization>>,

    /// The center tile of the city (cached for better performance)
    /// The center tile position of the city (use position instead of Arc<Tile>)
    pub center_tile_pos: Vector2<i32>,

    /// The tile map this city belongs to
    pub tile_map: Option<Arc<TileMap>>,

    /// Tiles within the city's work range
    pub tiles_in_range: HashSet<Vector2<i32>>,

    /// The state for conditionals
    pub state: StateForConditionals,

    /// Whether the city has just been conquered
    pub has_just_been_conquered: bool,

    /// The location of the city
    pub location: Vector2<i32>,

    /// The unique ID of the city
    pub id: String,

    /// The name of the city
    pub name: String,

    /// The founding civilization
    pub founding_civ: String,

    /// The previous owner of the city
    pub previous_owner: String,

    /// The turn when the city was acquired
    pub turn_acquired: i32,

    /// The health of the city
    pub health: i32,

    /// The population manager for this city
    pub population: CityPopulationManager,

    /// The constructions manager for this city
    pub city_constructions: CityConstructions,

    /// The expansion manager for this city
    pub expansion: CityExpansionManager,

    /// The religion manager for this city
    pub religion: CityReligionManager,

    /// The espionage manager for this city
    pub espionage: CityEspionageManager,

    /// The stats for this city
    pub city_stats: CityStats<'a>,

    /// Resource stockpiles for this city
    pub resource_stockpiles: HashMap<String, i32>,

    /// All tiles that this city controls
    pub tiles: HashSet<Vector2<i32>>,

    /// Tiles that have population assigned to them
    pub worked_tiles: HashSet<Vector2<i32>>,

    /// Tiles that the population in them won't be reassigned
    pub locked_tiles: HashSet<Vector2<i32>>,

    /// Whether manual specialists are enabled
    pub manual_specialists: bool,

    /// Whether the city is being razed
    pub is_being_razed: bool,

    /// Whether the city was attacked this turn
    pub attacked_this_turn: bool,

    /// Whether the city has sold a building this turn
    pub has_sold_building_this_turn: bool,

    /// Whether the city is a puppet
    pub is_puppet: bool,

    /// Whether the population should be reassigned
    pub should_reassign_population: bool,

    /// Whether a unit should use saved promotion
    pub unit_should_use_saved_promotion: HashMap<String, bool>,

    /// Unit to promotions mapping
    pub unit_to_promotions: HashMap<String, UnitPromotions>,

    /// The city AI focus
    pub city_ai_focus: String,

    /// Whether to avoid growth
    pub avoid_growth: bool,

    /// The current GPP bonus
    pub current_gpp_bonus: i32,

    /// Whether this is the original capital
    pub is_original_capital: bool,

    /// The demanded resource for We Love the King Day
    pub demanded_resource: String,

    /// The flags countdown
    pub flags_countdown: HashMap<String, i32>,

    /// The connected to capital status
    pub connected_to_capital_status: ConnectedToCapitalStatus,
}

/// The connected to capital status
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConnectedToCapitalStatus {
    /// Unknown status (for older saves)
    Unknown,
    /// Not connected to capital
    False,
    /// Connected to capital
    True,
}

impl<'a> City<'a> {
    /// Creates a new city
    pub fn new() -> Self {
        City {
            civ: None,
            center_tile: None,
            tile_map: None,
            tiles_in_range: HashSet::new(),
            state: StateForConditionals::empty_state(),
            has_just_been_conquered: false,
            location: Vector2::new(0, 0),
            id: Uuid::new_v4().to_string(),
            name: String::new(),
            founding_civ: String::new(),
            previous_owner: String::new(),
            turn_acquired: 0,
            health: 200,
            population: CityPopulationManager::new(),
            city_constructions: CityConstructions::new(),
            expansion: CityExpansionManager::new(),
            religion: CityReligionManager::new(),
            espionage: CityEspionageManager::new(),
            city_stats: CityStats::new(),
            resource_stockpiles: HashMap::new(),
            tiles: HashSet::new(),
            worked_tiles: HashSet::new(),
            locked_tiles: HashSet::new(),
            manual_specialists: false,
            is_being_razed: false,
            attacked_this_turn: false,
            has_sold_building_this_turn: false,
            is_puppet: false,
            should_reassign_population: false,
            unit_should_use_saved_promotion: HashMap::new(),
            unit_to_promotions: HashMap::new(),
            city_ai_focus: CityFocus::NoFocus.to_string(),
            avoid_growth: false,
            current_gpp_bonus: 0,
            is_original_capital: false,
            demanded_resource: String::new(),
            flags_countdown: HashMap::new(),
            connected_to_capital_status: ConnectedToCapitalStatus::Unknown,
        }
    }

    /// Clones the city
    pub fn clone(&self) -> Self {
        let mut to_return = City::new();
        to_return.location = self.location;
        to_return.id = self.id.clone();
        to_return.name = self.name.clone();
        to_return.health = self.health;
        to_return.population = self.population.clone();
        to_return.city_constructions = self.city_constructions.clone();
        to_return.expansion = self.expansion.clone();
        to_return.religion = self.religion.clone();
        to_return.tiles = self.tiles.clone();
        to_return.worked_tiles = self.worked_tiles.clone();
        to_return.locked_tiles = self.locked_tiles.clone();
        to_return.resource_stockpiles = self.resource_stockpiles.clone();
        to_return.is_being_razed = self.is_being_razed;
        to_return.attacked_this_turn = self.attacked_this_turn;
        to_return.founding_civ = self.founding_civ.clone();
        to_return.turn_acquired = self.turn_acquired;
        to_return.is_puppet = self.is_puppet;
        to_return.is_original_capital = self.is_original_capital;
        to_return.flags_countdown = self.flags_countdown.clone();
        to_return.demanded_resource = self.demanded_resource.clone();
        to_return.should_reassign_population = self.should_reassign_population;
        to_return.city_ai_focus = self.city_ai_focus.clone();
        to_return.avoid_growth = self.avoid_growth;
        to_return.manual_specialists = self.manual_specialists;
        to_return.connected_to_capital_status = self.connected_to_capital_status;
        to_return.unit_should_use_saved_promotion = self.unit_should_use_saved_promotion.clone();
        to_return.unit_to_promotions = self.unit_to_promotions.clone();
        to_return
    }

    /// Sets the transients for the city
    pub fn set_transients(&mut self, civ_info: Arc<Civilization>) {
        self.civ = Some(civ_info.clone());
        self.tile_map = Some(civ_info.game_info.tile_map.clone());
        self.center_tile = Some(self.tile_map.as_ref().unwrap()[self.location].clone());
        self.state = StateForConditionals::new();
        self.tiles_in_range = self
            .get_center_tile()
            .get_tiles_in_distance(self.get_work_range())
            .into_iter()
            .collect();
        self.population.city = Some(Arc::new(self.clone()));
        self.expansion.city = Some(Arc::new(self.clone()));
        self.expansion.set_transients(Arc::new(self.clone()));
        self.city_constructions.city = Some(Arc::new(self.clone()));
        self.religion.set_transients(Arc::new(self.clone()));
        self.city_constructions.set_transients();
        self.espionage.set_transients(Arc::new(self.clone()));
    }

    /// Gets the center tile of the city or null
    pub fn get_center_tile_or_null(&self) -> Option<Arc<Tile>> {
        self.center_tile.clone()
    }

    /// Gets the tiles of the city
    pub fn get_tiles(&self) -> Vec<Arc<Tile>> {
        self.tiles
            .iter()
            .map(|pos| self.tile_map.as_ref().unwrap()[*pos].clone())
            .collect()
    }

    /// Gets the workable tiles of the city
    pub fn get_workable_tiles(&self) -> Vec<Arc<Tile>> {
        self.tiles_in_range
            .iter()
            .map(|pos| self.tile_map.as_ref().unwrap()[*pos].clone())
            .filter(|tile| tile.get_owner() == Some(self.civ.as_ref().unwrap().clone()))
            .collect()
    }

    /// Checks if a tile is worked
    pub fn is_worked(&self, tile: &Arc<Tile>) -> bool {
        self.worked_tiles.contains(&tile.position)
    }

    /// Checks if the city is a capital
    pub fn is_capital(&self) -> bool {
        self.city_constructions
            .built_building_unique_map
            .has_unique(UniqueType::IndicatesCapital, &self.state)
    }

    /// Checks if the city is coastal
    pub fn is_coastal(&self) -> bool {
        self.get_center_tile().is_coastal_tile()
    }

    /// Gets the bombard range of the city
    pub fn get_bombard_range(&self) -> i32 {
        self.civ
            .as_ref()
            .unwrap()
            .game_info
            .ruleset
            .mod_options
            .constants
            .base_city_bombard_range
    }

    /// Gets the work range of the city
    pub fn get_work_range(&self) -> i32 {
        self.civ
            .as_ref()
            .unwrap()
            .game_info
            .ruleset
            .mod_options
            .constants
            .city_work_range
    }

    /// Gets the expand range of the city
    pub fn get_expand_range(&self) -> i32 {
        self.civ
            .as_ref()
            .unwrap()
            .game_info
            .ruleset
            .mod_options
            .constants
            .city_expand_range
    }

    /// Checks if the city is connected to the capital
    pub fn is_connected_to_capital<F>(&self, connection_type_predicate: F) -> bool
    where
        F: FnOnce(&HashSet<String>) -> bool,
    {
        if let Some(medium_types) = self
            .civ
            .as_ref()
            .unwrap()
            .cache
            .cities_connected_to_capital_to_mediums
            .get(self)
        {
            connection_type_predicate(medium_types)
        } else {
            false
        }
    }

    /// Checks if the city is garrisoned
    pub fn is_garrisoned(&self) -> bool {
        self.get_garrison().is_some()
    }

    /// Gets the garrison of the city
    pub fn get_garrison(&self) -> Option<Arc<MapUnit>> {
        if let Some(military_unit) = &self.get_center_tile().military_unit {
            if military_unit.civ == Some(self.civ.as_ref().unwrap().clone())
                && military_unit.can_garrison()
            {
                return Some(military_unit.clone());
            }
        }
        None
    }

    /// Checks if the city has a flag
    pub fn has_flag(&self, flag: CityFlags) -> bool {
        self.flags_countdown.contains_key(&flag.to_string())
    }

    /// Gets the flag countdown
    pub fn get_flag(&self, flag: CityFlags) -> i32 {
        *self.flags_countdown.get(&flag.to_string()).unwrap()
    }

    /// Checks if We Love The King Day is active
    pub fn is_we_love_the_king_day_active(&self) -> bool {
        self.has_flag(CityFlags::WeLoveTheKing)
    }

    /// Checks if the city is in resistance
    pub fn is_in_resistance(&self) -> bool {
        self.has_flag(CityFlags::Resistance)
    }

    /// Checks if the city is blockaded
    pub fn is_blockaded(&self) -> bool {
        if !self.is_coastal() {
            return false;
        }
        self.get_center_tile()
            .neighbors
            .iter()
            .filter(|tile| tile.is_water)
            .all(|tile| tile.is_blockaded())
    }

    /// Gets the ruleset
    pub fn get_ruleset(&self) -> Arc<Ruleset> {
        self.civ.as_ref().unwrap().game_info.ruleset.clone()
    }

    /// Gets the resources generated by the city
    pub fn get_resources_generated_by_city(
        &self,
        civ_resource_modifiers: &HashMap<String, f32>,
    ) -> HashMap<String, f32> {
        CityResources::get_resources_generated_by_city(self, civ_resource_modifiers)
    }

    /// Gets the available resource amount
    pub fn get_available_resource_amount(&self, resource_name: &str) -> i32 {
        CityResources::get_available_resource_amount(self, resource_name)
    }

    /// Checks if the city is growing
    pub fn is_growing(&self) -> bool {
        self.food_for_next_turn() > 0
    }

    /// Checks if the city is starving
    pub fn is_starving(&self) -> bool {
        self.food_for_next_turn() < 0
    }

    /// Gets the food for next turn
    pub fn food_for_next_turn(&self) -> i32 {
        self.city_stats.get_current_food() as i32
    }

    /// Checks if the city contains a building unique
    pub fn contains_building_unique(
        &self,
        unique_type: UniqueType,
        state: Option<&StateForConditionals>,
    ) -> bool {
        let state = state.unwrap_or(&self.state);
        self.city_constructions
            .built_building_unique_map
            .get_matching_uniques(unique_type, state)
            .next()
            .is_some()
    }

    /// Gets the great person percentage bonus
    pub fn get_great_person_percentage_bonus(&self) -> f32 {
        GreatPersonPointsBreakdown::get_great_person_percentage_bonus(self)
    }

    /// Gets the great person points
    pub fn get_great_person_points(&self) -> HashMap<String, f32> {
        GreatPersonPointsBreakdown::new(self).sum()
    }

    /// Gains a stockpiled resource
    pub fn gain_stockpiled_resource(&mut self, resource: &TileResource, amount: i32) {
        if resource.is_city_wide {
            *self
                .resource_stockpiles
                .entry(resource.name.clone())
                .or_insert(0) += amount;
        } else {
            self.civ
                .as_ref()
                .unwrap()
                .resource_stockpiles
                .entry(resource.name.clone())
                .or_insert(0) += amount;
        }
    }

    /// Adds a stat
    pub fn add_stat(&mut self, stat: Stat, amount: i32) {
        match stat {
            Stat::Production => self.city_constructions.add_production_points(amount),
            Stat::Food => self.population.food_stored += amount,
            _ => self.civ.as_ref().unwrap().add_stat(stat, amount),
        }
    }

    /// Adds a game resource
    pub fn add_game_resource(&mut self, stat: &GameResource, amount: i32) {
        if let GameResource::TileResource(resource) = stat {
            if !resource.is_stockpiled {
                return;
            }
            self.gain_stockpiled_resource(resource, amount);
            return;
        }

        match stat {
            GameResource::Stat(Stat::Production) => {
                self.city_constructions.add_production_points(amount)
            }
            GameResource::Stat(Stat::Food) | GameResource::SubStat(SubStat::StoredFood) => {
                self.population.food_stored += amount
            }
            _ => self.civ.as_ref().unwrap().add_game_resource(stat, amount),
        }
    }

    /// Gets the stat reserve
    pub fn get_stat_reserve(&self, stat: Stat) -> i32 {
        match stat {
            Stat::Production => self
                .city_constructions
                .get_work_done(&self.city_constructions.get_current_construction().name),
            Stat::Food => self.population.food_stored,
            _ => self.civ.as_ref().unwrap().get_stat_reserve(stat),
        }
    }

    /// Gets the reserve
    pub fn get_reserve(&self, stat: &GameResource) -> i32 {
        if let GameResource::TileResource(resource) = stat {
            if !resource.is_stockpiled {
                return 0;
            }
            if resource.is_city_wide {
                return *self.resource_stockpiles.get(&resource.name).unwrap_or(&0);
            }
            return *self
                .civ
                .as_ref()
                .unwrap()
                .resource_stockpiles
                .get(&resource.name)
                .unwrap_or(&0);
        }

        match stat {
            GameResource::Stat(Stat::Production) => self
                .city_constructions
                .get_work_done(&self.city_constructions.get_current_construction().name),
            GameResource::Stat(Stat::Food) | GameResource::SubStat(SubStat::StoredFood) => {
                self.population.food_stored
            }
            _ => self.civ.as_ref().unwrap().get_reserve(stat),
        }
    }

    /// Checks if the city has stat to buy
    pub fn has_stat_to_buy(&self, stat: Stat, price: i32) -> bool {
        if self
            .civ
            .as_ref()
            .unwrap()
            .game_info
            .game_parameters
            .god_mode
        {
            return true;
        }
        if price == 0 {
            return true;
        }
        self.get_stat_reserve(stat) >= price
    }

    /// Gets the max health of the city
    pub fn get_max_health(&self) -> i32 {
        200 + self
            .city_constructions
            .get_built_buildings()
            .iter()
            .map(|building| building.city_health)
            .sum::<i32>()
    }

    /// Gets the strength of the city
    pub fn get_strength(&self) -> f32 {
        self.city_constructions
            .get_built_buildings()
            .iter()
            .map(|building| building.city_strength)
            .sum::<i32>() as f32
    }

    /// Gets the max air units
    pub fn get_max_air_units(&self) -> i32 {
        6 // This should probably be configurable
    }

    /// Checks if the city is a holy city
    pub fn is_holy_city(&self) -> bool {
        self.religion.religion_this_is_the_holy_city_of.is_some()
            && !self.religion.is_blocked_holy_city
    }

    /// Checks if the city is a holy city of a religion
    pub fn is_holy_city_of(&self, religion_name: Option<&str>) -> bool {
        self.is_holy_city()
            && self.religion.religion_this_is_the_holy_city_of.as_deref() == religion_name
    }

    /// Checks if the city can be destroyed
    pub fn can_be_destroyed(&self, just_captured: bool) -> bool {
        if self
            .civ
            .as_ref()
            .unwrap()
            .game_info
            .game_parameters
            .no_city_razing
        {
            return false;
        }

        let allow_raze_capital = self
            .civ
            .as_ref()
            .unwrap()
            .game_info
            .ruleset
            .mod_options
            .has_unique(UniqueType::AllowRazeCapital);
        let allow_raze_holy_city = self
            .civ
            .as_ref()
            .unwrap()
            .game_info
            .ruleset
            .mod_options
            .has_unique(UniqueType::AllowRazeHolyCity);

        if self.is_original_capital && !allow_raze_capital {
            return false;
        }
        if self.is_holy_city() && !allow_raze_holy_city {
            return false;
        }
        if self.is_capital() && !just_captured && !allow_raze_capital {
            return false;
        }

        true
    }

    /// Sets a flag
    pub fn set_flag(&mut self, flag: CityFlags, amount: i32) {
        self.flags_countdown.insert(flag.to_string(), amount);
    }

    /// Removes a flag
    pub fn remove_flag(&mut self, flag: CityFlags) {
        self.flags_countdown.remove(&flag.to_string());
    }

    /// Resets We Love The King Day
    pub fn reset_wltkd(&mut self) {
        // Removes the flags for we love the king & resource demand
        // The resource demand flag will automatically be readded with 15 turns remaining, see startTurn()
        self.remove_flag(CityFlags::WeLoveTheKing);
        self.remove_flag(CityFlags::ResourceDemand);
        self.demanded_resource = String::new();
    }

    /// Reassigns all population
    pub fn reassign_all_population(&mut self) {
        self.manual_specialists = false;
        self.reassign_population(true);
    }

    /// Reassigns population
    pub fn reassign_population(&mut self, reset_locked: bool) {
        if reset_locked {
            self.worked_tiles = HashSet::new();
            self.locked_tiles = HashSet::new();
        } else if self.city_ai_focus != CityFocus::Manual.to_string() {
            self.worked_tiles = self.locked_tiles.clone();
        }
        if !self.manual_specialists {
            self.population.specialist_allocations.clear();
        }
        self.should_reassign_population = false;
        self.population.auto_assign_population();
    }

    /// Reassigns population deferred
    pub fn reassign_population_deferred(&mut self) {
        // TODO - is this the best (or even correct) way to detect "interactive" UI calls?
        if crate::gui::is_my_turn()
            && crate::gui::get_viewing_player() == Some(self.civ.as_ref().unwrap().clone())
        {
            self.reassign_population(false);
        } else {
            self.should_reassign_population = true;
        }
    }

    /// Destroys the city
    pub fn destroy_city(&mut self, override_safeties: bool) {
        // Original capitals and holy cities cannot be destroyed,
        // unless, of course, they are captured by a one-city-challenger.
        if !self.can_be_destroyed(false) && !override_safeties {
            return;
        }

        // Destroy planes stationed in city
        for air_unit in self.get_center_tile().air_units.iter() {
            air_unit.destroy();
        }

        // The relinquish ownership MUST come before removing the city,
        // because it updates the city stats which assumes there is a capital, so if you remove the capital it crashes
        for tile in self.get_tiles() {
            self.expansion.relinquish_ownership(&tile);
        }

        // Move the capital if destroyed (by a nuke or by razing)
        // Must be before removing existing capital because we may be annexing a puppet which means city stats update - see #8337
        if self.is_capital() {
            self.civ
                .as_ref()
                .unwrap()
                .move_capital_to_next_largest(None);
        }

        let mut cities = self.civ.as_ref().unwrap().cities.clone();
        cities.retain(|city| city.id != self.id);
        self.civ.as_ref().unwrap().cities = cities;

        if self
            .get_ruleset()
            .tile_improvements
            .contains_key("City ruins")
        {
            self.get_center_tile()
                .set_improvement("City ruins".to_string());
        }

        // Edge case! What if a water unit is in a city, and you raze the city?
        // Well, the water unit has to return to the water!
        for unit in self.get_center_tile().get_units() {
            if !unit.movement.can_pass_through(&self.get_center_tile()) {
                unit.movement.teleport_to_closest_moveable_tile();
            }
        }

        self.espionage
            .remove_all_present_spies(SpyFleeReason::CityDestroyed);

        // Update proximity rankings for all civs
        for other_civ in self.civ.as_ref().unwrap().game_info.get_alive_major_civs() {
            self.civ.as_ref().unwrap().update_proximity(
                &other_civ,
                other_civ.update_proximity(&self.civ.as_ref().unwrap()),
            );
        }
        for other_civ in self.civ.as_ref().unwrap().game_info.get_alive_city_states() {
            self.civ.as_ref().unwrap().update_proximity(
                &other_civ,
                other_civ.update_proximity(&self.civ.as_ref().unwrap()),
            );
        }

        self.civ
            .as_ref()
            .unwrap()
            .game_info
            .city_distances
            .set_dirty();
    }

    /// Annexes the city
    pub fn annex_city(&mut self) {
        CityConquestFunctions::new(Arc::new(self.clone())).annex_city();
    }

    /// Puppets the city
    pub fn puppet_city(&mut self, conquering_civ: Arc<Civilization>) {
        CityConquestFunctions::new(Arc::new(self.clone())).puppet_city(&conquering_civ);
    }

    /// Liberates the city
    pub fn liberate_city(&mut self, conquering_civ: Arc<Civilization>) {
        CityConquestFunctions::new(Arc::new(self.clone())).liberate_city(&conquering_civ);
    }

    /// Moves the city to a civilization
    pub fn move_to_civ(&mut self, new_civ_info: Arc<Civilization>) {
        CityConquestFunctions::new(Arc::new(self.clone())).move_to_civ(&new_civ_info);
    }

    /// Tries to update road status
    pub fn try_update_road_status(&mut self) {
        let required_road =
            if let Some(railroad_improvement) = self.get_ruleset().railroad_improvement.as_ref() {
                if railroad_improvement.tech_required.is_none()
                    || self
                        .civ
                        .as_ref()
                        .unwrap()
                        .tech
                        .techs_researched
                        .contains(&railroad_improvement.tech_required.unwrap())
                {
                    RoadStatus::Railroad
                } else {
                    RoadStatus::None
                }
            } else {
                RoadStatus::None
            };

        let required_road = if required_road == RoadStatus::None {
            if let Some(road_improvement) = self.get_ruleset().road_improvement.as_ref() {
                if road_improvement.tech_required.is_none()
                    || self
                        .civ
                        .as_ref()
                        .unwrap()
                        .tech
                        .techs_researched
                        .contains(&road_improvement.tech_required.unwrap())
                {
                    RoadStatus::Road
                } else {
                    RoadStatus::None
                }
            } else {
                RoadStatus::None
            }
        } else {
            required_road
        };

        self.get_center_tile().set_road_status(required_road);
    }

    /// Gets the gold for selling a building
    pub fn get_gold_for_selling_building(&self, building_name: &str) -> i32 {
        self.get_ruleset()
            .buildings
            .get(building_name)
            .unwrap()
            .get_cost()
            / 10
    }

    /// Sells a building
    pub fn sell_building(&mut self, building_name: &str) {
        let building = self
            .get_ruleset()
            .buildings
            .get(building_name)
            .unwrap()
            .clone();
        self.sell_building_internal(&building);
    }

    /// Sells a building
    pub fn sell_building_internal(&mut self, building: &Building) {
        self.city_constructions.remove_building(building);
        self.civ
            .as_ref()
            .unwrap()
            .add_gold(self.get_gold_for_selling_building(&building.name));
        self.has_sold_building_this_turn = true;

        self.population.unassign_extra_population(); // If the building provided specialists, release them to other work
        self.population.auto_assign_population(); // also updates city stats
        self.civ.as_ref().unwrap().cache.update_civ_resources(); // this building could be a resource-requiring one
    }

    /// Checks if a new unit can be placed
    pub fn can_place_new_unit(&self, construction: &BaseUnit) -> bool {
        let tile = self.get_center_tile();
        if construction.is_civilian() {
            tile.civilian_unit.is_none()
        } else if construction.moves_like_air_units {
            true // Dealt with in MapUnit.getRejectionReasons
        } else {
            tile.military_unit.is_none()
        }
    }

    /// Matches a filter
    pub fn matches_filter(
        &self,
        filter: &str,
        viewing_civ: Option<&Arc<Civilization>>,
        multi_filter: bool,
    ) -> bool {
        if multi_filter {
            MultiFilter::multi_filter(filter, |f| self.matches_single_filter(f, viewing_civ))
        } else {
            self.matches_single_filter(filter, viewing_civ)
        }
    }

    /// Matches a single filter
    fn matches_single_filter(&self, filter: &str, viewing_civ: Option<&Arc<Civilization>>) -> bool {
        match filter {
            "in this city" => true, // Filtered by the way uniques are found
            "in all cities" => true,
            f if Constants::all.contains(&f.to_string()) => true,
            "in your cities" | "Your" => viewing_civ == Some(self.civ.as_ref().unwrap()),
            "in all coastal cities" | "Coastal" => self.is_coastal(),
            "in capital" | "Capital" => self.is_capital(),
            "in all non-occupied cities" | "Non-occupied" => {
                !self.city_stats.has_extra_annex_unhappiness() || self.is_puppet
            }
            "in all cities with a world wonder" => self
                .city_constructions
                .get_built_buildings()
                .iter()
                .any(|b| b.is_wonder),
            "in all cities connected to capital" => self.is_connected_to_capital(|_| true),
            "in all cities with a garrison" | "Garrisoned" => self.is_garrisoned(),
            "in all cities in which the majority religion is a major religion" => {
                self.religion.get_majority_religion_name().is_some()
                    && self
                        .religion
                        .get_majority_religion()
                        .unwrap()
                        .is_major_religion()
            }
            "in all cities in which the majority religion is an enhanced religion" => {
                self.religion.get_majority_religion_name().is_some()
                    && self
                        .religion
                        .get_majority_religion()
                        .unwrap()
                        .is_enhanced_religion()
            }
            "in non-enemy foreign cities" => {
                if let Some(viewing_civ) = viewing_civ {
                    viewing_civ != self.civ.as_ref().unwrap()
                        && !self.civ.as_ref().unwrap().is_at_war_with(viewing_civ)
                } else {
                    false
                }
            }
            "in enemy cities" | "Enemy" => {
                if let Some(viewing_civ) = viewing_civ {
                    self.civ.as_ref().unwrap().is_at_war_with(viewing_civ)
                } else {
                    self.civ
                        .as_ref()
                        .unwrap()
                        .is_at_war_with(self.civ.as_ref().unwrap())
                }
            }
            "in foreign cities" | "Foreign" => {
                viewing_civ.is_some() && viewing_civ.unwrap() != self.civ.as_ref().unwrap()
            }
            "in annexed cities" | "Annexed" => {
                self.founding_civ != self.civ.as_ref().unwrap().civ_name && !self.is_puppet
            }
            "in puppeted cities" | "Puppeted" => self.is_puppet,
            "in resisting cities" | "Resisting" => self.is_in_resistance(),
            "in cities being razed" | "Razing" => self.is_being_razed,
            "in holy cities" | "Holy" => self.is_holy_city(),
            "in City-State cities" => self.civ.as_ref().unwrap().is_city_state,
            // This is only used in communication to the user indicating that only in cities with this
            // religion a unique is active. However, since religion uniques only come from the city itself,
            // this will always be true when checked.
            "in cities following this religion" => true,
            "in cities following our religion" => {
                if let Some(viewing_civ) = viewing_civ {
                    if let Some(religion) = &viewing_civ.religion_manager.religion {
                        Some(religion) == self.religion.get_majority_religion()
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => self
                .civ
                .as_ref()
                .unwrap()
                .matches_filter(filter, &self.state, false),
        }
    }

    /// Gets matching uniques
    pub fn get_matching_uniques(
        &self,
        unique_type: UniqueType,
        state_for_conditionals: Option<&StateForConditionals>,
        include_civ_uniques: bool,
    ) -> Vec<Unique> {
        let state_for_conditionals = state_for_conditionals.unwrap_or(&self.state);

        if include_civ_uniques {
            let mut uniques = self
                .civ
                .as_ref()
                .unwrap()
                .get_matching_uniques(unique_type, state_for_conditionals);
            uniques
                .extend(self.get_local_matching_uniques(unique_type, Some(state_for_conditionals)));
            uniques
        } else {
            let mut uniques = self
                .city_constructions
                .built_building_unique_map
                .get_uniques(unique_type);
            uniques.extend(self.religion.get_uniques(unique_type));

            uniques
                .into_iter()
                .filter(|unique| {
                    !unique.is_timed_triggerable
                        && unique.conditionals_apply(state_for_conditionals)
                })
                .flat_map(|unique| unique.get_multiplied(state_for_conditionals))
                .collect()
        }
    }

    /// Gets local matching uniques
    pub fn get_local_matching_uniques(
        &self,
        unique_type: UniqueType,
        state_for_conditionals: Option<&StateForConditionals>,
    ) -> Vec<Unique> {
        let state_for_conditionals = state_for_conditionals.unwrap_or(&self.state);

        let mut uniques = self
            .city_constructions
            .built_building_unique_map
            .get_uniques(unique_type)
            .into_iter()
            .filter(|unique| unique.is_local_effect)
            .collect::<Vec<_>>();

        uniques.extend(self.religion.get_uniques(unique_type));

        uniques
            .into_iter()
            .filter(|unique| {
                !unique.is_timed_triggerable && unique.conditionals_apply(state_for_conditionals)
            })
            .flat_map(|unique| unique.get_multiplied(state_for_conditionals))
            .collect()
    }

    /// Gets matching uniques with non-local effects
    pub fn get_matching_uniques_with_non_local_effects(
        &self,
        unique_type: UniqueType,
        state_for_conditionals: Option<&StateForConditionals>,
    ) -> Vec<Unique> {
        let state_for_conditionals = state_for_conditionals.unwrap_or(&self.state);

        let uniques = self
            .city_constructions
            .built_building_unique_map
            .get_uniques(unique_type);

        // Memory performance showed that this function was very memory intensive, thus we only create the filter if needed
        if !uniques.is_empty() {
            uniques
                .into_iter()
                .filter(|unique| {
                    !unique.is_local_effect
                        && !unique.is_timed_triggerable
                        && unique.conditionals_apply(state_for_conditionals)
                })
                .flat_map(|unique| unique.get_multiplied(state_for_conditionals))
                .collect()
        } else {
            uniques
        }
    }

    /// Gets the city focus
    pub fn get_city_focus(&self) -> CityFocus {
        CityFocus::from_str(&self.city_ai_focus).unwrap_or(CityFocus::NoFocus)
    }

    /// Sets the city focus
    pub fn set_city_focus(&mut self, city_focus: CityFocus) {
        self.city_ai_focus = city_focus.to_string();
    }

    /// Checks if the city has diplomatic marriage
    pub fn has_diplomatic_marriage(&self) -> bool {
        self.founding_civ.is_empty()
    }

    /// Checks if the city can bombard
    pub fn can_bombard(&self) -> bool {
        !self.attacked_this_turn && !self.is_in_resistance()
    }
}

impl<'a> Named for City<'a> {
    fn get_name(&self) -> String {
        self.name.clone()
    }
}

impl<'a> std::fmt::Display for City<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

// Handle missing references for each city
fn handle_missing_references_for_each_city(game_info: &mut GameInfo) {
    for civ in &mut game_info.civilizations {
        for city in &mut civ.cities {
            // Remove built buildings that are no longer defined in the ruleset
            let built_buildings: Vec<String> = city.city_constructions.built_buildings.clone();
            for building in built_buildings {
                if !game_info.ruleset.buildings.contains_key(&building) {
                    city.city_constructions.built_buildings.remove(&building);
                }
            }

            // Check if a construction is invalid (not in buildings, units, or perpetual constructions)
            let is_invalid_construction = |construction: &str| -> bool {
                !game_info.ruleset.buildings.contains_key(construction)
                    && !game_info.ruleset.units.contains_key(construction)
                    && !PerpetualConstruction::perpetual_constructions_map()
                        .contains_key(construction)
            };

            // Remove invalid buildings or units from the queue
            let construction_queue: Vec<String> =
                city.city_constructions.construction_queue.clone();
            for construction in construction_queue {
                if is_invalid_construction(&construction) {
                    city.city_constructions
                        .construction_queue
                        .retain(|c| c != &construction);
                }
            }

            // Remove invalid buildings or units from in-progress constructions
            let in_progress_keys: Vec<String> = city
                .city_constructions
                .in_progress_constructions
                .keys()
                .cloned()
                .collect();
            for construction in in_progress_keys {
                if is_invalid_construction(&construction) {
                    city.city_constructions
                        .in_progress_constructions
                        .remove(&construction);
                }
            }
        }
    }
}
