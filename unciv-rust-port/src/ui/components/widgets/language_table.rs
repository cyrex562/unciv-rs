use ggez::graphics::{Color, DrawParam};
use ggez::mint::Point2;
use std::sync::Arc;

use crate::constants::Constants;
use crate::ui::components::formatted_line::FormattedLine;
use crate::ui::components::input::{KeyCharAndCode, KeyShortcutDispatcher, KeyboardBinding};
use crate::ui::components::widgets::table::Table;
use crate::ui::components::widgets::widget::Widget;
use crate::ui::images::ImageGetter;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::civilopediascreen::MarkupRenderer;
use crate::UncivGame;

/// Represents a row in the Language picker, used both in OptionsPopup and in LanguagePickerScreen
pub struct LanguageTable {
    /// The base Table that this LanguageTable extends
    base: Table,

    /// The language code
    language: String,

    /// The percentage of completion for this language
    percent_complete: i32,

    /// The base color for the table
    base_color: Color,

    /// The darkened base color for the table
    dark_base_color: Color,
}

impl LanguageTable {
    /// Creates a new LanguageTable with the given language and completion percentage
    pub fn new(language: String, percent_complete: i32) -> Self {
        let base_screen = BaseScreen::get_instance();
        let skin = base_screen.skin();
        let base_color = base_screen.skin_strings().skin_config().base_color();
        let dark_base_color = base_color.darken(0.5);

        let mut table = Table::new(skin);
        table.pad(10.0);
        table.defaults().pad(10.0);
        table.left();

        // Add flag icon if it exists
        if ImageGetter::image_exists(&format!("FlagIcons/{}", language)) {
            let flag_image = ImageGetter::get_image(&format!("FlagIcons/{}", language));
            flag_image.set_size(40.0, 40.0);
            table.add_child(Arc::new(flag_image));
        }

        // Add language name and completion percentage
        let space_split_lang = language.replace("_", " ");
        let label = format!("{} ({}%)", space_split_lang, percent_complete).to_label();
        table.add_child(Arc::new(label));

        // Set initial background
        table.set_background(
            base_screen.skin_strings().get_ui_background(
                "LanguagePickerScreen/LanguageTable",
                Some(dark_base_color),
            ),
        );

        // Make table touchable
        table.set_touchable(true);
        table.pack();

        Self {
            base: table,
            language,
            percent_complete,
            base_color,
            dark_base_color,
        }
    }

    /// Updates the table's appearance based on the chosen language
    pub fn update(&mut self, chosen_language: &str) {
        let base_screen = BaseScreen::get_instance();
        let tint_color = if chosen_language == self.language {
            self.base_color
        } else {
            self.dark_base_color
        };

        self.base.set_background(
            base_screen.skin_strings().get_ui_background(
                "LanguagePickerScreen/LanguageTable",
                Some(tint_color),
            ),
        );
    }

    /// Extension to add the Language boxes to a Table
    pub fn add_language_tables(table: &mut Table, expected_width: f32) -> Vec<LanguageTable> {
        let mut language_tables = Vec::new();

        // Add translation disclaimer
        let translation_disclaimer = FormattedLine::new(
            "Please note that translations are a community-based work in progress and are \
            INCOMPLETE! The percentage shown is how much of the language is translated in-game. \
            If you want to help translating the game into your language, click here.",
            Some(format!("{}/Other/Translating/", Constants::wiki_url())),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(15),
            None,
            None,
            None,
            None,
            None,
        );

        let rendered_disclaimer = MarkupRenderer::render(
            vec![translation_disclaimer],
            expected_width,
            None,
            None,
        );
        table.add_child(Arc::new(rendered_disclaimer)).pad(5.0).row();

        // Create table for languages
        let mut table_languages = Table::new(table.skin().clone());
        table_languages.defaults().uniform_x().fill_x().pad(10.0);

        // Get system language
        let system_language = std::env::var("LANG")
            .unwrap_or_else(|_| "en".to_string())
            .split('_')
            .next()
            .unwrap_or("en")
            .to_string();

        // Get language completion percentages
        let language_completion_percentage = UncivGame::current()
            .translations()
            .percent_complete_of_languages();

        // Create language tables
        for (lang, percent) in language_completion_percentage {
            let is_english = lang == Constants::english();
            let is_system_lang = lang == system_language;
            let completion = if is_english { 100 } else { percent };

            language_tables.push(LanguageTable::new(lang, completion));
        }

        // Sort language tables
        language_tables.sort_by(|a, b| {
            let a_is_english = a.language == Constants::english();
            let b_is_english = b.language == Constants::english();
            if a_is_english != b_is_english {
                return a_is_english.cmp(&b_is_english);
            }

            let a_is_system = a.language == system_language;
            let b_is_system = b.language == system_language;
            if a_is_system != b_is_system {
                return a_is_system.cmp(&b_is_system);
            }

            b.percent_complete.cmp(&a.percent_complete)
        });

        // Add language tables to the table
        for language_table in &language_tables {
            table_languages.add_child(Arc::new(language_table.clone())).row();
        }
        table.add_child(Arc::new(table_languages)).row();

        language_tables
    }

    /// Create round-robin letter key handling
    pub fn add_language_key_shortcuts(
        actor: &mut dyn Widget,
        language_tables: &[LanguageTable],
        get_selection: Box<dyn Fn() -> String + Send + Sync>,
        action: Box<dyn Fn(String) + Send + Sync>,
    ) {
        let mut key_shortcuts = KeyShortcutDispatcher::new();

        // Group languages by first letter
        let mut letter_groups: std::collections::HashMap<char, Vec<&LanguageTable>> = std::collections::HashMap::new();
        for table in language_tables {
            if let Some(first_char) = table.language.chars().next() {
                letter_groups.entry(first_char).or_insert_with(Vec::new).push(table);
            }
        }

        // Add key shortcuts for each letter group
        for (letter, candidates) in letter_groups {
            if candidates.len() <= 1 {
                continue;
            }

            let get_selection = get_selection.clone();
            let action = action.clone();
            let candidates = candidates.to_vec();

            key_shortcuts.add(KeyShortcutDispatcher::KeyShortcut::new(
                KeyboardBinding::None,
                KeyCharAndCode::from_char(letter),
                0,
                Box::new(move || {
                    let current_selection = get_selection();
                    let current_index = candidates
                        .iter()
                        .position(|t| t.language == current_selection)
                        .unwrap_or(0);
                    let new_index = (current_index + 1) % candidates.len();
                    action(candidates[new_index].language.clone());
                }),
            ));
        }

        actor.set_key_shortcuts(key_shortcuts);
    }
}

// Implement the necessary traits for LanguageTable
impl std::ops::Deref for LanguageTable {
    type Target = Table;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for LanguageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Clone for LanguageTable {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            language: self.language.clone(),
            percent_complete: self.percent_complete,
            base_color: self.base_color,
            dark_base_color: self.dark_base_color,
        }
    }
}