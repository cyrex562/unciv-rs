use std::collections::{HashMap, HashSet};
use std::path::Path;
use crate::models::{
    ruleset::Ruleset,
    ruleset::unique::{IHasUniques, Unique, UniqueType, StateForConditionals},
    ruleset::validation::{RulesetErrorList, RulesetError, RulesetErrorSeverity},
    ruleset::unit::{BaseUnit, Promotion, UnitMovementType},
    ruleset::nation::Nation,
    ruleset::tile::TerrainType,
    ruleset::building::Building,
    ruleset::tech::Technology,
    ruleset::improvement::TileImprovement,
    ruleset::resource::TileResource,
    ruleset::era::Era,
    ruleset::policy::Policy, PolicyBranch,
    ruleset::belief::Belief, BeliefType,
    ruleset::victory::VictoryType, MilestoneType,
    ruleset::difficulty::Difficulty,
    ruleset::event::Event,
    ruleset::city_state::CityStateType,
    ruleset::speed::Speed,
    ruleset::personality::Personality,
    ruleset::specialist::Specialist,
    ruleset::ruin::RuinReward,
    ruleset::tech_column::TechColumn,
    ruleset::road::RoadStatus,
    stats::Stats, INamed,
    constants::Constants,
    files::FileHandle,
    tileset::{TileSetCache, TileSetConfig},
    images::{AtlasPreview, ImageGetter, Portrait, PortraitPromotion},
};

/// Validates a ruleset by checking for various errors and issues
pub struct RulesetValidator<'a> {
    /// The ruleset being validated
    ruleset: &'a Ruleset,
    /// Validator for unique abilities
    unique_validator: UniqueValidator<'a>,
    /// Cache of texture names
    texture_names_cache: Option<AtlasPreview>,
}

impl<'a> RulesetValidator<'a> {
    /// Creates a new RulesetValidator for the given ruleset
    pub fn new(ruleset: &'a Ruleset) -> Self {
        Self {
            ruleset,
            unique_validator: UniqueValidator::new(ruleset),
            texture_names_cache: None,
        }
    }

    /// Gets a list of errors in the ruleset
    ///
    /// # Arguments
    ///
    /// * `try_fix_unknown_uniques` - Whether to try to fix unknown uniques
    pub fn get_error_list(&mut self, try_fix_unknown_uniques: bool) -> RulesetErrorList {
        // When no base ruleset is loaded - references cannot be checked
        if !self.ruleset.mod_options.is_base_ruleset {
            return self.get_non_base_ruleset_error_list(try_fix_unknown_uniques);
        }

        self.get_base_ruleset_error_list(try_fix_unknown_uniques)
    }

    /// Gets a list of errors for a non-base ruleset
    fn get_non_base_ruleset_error_list(&mut self, try_fix_unknown_uniques: bool) -> RulesetErrorList {
        let mut lines = RulesetErrorList::new(Some(self.ruleset));

        // When not checking the entire ruleset, we can only really detect ruleset-invariant errors in uniques
        self.add_mod_options_errors(&mut lines, try_fix_unknown_uniques);
        self.unique_validator.check_uniques(&self.ruleset.global_uniques, &mut lines, false, try_fix_unknown_uniques);
        self.add_unit_errors_ruleset_invariant(&mut lines, try_fix_unknown_uniques);
        self.add_tech_errors_ruleset_invariant(&mut lines, try_fix_unknown_uniques);
        self.add_tech_column_errors_ruleset_invariant(&mut lines);
        self.add_building_errors_ruleset_invariant(&mut lines, try_fix_unknown_uniques);
        self.add_nation_errors_ruleset_invariant(&mut lines, try_fix_unknown_uniques);
        self.add_promotion_errors_ruleset_invariant(&mut lines, try_fix_unknown_uniques);
        self.add_resource_errors_ruleset_invariant(&mut lines, try_fix_unknown_uniques);

        if self.texture_names_cache.is_none() {
            self.texture_names_cache = Some(AtlasPreview::new(self.ruleset, &mut lines));
        }

        // Tileset tests - e.g. json configs complete and parseable
        self.check_tileset_sanity(&mut lines);
        self.check_civilopedia_text(&mut lines);
        self.check_file_names(&mut lines);

        lines
    }

    /// Gets a list of errors for a base ruleset
    fn get_base_ruleset_error_list(&mut self, try_fix_unknown_uniques: bool) -> RulesetErrorList {
        self.unique_validator.populate_filtering_unique_hashsets();

        let mut lines = RulesetErrorList::new(Some(self.ruleset));
        self.add_mod_options_errors(&mut lines, try_fix_unknown_uniques);
        self.unique_validator.check_uniques(&self.ruleset.global_uniques, &mut lines, true, try_fix_unknown_uniques);

        self.add_unit_errors_base_ruleset(&mut lines, try_fix_unknown_uniques);
        self.add_building_errors(&mut lines, try_fix_unknown_uniques);
        self.add_specialist_errors(&mut lines);
        self.add_resource_errors(&mut lines, try_fix_unknown_uniques);
        self.add_improvement_errors(&mut lines, try_fix_unknown_uniques);
        self.add_terrain_errors(&mut lines, try_fix_unknown_uniques);
        self.add_tech_errors(&mut lines, try_fix_unknown_uniques);
        self.add_tech_column_errors_ruleset_invariant(&mut lines);
        self.add_era_errors(&mut lines, try_fix_unknown_uniques);
        self.add_speed_errors(&mut lines);
        self.add_personality_errors(&mut lines);
        self.add_belief_errors(&mut lines, try_fix_unknown_uniques);
        self.add_nation_errors(&mut lines, try_fix_unknown_uniques);
        self.add_policy_errors(&mut lines, try_fix_unknown_uniques);
        self.add_ruins_errors(&mut lines, try_fix_unknown_uniques);
        self.add_promotion_errors(&mut lines, try_fix_unknown_uniques);
        self.add_unit_type_errors(&mut lines, try_fix_unknown_uniques);
        self.add_victory_type_errors(&mut lines);
        self.add_difficulty_errors(&mut lines);
        self.add_event_errors(&mut lines, try_fix_unknown_uniques);
        self.add_city_state_type_errors(try_fix_unknown_uniques, &mut lines);

        if self.texture_names_cache.is_none() {
            self.texture_names_cache = Some(AtlasPreview::new(self.ruleset, &mut lines));
        }

        // Tileset tests - e.g. json configs complete and parseable
        // Check for mod or Civ_V_GnK to avoid running the same test twice (~200ms for the builtin assets)
        if self.ruleset.folder_location.is_some() || self.ruleset.name == "Civ_V_GnK" {
            self.check_tileset_sanity(&mut lines);
        }

        self.check_civilopedia_text(&mut lines);
        self.check_file_names(&mut lines);

        lines
    }

    /// Gets possible misspellings of a text
    fn get_possible_misspellings(&self, original_text: &str, possible_misspellings: &[String]) -> Vec<String> {
        possible_misspellings.iter()
            .filter(|&it| {
                self.get_relative_text_distance(it, original_text) <= 0.7 // Using a threshold of 0.7 for similarity
            })
            .cloned()
            .collect()
    }

    /// Gets the relative distance between two texts
    fn get_relative_text_distance(&self, text1: &str, text2: &str) -> f32 {
        // Simple implementation of Levenshtein distance
        let m = text1.len();
        let n = text2.len();
        let mut dp = vec![vec![0; n + 1]; m + 1];

        for i in 0..=m {
            dp[i][0] = i;
        }
        for j in 0..=n {
            dp[0][j] = j;
        }

        for i in 1..=m {
            for j in 1..=n {
                if text1.chars().nth(i - 1) == text2.chars().nth(j - 1) {
                    dp[i][j] = dp[i - 1][j - 1];
                } else {
                    dp[i][j] = 1 + dp[i - 1][j - 1].min(dp[i - 1][j].min(dp[i][j - 1]));
                }
            }
        }

        let distance = dp[m][n] as f32;
        let max_len = m.max(n) as f32;
        1.0 - (distance / max_len)
    }

    /// Checks if an uncached image exists
    pub fn uncached_image_exists(&self, name: &str) -> bool {
        if self.ruleset.folder_location.is_none() {
            return false; // Can't check in this case
        }

        if let Some(cache) = &self.texture_names_cache {
            cache.image_exists(name)
        } else {
            false
        }
    }

    /// Checks file names in the ruleset
    fn check_file_names(&self, lines: &mut RulesetErrorList) {
        let folder = match &self.ruleset.folder_location {
            Some(folder) => folder,
            None => return,
        };

        self.check_misplaced_json_files(folder, lines);
        self.check_misspelled_folders(folder, lines);
        self.check_images_folders(folder, lines);
        self.check_unknown_json_filenames(folder, lines);
    }

    /// Checks for misspelled folders
    fn check_misspelled_folders(&self, folder: &FileHandle, lines: &mut RulesetErrorList) {
        let known_folder_names = vec!["jsons", "maps", "sounds", "Images", "fonts"];

        for child in folder.list() {
            if !child.is_directory() || known_folder_names.contains(&child.name().as_str()) {
                continue;
            }

            let possible_misspellings = self.get_possible_misspellings(&child.name(), &known_folder_names);
            if !possible_misspellings.is_empty() {
                lines.add_text(
                    format!("Folder \"{}\" is probably a misspelling of {}",
                        child.name(),
                        possible_misspellings.join("/")
                    ),
                    RulesetErrorSeverity::OK,
                    None,
                    None
                );
            }
        }
    }

    /// Checks for misplaced JSON files
    fn check_misplaced_json_files(&self, folder: &FileHandle, lines: &mut RulesetErrorList) {
        for child in folder.list() {
            if child.name().ends_with("json") && !child.name().starts_with("Atlas") {
                lines.add_text(
                    format!("File {} is located in the root folder - it should be moved to a 'jsons' folder",
                        child.name()
                    ),
                    RulesetErrorSeverity::OK,
                    None,
                    None
                );
            }
        }
    }

    /// Checks images folders
    fn check_images_folders(&self, folder: &FileHandle, lines: &mut RulesetErrorList) {
        let known_image_folders = Portrait::Type::entries()
            .iter()
            .flat_map(|t| vec![format!("{}Icons", t.directory()), format!("{}Portraits", t.directory())])
            .chain(vec![
                "CityStateIcons".to_string(),
                "PolicyBranchIcons".to_string(),
                "PolicyIcons".to_string(),
                "OtherIcons".to_string(),
                "EmojiIcons".to_string(),
                "StatIcons".to_string(),
                "TileIcons".to_string(),
                "TileSets".to_string(),
            ])
            .collect::<Vec<String>>();

        let image_folders: Vec<_> = folder.list()
            .into_iter()
            .filter(|f| f.name().starts_with("Images"))
            .collect();

        for image_folder in image_folders {
            for child in image_folder.list() {
                if !child.is_directory() {
                    lines.add_text(
                        format!("File \"{}/{}\" is misplaced - Images folders should not contain any files directly - only subfolders",
                            image_folder.name(),
                            child.name()
                        ),
                        RulesetErrorSeverity::OK,
                        None,
                        None
                    );
                } else if !known_image_folders.contains(&child.name()) {
                    let possible_misspellings = self.get_possible_misspellings(&child.name(), &known_image_folders);
                    if !possible_misspellings.is_empty() {
                        lines.add_text(
                            format!("Folder \"{}/{}\" is probably a misspelling of {}",
                                image_folder.name(),
                                child.name(),
                                possible_misspellings.join("/")
                            ),
                            RulesetErrorSeverity::OK,
                            None,
                            None
                        );
                    }
                }
            }
        }
    }

    /// Checks for unknown JSON filenames
    fn check_unknown_json_filenames(&self, folder: &FileHandle, lines: &mut RulesetErrorList) {
        let json_folder = folder.child("jsons");
        if !json_folder.exists() {
            return;
        }

        for file in json_folder.list("json") {
            if RulesetFile::entries().iter().any(|e| e.filename() == file.name()) {
                continue;
            }

            let mut text = format!("File {} is in the jsons folder but is not a recognized ruleset file", file.name());
            let possible_misspellings = self.get_possible_misspellings(
                &file.name(),
                &RulesetFile::entries().iter().map(|e| e.filename().to_string()).collect::<Vec<_>>()
            );

            if !possible_misspellings.is_empty() {
                text.push_str(&format!("\nPossible misspelling of: {}", possible_misspellings.join("/")));
            }

            lines.add_text(text, RulesetErrorSeverity::OK, None, None);
        }
    }

    /// Adds mod options errors to the error list
    fn add_mod_options_errors(&self, lines: &mut RulesetErrorList, try_fix_unknown_uniques: bool) {
        // Basic Unique validation (type, target, parameters) should always run.
        // Using report_ruleset_specific_errors=true as ModOptions never should use Uniques depending on objects from a base ruleset anyway.
        self.unique_validator.check_uniques(&self.ruleset.mod_options, lines, true, try_fix_unknown_uniques);

        if self.ruleset.name.is_empty() {
            return; // The rest of these tests don't make sense for combined rulesets
        }

        let audio_visual_unique_types = vec![
            UniqueType::ModIsAudioVisual,
            UniqueType::ModIsAudioVisualOnly,
            UniqueType::ModIsNotAudioVisual,
        ];

        // modOptions is a valid sourceObject, but unnecessary
        if self.ruleset.mod_options.unique_objects.iter()
            .filter(|u| audio_visual_unique_types.contains(&u.type_))
            .count() > 1 {
            lines.add_text(
                "A mod should only specify one of the 'can/should/cannot be used as permanent audiovisual mod' options.",
                RulesetErrorSeverity::Warning,
                None,
                None
            );
        }

        let map_select_uniques = self.ruleset.mod_options.get_matching_uniques(UniqueType::ModMapPreselection);
        if map_select_uniques.len() > 1 {
            lines.add_text(
                "Specifying more than one map as preselection makes no sense",
                RulesetErrorSeverity::WarningOptionsOnly,
                None,
                None
            );
        }

        if !map_select_uniques.is_empty() {
            let maps_folder = self.ruleset.get_mod_folder().child("maps");
            if maps_folder.exists() {
                let maps: Vec<String> = maps_folder.list()
                    .iter()
                    .map(|f| f.name().to_lowercase())
                    .collect();

                for unique in map_select_uniques {
                    if !maps.contains(&unique.params[0].to_lowercase()) {
                        lines.add_text(
                            format!("Mod names map '{}' as preselection, which does not exist.", unique.params[0]),
                            RulesetErrorSeverity::WarningOptionsOnly,
                            None,
                            None
                        );
                    }
                }
            } else {
                lines.add_text(
                    "Mod option for map preselection exists but Mod has no 'maps' folder.",
                    RulesetErrorSeverity::WarningOptionsOnly,
                    None,
                    None
                );
            }
        }

        if !self.ruleset.mod_options.is_base_ruleset {
            return;
        }

        for unique in self.ruleset.mod_options.get_matching_uniques(UniqueType::ModRequires) {
            lines.add_text(
                format!("Mod option '{}' is invalid for a base ruleset.", unique.text()),
                RulesetErrorSeverity::Error,
                None,
                None
            );
        }
    }

    /// Adds city state type errors to the error list
    fn add_city_state_type_errors(&self, try_fix_unknown_uniques: bool, lines: &mut RulesetErrorList) {
        for city_state_type in self.ruleset.city_state_types.values() {
            for unique in city_state_type.ally_bonus_unique_map.get_all_uniques()
                .iter()
                .chain(city_state_type.friend_bonus_unique_map.get_all_uniques().iter()) {
                let errors = self.unique_validator.check_unique(
                    unique,
                    try_fix_unknown_uniques,
                    None,
                    true
                );
                lines.extend(&errors);
            }
        }
    }

    /// Adds difficulty errors to the error list
    fn add_difficulty_errors(&self, lines: &mut RulesetErrorList) {
        // A Difficulty is not a IHasUniques, so not suitable as sourceObject
        for difficulty in self.ruleset.difficulties.values() {
            for unit_name in difficulty.ai_city_state_bonus_starting_units.iter()
                .chain(difficulty.ai_major_civ_bonus_starting_units.iter())
                .chain(difficulty.player_bonus_starting_units.iter()) {
                if unit_name != &Constants::ERA_SPECIFIC_UNIT && !self.ruleset.units.contains_key(unit_name) {
                    lines.add_text(
                        format!("Difficulty {} contains starting unit {} which does not exist!",
                            difficulty.name, unit_name),
                        RulesetErrorSeverity::Error,
                        None,
                        None
                    );
                }
            }
        }
    }

    /// Adds event errors to the error list
    fn add_event_errors(&self, lines: &mut RulesetErrorList, try_fix_unknown_uniques: bool) {
        // An Event is not a IHasUniques, so not suitable as sourceObject
        for event in self.ruleset.events.values() {
            for choice in &event.choices {
                self.unique_validator.check_uniques(choice, lines, true, try_fix_unknown_uniques);
            }
            self.unique_validator.check_uniques(event, lines, true, try_fix_unknown_uniques);
        }
    }

    /// Adds victory type errors to the error list
    fn add_victory_type_errors(&self, lines: &mut RulesetErrorList) {
        // Victory and Milestone aren't IHasUniques and are unsuitable as sourceObject
        for victory_type in self.ruleset.victories.values() {
            for required_unit in &victory_type.required_spaceship_parts {
                if !self.ruleset.units.contains_key(required_unit) {
                    lines.add_text(
                        format!("Victory type {} requires adding the non-existant unit {} to the capital to win!",
                            victory_type.name, required_unit),
                        RulesetErrorSeverity::Warning,
                        None,
                        None
                    );
                }
            }

            for milestone in &victory_type.milestone_objects {
                if milestone.type_ == MilestoneType::None {
                    lines.add_text(
                        format!("Victory type {} has milestone \"{}\" that is of an unknown type!",
                            victory_type.name, milestone.unique_description),
                        RulesetErrorSeverity::Error,
                        None,
                        None
                    );
                }

                if (milestone.type_ == MilestoneType::BuiltBuilding || milestone.type_ == MilestoneType::BuildingBuiltGlobally)
                    && !self.ruleset.buildings.contains_key(&milestone.params[0]) {
                    lines.add_text(
                        format!("Victory type {} has milestone \"{}\" that references an unknown building {}!",
                            victory_type.name, milestone.unique_description, milestone.params[0]),
                        RulesetErrorSeverity::Error,
                        None,
                        None
                    );
                }
            }

            for victory in self.ruleset.victories.values() {
                if victory.name != victory_type.name && victory.milestones == victory_type.milestones {
                    lines.add_text(
                        format!("Victory types {} and {} have the same requirements!",
                            victory_type.name, victory.name),
                        RulesetErrorSeverity::Warning,
                        None,
                        None
                    );
                }
            }
        }
    }

    /// Adds unit type errors to the error list
    fn add_unit_type_errors(&self, lines: &mut RulesetErrorList, try_fix_unknown_uniques: bool) {
        let unit_movement_types: HashSet<String> = UnitMovementType::entries()
            .iter()
            .map(|t| t.name().to_string())
            .collect();

        for unit_type in self.ruleset.unit_types.values() {
            if !unit_movement_types.contains(&unit_type.movement_type) {
                lines.add_text(
                    format!("Unit type {} has an invalid movement type {}",
                        unit_type.name, unit_type.movement_type),
                    RulesetErrorSeverity::Error,
                    Some(unit_type),
                    None
                );
            }
            self.unique_validator.check_uniques(unit_type, lines, true, try_fix_unknown_uniques);
        }
    }

    /// Adds promotion errors to the error list
    fn add_promotion_errors(&self, lines: &mut RulesetErrorList, try_fix_unknown_uniques: bool) {
        for promotion in self.ruleset.unit_promotions.values() {
            self.add_promotion_error_ruleset_invariant(promotion, lines);

            // These are warning as of 3.17.5 to not break existing mods and give them time to correct, should be upgraded to error in the future
            for prereq in &promotion.prerequisites {
                if !self.ruleset.unit_promotions.contains_key(prereq) {
                    lines.add_text(
                        format!("{} requires promotion {} which does not exist!",
                            promotion.name, prereq),
                        RulesetErrorSeverity::Warning,
                        Some(promotion),
                        None
                    );
                }
            }

            for unit_type in &promotion.unit_types {
                if !self.ruleset.unit_types.contains_key(unit_type) {
                    lines.add_text(
                        format!("{} references unit type {}, which does not exist!",
                            promotion.name, unit_type),
                        RulesetErrorSeverity::Warning,
                        Some(promotion),
                        None
                    );
                }
            }

            self.unique_validator.check_uniques(promotion, lines, true, try_fix_unknown_uniques);
        }

        self.check_promotion_circular_references(lines);
    }

    /// Adds ruins errors to the error list
    fn add_ruins_errors(&self, lines: &mut RulesetErrorList, try_fix_unknown_uniques: bool) {
        for reward in self.ruleset.ruin_rewards.values() {
            if reward.weight < 0 {
                lines.add_text(
                    format!("{} has a negative weight, which is not allowed!", reward.name),
                    RulesetErrorSeverity::Error,
                    Some(reward),
                    None
                );
            }

            for difficulty in &reward.excluded_difficulties {
                if !self.ruleset.difficulties.contains_key(difficulty) {
                    lines.add_text(
                        format!("{} references difficulty {}, which does not exist!",
                            reward.name, difficulty),
                        RulesetErrorSeverity::Error,
                        Some(reward),
                        None
                    );
                }
            }

            self.unique_validator.check_uniques(reward, lines, true, try_fix_unknown_uniques);
        }
    }

    /// Adds policy errors to the error list
    fn add_policy_errors(&self, lines: &mut RulesetErrorList, try_fix_unknown_uniques: bool) {
        for policy in self.ruleset.policies.values() {
            if let Some(requires) = &policy.requires {
                for prereq in requires {
                    if !self.ruleset.policies.contains_key(prereq) {
                        lines.add_text(
                            format!("{} requires policy {} which does not exist!",
                                policy.name, prereq),
                            RulesetErrorSeverity::Error,
                            Some(policy),
                            None
                        );
                    }
                }
            }

            self.unique_validator.check_uniques(policy, lines, true, try_fix_unknown_uniques);
        }

        for branch in self.ruleset.policy_branches.values() {
            if !self.ruleset.eras.contains_key(&branch.era) {
                lines.add_text(
                    format!("{} requires era {} which does not exist!",
                        branch.name, branch.era),
                    RulesetErrorSeverity::Error,
                    Some(branch),
                    None
                );
            }

            let mut policy_locations = HashMap::new();
            for policy in &branch.policies {
                let policy_location = format!("{}/{}", policy.row, policy.column);
                if let Some(existing_policy) = policy_locations.get(&policy_location) {
                    lines.add_text(
                        format!("Policies {} and {} in branch {} are both located at column {} row {}!",
                            policy.name, existing_policy.name, branch.name, policy.column, policy.row),
                        RulesetErrorSeverity::Error,
                        Some(policy),
                        None
                    );
                } else {
                    policy_locations.insert(policy_location, policy);
                }
            }
        }

        for policy in self.ruleset.policy_branches.values()
            .flat_map(|b| b.policies.iter().chain(std::iter::once(b))) {
            if let Some(existing_policy) = self.ruleset.policies.get(&policy.name) {
                if policy as *const _ != existing_policy as *const _ {
                    lines.add_text(
                        format!("More than one policy with the name {} exists!", policy.name),
                        RulesetErrorSeverity::Error,
                        Some(policy),
                        None
                    );
                }
            }
        }
    }

    /// Adds nation errors to the error list
    fn add_nation_errors(&self, lines: &mut RulesetErrorList, try_fix_unknown_uniques: bool) {
        for nation in self.ruleset.nations.values() {
            self.add_nation_error_ruleset_invariant(nation, lines);

            self.unique_validator.check_uniques(nation, lines, true, try_fix_unknown_uniques);

            if nation.preferred_victory_type != Constants::NEUTRAL_VICTORY_TYPE
                && !self.ruleset.victories.contains_key(&nation.preferred_victory_type) {
                lines.add_text(
                    format!("{}'s preferredVictoryType is {} which does not exist!",
                        nation.name, nation.preferred_victory_type),
                    RulesetErrorSeverity::Error,
                    Some(nation),
                    None
                );
            }

            if let Some(city_state_type) = &nation.city_state_type {
                if !self.ruleset.city_state_types.contains_key(city_state_type) {
                    lines.add_text(
                        format!("{} is of city-state type {} which does not exist!",
                            nation.name, city_state_type),
                        RulesetErrorSeverity::Error,
                        Some(nation),
                        None
                    );
                }
            }

            if let Some(favored_religion) = &nation.favored_religion {
                if !self.ruleset.religions.contains_key(favored_religion) {
                    lines.add_text(
                        format!("{} has {} as their favored religion, which does not exist!",
                            nation.name, favored_religion),
                        RulesetErrorSeverity::Error,
                        Some(nation),
                        None
                    );
                }
            }
        }
    }

    /// Adds belief errors to the error list
    fn add_belief_errors(&self, lines: &mut RulesetErrorList, try_fix_unknown_uniques: bool) {
        for belief in self.ruleset.beliefs.values() {
            if belief.type_ == BeliefType::Any || belief.type_ == BeliefType::None {
                lines.add_text(
                    format!("{} type is {}, which is not allowed!",
                        belief.name, belief.type_.to_string()),
                    RulesetErrorSeverity::Error,
                    Some(belief),
                    None
                );
            }
            self.unique_validator.check_uniques(belief, lines, true, try_fix_unknown_uniques);
        }
    }

    /// Adds speed errors to the error list
    fn add_speed_errors(&self, lines: &mut RulesetErrorList) {
        for speed in self.ruleset.speeds.values() {
            if speed.modifier < 0.0 {
                lines.add_text(
                    format!("Negative speed modifier for game speed {}", speed.name),
                    RulesetErrorSeverity::Error,
                    Some(speed),
                    None
                );
            }
            if speed.years_per_turn.is_empty() {
                lines.add_text(
                    format!("Empty turn increment list for game speed {}", speed.name),
                    RulesetErrorSeverity::Error,
                    Some(speed),
                    None
                );
            }
        }
    }

    /// Adds personality errors to the error list
    fn add_personality_errors(&self, lines: &mut RulesetErrorList) {
        for personality in self.ruleset.personalities.values() {
            if personality.preferred_victory_type != Constants::NEUTRAL_VICTORY_TYPE
                && !self.ruleset.victories.contains_key(&personality.preferred_victory_type) {
                lines.add_text(
                    format!("Preferred victory type {} does not exist in ruleset",
                        personality.preferred_victory_type),
                    RulesetErrorSeverity::Warning,
                    Some(personality),
                    None
                );
            }
        }
    }

    /// Adds era errors to the error list
    fn add_era_errors(&self, lines: &mut RulesetErrorList, try_fix_unknown_uniques: bool) {
        if self.ruleset.eras.is_empty() {
            lines.add_text(
                "Eras file is empty! This will likely lead to crashes. Ask the mod maker to update this mod!",
                RulesetErrorSeverity::Error,
                None,
                None
            );
        }

        let mut all_difficulties_starting_units = HashSet::new();
        for difficulty in self.ruleset.difficulties.values() {
            all_difficulties_starting_units.extend(difficulty.ai_city_state_bonus_starting_units.iter().cloned());
            all_difficulties_starting_units.extend(difficulty.ai_major_civ_bonus_starting_units.iter().cloned());
            all_difficulties_starting_units.extend(difficulty.player_bonus_starting_units.iter().cloned());
        }

        for era in self.ruleset.eras.values() {
            for wonder in &era.starting_obsolete_wonders {
                if !self.ruleset.buildings.contains_key(wonder) {
                    lines.add_text(
                        format!("Nonexistent wonder {} obsoleted when starting in {}!",
                            wonder, era.name),
                        RulesetErrorSeverity::Error,
                        Some(era),
                        None
                    );
                }
            }

            for building in &era.settler_buildings {
                if !self.ruleset.buildings.contains_key(building) {
                    lines.add_text(
                        format!("Nonexistent building {} built by settlers when starting in {}",
                            building, era.name),
                        RulesetErrorSeverity::Error,
                        Some(era),
                        None
                    );
                }
            }

            // todo the whole 'starting unit' thing needs to be redone, there's no reason we can't have a single list containing all the starting units.
            if !self.ruleset.units.contains_key(&era.starting_settler_unit)
                && !self.ruleset.units.values().any(|u| u.is_city_founder()) {
                lines.add_text(
                    format!("Nonexistent unit {} marked as starting unit when starting in {}",
                        era.starting_settler_unit, era.name),
                    RulesetErrorSeverity::Error,
                    Some(era),
                    None
                );
            }

            if era.starting_worker_count != 0 && !self.ruleset.units.contains_key(&era.starting_worker_unit)
                && !self.ruleset.units.values().any(|u| u.has_unique(UniqueType::BuildImprovements)) {
                lines.add_text(
                    format!("Nonexistent unit {} marked as starting unit when starting in {}",
                        era.starting_worker_unit, era.name),
                    RulesetErrorSeverity::Error,
                    Some(era),
                    None
                );
            }

            let grants_starting_military_unit = era.starting_military_unit_count != 0
                || all_difficulties_starting_units.contains(&Constants::ERA_SPECIFIC_UNIT.to_string());

            if grants_starting_military_unit && !self.ruleset.units.contains_key(&era.starting_military_unit) {
                lines.add_text(
                    format!("Nonexistent unit {} marked as starting unit when starting in {}",
                        era.starting_military_unit, era.name),
                    RulesetErrorSeverity::Error,
                    Some(era),
                    None
                );
            }

            if era.research_agreement_cost < 0 || era.starting_settler_count < 0
                || era.starting_worker_count < 0 || era.starting_military_unit_count < 0
                || era.starting_gold < 0 || era.starting_culture < 0 {
                lines.add_text(
                    format!("Unexpected negative number found while parsing era {}", era.name),
                    RulesetErrorSeverity::Error,
                    Some(era),
                    None
                );
            }

            if era.settler_population <= 0 {
                lines.add_text(
                    format!("Population in cities from settlers must be strictly positive! Found value {} for era {}",
                        era.settler_population, era.name),
                    RulesetErrorSeverity::Error,
                    Some(era),
                    None
                );
            }

            if !era.ally_bonus.is_empty() {
                lines.add_text(
                    format!("Era {} contains city-state bonuses. City-state bonuses are now defined in CityStateType.json",
                        era.name),
                    RulesetErrorSeverity::WarningOptionsOnly,
                    Some(era),
                    None
                );
            }

            if !era.friend_bonus.is_empty() {
                lines.add_text(
                    format!("Era {} contains city-state bonuses. City-state bonuses are now defined in CityStateType.json",
                        era.name),
                    RulesetErrorSeverity::WarningOptionsOnly,
                    Some(era),
                    None
                );
            }

            self.unique_validator.check_uniques(era, lines, true, try_fix_unknown_uniques);
        }
    }

    /// Adds tech errors to the error list
    fn add_tech_errors(&self, lines: &mut RulesetErrorList, try_fix_unknown_uniques: bool) {
        for tech in self.ruleset.technologies.values() {
            for prereq in &tech.prerequisites {
                if !self.ruleset.technologies.contains_key(prereq) {
                    lines.add_text(
                        format!("{} requires tech {} which does not exist!",
                            tech.name, prereq),
                        RulesetErrorSeverity::Error,
                        Some(tech),
                        None
                    );
                }

                if tech.prerequisites.iter().any(|p| p != prereq && self.get_prereq_tree(p).contains(prereq)) {
                    lines.add_text(
                        format!("No need to add {} as a prerequisite of {} - it is already implicit from the other prerequisites!",
                            prereq, tech.name),
                        RulesetErrorSeverity::Warning,
                        Some(tech),
                        None
                    );
                }

                if self.get_prereq_tree(prereq).contains(&tech.name) {
                    lines.add_text(
                        format!("Techs {} and {} require each other!", tech.name, prereq),
                        RulesetErrorSeverity::Error,
                        Some(tech),
                        None
                    );
                }
            }

            if !self.ruleset.eras.contains_key(&tech.era()) {
                lines.add_text(
                    format!("Unknown era {} referenced in column of tech {}",
                        tech.era(), tech.name),
                    RulesetErrorSeverity::Error,
                    Some(tech),
                    None
                );
            }

            self.unique_validator.check_uniques(tech, lines, true, try_fix_unknown_uniques);
        }
    }

    /// Adds terrain errors to the error list
    fn add_terrain_errors(&self, lines: &mut RulesetErrorList, try_fix_unknown_uniques: bool) {
        if !self.ruleset.terrains.values().any(|t|
            t.type_ == TerrainType::Land && !t.impassable && !t.has_unique(UniqueType::NoNaturalGeneration)) {
            lines.add_text(
                "No passable land terrains exist!",
                RulesetErrorSeverity::Error,
                None,
                None
            );
        }

        for terrain in self.ruleset.terrains.values() {
            for base_terrain_name in &terrain.occurs_on {
                if let Some(base_terrain) = self.ruleset.terrains.get(base_terrain_name) {
                    if base_terrain.type_ == TerrainType::NaturalWonder {
                        lines.add_text(
                            format!("{} occurs on natural wonder {}: Unsupported.",
                                terrain.name, base_terrain_name),
                            RulesetErrorSeverity::WarningOptionsOnly,
                            Some(terrain),
                            None
                        );
                    }
                } else {
                    lines.add_text(
                        format!("{} occurs on terrain {} which does not exist!",
                            terrain.name, base_terrain_name),
                        RulesetErrorSeverity::Error,
                        Some(terrain),
                        None
                    );
                }
            }

            if terrain.type_ == TerrainType::NaturalWonder {
                if let Some(turns_into) = &terrain.turns_into {
                    if let Some(base_terrain) = self.ruleset.terrains.get(turns_into) {
                        if !base_terrain.type_.is_base_terrain() {
                            // See https://github.com/hackedpassword/Z2/blob/main/HybridTileTech.md for a clever exploit
                            lines.add_text(
                                format!("{} turns into terrain {} which is not a base terrain!",
                                    terrain.name, turns_into),
                                RulesetErrorSeverity::Warning,
                                Some(terrain),
                                None
                            );
                        }
                    } else {
                        lines.add_text(
                            format!("{} turns into terrain {} which does not exist!",
                                terrain.name, turns_into),
                            RulesetErrorSeverity::Error,
                            Some(terrain),
                            None
                        );
                    }
                }
            }

            self.unique_validator.check_uniques(terrain, lines, true, try_fix_unknown_uniques);
        }
    }

    /// Adds improvement errors to the error list
    fn add_improvement_errors(&self, lines: &mut RulesetErrorList, try_fix_unknown_uniques: bool) {
        for improvement in self.ruleset.tile_improvements.values() {
            if let Some(tech_required) = &improvement.tech_required {
                if !self.ruleset.technologies.contains_key(tech_required) {
                    lines.add_text(
                        format!("{} requires tech {} which does not exist!",
                            improvement.name, tech_required),
                        RulesetErrorSeverity::Error,
                        Some(improvement),
                        None
                    );
                }
            }

            if let Some(replaces) = &improvement.replaces {
                if !self.ruleset.tile_improvements.contains_key(replaces) {
                    lines.add_text(
                        format!("{} replaces {} which does not exist!",
                            improvement.name, replaces),
                        RulesetErrorSeverity::Error,
                        Some(improvement),
                        None
                    );
                }
            }

            if improvement.replaces.is_some() && improvement.unique_to.is_none() {
                lines.add_text(
                    format!("{} should replace {} but does not have uniqueTo assigned!",
                        improvement.name, improvement.replaces.as_ref().unwrap()),
                    RulesetErrorSeverity::Error,
                    Some(improvement),
                    None
                );
            }

            for terrain in &improvement.terrains_can_be_built_on {
                if !self.ruleset.terrains.contains_key(terrain) && terrain != "Land" && terrain != "Water" {
                    lines.add_text(
                        format!("{} can be built on terrain {} which does not exist!",
                            improvement.name, terrain),
                        RulesetErrorSeverity::Error,
                        Some(improvement),
                        None
                    );
                }
            }

            if improvement.terrains_can_be_built_on.is_empty()
                && !improvement.has_unique(UniqueType::CanOnlyImproveResource)
                && !improvement.has_unique(UniqueType::Unbuildable)
                && !improvement.name.starts_with(&Constants::REMOVE)
                && !RoadStatus::entries().iter().any(|r| r.remove_action == improvement.name)
                && improvement.name != Constants::CANCEL_IMPROVEMENT_ORDER {
                lines.add_text(
                    format!("{} has an empty `terrainsCanBeBuiltOn`, isn't allowed to only improve resources. As such it isn't buildable! Either give this the unique \"Unbuildable\", \"Can only be built to improve a resource\", or add \"Land\", \"Water\" or any other value to `terrainsCanBeBuiltOn`.",
                        improvement.name),
                    RulesetErrorSeverity::Warning,
                    Some(improvement),
                    None
                );
            }

            for unique in improvement.unique_objects.iter()
                .filter(|u| u.type_ == UniqueType::PillageYieldRandom || u.type_ == UniqueType::PillageYieldFixed) {
                if !Stats::is_stats(&unique.params[0]) {
                    continue;
                }

                let params = Stats::parse(&unique.params[0]);
                if params.values.iter().any(|&v| v < 0) {
                    lines.add_text(
                        format!("{} cannot have a negative value for a pillage yield!",
                            improvement.name),
                        RulesetErrorSeverity::Error,
                        Some(improvement),
                        None
                    );
                }
            }

            let has_pillage_unique = improvement.has_unique(UniqueType::PillageYieldRandom, &StateForConditionals::ignore_conditionals())
                || improvement.has_unique(UniqueType::PillageYieldFixed, &StateForConditionals::ignore_conditionals());

            if has_pillage_unique && improvement.has_unique(UniqueType::Unpillagable, &StateForConditionals::ignore_conditionals()) {
                lines.add_text(
                    format!("{} has both an `Unpillagable` unique type and a `PillageYieldRandom` or `PillageYieldFixed` unique type!",
                        improvement.name),
                    RulesetErrorSeverity::Warning,
                    Some(improvement),
                    None
                );
            }

            self.unique_validator.check_uniques(improvement, lines, true, try_fix_unknown_uniques);
        }
    }

    fn add_building_errors(&self, errors: &mut RulesetErrorList) {
        if self.ruleset.buildings.is_empty() {
            errors.add_error("No buildings found in ruleset");
            return;
        }

        for building in &self.ruleset.buildings {
            // Check for nonexistent techs
            if let Some(tech) = &building.required_tech {
                if !self.ruleset.technologies.contains_key(tech) {
                    errors.add_error(format!(
                        "Building '{}' requires nonexistent tech '{}'",
                        building.name, tech
                    ));
                }
            }

            // Check for nonexistent replacements
            if let Some(replacement) = &building.replaces {
                if !self.ruleset.buildings.contains_key(replacement) {
                    errors.add_error(format!(
                        "Building '{}' replaces nonexistent building '{}'",
                        building.name, replacement
                    ));
                }
            }

            // Check for nonexistent unique types
            for unique in &building.uniques {
                if let Some(unique_type) = unique.get_type() {
                    if !self.ruleset.unique_types.contains_key(&unique_type) {
                        errors.add_error(format!(
                            "Building '{}' has invalid unique type '{}'",
                            building.name, unique_type
                        ));
                    }
                }
            }
        }
    }

    fn add_resource_errors(&self, errors: &mut RulesetErrorList) {
        if self.ruleset.resources.is_empty() {
            errors.add_error("No resources found in ruleset");
            return;
        }

        for resource in &self.ruleset.resources {
            // Check for nonexistent techs
            if let Some(tech) = &resource.required_tech {
                if !self.ruleset.technologies.contains_key(tech) {
                    errors.add_error(format!(
                        "Resource '{}' requires nonexistent tech '{}'",
                        resource.name, tech
                    ));
                }
            }

            // Check for nonexistent improvements
            if let Some(improvement) = &resource.improvement {
                if !self.ruleset.improvements.contains_key(improvement) {
                    errors.add_error(format!(
                        "Resource '{}' requires nonexistent improvement '{}'",
                        resource.name, improvement
                    ));
                }
            }

            // Check for nonexistent unique types
            for unique in &resource.uniques {
                if let Some(unique_type) = unique.get_type() {
                    if !self.ruleset.unique_types.contains_key(&unique_type) {
                        errors.add_error(format!(
                            "Resource '{}' has invalid unique type '{}'",
                            resource.name, unique_type
                        ));
                    }
                }
            }
        }
    }

    fn add_nation_errors(&self, errors: &mut RulesetErrorList) {
        if self.ruleset.nations.is_empty() {
            errors.add_error("No nations found in ruleset");
            return;
        }

        for nation in &self.ruleset.nations {
            // Check for nonexistent unique types
            for unique in &nation.uniques {
                if let Some(unique_type) = unique.get_type() {
                    if !self.ruleset.unique_types.contains_key(&unique_type) {
                        errors.add_error(format!(
                            "Nation '{}' has invalid unique type '{}'",
                            nation.name, unique_type
                        ));
                    }
                }
            }

            // Check for nonexistent starting units
            for unit in &nation.starting_units {
                if !self.ruleset.units.contains_key(unit) {
                    errors.add_error(format!(
                        "Nation '{}' starts with nonexistent unit '{}'",
                        nation.name, unit
                    ));
                }
            }

            // Check for nonexistent starting buildings
            for building in &nation.starting_buildings {
                if !self.ruleset.buildings.contains_key(building) {
                    errors.add_error(format!(
                        "Nation '{}' starts with nonexistent building '{}'",
                        nation.name, building
                    ));
                }
            }
        }
    }

    fn add_policy_errors(&self, errors: &mut RulesetErrorList) {
        if self.ruleset.policies.is_empty() {
            errors.add_error("No policies found in ruleset");
            return;
        }

        for policy in &self.ruleset.policies {
            // Check for nonexistent unique types
            for unique in &policy.uniques {
                if let Some(unique_type) = unique.get_type() {
                    if !self.ruleset.unique_types.contains_key(&unique_type) {
                        errors.add_error(format!(
                            "Policy '{}' has invalid unique type '{}'",
                            policy.name, unique_type
                        ));
                    }
                }
            }

            // Check for nonexistent prerequisites
            for prerequisite in &policy.prerequisites {
                if !self.ruleset.policies.contains_key(prerequisite) {
                    errors.add_error(format!(
                        "Policy '{}' requires nonexistent policy '{}'",
                        policy.name, prerequisite
                    ));
                }
            }
        }
    }

    fn add_unique_type_errors(&self, errors: &mut RulesetErrorList) {
        if self.ruleset.unique_types.is_empty() {
            errors.add_error("No unique types found in ruleset");
            return;
        }

        for (name, unique_type) in &self.ruleset.unique_types {
            // Check for invalid parameters
            if unique_type.requires_parameters() && unique_type.parameters.is_empty() {
                errors.add_error(format!(
                    "Unique type '{}' requires parameters but none are defined",
                    name
                ));
            }

            // Check for invalid targets
            if let Some(target) = &unique_type.target {
                match target.as_str() {
                    "Unit" | "Building" | "Tile" | "City" | "Nation" | "Policy" => (),
                    _ => errors.add_error(format!(
                        "Unique type '{}' has invalid target '{}'",
                        name, target
                    )),
                }
            }
        }
    }
}