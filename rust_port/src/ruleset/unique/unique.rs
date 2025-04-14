use std::collections::{HashMap, HashSet};
use std::iter::repeat;
use std::sync::OnceLock;

use crate::models::city::City;
use crate::models::civilization::Civilization;
use crate::models::constants::UNIQUE_OR_DELIMITER;
use crate::models::game_info::IsPartOfGameInfoSerialization;
use crate::models::ruleset::{GlobalUniques, Ruleset, UniqueTarget, UniqueType, UniqueValidator};
use crate::models::stats::Stats;
use crate::models::translations::{get_modifiers, get_placeholder_parameters, get_placeholder_text, remove_conditionals};
use crate::models::unique::conditionals::Conditionals;
use crate::models::unique::countables::Countables;
use crate::models::unique::state_for_conditionals::StateForConditionals;

/// A unique ability in the game.
#[derive(Clone, Debug)]
pub struct Unique {
    /// The raw text of the unique
    pub text: String,
    /// The type of object this unique is for
    pub source_object_type: Option<UniqueTarget>,
    /// The name of the object this unique is for
    pub source_object_name: Option<String>,

    /// The placeholder text of the unique (cached)
    pub placeholder_text: String,
    /// The parameters of the unique (without conditionals)
    pub params: Vec<String>,
    /// The type of the unique
    pub unique_type: Option<UniqueType>,

    /// The stats of the unique (cached)
    pub stats: Stats,
    /// The modifiers of the unique
    pub modifiers: Vec<Unique>,
    /// The modifiers of the unique mapped by type
    pub modifiers_map: HashMap<UniqueType, Vec<Unique>>,

    /// Whether this unique is timed triggerable
    pub is_timed_triggerable: bool,
    /// Whether this unique is triggerable
    pub is_triggerable: bool,

    /// All parameters of the unique (including conditionals)
    pub all_params: Vec<String>,
    /// Whether this unique is a local effect
    pub is_local_effect: bool,
}

impl Unique {
    /// Create a new unique
    pub fn new(text: String, source_object_type: Option<UniqueTarget>, source_object_name: Option<String>) -> Self {
        let placeholder_text = get_placeholder_text(&text);
        let params = get_placeholder_parameters(&text);
        let unique_type = UniqueType::unique_type_map.get(&placeholder_text).cloned();

        let stats = {
            let first_stat_param = params.iter().find(|param| Stats::is_stats(param));
            if first_stat_param.is_none() {
                Stats::new() // So badly-defined stats don't crash the entire game
            } else {
                Stats::parse(first_stat_param.unwrap())
            }
        };

        let modifiers = get_modifiers(&text);
        let modifiers_map = modifiers
            .iter()
            .filter(|m| m.unique_type.is_some())
            .fold(HashMap::new(), |mut map, modifier| {
                let unique_type = modifier.unique_type.unwrap();
                map.entry(unique_type).or_insert_with(Vec::new).push(modifier.clone());
                map
            });

        let is_timed_triggerable = modifiers.iter().any(|m| m.unique_type == Some(UniqueType::ConditionalTimedUnique));

        let is_triggerable = unique_type.as_ref().map_or(false, |t| {
            t.target_types.contains(&UniqueTarget::Triggerable) ||
            t.target_types.contains(&UniqueTarget::UnitTriggerable) ||
            is_timed_triggerable
        });

        let all_params = {
            let mut params = params.clone();
            params.extend(modifiers.iter().flat_map(|m| m.params.clone()));
            params
        };

        let is_local_effect = params.contains(&"in this city".to_string()) ||
            modifiers.iter().any(|m| m.unique_type == Some(UniqueType::ConditionalInThisCity));

        Self {
            text,
            source_object_type,
            source_object_name,
            placeholder_text,
            params,
            unique_type,
            stats,
            modifiers,
            modifiers_map,
            is_timed_triggerable,
            is_triggerable,
            all_params,
            is_local_effect,
        }
    }

    /// Check if this unique has a flag
    pub fn has_flag(&self, flag: UniqueFlag) -> bool {
        self.unique_type.as_ref().map_or(false, |t| t.flags.contains(&flag))
    }

    /// Check if this unique is hidden to users
    pub fn is_hidden_to_users(&self) -> bool {
        self.has_flag(UniqueFlag::HiddenToUsers) ||
            self.modifiers.iter().any(|m| m.unique_type == Some(UniqueType::ModifierHiddenFromUsers))
    }

    /// Get modifiers of a specific type
    pub fn get_modifiers(&self, unique_type: UniqueType) -> &[Unique] {
        self.modifiers_map.get(&unique_type).map_or(&[], |v| v)
    }

    /// Check if this unique has a modifier of a specific type
    pub fn has_modifier(&self, unique_type: UniqueType) -> bool {
        self.modifiers_map.contains_key(&unique_type)
    }

    /// Check if this unique is modified by game speed
    pub fn is_modified_by_game_speed(&self) -> bool {
        self.has_modifier(UniqueType::ModifiedByGameSpeed)
    }

    /// Check if this unique has a trigger conditional
    pub fn has_trigger_conditional(&self) -> bool {
        if self.modifiers.is_empty() {
            return false;
        }

        self.modifiers.iter().any(|conditional| {
            conditional.unique_type.as_ref().map_or(false, |t| {
                t.target_types.iter().any(|target_type| {
                    target_type.can_accept_unique_target(&UniqueTarget::TriggerCondition) ||
                    target_type.can_accept_unique_target(&UniqueTarget::UnitActionModifier)
                })
            })
        })
    }

    /// Check if conditionals apply with a civilization and city
    pub fn conditionals_apply_with_civ_city(&self, civ_info: Option<&Civilization>, city: Option<&City>) -> bool {
        let state = StateForConditionals::new_with_game_info(
            civ_info.map(|c| c.game_info.clone()),
            civ_info.cloned(),
            city.cloned(),
        );
        self.conditionals_apply(&state)
    }

    /// Check if conditionals apply with a state
    pub fn conditionals_apply(&self, state: &StateForConditionals) -> bool {
        if state.ignore_conditionals {
            return true;
        }
        // Always allow Timed conditional uniques. They are managed elsewhere
        if self.is_timed_triggerable {
            return true;
        }
        if self.modifiers.is_empty() {
            return true;
        }
        for modifier in &self.modifiers {
            if !Conditionals::conditional_applies(self, modifier, state) {
                return false;
            }
        }
        true
    }

    /// Get the unique multiplier for a state
    fn get_unique_multiplier(&self, state_for_conditionals: &StateForConditionals) -> i32 {
        let mut amount = 1;

        let for_every_modifiers = self.get_modifiers(UniqueType::ForEveryCountable);
        for conditional in for_every_modifiers {
            // multiple multipliers DO multiply.
            if let Some(multiplier) = Countables::get_countable_amount(&conditional.params[0], state_for_conditionals) {
                amount *= multiplier;
            }
        }

        let for_every_amount_modifiers = self.get_modifiers(UniqueType::ForEveryAmountCountable);
        for conditional in for_every_amount_modifiers {
            // multiple multipliers DO multiply.
            if let Some(multiplier) = Countables::get_countable_amount(&conditional.params[1], state_for_conditionals) {
                let per_every = conditional.params[0].parse::<i32>().unwrap_or(1);
                amount *= multiplier / per_every;
            }
        }

        if let Some(tile) = state_for_conditionals.relevant_tile() {
            let for_every_adjacent_tile_modifiers = self.get_modifiers(UniqueType::ForEveryAdjacentTile);
            for conditional in for_every_adjacent_tile_modifiers {
                let multiplier = tile.neighbors
                    .iter()
                    .filter(|t| t.matches_filter(&conditional.params[0]))
                    .count() as i32;
                amount *= multiplier;
            }
        }

        amount.max(0)
    }

    /// Get the multiplied uniques for a state
    pub fn get_multiplied(&self, state_for_conditionals: &StateForConditionals) -> impl Iterator<Item = Unique> {
        let multiplier = self.get_unique_multiplier(state_for_conditionals);
        repeat(self.clone()).take(multiplier as usize)
    }

    /// Get the deprecation annotation for this unique
    pub fn get_deprecation_annotation(&self) -> Option<Deprecated> {
        self.unique_type.as_ref().and_then(|t| t.get_deprecation_annotation())
    }

    /// Get the source name for user display
    pub fn get_source_name_for_user(&self) -> String {
        match self.source_object_type {
            None => String::new(),
            Some(UniqueTarget::Global) => GlobalUniques::get_unique_source_description(self),
            Some(UniqueTarget::Wonder) => "Wonders".to_string(),
            Some(UniqueTarget::Building) => "Buildings".to_string(),
            Some(UniqueTarget::Policy) => "Policies".to_string(),
            Some(UniqueTarget::CityState) => "City-States".to_string(),
            _ => self.source_object_type.unwrap().to_string(),
        }
    }

    /// Get the replacement text for this unique
    pub fn get_replacement_text(&self, ruleset: &Ruleset) -> String {
        let deprecation_annotation = match self.get_deprecation_annotation() {
            Some(annotation) => annotation,
            None => return String::new(),
        };

        let replacement_unique_text = deprecation_annotation.replace_with.expression;
        let deprecated_unique_placeholders = self.unique_type.as_ref().unwrap().text.get_placeholder_parameters();
        let possible_uniques = replacement_unique_text.split(UNIQUE_OR_DELIMITER).collect::<Vec<_>>();

        // Here, for once, we DO want the conditional placeholder parameters together with the regular ones,
        // so we cheat the conditional detector by removing the '<'
        // note this is only done for the replacement, not the deprecated unique, thus parameters of
        // conditionals on the deprecated unique are ignored

        let mut final_possible_uniques = Vec::new();

        for possible_unique in possible_uniques {
            let mut resulting_unique = possible_unique.to_string();
            for parameter in get_placeholder_parameters(&possible_unique.replace('<', " ")) {
                let parameter_has_sign = parameter.starts_with('-') || parameter.starts_with('+');
                let parameter_unsigned = if parameter_has_sign { parameter[1..].to_string() } else { parameter.clone() };
                let parameter_number_in_deprecated_unique = deprecated_unique_placeholders.iter().position(|p| p == &parameter_unsigned);

                if let Some(index) = parameter_number_in_deprecated_unique {
                    if index >= self.params.len() {
                        continue;
                    }

                    let position_in_deprecated_unique = self.unique_type.as_ref().unwrap().text.find(&format!("[{}]", parameter_unsigned)).unwrap_or(0);
                    let mut replacement_text = self.params[index].clone();

                    if self.unique_type.as_ref().unwrap().parameter_type_map.get(&index).map_or(false, |types| types.contains(&UniqueParameterType::Number)) {
                        // The following looks for a sign just before [amount] and detects replacing "-[-33]" with "[+33]" and similar situations
                        let deprecated_had_plus_sign = position_in_deprecated_unique > 0 && self.unique_type.as_ref().unwrap().text.chars().nth(position_in_deprecated_unique - 1) == Some('+');
                        let deprecated_had_minus_sign = position_in_deprecated_unique > 0 && self.unique_type.as_ref().unwrap().text.chars().nth(position_in_deprecated_unique - 1) == Some('-');
                        let deprecated_had_sign = deprecated_had_plus_sign || deprecated_had_minus_sign;

                        let position_in_new_unique = possible_unique.find(&format!("[{}]", parameter)).unwrap_or(0);
                        let new_has_minus_sign = position_in_new_unique > 0 && possible_unique.chars().nth(position_in_new_unique - 1) == Some('-');

                        let replacement_has_minus_sign = replacement_text.starts_with('-');
                        let replacement_has_plus_sign = replacement_text.starts_with('+');
                        let replacement_is_signed = replacement_has_plus_sign || replacement_has_minus_sign;
                        let replacement_text_unsigned = if replacement_is_signed { replacement_text[1..].to_string() } else { replacement_text.clone() };

                        let replacement_should_be_negative = if deprecated_had_minus_sign == new_has_minus_sign { replacement_has_minus_sign } else { !replacement_has_minus_sign };
                        let replacement_should_be_signed = deprecated_had_sign && !new_has_minus_sign || parameter_has_sign;

                        replacement_text = match (deprecated_had_sign || new_has_minus_sign || replacement_is_signed, replacement_should_be_negative, replacement_should_be_signed) {
                            (false, _, _) => replacement_text,
                            (true, true, _) => format!("-{}", replacement_text_unsigned),
                            (true, false, true) => format!("+{}", replacement_text_unsigned),
                            (true, false, false) => replacement_text_unsigned,
                        };
                    }

                    resulting_unique = resulting_unique.replace(&format!("[{}]", parameter), &format!("[{}]", replacement_text));
                }
            }
            final_possible_uniques.push(resulting_unique);
        }

        if final_possible_uniques.len() == 1 {
            return final_possible_uniques[0].clone();
        }

        // filter out possible replacements that are obviously wrong
        let uniques_with_no_errors = final_possible_uniques.iter()
            .filter(|unique_text| {
                let unique = Unique::new(unique_text.clone(), None, None);
                let errors = UniqueValidator::new(ruleset).check_unique(&unique, true, None, true);
                errors.is_empty()
            })
            .cloned()
            .collect::<Vec<_>>();

        if uniques_with_no_errors.len() == 1 {
            return uniques_with_no_errors[0].clone();
        }

        let uniques_to_unify = if uniques_with_no_errors.is_empty() { possible_uniques } else { &uniques_with_no_errors };
        uniques_to_unify.join("\", \"")
    }

    /// Get the display text for this unique
    pub fn get_display_text(&self) -> String {
        if !self.modifiers.iter().any(|m| m.is_hidden_to_users()) {
            self.text.clone()
        } else {
            let mut text = remove_conditionals(&self.text);
            text.push(' ');
            text.push_str(&self.modifiers.iter()
                .filter(|m| !m.is_hidden_to_users())
                .map(|m| format!("<{}>", m.text))
                .collect::<Vec<_>>()
                .join(" "));
            text
        }
    }
}

impl std::fmt::Display for Unique {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(unique_type) = &self.unique_type {
            write!(f, "{} (\"{}\")", unique_type, self.text)
        } else {
            write!(f, "\"{}\"", self.text)
        }
    }
}

/// Used to cache results of getMatchingUniques
/// Must only be used when we're sure the matching uniques will not change in the meantime
pub struct LocalUniqueCache {
    /// Whether to use caching
    cache: bool,
    /// The cache of uniques
    key_to_uniques: HashMap<String, Vec<Unique>>,
}

impl LocalUniqueCache {
    /// Create a new local unique cache
    pub fn new(cache: bool) -> Self {
        Self {
            cache,
            key_to_uniques: HashMap::new(),
        }
    }

    /// Get matching uniques for a city
    pub fn for_city_get_matching_uniques(
        &mut self,
        city: &City,
        unique_type: UniqueType,
        state_for_conditionals: &StateForConditionals,
    ) -> Vec<Unique> {
        // City uniques are a combination of *global civ* uniques plus *city relevant* uniques (see City.getMatchingUniques())
        // We can cache the civ uniques separately, so if we have several cities using the same cache,
        // we can cache the list of *civ uniques* to reuse between cities.

        let city_specific_uniques = self.get(
            &format!("city-{}-{}", city.id, unique_type.to_string()),
            city.get_local_matching_uniques(unique_type, &StateForConditionals::ignore_conditionals()),
        ).into_iter()
            .filter(|u| u.conditionals_apply(state_for_conditionals))
            .collect::<Vec<_>>();

        let civ_uniques = self.for_civ_get_matching_uniques(&city.civ, unique_type, state_for_conditionals);

        let mut result = city_specific_uniques;
        result.extend(civ_uniques);
        result
    }

    /// Get matching uniques for a civilization
    pub fn for_civ_get_matching_uniques(
        &mut self,
        civ: &Civilization,
        unique_type: UniqueType,
        state_for_conditionals: &StateForConditionals,
    ) -> Vec<Unique> {
        let sequence = civ.get_matching_uniques(unique_type, &StateForConditionals::ignore_conditionals());
        // The uniques CACHED are ALL civ uniques, regardless of conditional matching.
        // The uniques RETURNED are uniques AFTER conditional matching.
        // This allows reuse of the cached values, between runs with different conditionals -
        // for example, iterate on all tiles and get StatPercentForObject uniques relevant for each tile,
        // each tile will have different conditional state, but they will all reuse the same list of uniques for the civ
        self.get(
            &format!("civ-{}-{}", civ.civ_name, unique_type.to_string()),
            sequence,
        ).into_iter()
            .filter(|u| u.conditionals_apply(state_for_conditionals))
            .collect::<Vec<_>>()
    }

    /// Get cached results
    fn get(&mut self, key: &str, sequence: Vec<Unique>) -> Vec<Unique> {
        if !self.cache {
            return sequence;
        }

        if let Some(value_in_map) = self.key_to_uniques.get(key) {
            return value_in_map.clone();
        }

        // Iterate the sequence, save actual results as a list
        let results = sequence;
        self.key_to_uniques.insert(key.to_string(), results.clone());
        results
    }
}

/// A map of uniques
pub struct UniqueMap {
    /// The inner map of uniques
    inner_unique_map: HashMap<String, Vec<Unique>>,
    /// The typed map of uniques
    typed_unique_map: HashMap<UniqueType, Vec<Unique>>,
}

impl UniqueMap {
    /// Create a new empty unique map
    pub fn new() -> Self {
        Self {
            inner_unique_map: HashMap::new(),
            typed_unique_map: HashMap::new(),
        }
    }

    /// Create a new unique map with uniques
    pub fn new_with_uniques(uniques: impl Iterator<Item = Unique>) -> Self {
        let mut map = Self::new();
        map.add_uniques(uniques);
        map
    }

    /// Check if the map is empty
    pub fn is_empty(&self) -> bool {
        self.inner_unique_map.is_empty()
    }

    /// Add a unique to the map
    pub fn add_unique(&mut self, unique: Unique) {
        let existing_array_list = self.inner_unique_map.entry(unique.placeholder_text.clone()).or_insert_with(Vec::new);
        existing_array_list.push(unique.clone());

        if let Some(unique_type) = unique.unique_type {
            if !self.typed_unique_map.contains_key(&unique_type) {
                self.typed_unique_map.insert(unique_type, existing_array_list.clone());
            }
        }
    }

    /// Add uniques to the map
    pub fn add_uniques(&mut self, uniques: impl Iterator<Item = Unique>) {
        for unique in uniques {
            self.add_unique(unique);
        }
    }

    /// Remove a unique from the map
    pub fn remove_unique(&mut self, unique: &Unique) {
        if let Some(existing_array_list) = self.inner_unique_map.get_mut(&unique.placeholder_text) {
            existing_array_list.retain(|u| u != unique);
        }
    }

    /// Clear the map
    pub fn clear(&mut self) {
        self.inner_unique_map.clear();
        self.typed_unique_map.clear();
    }

    /// Check if the map has a unique of a specific type
    pub fn has_unique(&self, unique_type: UniqueType, state: &StateForConditionals) -> bool {
        self.get_uniques(unique_type)
            .iter()
            .any(|u| u.conditionals_apply(state) && !u.is_timed_triggerable)
    }

    /// Check if the map has a unique with a specific tag
    pub fn has_unique_tag(&self, unique_tag: &str, state: &StateForConditionals) -> bool {
        self.get_uniques_by_tag(unique_tag)
            .iter()
            .any(|u| u.conditionals_apply(state) && !u.is_timed_triggerable)
    }

    /// Check if the map has a tag unique
    pub fn has_tag_unique(&self, tag_unique: &str) -> bool {
        self.inner_unique_map.contains_key(tag_unique)
    }

    /// Get uniques of a specific type
    pub fn get_uniques(&self, unique_type: UniqueType) -> &[Unique] {
        self.typed_unique_map.get(&unique_type).map_or(&[], |v| v)
    }

    /// Get uniques with a specific tag
    pub fn get_uniques_by_tag(&self, unique_tag: &str) -> &[Unique] {
        self.inner_unique_map.get(unique_tag).map_or(&[], |v| v)
    }

    /// Get matching uniques of a specific type
    pub fn get_matching_uniques(&self, unique_type: UniqueType, state: &StateForConditionals) -> Vec<Unique> {
        self.get_uniques(unique_type)
            .iter()
            .flat_map(|u| {
                if u.is_timed_triggerable {
                    Vec::new()
                } else if !u.conditionals_apply(state) {
                    Vec::new()
                } else {
                    u.get_multiplied(state).collect()
                }
            })
            .collect()
    }

    /// Get matching uniques with a specific tag
    pub fn get_matching_uniques_by_tag(&self, unique_tag: &str, state: &StateForConditionals) -> Vec<Unique> {
        self.get_uniques_by_tag(unique_tag)
            .iter()
            .flat_map(|u| {
                if u.is_timed_triggerable {
                    Vec::new()
                } else if !u.conditionals_apply(state) {
                    Vec::new()
                } else {
                    u.get_multiplied(state).collect()
                }
            })
            .collect()
    }

    /// Check if the map has a matching unique of a specific type
    pub fn has_matching_unique(&self, unique_type: UniqueType, state: &StateForConditionals) -> bool {
        self.get_uniques(unique_type)
            .iter()
            .any(|u| u.conditionals_apply(state))
    }

    /// Check if the map has a matching unique with a specific tag
    pub fn has_matching_unique_tag(&self, unique_tag: &str, state: &StateForConditionals) -> bool {
        self.get_uniques_by_tag(unique_tag)
            .iter()
            .any(|u| u.conditionals_apply(state))
    }

    /// Get all uniques in the map
    pub fn get_all_uniques(&self) -> Vec<Unique> {
        self.inner_unique_map.values().flatten().cloned().collect()
    }

    /// Get triggered uniques
    pub fn get_triggered_uniques<F>(&self, trigger: UniqueType, state_for_conditionals: &StateForConditionals, trigger_filter: F) -> Vec<Unique>
    where
        F: Fn(&Unique) -> bool,
    {
        self.get_all_uniques()
            .into_iter()
            .filter(|unique| {
                unique.get_modifiers(trigger).iter().any(trigger_filter) && unique.conditionals_apply(state_for_conditionals)
            })
            .flat_map(|u| u.get_multiplied(state_for_conditionals))
            .collect()
    }
}

impl Default for UniqueMap {
    fn default() -> Self {
        Self::new()
    }
}

/// A temporary unique that expires after a certain number of turns
#[derive(Clone, Debug)]
pub struct TemporaryUnique {
    /// The unique text
    pub unique: String,
    /// The source object type
    source_object_type: Option<UniqueTarget>,
    /// The source object name
    source_object_name: Option<String>,
    /// The unique object (cached)
    unique_object: OnceLock<Unique>,
    /// The number of turns left
    pub turns_left: i32,
}

impl TemporaryUnique {
    /// Create a new temporary unique
    pub fn new(unique_object: &Unique, turns: i32) -> Self {
        let turns_text = unique_object.get_modifiers(UniqueType::ConditionalTimedUnique)[0].text.clone();
        let unique = unique_object.text.replace(&format!("<{}>", turns_text), "").trim().to_string();

        Self {
            unique,
            source_object_type: unique_object.source_object_type.clone(),
            source_object_name: unique_object.source_object_name.clone(),
            unique_object: OnceLock::new(),
            turns_left: turns,
        }
    }

    /// Get the unique object
    pub fn unique_object(&self) -> &Unique {
        self.unique_object.get_or_init(|| {
            Unique::new(
                self.unique.clone(),
                self.source_object_type.clone(),
                self.source_object_name.clone(),
            )
        })
    }
}

impl IsPartOfGameInfoSerialization for TemporaryUnique {}

/// Extension trait for Vec<TemporaryUnique>
pub trait TemporaryUniqueVecExt {
    /// End the turn for all temporary uniques
    fn end_turn(&mut self);

    /// Get matching uniques
    fn get_matching_uniques(&self, unique_type: UniqueType, state_for_conditionals: &StateForConditionals) -> Vec<Unique>;
}

impl TemporaryUniqueVecExt for Vec<TemporaryUnique> {
    /// End the turn for all temporary uniques
    fn end_turn(&mut self) {
        for unique in self.iter_mut() {
            if unique.turns_left >= 0 {
                unique.turns_left -= 1;
            }
        }
        self.retain(|u| u.turns_left != 0);
    }

    /// Get matching uniques
    fn get_matching_uniques(&self, unique_type: UniqueType, state_for_conditionals: &StateForConditionals) -> Vec<Unique> {
        self.iter()
            .map(|u| u.unique_object())
            .filter(|u| u.unique_type == Some(unique_type) && u.conditionals_apply(state_for_conditionals))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique_new() {
        let unique = Unique::new("Test unique".to_string(), Some(UniqueTarget::Building), Some("Test Building".to_string()));
        assert_eq!(unique.text, "Test unique");
        assert_eq!(unique.source_object_type, Some(UniqueTarget::Building));
        assert_eq!(unique.source_object_name, Some("Test Building".to_string()));
    }

    #[test]
    fn test_unique_map_new() {
        let map = UniqueMap::new();
        assert!(map.is_empty());
    }

    #[test]
    fn test_temporary_unique_new() {
        let unique = Unique::new("Test unique <turns=5>".to_string(), None, None);
        let temp_unique = TemporaryUnique::new(&unique, 5);
        assert_eq!(temp_unique.unique, "Test unique");
        assert_eq!(temp_unique.turns_left, 5);
    }
}