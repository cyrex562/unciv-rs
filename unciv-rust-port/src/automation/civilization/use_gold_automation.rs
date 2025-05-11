use crate::models::civilization::Civilization;
use crate::models::city::City;
use crate::models::map::tile::Tile;
use crate::models::map::bfs::BFS;
use crate::models::ruleset::INonPerpetualConstruction;
use crate::models::ruleset::tile::ResourceType;
use crate::models::ruleset::unique::UniqueType;
use crate::models::stats::Stat;
use crate::automation::unit::UnitAutomation;
use crate::automation::civilization::MotivationToAttackAutomation;
use crate::automation::civilization::NextTurnAutomation;
use std::collections::{BTreeMap, HashSet};
use std::cmp::min;

/// Contains logic for automating gold spending decisions
pub struct UseGoldAutomation;

impl UseGoldAutomation {
    /// Allows AI to spend money to purchase city-state friendship, buildings & units
    pub fn use_gold(civ: &mut Civilization) {
        // Upgrade units
        for unit in civ.units.get_civ_units() {
            UnitAutomation::try_upgrade_unit(unit);
        }

        // Handle city-state influence if major civ
        if civ.is_major_civ() {
            Self::use_gold_for_city_states(civ);
        }

        // Purchase constructions in cities
        for city in civ.cities.iter().sorted_by_descending(|c| c.population.population) {
            let construction = city.city_constructions.get_current_construction();

            // Skip if not a non-perpetual construction
            if !construction.is_non_perpetual_construction() {
                continue;
            }

            // Get gold cost
            let stat_buy_cost = match construction.get_stat_buy_cost(city, Stat::Gold) {
                Some(cost) => cost,
                None => continue,
            };

            // Check if purchase is allowed
            if !city.city_constructions.is_construction_purchase_allowed(construction, Stat::Gold, stat_buy_cost) {
                continue;
            }

            // Check if we have enough gold (3x the cost)
            if civ.gold < stat_buy_cost * 3 {
                continue;
            }

            // Purchase the construction
            city.city_constructions.purchase_construction(construction, 0, true);
        }

        // Consider buying city tiles
        Self::maybe_buy_city_tiles(civ);
    }

    /// Uses gold to influence city-states
    fn use_gold_for_city_states(civ: &mut Civilization) {
        // RARE EDGE CASE: If you ally with a city-state, you may reveal more map that includes ANOTHER civ!
        // So if we don't lock this list, we may later discover that there are more known civs, concurrent modification exception!
        let known_city_states: Vec<_> = civ.get_known_civs()
            .iter()
            .filter(|c| c.is_city_state() && MotivationToAttackAutomation::has_at_least_motivation_to_attack(civ, c, 0.0) <= 0)
            .cloned()
            .collect();

        // canBeMarriedBy checks actual cost, but it can't be below 500*speedmodifier, and the later check is expensive
        if civ.gold >= 330 && civ.get_happiness() > 0 &&
            (civ.has_unique(UniqueType::CityStateCanBeBoughtForGold) ||
             civ.has_unique(UniqueType::CityStateCanBeBoughtForGoldOld))
        {
            // Materialize sequence as diplomaticMarriage may kill a CS
            for city_state in known_city_states.iter() {
                if city_state.city_state_functions.can_be_married_by(civ) {
                    city_state.city_state_functions.diplomatic_marriage(civ);
                }
                if civ.get_happiness() <= 0 {
                    break; // Stop marrying if happiness is getting too low
                }
            }
        }

        if civ.gold < 500 || known_city_states.is_empty() {
            return; // skip checks if tryGainInfluence will bail anyway
        }

        // Find the most valuable city-state to influence
        let city_state_with_value = known_city_states.iter()
            .filter(|cs| cs.get_ally_civ() != Some(civ.civ_name.clone()))
            .map(|cs| {
                let value = NextTurnAutomation::value_city_state_alliance(civ, cs, true);
                (cs, value)
            })
            .max_by_key(|(_, value)| *value);

        if let Some((city_state, value)) = city_state_with_value {
            if value > 0 {
                Self::try_gain_influence(civ, city_state);
            }
        }
    }

    /// Attempts to buy city tiles
    fn maybe_buy_city_tiles(civ_info: &mut Civilization) {
        if civ_info.gold <= 0 {
            return;
        }

        // Don't buy tiles in the very early game. It is unlikely that we already have the required
        // tech, the necessary worker and that there is a reasonable threat from another player to
        // grab the tile. We could also check all that, but it would require a lot of cycles each
        // turn and this is probably a good approximation.
        if civ_info.game_info.turns < (civ_info.game_info.speed.science_cost_modifier * 20.0) as i32 {
            return;
        }

        let highly_desirable_tiles = Self::get_highly_desirable_tiles_to_city_map(civ_info);

        // Always try to buy highly desirable tiles if it can be afforded.
        for (tile, cities) in highly_desirable_tiles {
            let city_with_least_cost_to_buy = cities.iter()
                .min_by_key(|city| city.get_center_tile().aerial_distance_to(&tile))
                .unwrap();

            let mut bfs = BFS::new(city_with_least_cost_to_buy.get_center_tile());
            bfs.set_filter(|t| t.get_owner().is_none() || t.owning_city == Some(city_with_least_cost_to_buy.clone()));
            bfs.step_until_destination(&tile);

            let tiles_that_need_buying: Vec<_> = bfs.get_path_to(&tile)
                .iter()
                .filter(|t| t.get_owner().is_none())
                .cloned()
                .collect();

            let tiles_that_need_buying: Vec<_> = tiles_that_need_buying.iter().rev().cloned().collect(); // getPathTo is from destination to source

            // We're trying to acquire everything and revert if it fails, because of the difficult
            // way how tile acquisition cost is calculated. Everytime you buy a tile, the next one
            // gets more expensive and by how much depends on other things such as game speed. To
            // not introduce hidden dependencies on that and duplicate that logic here to calculate
            // the price of the whole path, this is probably simpler.
            let mut ran_out_of_money = false;
            let mut gold_spent = 0;

            for tile_that_needs_buying in tiles_that_need_buying.iter() {
                let gold_cost_of_tile = city_with_least_cost_to_buy.expansion.get_gold_cost_of_tile(tile_that_needs_buying);

                if civ_info.gold >= gold_cost_of_tile {
                    city_with_least_cost_to_buy.expansion.buy_tile(tile_that_needs_buying);
                    gold_spent += gold_cost_of_tile;
                } else {
                    ran_out_of_money = true;
                    break;
                }
            }

            if ran_out_of_money {
                for tile_that_needs_buying in tiles_that_need_buying.iter() {
                    city_with_least_cost_to_buy.expansion.relinquish_ownership(tile_that_needs_buying);
                }
                civ_info.add_gold(gold_spent);
            }
        }
    }

    /// Gets a map of highly desirable tiles to cities that want them
    fn get_highly_desirable_tiles_to_city_map(civ_info: &Civilization) -> BTreeMap<Tile, HashSet<City>> {
        let mut highly_desirable_tiles: BTreeMap<Tile, HashSet<City>> = BTreeMap::new();

        // Custom comparator for the BTreeMap
        let compare_by_descending = |a: &Tile, b: &Tile| {
            // First compare by natural wonder
            match (a.natural_wonder.is_some(), b.natural_wonder.is_some()) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => {
                    // Then compare by luxury resource
                    match (a.has_luxury_resource(), b.has_luxury_resource()) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => {
                            // Then compare by strategic resource
                            match (a.has_strategic_resource(), b.has_strategic_resource()) {
                                (true, false) => std::cmp::Ordering::Less,
                                (false, true) => std::cmp::Ordering::Greater,
                                _ => {
                                    // Finally compare by hash code to maintain uniqueness
                                    a.hash_code().cmp(&b.hash_code())
                                }
                            }
                        }
                    }
                }
            }
        };

        for city in civ_info.cities.iter().filter(|c| !c.is_puppet && !c.is_being_razed) {
            let highly_desirable_tiles_in_city: Vec<_> = city.tiles_in_range.iter()
                .filter(|t| Self::is_highly_desirable_tile(t, civ_info, city))
                .cloned()
                .collect();

            for tile in highly_desirable_tiles_in_city {
                highly_desirable_tiles.entry(tile)
                    .or_insert_with(HashSet::new)
                    .insert(city.clone());
            }
        }

        highly_desirable_tiles
    }

    /// Checks if a tile is highly desirable
    fn is_highly_desirable_tile(tile: &Tile, civ_info: &Civilization, city: &City) -> bool {
        if !tile.is_visible(civ_info) {
            return false;
        }

        if tile.get_owner().is_some() {
            return false;
        }

        if !tile.neighbors.iter().any(|neighbor| neighbor.get_city() == Some(city.clone())) {
            return false;
        }

        // Check for natural wonder
        let has_natural_wonder = tile.natural_wonder.is_some();

        // Check for luxury resource we don't own
        let has_luxury_civ_doesnt_own = tile.has_viewable_resource(civ_info) &&
            tile.tile_resource.resource_type == ResourceType::Luxury &&
            !civ_info.has_resource(&tile.resource.unwrap());

        // Check for strategic resource we have none or little of
        let has_resource_civ_has_none_or_little = tile.has_viewable_resource(civ_info) &&
            tile.tile_resource.resource_type == ResourceType::Strategic &&
            civ_info.get_resource_amount(&tile.resource.unwrap()) <= 3;

        has_natural_wonder || has_luxury_civ_doesnt_own || has_resource_civ_has_none_or_little
    }

    /// Attempts to gain influence with a city-state
    fn try_gain_influence(civ_info: &mut Civilization, city_state: &Civilization) {
        if civ_info.gold < 500 {
            return; // Save up, giving 500 gold in one go typically grants +5 influence compared to giving 2×250 gold
        }

        let influence = city_state.get_diplomacy_manager(civ_info).get_influence();
        let stop_spending = influence > 60 + 2 * NextTurnAutomation::value_city_state_alliance(civ_info, city_state, true);

        // Don't go into a gold gift race: be content with friendship for cheap, or use the gold on more productive uses,
        // for example upgrading an army to conquer the player who's contesting our city states
        if influence < 10 || stop_spending {
            return;
        }

        // Only make an investment if we got our Pledge to Protect influence at the highest level
        if civ_info.gold >= 1000 {
            city_state.city_state_functions.receive_gold_gift(civ_info, 1000);
        } else {
            city_state.city_state_functions.receive_gold_gift(civ_info, 500);
        }
    }
}

/// Helper trait for sorting collections
trait SortedByDescending<T> {
    fn sorted_by_descending<F>(self, compare: F) -> Vec<T> where
        F: FnMut(&T, &T) -> std::cmp::Ordering;
}

impl<T> SortedByDescending<T> for Vec<T> where T: Clone {
    fn sorted_by_descending<F>(self, mut compare: F) -> Vec<T> where
        F: FnMut(&T, &T) -> std::cmp::Ordering {
        let mut result = self;
        result.sort_by(|a, b| compare(b, a));
        result
    }
}

/// Helper trait for collections
trait IsEmpty {
    fn is_empty(&self) -> bool;
}

impl<T> IsEmpty for Vec<T> {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl<K, V> IsEmpty for BTreeMap<K, V> {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

/// Helper trait for collections
trait HashCode {
    fn hash_code(&self) -> i32;
}

impl HashCode for Tile {
    fn hash_code(&self) -> i32 {
        // Simple hash function for tiles
        let x = self.position.x;
        let y = self.position.y;
        ((x * 31) + y) as i32
    }
}

/// Helper trait for tiles
trait HasLuxuryResource {
    fn has_luxury_resource(&self) -> bool;
}

impl HasLuxuryResource for Tile {
    fn has_luxury_resource(&self) -> bool {
        self.resource.is_some() && self.tile_resource.resource_type == ResourceType::Luxury
    }
}

/// Helper trait for tiles
trait HasStrategicResource {
    fn has_strategic_resource(&self) -> bool;
}

impl HasStrategicResource for Tile {
    fn has_strategic_resource(&self) -> bool {
        self.resource.is_some() && self.tile_resource.resource_type == ResourceType::Strategic
    }
}