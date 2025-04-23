use crate::models::Counter;
use crate::models::ruleset::Ruleset;
use crate::models::ruleset::unique::{Unique, UniqueType};
use crate::city::City;
use crate::diplomacy::DiplomacyFlags;

/// Manages calculating Great Person Points per City for nextTurn.
pub struct GreatPersonPointsBreakdown<'a> {
    ruleset: &'a Ruleset,
    all_names: std::collections::HashSet<String>,
    base_points: Vec<Entry>,
    percent_bonuses: Vec<Entry>,
}

/// Represents any source of Great Person Points or GPP percentage bonuses
pub struct Entry {
    /// Simple label for the source of these points
    pub source: String,
    /// In case we want to show the breakdown with decorations and/or Civilopedia linking
    pub pedia_link: Option<String>,
    /// For display only - this entry affects all Great Persons and can be displayed as simple percentage without listing all GP keys
    pub is_all_gp: bool,
    /// Reference to the points, **do not mutate**
    pub counter: Counter<String>,
}

impl<'a> GreatPersonPointsBreakdown<'a> {
    const FIXED_POINT_FACTOR: i32 = 1000;

    /// Creates a new GreatPersonPointsBreakdown for the given city
    pub fn new(city: &City) -> Self {
        let mut breakdown = Self {
            ruleset: city.get_ruleset(),
            all_names: std::collections::HashSet::new(),
            base_points: Vec::new(),
            percent_bonuses: Vec::new(),
        };

        // Collect points from Specialists
        let mut specialists = Entry {
            source: "Specialists".to_string(),
            pedia_link: None,
            is_all_gp: false,
            counter: Counter::new(),
        };

        for (specialist_name, amount) in city.population.get_new_specialists() {
            if let Some(specialist) = breakdown.ruleset.specialists.get(specialist_name) {
                specialists.counter.add(&specialist.great_person_points.times(amount));
            }
        }
        breakdown.base_points.push(specialists);
        breakdown.all_names.extend(specialists.counter.keys().cloned());

        // Collect points from buildings
        for building in city.city_constructions.get_built_buildings() {
            if building.great_person_points.is_empty() {
                continue;
            }
            breakdown.base_points.push(Entry {
                source: building.name.clone(),
                pedia_link: Some(building.make_link()),
                is_all_gp: false,
                counter: building.great_person_points.clone(),
            });
            breakdown.all_names.extend(building.great_person_points.keys().cloned());
        }

        // Translate bonuses applying to all GP equally
        for item in Self::get_percentages_applying_to_all_gp(city) {
            let mut bonus_entry = Entry {
                source: item.source,
                pedia_link: item.pedia_link,
                is_all_gp: true,
                counter: Counter::new(),
            };
            for name in &breakdown.all_names {
                bonus_entry.counter.add(name, item.bonus);
            }
            breakdown.percent_bonuses.push(bonus_entry);
        }

        // And last, the GPP-type-specific GreatPersonEarnedFaster Unique
        let state_for_conditionals = &city.state;
        for unique in city.civ.get_matching_uniques(UniqueType::GreatPersonEarnedFaster, state_for_conditionals) {
            let gpp_name = &unique.params[0];
            if !breakdown.all_names.contains(gpp_name) {
                continue; // No sense applying a percentage without base points
            }
            let mut bonus_entry = Entry {
                source: Self::get_unique_source_name(unique),
                pedia_link: Self::guess_pedia_link(unique),
                is_all_gp: false,
                counter: Counter::new(),
            };
            bonus_entry.counter.add(gpp_name, unique.params[1].parse::<i32>().unwrap_or(0));
            breakdown.percent_bonuses.push(bonus_entry);
        }

        breakdown
    }

    /// Aggregate over sources, applying percentage bonuses using fixed-point math to avoid rounding surprises
    pub fn sum(&self) -> Counter<String> {
        // Accumulate base points as fake "fixed-point"
        let mut result = Counter::new();
        for entry in &self.base_points {
            result.add(&(entry.counter.clone() * Self::FIXED_POINT_FACTOR));
        }

        // Accumulate percentage bonuses additively not multiplicatively
        let mut bonuses = Counter::new();
        for entry in &self.percent_bonuses {
            bonuses.add(&entry.counter);
        }

        // Apply percent bonuses
        for key in result.keys().filter(|k| bonuses.contains_key(k)).cloned().collect::<Vec<_>>() {
            if let Some(base_value) = result.get(&key) {
                if let Some(bonus_value) = bonuses.get(&key) {
                    result.add(&key, base_value * bonus_value / 100);
                }
            }
        }

        // Round fixed-point to integers
        for key in result.keys().cloned().collect::<Vec<_>>() {
            if let Some(value) = result.get_mut(&key) {
                *value = (*value + Self::FIXED_POINT_FACTOR / 2) / Self::FIXED_POINT_FACTOR;
            }
        }

        // Remove all "gpp" values that are not valid units
        for key in result.keys().cloned().collect::<Vec<_>>() {
            if !self.ruleset.units.contains_key(&key) {
                result.remove(&key);
            }
        }

        result
    }

    /// Get all percentage bonuses that apply to all GPP
    fn get_percentages_applying_to_all_gp(city: &City) -> Vec<AllGPPercentageEntry> {
        let mut entries = Vec::new();

        // Add bonuses for GreatPersonPointPercentage
        for unique in city.get_matching_uniques(UniqueType::GreatPersonPointPercentage) {
            if !city.matches_filter(&unique.params[1]) {
                continue;
            }
            entries.push(AllGPPercentageEntry {
                source: Self::get_unique_source_name(unique),
                pedia_link: Self::guess_pedia_link(unique),
                bonus: unique.params[0].parse::<i32>().unwrap_or(0),
            });
        }

        // Add bonuses for GreatPersonBoostWithFriendship (Sweden UP)
        let civ = &city.civ;
        for other_civ in civ.get_known_civs() {
            if !civ.get_diplomacy_manager(other_civ)
                .map_or(false, |dm| dm.has_flag(DiplomacyFlags::DeclarationOfFriendship))
            {
                continue;
            }
            let boost_uniques = civ.get_matching_uniques(UniqueType::GreatPersonBoostWithFriendship)
                .into_iter()
                .chain(other_civ.get_matching_uniques(UniqueType::GreatPersonBoostWithFriendship));
            for unique in boost_uniques {
                entries.push(AllGPPercentageEntry {
                    source: "Declaration of Friendship".to_string(),
                    pedia_link: None,
                    bonus: unique.params[0].parse::<i32>().unwrap_or(0),
                });
            }
        }

        entries
    }

    /// Get the source name for a unique
    fn get_unique_source_name(unique: &Unique) -> String {
        unique.source_object_name.clone().unwrap_or_else(|| "Bonus".to_string())
    }

    /// Guess the pedia link for a unique
    fn guess_pedia_link(unique: &Unique) -> Option<String> {
        unique.source_object_name.as_ref().and_then(|name| {
            unique.source_object_type.as_ref().map(|t| format!("{}/{}", t.name, name))
        })
    }

    /// Get the total percentage bonus for great person points
    pub fn get_great_person_percentage_bonus(city: &City) -> i32 {
        Self::get_percentages_applying_to_all_gp(city)
            .iter()
            .map(|entry| entry.bonus)
            .sum()
    }
}

/// Return type component of the get_percentages_applying_to_all_gp helper
struct AllGPPercentageEntry {
    source: String,
    pedia_link: Option<String>,
    bonus: i32,
}