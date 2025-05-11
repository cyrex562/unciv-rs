use std::collections::{HashMap, HashSet, LinkedHashMap};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use regex::Regex;
use serde_json;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::translations::{TranslationEntry, Translations};
use crate::models::ruleset::{
    Belief, Building, CityStateType, Difficulty, Era, Event, GlobalUniques,
    Nation, PolicyBranch, Quest, RuinReward, Specialist, Speed, TechColumn,
    Terrain, TileImprovement, TileResource, Tutorial, Promotion, BaseUnit,
    UnitType, Victory
};
use crate::models::ruleset::unique::{Unique, UniqueType, UniqueFlag, UniqueParameterType, UniqueTarget};
use crate::models::ruleset::unique::UniqueParameterType;
use crate::models::spy_action::SpyAction;
use crate::models::diplomatic_modifiers::DiplomaticModifiers;
use crate::ui::components::input::KeyboardBinding;
use crate::utils::log::Log;
use crate::utils::debug;
use crate::models::metadata::game_settings::LocaleCode;
use crate::models::metadata::base_ruleset::BaseRuleset;
use crate::models::ruleset::ruleset_cache::RulesetCache;

pub struct TranslationFileWriter;

impl TranslationFileWriter {
    const SPECIAL_NEW_LINE_CODE: &'static str = "# This is an empty line ";
    const TEMPLATE_FILE_LOCATION: &'static str = "jsons/translations/template.properties";
    const LANGUAGE_FILE_LOCATION: &'static str = "jsons/translations/%s.properties";
    const SHORT_DESCRIPTION_KEY: &'static str = "Fastlane_short_description";
    const SHORT_DESCRIPTION_FILE: &'static str = "short_description.txt";
    const FULL_DESCRIPTION_KEY: &'static str = "Fastlane_full_description";
    const FULL_DESCRIPTION_FILE: &'static str = "full_description.txt";
    // Current dir on desktop should be assets, so use two '..' get us to project root
    const FASTLANE_PATH: &'static str = "../../fastlane/metadata/android/";

    // Untranslatable fields that should be excluded
    const UNTRANSLATABLE_FIELD_SET: &[&str] = &[
        "aiFreeTechs", "aiFreeUnits", "attackSound", "building", "cannotBeBuiltWith",
        "cultureBuildings", "excludedDifficulties", "improvement", "improvingTech",
        "obsoleteTech", "occursOn", "prerequisites", "promotions",
        "providesFreeBuilding", "replaces", "requiredBuilding", "requiredBuildingInAllCities",
        "requiredNearbyImprovedResources", "requiredResource", "requiredTech", "requires",
        "revealedBy", "startBias", "techRequired", "terrainsCanBeBuiltOn",
        "terrainsCanBeFoundOn", "turnsInto", "uniqueTo", "upgradesTo",
        "link", "icon", "extraImage", "color",  // FormattedLine
        "RuinReward.uniques", "TerrainType.name",
        "CityStateType.friendBonusUniques", "CityStateType.allyBonusUniques",
        "Era.citySound",
        "keyShortcut",
        "Event.name" // Presently not shown anywhere
    ];

    // Enums where the name property is translatable
    const TRANSLATABLE_ENUMS_SET: &[&str] = &["BeliefType"];

    // Only these Unique parameter types will be offered as translatables
    const TRANSLATABLE_UNIQUE_PARAMETER_TYPES: &[UniqueParameterType] = &[
        UniqueParameterType::Unknown,
        UniqueParameterType::Comment
    ];

    // Fields that need parameter processing
    const FIELDS_TO_PROCESS_PARAMETERS: &[&str] = &[
        "uniques", "promotions", "milestones",
    ];

    pub fn write_new_translation_files() -> String {
        match Self::write_new_translation_files_internal() {
            Ok(message) => message,
            Err(e) => {
                Log::error("Failed to generate translation files", &e);
                e.to_string()
            }
        }
    }

    fn write_new_translation_files_internal() -> Result<String, Box<dyn std::error::Error>> {
        let mut translations = Translations::new();
        translations.read_all_languages_translation()?;

        let mut fastlane_output = String::new();

        // Check if we're not running from a jar
        if !Self::is_running_from_jar() {
            let percentages = Self::generate_translation_files(&translations, None, None)?;
            Self::write_language_percentages(&percentages, None)?;
            fastlane_output = format!("\n{}", Self::write_translated_fastlane_files(&translations)?);
        }

        // Handle mods with translations
        for (mod_name, mod_translations) in &translations.mods_with_translations {
            let mod_folder = Self::get_mod_folder(mod_name)?;
            let mod_percentages = Self::generate_translation_files(mod_translations, Some(&mod_folder), Some(&translations))?;
            Self::write_language_percentages(&mod_percentages, Some(&mod_folder))?;
        }

        Ok(format!("Translation files are generated successfully.{}", fastlane_output))
    }

    fn is_running_from_jar() -> bool {
        // In Rust, we can check if we're running from a jar by checking if the executable path
        // contains ".jar" or by checking environment variables
        std::env::current_exe()
            .map(|path| path.to_string_lossy().contains(".jar"))
            .unwrap_or(false)
    }

    fn get_mod_folder(mod_name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        // This would need to be implemented based on your game's file structure
        // For now, we'll just return a path
        Ok(PathBuf::from(format!("mods/{}", mod_name)))
    }

    fn get_file_handle(mod_folder: Option<&PathBuf>, file_location: &str) -> PathBuf {
        match mod_folder {
            Some(folder) => folder.join(file_location),
            None => PathBuf::from(file_location),
        }
    }

    /// Writes new language files per Mod or for BaseRuleset - only each language that exists in translations.
    ///
    /// # Arguments
    ///
    /// * `translations` - The translations to write
    /// * `mod_folder` - Optional mod folder path
    /// * `base_translations` - Optional base translations for reference
    ///
    /// # Returns
    ///
    /// A map with the percentages of translated lines per language
    fn generate_translation_files(
        translations: &Translations,
        mod_folder: Option<&PathBuf>,
        base_translations: Option<&Translations>,
    ) -> Result<HashMap<String, i32>, Box<dyn std::error::Error>> {
        let mut file_name_to_generated_strings = LinkedHashMap::new();
        let mut lines_to_translate = Vec::new();

        if mod_folder.is_none() {
            // Base game
            let template_file = Self::get_file_handle(None, Self::TEMPLATE_FILE_LOCATION);
            if template_file.exists() {
                if let Ok(file) = File::open(&template_file) {
                    let reader = BufReader::new(file);
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            lines_to_translate.push(line);
                        }
                    }
                }
            }

            lines_to_translate.push("\n\n#################### Lines from Unique Types #######################\n".to_string());

            // Add unique types
            for unique_type in UniqueType::iter() {
                if unique_type.is_deprecated() || unique_type.flags().contains(&UniqueFlag::HiddenToUsers) {
                    continue;
                }
                lines_to_translate.push(format!("{} = ", unique_type.get_translatable()));
            }

            // Add unique parameter types
            for param_type in UniqueParameterType::iter() {
                let strings = param_type.get_translation_writer_strings_for_output();
                if strings.is_empty() {
                    continue;
                }
                lines_to_translate.push(format!("\n######### {} ###########\n", param_type.display_name()));
                for string in strings {
                    lines_to_translate.push(format!("{} = ", string));
                }
            }

            // Add unique targets
            for target in UniqueTarget::iter() {
                lines_to_translate.push(format!("{} = ", target));
            }

            // Add spy actions
            lines_to_translate.push("\n\n#################### Lines from spy actions #######################\n".to_string());
            for spy_action in SpyAction::iter() {
                lines_to_translate.push(format!("{} = ", spy_action.display_string()));
            }

            // Add diplomatic modifiers
            lines_to_translate.push("\n\n#################### Lines from diplomatic modifiers #######################\n".to_string());
            for modifier in DiplomaticModifiers::iter() {
                lines_to_translate.push(format!("{} = ", modifier.text()));
            }

            // Add key bindings
            lines_to_translate.push("\n\n#################### Lines from key bindings #######################\n".to_string());
            for binding in KeyboardBinding::get_translation_entries() {
                lines_to_translate.push(format!("{} = ", binding));
            }

            // Process base rulesets
            for base_ruleset in BaseRuleset::iter() {
                let ruleset_path = format!("jsons/{}", base_ruleset.full_name());
                let generated_strings = Self::generate_strings_from_jsons(&PathBuf::from(ruleset_path))?;
                for (key, value) in generated_strings {
                    file_name_to_generated_strings.insert(format!("{} from {}", key, base_ruleset.full_name()), value);
                }
            }

            // Process tutorials
            let tutorial_strings = Self::generate_strings_from_jsons(&PathBuf::from("jsons"), |file| file.file_name().map_or(false, |name| name == "Tutorials.json"))?;
            if let Some((_, value)) = tutorial_strings.iter().next() {
                file_name_to_generated_strings.insert("Tutorials".to_string(), value.clone());
            }
        } else {
            // Process mod
            if let Some(folder) = mod_folder {
                let jsons_folder = folder.join("jsons");
                let generated_strings = Self::generate_strings_from_jsons(&jsons_folder)?;
                for (key, value) in generated_strings {
                    file_name_to_generated_strings.insert(key, value);
                }
            }
        }

        // Add all generated strings to lines_to_translate
        for (key, value) in &file_name_to_generated_strings {
            if value.is_empty() {
                continue;
            }
            lines_to_translate.push(format!("\n#################### Lines from {} ####################\n", key));
            lines_to_translate.extend(value.iter().cloned());
        }
        file_name_to_generated_strings.clear(); // No longer needed

        let mut count_of_translatable_lines = 0;
        let mut count_of_translated_lines = HashMap::new();

        // Iterate through all available languages
        for (language_index, language) in translations.get_languages().iter().enumerate() {
            let mut translations_of_this_language = 0;
            let mut string_builder = String::new();

            // This is so we don't add the same keys twice
            let mut existing_translation_keys = HashSet::new();

            for line in &lines_to_translate {
                if !line.contains(" = ") {
                    // Small hack to insert empty lines
                    if line.starts_with(Self::SPECIAL_NEW_LINE_CODE) {
                        string_builder.push('\n');
                    } else {
                        // Copy as-is
                        string_builder.push_str(line);
                        string_builder.push('\n');
                    }
                    continue;
                }

                let parts: Vec<&str> = line.split(" = ").collect();
                if parts.len() != 2 {
                    continue;
                }

                let translation_key = parts[0].replace("\\n", "\n");
                let hash_map_key = if translation_key == Translations::ENGLISH_CONDITIONAL_ORDERING_STRING {
                    Translations::ENGLISH_CONDITIONAL_ORDERING_STRING.to_string()
                } else {
                    translation_key
                        .replace(r"<[^>]*>", "")
                        .replace(r"\[[^\]]*\]", "[]")
                };

                if existing_translation_keys.contains(&hash_map_key) {
                    continue; // Don't add it twice
                }
                existing_translation_keys.insert(hash_map_key.clone());

                // Count translatable lines only once
                if language_index == 0 {
                    count_of_translatable_lines += 1;
                }

                let existing_translation = translations.get(&hash_map_key);
                let mut translation_value = if let Some(entry) = existing_translation {
                    if entry.contains_key(language) {
                        translations_of_this_language += 1;
                        entry.get(language).unwrap().clone()
                    } else if let Some(base) = base_translations {
                        if let Some(base_entry) = base.get(&hash_map_key) {
                            if base_entry.contains_key(language) {
                                // String is used in the mod but also exists in base - ignore
                                continue;
                            }
                        }
                        // String is not translated either here or in base
                        string_builder.push_str(" # Requires translation!\n");
                        String::new()
                    } else {
                        // String is not translated
                        string_builder.push_str(" # Requires translation!\n");
                        String::new()
                    }
                } else {
                    // String is not translated
                    string_builder.push_str(" # Requires translation!\n");
                    String::new()
                };

                // Handle parameter autocorrection
                if translation_value.contains('[') {
                    let params_of_key = Self::get_placeholder_parameters(&translation_key);
                    let params_of_value = Self::get_placeholder_parameters(&translation_value);

                    let params_of_key_not_in_value: Vec<_> = params_of_key.iter()
                        .filter(|p| !params_of_value.contains(p))
                        .collect();

                    let params_of_value_not_in_key: Vec<_> = params_of_value.iter()
                        .filter(|p| !params_of_key.contains(p))
                        .collect();

                    if params_of_key_not_in_value.len() == 1 && params_of_value_not_in_key.len() == 1 {
                        let old_param = format!("[{}]", params_of_value_not_in_key[0]);
                        let new_param = format!("[{}]", params_of_key_not_in_value[0]);
                        translation_value = translation_value.replace(&old_param, &new_param);
                    }
                }

                Self::append_translation(&mut string_builder, &translation_key, &translation_value);
            }

            count_of_translated_lines.insert(language.clone(), translations_of_this_language);

            let file_writer = Self::get_file_handle(mod_folder, &Self::LANGUAGE_FILE_LOCATION.replace("%s", language));

            // Any time you have more than 3 line breaks, make it 3
            let final_file_text = string_builder.replace("\n\n\n\n", "\n\n\n");

            if let Some(parent) = file_writer.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut file = File::create(&file_writer)?;
            file.write_all(final_file_text.as_bytes())?;
        }

        // Calculate the percentages of translations
        for (language, count) in &mut count_of_translated_lines {
            *count = if count_of_translatable_lines <= 0 {
                100
            } else {
                *count * 100 / count_of_translatable_lines
            };
        }

        Ok(count_of_translated_lines)
    }

    fn append_translation(string_builder: &mut String, key: &str, value: &str) {
        string_builder.push_str(&key.replace("\n", "\\n"));
        string_builder.push_str(" = ");
        string_builder.push_str(&value.replace("\n", "\\n"));
        string_builder.push('\n');
    }

    fn write_language_percentages(percentages: &HashMap<String, i32>, mod_folder: Option<&PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
        let mut output = String::new();

        // Sort by language name
        let mut sorted_languages: Vec<_> = percentages.keys().collect();
        sorted_languages.sort();

        for language in sorted_languages {
            if let Some(percentage) = percentages.get(language) {
                output.push_str(&format!("{} = {}\n", language, percentage));
            }
        }

        let file_path = Self::get_file_handle(mod_folder, crate::models::translations::TranslationFileReader::PERCENTAGES_FILE_LOCATION);

        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = File::create(&file_path)?;
        file.write_all(output.as_bytes())?;

        Ok(())
    }

    fn get_placeholder_parameters(text: &str) -> Vec<String> {
        let mut params = Vec::new();
        let re = Regex::new(r"\[([^\]]+)\]").unwrap();

        for cap in re.captures_iter(text) {
            if let Some(param) = cap.get(1) {
                params.push(param.as_str().to_string());
            }
        }

        params
    }

    /// This scans one folder for json files and generates lines to translate (left side).
    fn generate_strings_from_jsons<F>(jsons_folder: &Path, file_filter: F) -> Result<LinkedHashMap<String, HashSet<String>>, Box<dyn std::error::Error>>
    where
        F: Fn(&Path) -> bool + 'static,
    {
        let mut result = LinkedHashMap::new();
        let ruleset = RulesetCache::get_vanilla_ruleset();
        let start_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        let mut unique_index_of_new_line = 0;

        // Get list of JSON files
        let mut json_files = Vec::new();
        if jsons_folder.is_dir() {
            for entry in fs::read_dir(jsons_folder)? {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() && file_filter(&path) {
                        json_files.push(path);
                    }
                }
            }
        }

        // Sort by filename for predictable order
        json_files.sort_by(|a, b| {
            a.file_name()
                .and_then(|a| a.to_str())
                .unwrap_or("")
                .cmp(b.file_name().and_then(|b| b.to_str()).unwrap_or(""))
        });

        // Process each JSON file
        for json_file in json_files {
            let filename = json_file.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Get the appropriate class for this JSON file
            let class_name = Self::get_class_name_by_filename(&filename);
            if class_name.is_none() {
                continue; // Unknown JSON, skip it
            }

            let mut result_strings = HashSet::new();
            result.insert(filename.clone(), result_strings.clone());

            // Read and parse the JSON file
            if let Ok(file) = File::open(&json_file) {
                let reader = BufReader::new(file);
                match serde_json::from_reader::<_, serde_json::Value>(reader) {
                    Ok(json_value) => {
                        if let Some(array) = json_value.as_array() {
                            for element in array {
                                Self::serialize_element(element, &mut result_strings, &ruleset, &mut unique_index_of_new_line);
                                // Add a newline marker
                                result_strings.insert(format!("{} {}", Self::SPECIAL_NEW_LINE_CODE, unique_index_of_new_line));
                                unique_index_of_new_line += 1;
                            }
                        }
                    }
                    Err(e) => {
                        Log::error(&format!("Failed to parse JSON file: {}", json_file.display()), &e);
                    }
                }
            }

            result.insert(filename, result_strings);
        }

        let display_name = if jsons_folder.file_name().and_then(|s| s.to_str()) != Some("jsons") {
            jsons_folder.file_name().and_then(|s| s.to_str()).unwrap_or("unknown").to_string()
        } else {
            jsons_folder.parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                .map(|s| if s.is_empty() { "Tutorials" } else { s })
                .unwrap_or("Tutorials")
                .to_string()
        };

        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() - start_millis;

        debug!("Translation writer took {}ms for {}", elapsed, display_name);

        Ok(result)
    }

    /// Default implementation that accepts all .json files
    fn generate_strings_from_jsons_default(jsons_folder: &Path) -> Result<LinkedHashMap<String, HashSet<String>>, Box<dyn std::error::Error>> {
        Self::generate_strings_from_jsons(jsons_folder, |file| {
            file.extension().and_then(|ext| ext.to_str()) == Some("json")
        })
    }

    fn submit_string(string: &str, result_strings: &mut HashSet<String>) {
        if string.is_empty() {
            return; // Entries in Collection<String> do not pass isFieldTranslatable
        }

        if string.contains('{') {
            let re = Regex::new(r"\{([^}]+)\}").unwrap();
            let matches: Vec<_> = re.captures_iter(string).collect();
            if !matches.is_empty() {
                // Ignore outer string, only translate the parts within `{}`
                for cap in matches {
                    if let Some(m) = cap.get(1) {
                        Self::submit_string(m.as_str(), result_strings);
                    }
                }
                return;
            }
        }

        result_strings.insert(format!("{} = ", string));
    }

    fn submit_string_with_unique(string: &str, unique: &Unique, result_strings: &mut HashSet<String>, ruleset: &RulesetCache) {
        if unique.is_hidden_to_users() {
            return; // We don't need to translate this at all, not user-visible
        }

        let string_to_translate = string.remove_conditionals();

        for conditional in unique.modifiers() {
            Self::submit_string_with_unique(&conditional.text(), conditional, result_strings, ruleset);
        }

        if unique.params().is_empty() {
            Self::submit_string(&string_to_translate, result_strings);
            return;
        }

        // Do simpler parameter numbering when typed
        if let Some(unique_type) = unique.unique_type() {
            for (index, type_list) in unique_type.parameter_type_map().iter().enumerate() {
                if type_list.iter().any(|t| Self::TRANSLATABLE_UNIQUE_PARAMETER_TYPES.contains(t)) {
                    continue;
                }
                // Unknown/Comment parameter contents better be offered to translators too
                if let Some(param) = unique.params().get(index) {
                    result_strings.insert(format!("{} = ", param));
                }
            }
            result_strings.insert(format!("{} = ", unique_type.get_translatable()));
            return;
        }

        let mut parameter_names = Vec::new();
        for parameter in unique.params() {
            let parameter_name = UniqueParameterType::guess_type_for_translation_writer(parameter, ruleset).parameter_name;
            Self::add_numbered_parameter(&mut parameter_names, &parameter_name);

            if !Self::TRANSLATABLE_UNIQUE_PARAMETER_TYPES.iter().any(|t| t.parameter_name == parameter_name) {
                continue;
            }
            result_strings.insert(format!("{} = ", parameter));
        }

        result_strings.insert(format!("{} = ", string_to_translate.fill_placeholders(&parameter_names)));
    }

    fn add_numbered_parameter(parameters: &mut Vec<String>, name: &str) {
        if !parameters.contains(&name.to_string()) {
            parameters.push(name.to_string());
            return;
        }

        let mut i = 2;
        while parameters.contains(&format!("{}{}", name, i)) {
            i += 1;
        }
        parameters.push(format!("{}{}", name, i));
    }

    fn serialize_element(element: &serde_json::Value, result_strings: &mut HashSet<String>, ruleset: &RulesetCache, unique_index_of_new_line: &mut i32) {
        if let Some(string) = element.as_str() {
            Self::submit_string(string, result_strings);
            return;
        }

        if let Some(obj) = element.as_object() {
            for (key, value) in obj {
                // Skip fields that should not be translated
                if !Self::is_field_translatable(key, value) {
                    continue;
                }

                match value {
                    serde_json::Value::String(s) => {
                        Self::submit_string(s, result_strings);
                    }
                    serde_json::Value::Array(arr) => {
                        for item in arr {
                            Self::serialize_element(item, result_strings, ruleset, unique_index_of_new_line);
                        }
                    }
                    serde_json::Value::Object(obj) => {
                        Self::serialize_element(&serde_json::Value::Object(obj.clone()), result_strings, ruleset, unique_index_of_new_line);
                    }
                    _ => {}
                }
            }
        }
    }

    fn is_field_translatable(field_name: &str, value: &serde_json::Value) -> bool {
        // Skip null or empty values
        if value.is_null() || (value.is_string() && value.as_str().unwrap_or("").is_empty()) {
            return false;
        }

        // Check if field is in the untranslatable set
        if Self::UNTRANSLATABLE_FIELD_SET.contains(&field_name) {
            return false;
        }

        // For enum types, check if they're in the translatable set
        // This would need to be implemented based on your type system

        true
    }

    fn get_class_name_by_filename(filename: &str) -> Option<&'static str> {
        match filename {
            "Beliefs" => Some("Belief"),
            "Buildings" => Some("Building"),
            "CityStateTypes" => Some("CityStateType"),
            "Difficulties" => Some("Difficulty"),
            "Eras" => Some("Era"),
            "Events" => Some("Event"),
            "GlobalUniques" => Some("GlobalUniques"),
            "Nations" => Some("Nation"),
            "Policies" => Some("PolicyBranch"),
            "Quests" => Some("Quest"),
            "Religions" => Some("String"),
            "Ruins" => Some("RuinReward"),
            "Specialists" => Some("Specialist"),
            "Speeds" => Some("Speed"),
            "Techs" => Some("TechColumn"),
            "Terrains" => Some("Terrain"),
            "TileImprovements" => Some("TileImprovement"),
            "TileResources" => Some("TileResource"),
            "Tutorials" => Some("Tutorial"),
            "UnitPromotions" => Some("Promotion"),
            "Units" => Some("BaseUnit"),
            "UnitTypes" => Some("UnitType"),
            "VictoryTypes" => Some("Victory"),
            _ => None
        }
    }

    /// This writes translated short_description.txt and full_description.txt files into the Fastlane structure.
    fn write_translated_fastlane_files(translations: &Translations) -> Result<String, Box<dyn std::error::Error>> {
        Self::write_fastlane_files(Self::SHORT_DESCRIPTION_FILE, translations.get(Self::SHORT_DESCRIPTION_KEY), false)?;
        Self::write_fastlane_files(Self::FULL_DESCRIPTION_FILE, translations.get(Self::FULL_DESCRIPTION_KEY), true)?;
        Self::update_fastlane_changelog()?;

        Ok("Fastlane files are generated successfully.".to_string())
    }

    fn write_fastlane_files(file_name: &str, translation_entry: Option<&TranslationEntry>, end_with_newline: bool) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(entry) = translation_entry {
            for (language, translated) in entry.iter() {
                let mut file_content = translated.clone();

                if end_with_newline && !file_content.ends_with('\n') {
                    file_content.push('\n');
                } else if !end_with_newline && file_content.ends_with('\n') {
                    file_content.pop(); // Remove the trailing newline
                }

                let locale_code = LocaleCode::from_str(&language.replace("_", ""))?;
                let path = format!("{}{}", Self::FASTLANE_PATH, locale_code.true_language().unwrap_or(locale_code.language()));

                fs::create_dir_all(&path)?;
                let file_path = Path::new(&path).join(file_name);

                let mut file = File::create(&file_path)?;
                file.write_all(file_content.as_bytes())?;
            }
        }

        Ok(())
    }

    fn update_fastlane_changelog() -> Result<(), Box<dyn std::error::Error>> {
        // Read the changelog file
        let changelog_path = Path::new("../../changelog.md");
        let changelog_content = fs::read_to_string(changelog_path)?;

        // Extract the latest version changelog
        let re = Regex::new(r"## \S*([^#]*)")?;
        let version_changelog = if let Some(cap) = re.captures(&changelog_content) {
            if let Some(m) = cap.get(1) {
                m.as_str().trim().to_string()
            } else {
                return Err("Failed to extract version changelog".into());
            }
        } else {
            return Err("Failed to find version changelog".into());
        };

        // Read the build config file to get the version number
        let build_config_path = Path::new("../../buildSrc/src/main/kotlin/BuildConfig.kt");
        let build_config_content = fs::read_to_string(build_config_path)?;

        let version_re = Regex::new(r"appCodeNumber = (\d*)")?;
        let version_number = if let Some(cap) = version_re.captures(&build_config_content) {
            if let Some(m) = cap.get(1) {
                m.as_str().to_string()
            } else {
                return Err("Failed to extract version number".into());
            }
        } else {
            return Err("Failed to find version number".into());
        };

        // Write the changelog to the fastlane directory
        let file_name = format!("{}/en-US/changelogs/{}.txt", Self::FASTLANE_PATH, version_number);
        let file_path = Path::new(&file_name);

        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = File::create(file_path)?;
        file.write_all(version_changelog.as_bytes())?;

        Ok(())
    }
}