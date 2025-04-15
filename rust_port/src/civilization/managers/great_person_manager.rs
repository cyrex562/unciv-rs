use crate::civilization::{Civilization, MayaLongCountAction, NotificationCategory};
use crate::models::counter::Counter;
use crate::models::ruleset::unique::UniqueType;
use crate::ui::components::MayaCalendar;
use std::collections::HashSet;
use std::sync::Arc;
use serde::{Serialize, Deserialize};

/// Manages great people for a civilization
#[derive(Clone, Serialize, Deserialize)]
pub struct GreatPersonManager {
    /// Reference to the civilization this manager belongs to
    #[serde(skip)]
    pub civ_info: Option<Arc<Civilization>>,

    /// Base points required for next great person, without speed modifier
    pub points_for_next_great_person_counter: Counter<String>,

    /// Base points required for next great general
    pub points_for_next_great_general: i32,

    /// Base points required for next great general by type
    pub points_for_next_great_general_counter: Counter<String>,

    /// Current great person points by type
    pub great_person_points_counter: Counter<String>,

    /// Current great general points by type
    pub great_general_points_counter: Counter<String>,

    /// Current great general points
    pub great_general_points: i32,

    /// Number of free great people available
    pub free_great_people: i32,

    /// Number of free great people limited by Maya ability (each only once until all used)
    pub maya_limited_free_gp: i32,

    /// Remaining candidates for Maya ability - whenever empty refilled from all GP, starts out empty
    pub long_count_gp_pool: HashSet<String>,
}

impl GreatPersonManager {
    /// Creates a new GreatPersonManager
    pub fn new() -> Self {
        Self {
            civ_info: None,
            points_for_next_great_person_counter: Counter::new(),
            points_for_next_great_general: 200,
            points_for_next_great_general_counter: Counter::new(),
            great_person_points_counter: Counter::new(),
            great_general_points_counter: Counter::new(),
            great_general_points: 0,
            free_great_people: 0,
            maya_limited_free_gp: 0,
            long_count_gp_pool: HashSet::new(),
        }
    }

    /// Sets the transient references to the civilization
    pub fn set_transients(&mut self, civ_info: Arc<Civilization>) {
        this.civ_info = Some(civ_info);
    }

    /// Gets the pool key for a great person
    fn get_pool_key(&self, great_person: &str) -> String {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");

        civ_info.get_equivalent_unit(great_person)
            .get_matching_uniques(UniqueType::GPPointPool)
            .first()
            .and_then(|unique| unique.params.get(0))
            .cloned()
            .unwrap_or_default() // An empty string is used to indicate the Unique wasn't found
    }

    /// Gets the points required for a great person
    pub fn get_points_required_for_great_person(&self, great_person: &str) -> i32 {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");
        let key = self.get_pool_key(great_person);

        let points = self.points_for_next_great_person_counter.get(&key).unwrap_or(&0);
        if *points == 0 {
            // Initialize with default value if not set
            self.points_for_next_great_person_counter.insert(key.clone(), 100);
            (100.0 * civ_info.game_info.speed.modifier) as i32
        } else {
            (*points as f32 * civ_info.game_info.speed.modifier) as i32
        }
    }

    /// Gets a new great person if enough points have been accumulated
    pub fn get_new_great_person(&mut self) -> Option<String> {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");

        // Check for great generals first
        for (unit, value) in self.great_general_points_counter.iter() {
            let required_points = self.points_for_next_great_general_counter.get(unit).unwrap_or(&0);
            let required_points = if *required_points == 0 {
                self.points_for_next_great_general_counter.insert(unit.clone(), 200);
                200
            } else {
                *required_points
            };

            if *value > required_points {
                self.great_general_points_counter.add(unit, -required_points);
                self.points_for_next_great_general_counter.add(unit, 50);
                return Some(unit.clone());
            }
        }

        // Then check for other great people
        for (great_person, value) in self.great_person_points_counter.iter() {
            let required_points = self.get_points_required_for_great_person(great_person);

            if *value >= required_points {
                self.great_person_points_counter.add(great_person, -required_points);
                let pool_key = self.get_pool_key(great_person);
                let current_points = self.points_for_next_great_person_counter.get(&pool_key).unwrap_or(&0);
                self.points_for_next_great_person_counter.insert(pool_key, current_points * 2);
                return Some(great_person.clone());
            }
        }

        None
    }

    /// Adds great person points for the next turn
    pub fn add_great_person_points(&mut self) {
        self.great_person_points_counter.add_all(self.get_great_person_points_for_next_turn());
    }

    /// Triggers the Maya great person ability
    pub fn trigger_mayan_great_person(&mut self) {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");

        if civ_info.is_spectator() {
            return;
        }

        let great_people = self.get_great_people();

        if self.long_count_gp_pool.is_empty() {
            self.long_count_gp_pool = great_people.iter()
                .map(|gp| gp.name.clone())
                .collect();
        }

        self.free_great_people += 1;
        this.maya_limited_free_gp += 1;

        // Anyone an idea for a good icon?
        let notification = "{A new b'ak'tun has just begun!}\n{A Great Person joins you!}";
        civ_info.add_notification(
            notification,
            MayaLongCountAction::new(),
            NotificationCategory::General,
            MayaCalendar::notification_icon()
        );
    }

    /// Gets great people specific to this manager's Civilization, already filtered by `isHiddenBySettings`
    pub fn get_great_people(&self) -> HashSet<Unit> {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");

        civ_info.game_info.ruleset.units.values()
            .filter(|unit| unit.is_great_person)
            .map(|unit| civ_info.get_equivalent_unit(&unit.name))
            .filter(|unit| !unit.is_unavailable_by_settings(&civ_info.game_info))
            .collect()
    }

    /// Gets great person points for the next turn
    pub fn get_great_person_points_for_next_turn(&self) -> Counter<String> {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");
        let mut great_person_points = Counter::new();

        for city in &civ_info.cities {
            great_person_points.add_all(city.get_great_person_points());
        }

        great_person_points
    }
}