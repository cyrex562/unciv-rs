use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use crate::city::City;
use crate::civilization::Civilization;
use crate::diplomacy::DiplomaticStatus;
use crate::map::bfs::BFS;
use crate::map::tile::{RoadStatus, Tile};
use crate::models::ruleset::unique::UniqueType;

/// Finds connections between cities and the capital
pub struct CapitalConnectionsFinder {
    civ_info: Arc<Civilization>,
    cities_reached_to_mediums: HashMap<Arc<City>, HashSet<String>>,
    cities_to_check: VecDeque<Arc<City>>,
    new_cities_to_check: VecDeque<Arc<City>>,
    open_borders_civ_cities: Vec<Arc<City>>,

    // Constants
    harbor: String,
    road: String,
    railroad: String,
    harbor_from_road: String,
    harbor_from_railroad: String,

    // Cached values
    road_is_researched: bool,
    railroad_is_researched: bool,
}

impl CapitalConnectionsFinder {
    /// Creates a new CapitalConnectionsFinder
    pub fn new(civ_info: Arc<Civilization>) -> Self {
        let harbor = "Harbor".to_string(); // hardcoding at least centralized for this class for now
        let road = RoadStatus::Road.name();
        let railroad = RoadStatus::Railroad.name();
        let harbor_from_road = format!("{}-{}", harbor, road);
        let harbor_from_railroad = format!("{}-{}", harbor, railroad);

        let ruleset = &civ_info.game_info.ruleset;
        let road_is_researched = ruleset.tile_improvements.get(&road)
            .map_or(false, |improvement| {
                improvement.tech_required.is_none() ||
                civ_info.tech_manager.is_researched(improvement.tech_required.as_ref().unwrap())
            });

        let railroad_is_researched = ruleset.tile_improvements.get(&railroad)
            .map_or(false, |improvement| {
                improvement.tech_required.is_none() ||
                civ_info.tech_manager.is_researched(improvement.tech_required.as_ref().unwrap())
            });

        let capital = civ_info.get_capital().expect("Civilization should have a capital");
        let mut cities_reached_to_mediums = HashMap::new();
        cities_reached_to_mediums.insert(capital.clone(), HashSet::from(["Start".to_string()]));

        let mut cities_to_check = VecDeque::new();
        cities_to_check.push_back(capital);

        let open_borders_civ_cities = civ_info.game_info.get_cities()
            .into_iter()
            .filter(|city| Self::can_enter_borders_of(&civ_info, &city.civ))
            .collect();

        Self {
            civ_info,
            cities_reached_to_mediums,
            cities_to_check,
            new_cities_to_check: VecDeque::new(),
            open_borders_civ_cities,
            harbor,
            road,
            railroad,
            harbor_from_road,
            harbor_from_railroad,
            road_is_researched,
            railroad_is_researched,
        }
    }

    /// Finds all connections between cities and the capital
    pub fn find(&mut self) -> &HashMap<Arc<City>, HashSet<String>> {
        // We map which cities we've reached, to the mediums they've been reached by -
        // this is so we know that if we've seen which cities can be connected by port A, and one
        // of those is city B, then we don't need to check the cities that B can connect to by port,
        // since we'll get the same cities we got from A, since they're connected to the same sea.
        while !self.cities_to_check.is_empty() && self.cities_reached_to_mediums.len() < self.open_borders_civ_cities.len() {
            self.new_cities_to_check.clear();

            for city_to_connect_from in self.cities_to_check.drain(..) {
                if self.contains_harbor(&city_to_connect_from) {
                    self.check_harbor(&city_to_connect_from);
                }

                if self.railroad_is_researched {
                    let mediums_reached = self.cities_reached_to_mediums.get(&city_to_connect_from).unwrap();
                    if mediums_reached.contains("Start") ||
                       mediums_reached.contains(&self.railroad) ||
                       mediums_reached.contains(&self.harbor_from_railroad) {
                        self.check_railroad(&city_to_connect_from); // This is only relevant for city connection if there is an unbreaking line from the capital
                    }
                }

                if self.road_is_researched {
                    self.check_road(&city_to_connect_from);
                }
            }

            self.cities_to_check = self.new_cities_to_check.clone();
        }

        &self.cities_reached_to_mediums
    }

    /// Checks if a city can be reached by road
    fn check_road(&mut self, city_to_connect_from: &Arc<City>) {
        self.check(
            city_to_connect_from,
            &self.road,
            Some(&self.railroad),
            |tile| tile.has_connection(&self.civ_info),
            |_| true,
        );
    }

    /// Checks if a city can be reached by railroad
    fn check_railroad(&mut self, city_to_connect_from: &Arc<City>) {
        self.check(
            city_to_connect_from,
            &self.railroad,
            None,
            |tile| tile.get_unpillaged_road() == RoadStatus::Railroad,
            |_| true,
        );
    }

    /// Checks if a city can be reached by harbor
    fn check_harbor(&mut self, city_to_connect_from: &Arc<City>) {
        let transport_type = if self.was_previously_reached(city_to_connect_from, &self.railroad, None) {
            &self.harbor_from_railroad
        } else {
            &self.harbor_from_road
        };

        self.check(
            city_to_connect_from,
            transport_type,
            Some(&self.harbor_from_railroad),
            |tile| tile.is_water(),
            |city| city.civ == self.civ_info && self.contains_harbor(city) && !city.is_blockaded(), // use only own harbors
        );
    }

    /// Checks if a city contains a harbor
    fn contains_harbor(&self, city: &City) -> bool {
        city.contains_building_unique(UniqueType::ConnectTradeRoutes)
    }

    /// Generic check method for finding connections
    fn check<F, G>(&mut self,
                  city_to_connect_from: &Arc<City>,
                  transport_type: &str,
                  overriding_transport_type: Option<&str>,
                  tile_filter: F,
                  city_filter: G)
    where
        F: Fn(&Tile) -> bool,
        G: Fn(&City) -> bool,
    {
        // This is the time-saving mechanism we discussed earlier - If I arrived at this city via a certain BFS,
        // then obviously I already have all the cities that can be reached via that BFS so I don't need to run it again.
        if self.was_previously_reached(city_to_connect_from, transport_type, overriding_transport_type) {
            return;
        }

        let bfs = BFS::new(city_to_connect_from.get_center_tile(), |tile| {
            let owner = tile.get_owner();
            (tile.is_city_center() || tile_filter(tile)) &&
            (owner.is_none() || Self::can_enter_borders_of(&self.civ_info, owner.unwrap()))
        });

        let reached_tiles = bfs.step_to_end();

        let reached_cities: Vec<_> = self.open_borders_civ_cities.iter()
            .filter(|city| {
                reached_tiles.contains(&city.get_center_tile()) && city_filter(city)
            })
            .cloned()
            .collect();

        for reached_city in reached_cities {
            self.add_city_if_first_encountered(&reached_city);

            if reached_city == *city_to_connect_from {
                continue;
            }

            if !self.was_previously_reached(&reached_city, transport_type, overriding_transport_type) {
                self.add_medium(&reached_city, transport_type);
            }
        }
    }

    /// Adds a city to the list of cities to check if it hasn't been encountered before
    fn add_city_if_first_encountered(&mut self, reached_city: &Arc<City>) {
        if !self.cities_reached_to_mediums.contains_key(reached_city) {
            self.new_cities_to_check.push_back(reached_city.clone());
            self.cities_reached_to_mediums.insert(reached_city.clone(), HashSet::new());
        }
    }

    /// Checks if a city was previously reached by a certain transport type
    fn was_previously_reached(&self, city: &City, transport_type: &str, overriding_transport_type: Option<&str>) -> bool {
        let mediums = self.cities_reached_to_mediums.get(city).unwrap();
        mediums.contains(transport_type) ||
        overriding_transport_type.map_or(false, |overriding| mediums.contains(overriding))
    }

    /// Adds a transport medium to a city
    fn add_medium(&mut self, city: &City, transport_type: &str) {
        if let Some(mediums) = self.cities_reached_to_mediums.get_mut(city) {
            mediums.insert(transport_type.to_string());
        }
    }

    /// Checks if a civilization can enter the borders of another civilization
    fn can_enter_borders_of(civ_info: &Civilization, other_civ: &Civilization) -> bool {
        if other_civ == civ_info {
            return true; // own borders are always open
        }

        if other_civ.is_barbarian || civ_info.is_barbarian {
            return false; // barbarians blocks the routes
        }

        let diplomacy_manager = civ_info.diplomacy.get(&other_civ.civ_name)?;

        if other_civ.is_city_state && diplomacy_manager.diplomatic_status != DiplomaticStatus::War {
            return true;
        }

        diplomacy_manager.has_open_borders
    }
}