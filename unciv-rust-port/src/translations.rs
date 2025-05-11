use std::collections::{HashMap, LinkedHashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use regex::Regex;
use lazy_static::lazy_static;

use crate::translation_file_reader::TranslationFileReader;
use crate::translation_file_writer::TranslationFileWriter;

pub struct Translations {
    translations: HashMap<String, LinkedHashMap<String, String>>,
    current_language: String,
    fallback_language: String,
    language_percentages: HashMap<String, i32>,
}

impl Translations {
    pub fn new() -> Self {
        let mut translations = HashMap::new();
        let mut language_percentages = HashMap::new();

        // Load translations for all languages
        let languages = Self::get_languages();
        for language in languages {
            if let Ok(lang_translations) = TranslationFileReader::read_language(language.clone()) {
                translations.insert(language.clone(), lang_translations);
            }
        }

        // Load language percentages
        if let Ok(percentages) = TranslationFileReader::read_language_percentages() {
            language_percentages = percentages;
        }

        Self {
            translations,
            current_language: "English".to_string(),
            fallback_language: "English".to_string(),
            language_percentages,
        }
    }

    pub fn get_languages() -> Vec<String> {
        let mut languages = Vec::new();
        if let Ok(entries) = fs::read_dir("jsons/translations") {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.ends_with(".properties") && file_name != "template.properties" {
                        let language = file_name.replace(".properties", "");
                        languages.push(language);
                    }
                }
            }
        }
        languages.sort();
        languages
    }

    pub fn translate(&self, key: &str, params: Option<&HashMap<String, String>>) -> String {
        let translation = self.get_translation(key);
        if let Some(params) = params {
            self.replace_placeholders(translation, params)
        } else {
            translation
        }
    }

    fn get_translation(&self, key: &str) -> String {
        // Try current language first
        if let Some(lang_translations) = self.translations.get(&self.current_language) {
            if let Some(translation) = lang_translations.get(key) {
                return translation.clone();
            }
        }

        // Try fallback language
        if let Some(lang_translations) = self.translations.get(&self.fallback_language) {
            if let Some(translation) = lang_translations.get(key) {
                return translation.clone();
            }
        }

        // Return key if no translation found
        key.to_string()
    }

    fn replace_placeholders(&self, text: String, params: &HashMap<String, String>) -> String {
        let mut result = text;
        for (key, value) in params {
            let placeholder = format!("[{}]", key);
            result = result.replace(&placeholder, value);
        }
        result
    }

    pub fn get_language_percentage(&self, language: &str) -> i32 {
        *self.language_percentages.get(language).unwrap_or(&0)
    }

    pub fn set_language(&mut self, language: String) {
        if self.translations.contains_key(&language) {
            self.current_language = language;
        }
    }

    pub fn get_current_language(&self) -> &str {
        &self.current_language
    }

    pub fn get_fallback_language(&self) -> &str {
        &self.fallback_language
    }

    pub fn set_fallback_language(&mut self, language: String) {
        if self.translations.contains_key(&language) {
            self.fallback_language = language;
        }
    }
}

// Global instance
static TRANSLATIONS: OnceLock<Translations> = OnceLock::new();

pub fn get_translations() -> &'static Translations {
    TRANSLATIONS.get_or_init(Translations::new)
}

pub fn translate(key: &str, params: Option<&HashMap<String, String>>) -> String {
    get_translations().translate(key, params)
}