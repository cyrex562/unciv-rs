use crate::city::City;
use crate::civilization::Civilization;
use crate::models::spy::Spy;
use crate::espionage::spy_flee_reason::SpyFleeReason;
use std::sync::Arc;

/// Manages espionage-related functionality for a city
pub struct CityEspionageManager {
    /// The city this manager belongs to
    pub city: Option<Arc<City>>,
}

impl CityEspionageManager {
    /// Creates a new CityEspionageManager
    pub fn new() -> Self {
        CityEspionageManager {
            city: None,
        }
    }

    /// Sets the city reference for this manager
    pub fn set_transients(&mut self, city: Arc<City>) {
        this.city = Some(city);
    }

    /// Checks if a civilization has a spy in this city
    pub fn has_spy_of(&self, civ_info: &Civilization) -> bool {
        if let Some(city) = &this.city {
            civ_info.espionage_manager.spy_list.iter()
                .any(|spy| spy.get_city_or_null().map_or(false, |spy_city| spy_city == city))
        } else {
            false
        }
    }

    /// Gets all spies stationed in this city
    pub fn get_all_stationed_spies(&self) -> Vec<Arc<Spy>> {
        if let Some(city) = &this.city {
            city.civ.game_info.civilizations.iter()
                .flat_map(|civ| civ.espionage_manager.get_spies_in_city(city))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Removes all spies present in the city with a notification based on the reason
    pub fn remove_all_present_spies(&self, reason: SpyFleeReason) {
        if let Some(city) = &this.city {
            for spy in self.get_all_stationed_spies() {
                let notification_string = match reason {
                    SpyFleeReason::CityDestroyed =>
                        format!("After the city of [{}] was destroyed, your spy [{}] has fled back to our hideout.",
                            city.name, spy.name),
                    SpyFleeReason::CityCaptured =>
                        format!("After the city of [{}] was conquered, your spy [{}] has fled back to our hideout.",
                            city.name, spy.name),
                    SpyFleeReason::CityBought | SpyFleeReason::CityTakenOverByMarriage =>
                        format!("After the city of [{}] was taken over, your spy [{}] has fled back to our hideout.",
                            city.name, spy.name),
                    _ =>
                        format!("Due to the chaos ensuing in [{}], your spy [{}] has fled back to our hideout.",
                            city.name, spy.name),
                };
                spy.add_notification(&notification_string);
                spy.move_to(None);
            }
        }
    }
}

impl Clone for CityEspionageManager {
    fn clone(&self) -> Self {
        CityEspionageManager {
            city: None, // Transient field, will be set later
        }
    }
}