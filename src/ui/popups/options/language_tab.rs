use ggez::graphics::{Color, DrawParam, Text};
use ggez::{Context, GameResult};

use crate::constants::Constants;
use crate::models::metadata::GameSettings;
use crate::ui::components::widgets::{LanguageTable, TabbedPager};
use crate::ui::popups::options::OptionsPopup;
use crate::ui::screens::base_screen::BaseScreen;
use crate::utils::translation::tr;

pub struct LanguageTab {
    options_popup: OptionsPopup,
    language_tables: Vec<LanguageTable>,
    chosen_language: String,
    on_language_selected: Box<dyn Fn()>,
}

impl LanguageTab {
    pub fn new(
        options_popup: OptionsPopup,
        on_language_selected: Box<dyn Fn()>
    ) -> Self {
        let settings = &options_popup.settings;
        let chosen_language = settings.language.clone();

        // Calculate width for language tables (90% of tabs width minus padding)
        let table_width = options_popup.tabs.pref_width * 0.9 - 10.0;

        // Create language tables
        let language_tables = LanguageTable::add_language_tables(table_width);

        Self {
            options_popup,
            language_tables,
            chosen_language,
            on_language_selected,
        }
    }

    pub fn render(&mut self, ctx: &mut Context, screen: &BaseScreen) -> GameResult<()> {
        let mut table = screen.create_table();
        table.pad(10.0);
        table.defaults().pad(5.0);

        // Add each language table
        for lang_table in &mut self.language_tables {
            table.add(lang_table).row();
        }

        // Render the table
        table.render(ctx, screen)?;

        Ok(())
    }

    fn select_language(&mut self) {
        let settings = &mut self.options_popup.settings;
        settings.language = self.chosen_language.clone();
        settings.update_locale_from_language();

        // Update translations
        // In Rust, we would need to implement this functionality
        // UncivGame::current().translations.try_read_translation_for_current_language();

        // Call the callback
        (self.on_language_selected)();
    }

    fn update_selection(&mut self) {
        // Update all language tables to reflect the current selection
        for lang_table in &mut self.language_tables {
            lang_table.update(&self.chosen_language);
        }

        // If the chosen language is different from the current setting, apply it
        if self.chosen_language != self.options_popup.settings.language {
            self.select_language();
        }
    }

    pub fn on_language_clicked(&mut self, language: String) {
        self.chosen_language = language;
        self.update_selection();
    }

    pub fn setup_key_shortcuts(&mut self) {
        // Add keyboard shortcuts for language selection
        LanguageTable::add_language_key_shortcuts(
            &mut self.language_tables,
            Box::new(move || self.chosen_language.clone()),
            Box::new(move |language| {
                self.chosen_language = language;
                // In Rust, we would need to implement this functionality
                // let pager = self.get_ascendant::<TabbedPager>();
                // if let Some(pager) = pager {
                //     self.activated(pager.active_page, "", pager);
                // }
            })
        );
    }
}

impl TabbedPager for LanguageTab {
    fn activated(&mut self, _index: i32, _caption: &str) {
        self.update_selection();

        // Find the selected table and scroll to it
        if let Some(selected_table) = self.language_tables.iter().find(|table| table.language == self.chosen_language) {
            // In Rust, we would need to implement this functionality
            // pager.page_scroll_to(selected_table, true);
        }
    }

    fn deactivated(&mut self, _index: i32, _caption: &str) {
        // Nothing to do on deactivation
    }
}