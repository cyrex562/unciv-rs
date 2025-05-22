use crate::city::city_flags::CityFlags;
use crate::city::city_focus::CityFocus;
use crate::city::managers::spy_flee_reason::SpyFleeReason;
use crate::city::City;
use crate::civilization::city_action::CityAction;
use crate::civilization::location_action::LocationAction;
use crate::civilization::notification_category::NotificationCategory;
use crate::civilization::notification_icon::NotificationIcon;
use crate::civilization::overview_action::OverviewAction;
use crate::models::ruleset::tile::resource_type::ResourceType;
use crate::models::ruleset::unique::unique_type::UniqueType;
use crate::ui::screens::overviewscreen::empire_overview_categories::EmpireOverviewCategories;
use rand::Rng;
use std::cmp::min;
use std::sync::Arc;

/// Manages turn-based operations for cities
pub struct CityTurnManager {
    /// The city this manager belongs to
    pub city: Arc<City>,
}

impl CityTurnManager {
    /// Creates a new CityTurnManager
    pub fn new(city: Arc<City>) -> Self {
        CityTurnManager { city }
    }

    /// Processes the start of a turn for the city
    pub fn start_turn(&mut self) {
        // Construct units at the beginning of the turn,
        // so they won't be generated out in the open and vulnerable to enemy attacks before you can control them
        self.city.city_constructions.construct_if_enough();

        self.city.try_update_road_status();
        self.city.attacked_this_turn = false;

        // The ordering is intentional - you get a turn without WLTKD even if you have the next resource already
        // Also resolve end of resistance before updateCitizens
        if !self.city.has_flag(CityFlags::WeLoveTheKing) {
            self.try_we_love_the_king();
        }
        self.next_turn_flags();

        if self.city.is_puppet {
            self.city.set_city_focus(CityFocus::GoldFocus);
            self.city.reassign_all_population();
        } else if self.city.should_reassign_population {
            self.city.reassign_population(); // includes cityStats.update
        } else {
            self.city.city_stats.update();
        }

        // Seed resource demand countdown
        if self.city.demanded_resource.is_empty() && !self.city.has_flag(CityFlags::ResourceDemand)
        {
            let base_countdown = if self.city.is_capital() { 25 } else { 15 };
            let random_addition = rand::thread_rng().gen_range(0..10);
            self.city
                .set_flag(CityFlags::ResourceDemand, base_countdown + random_addition);
        }
    }

    /// Tries to trigger We Love The King Day
    fn try_we_love_the_king(&mut self) {
        if self.city.demanded_resource.is_empty() {
            return;
        }
        if self
            .city
            .get_available_resource_amount(&self.city.demanded_resource)
            > 0
        {
            self.city.set_flag(CityFlags::WeLoveTheKing, 20 + 1); // +1 because it will be decremented by 1 in the same startTurn()
            self.city.civ.add_notification(
                format!("Because they have [{}], the citizens of [{}] are celebrating We Love The King Day!",
                    self.city.demanded_resource,
                    self.city.name),
                CityAction::with_location(&self.city),
                NotificationCategory::General,
                NotificationIcon::City,
                Some(NotificationIcon::Happiness)
            );
        }
    }

    /// Processes flags for the next turn
    fn next_turn_flags(&mut self) {
        let flag_keys: Vec<_> = self.city.flags_countdown.keys().cloned().collect();
        for flag in flag_keys {
            if let Some(countdown) = self.city.flags_countdown.get_mut(&flag) {
                if *countdown > 0 {
                    *countdown -= 1;
                }

                if *countdown == 0 {
                    self.city.flags_countdown.remove(&flag);

                    match flag.as_str() {
                        "ResourceDemand" => {
                            self.demand_new_resource();
                        }
                        "WeLoveTheKing" => {
                            self.city.civ.add_notification(
                                format!("We Love The King Day in [{}] has ended.", self.city.name),
                                CityAction::with_location(&self.city),
                                NotificationCategory::General,
                                NotificationIcon::City,
                                None,
                            );
                            self.demand_new_resource();
                        }
                        "Resistance" => {
                            self.city.should_reassign_population = true;
                            self.city.civ.add_notification(
                                format!("The resistance in [{}] has ended!", self.city.name),
                                CityAction::with_location(&self.city),
                                NotificationCategory::General,
                                "StatIcons/Resistance",
                            );
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Demands a new resource from the city
    fn demand_new_resource(&mut self) {
        let candidates: Vec<_> = self
            .city
            .get_ruleset()
            .tile_resources
            .values()
            .filter(|resource| {
                resource.resource_type == ResourceType::Luxury && // Must be luxury
                !resource.has_unique(UniqueType::CityStateOnlyResource) && // Not a city-state only resource eg jewelry
                resource.name != self.city.demanded_resource && // Not same as last time
                self.city.tile_map.resources.contains(&resource.name) && // Must exist somewhere on the map
                !self.city.get_center_tile().get_tiles_in_distance(self.city.get_work_range())
                    .iter()
                    .any(|near_tile| near_tile.resource == Some(resource.name.clone()))
                // Not in this city's radius
            })
            .collect();

        let missing_resources: Vec<_> = candidates
            .iter()
            .filter(|resource| !self.city.civ.has_resource(&resource.name))
            .collect();

        if missing_resources.is_empty() {
            // hooray happpy day forever!
            if let Some(random_resource) = candidates.choose(&mut rand::thread_rng()) {
                self.city.demanded_resource = random_resource.name.clone();
            } else {
                self.city.demanded_resource = String::new();
            }
            return; // actually triggering "wtlk" is done in tryWeLoveTheKing(), *next turn*
        }

        if let Some(chosen_resource) = missing_resources.choose(&mut rand::thread_rng()) {
            self.city.demanded_resource = chosen_resource.name.clone();
        } else {
            self.city.demanded_resource = String::new();
        }

        if self.city.demanded_resource.is_empty() {
            // Failed to get a valid resource, try again some time later
            let random_addition = rand::thread_rng().gen_range(0..10);
            self.city
                .set_flag(CityFlags::ResourceDemand, 15 + random_addition);
        } else {
            self.city.civ.add_notification(
                format!(
                    "[{}] demands [{}]!",
                    self.city.name, self.city.demanded_resource
                ),
                vec![
                    LocationAction::new(self.city.location),
                    OverviewAction::new(EmpireOverviewCategories::Resources),
                ],
                NotificationCategory::General,
                NotificationIcon::City,
                Some(format!("ResourceIcons/{}", self.city.demanded_resource)),
            );
        }
    }

    /// Processes the end of a turn for the city
    pub fn end_turn(&mut self) {
        let stats = self.city.city_stats.current_city_stats;

        self.city.city_constructions.end_turn(&stats);
        self.city.expansion.next_turn(stats.culture);

        if self.city.is_being_razed {
            let mut removed_population = 1;
            removed_population += self
                .city
                .civ
                .get_matching_uniques(UniqueType::CitiesAreRazedXTimesFaster)
                .iter()
                .map(|unique| unique.params[0].parse::<i32>().unwrap_or(1) - 1)
                .sum::<i32>();

            if self.city.population.population <= removed_population {
                self.city
                    .espionage
                    .remove_all_present_spies(SpyFleeReason::Other);
                self.city.civ.add_notification(
                    format!("[{}] has been razed to the ground!", self.city.name),
                    self.city.location,
                    NotificationCategory::General,
                    "OtherIcons/Fire",
                    None,
                );
                self.city.destroy_city();
            } else {
                //if not razed yet:
                self.city.population.add_population(-removed_population);
                if self.city.population.food_stored
                    >= self.city.population.get_food_to_next_population()
                {
                    //if surplus in the granary...
                    self.city.population.food_stored =
                        self.city.population.get_food_to_next_population() - 1; //...reduce below the new growth threshold
                }
            }
        } else {
            self.city
                .population
                .next_turn(self.city.food_for_next_turn());
        }

        // This should go after the population change, as that might impact the amount of followers in this city
        if self.city.civ.game_info.is_religion_enabled() {
            self.city.religion.end_turn();
        }

        if self.city.civ.cities.contains(&self.city) {
            // city was not destroyed
            self.city.health = min(self.city.health + 20, self.city.get_max_health());
            self.city.population.unassign_extra_population();
        }
    }
}
