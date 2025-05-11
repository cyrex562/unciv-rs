use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use ggez::graphics::{Color, DrawParam, Text};
use ggez::{Context, GameResult};

use crate::constants::Constants;
use crate::models::metadata::GameSettings;
use crate::models::ruleset::{Ruleset, RulesetCache};
use crate::models::ruleset::validation::{RulesetErrorSeverity, UniqueAutoUpdater};
use crate::ui::components::widgets::{Button, ExpanderTab, TabbedPager, TranslatedSelectBox};
use crate::ui::popups::options::OptionsPopup;
use crate::ui::popups::ToastPopup;
use crate::ui::screens::base_screen::BaseScreen;
use crate::ui::screens::unique_builder_screen::UniqueBuilderScreen;
use crate::utils::concurrency::Concurrency;
use crate::utils::image_getter::ImageGetter;
use crate::utils::translation::tr;

const MOD_CHECK_WITHOUT_BASE: &str = "-none-";

pub struct ModCheckTab {
    screen: Arc<BaseScreen>,
    fixed_content: BaseScreen,
    mod_check_first_run: bool,
    mod_check_base_select: Option<TranslatedSelectBox>,
    mod_check_result_table: BaseScreen,
}

impl ModCheckTab {
    pub fn new(screen: Arc<BaseScreen>) -> Self {
        let mut fixed_content = BaseScreen::new();
        fixed_content.pad(10.0);
        fixed_content.defaults().pad(10.0).align_top();

        let mut mod_check_result_table = BaseScreen::new();

        Self {
            screen,
            fixed_content,
            mod_check_first_run: true,
            mod_check_base_select: None,
            mod_check_result_table,
        }
    }

    pub fn initialize(&mut self) {
        // Add reload mods button
        let reload_mods_button = Button::new("Reload mods")
            .on_click(Box::new(move || self.run_action()));
        self.fixed_content.add(reload_mods_button).row();

        // Add base select dropdown
        let mut labeled_base_select = BaseScreen::new();
        labeled_base_select.add_label("Check extension mods based on:").pad_right(10.0);

        // Get base mods
        let mut base_mods = vec![MOD_CHECK_WITHOUT_BASE.to_string()];
        base_mods.extend(RulesetCache::get_sorted_base_rulesets());

        // Create select box
        let mut mod_check_base_select = TranslatedSelectBox::new(base_mods, MOD_CHECK_WITHOUT_BASE.to_string());
        mod_check_base_select.set_selected_index(0);
        mod_check_base_select.on_change(Box::new(move || self.run_action()));

        labeled_base_select.add(mod_check_base_select);
        self.fixed_content.add(labeled_base_select).row();

        // Add result table
        self.fixed_content.add(&mut self.mod_check_result_table);

        // Store the select box
        self.mod_check_base_select = Some(mod_check_base_select);
    }

    pub fn render(&mut self, ctx: &mut Context, screen: &BaseScreen) -> GameResult<()> {
        // Render the fixed content
        self.fixed_content.render(ctx, screen)?;

        // Render the result table
        self.mod_check_result_table.render(ctx, screen)?;

        Ok(())
    }

    fn run_action(&mut self) {
        if self.mod_check_first_run {
            self.run_mod_checker();
        } else if let Some(base_select) = &self.mod_check_base_select {
            self.run_mod_checker(base_select.get_selected().value.clone());
        }
    }

    fn run_mod_checker(&mut self, base: String) {
        self.mod_check_first_run = false;

        if self.mod_check_base_select.is_none() {
            return;
        }

        // Get currently opened expander titles
        let opened_expander_titles: HashSet<String> = self.mod_check_result_table
            .get_children()
            .iter()
            .filter_map(|child| {
                if let Some(expander) = child.downcast_ref::<ExpanderTab>() {
                    if expander.is_open() {
                        Some(expander.get_title().to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        // Clear the result table
        self.mod_check_result_table.clear();

        // Load rulesets and check for errors
        let ruleset_errors = RulesetCache::load_rulesets();
        if !ruleset_errors.is_empty() {
            let mut error_table = BaseScreen::new();
            error_table.defaults().pad(2.0);

            for ruleset_error in ruleset_errors {
                error_table.add_label(ruleset_error.to_string())
                    .width(self.screen.get_width() / 2.0)
                    .row();
            }

            self.mod_check_result_table.add(error_table);
        }

        // Add loading indicator
        self.mod_check_result_table.add_label("Checking mods for errors...").row();

        // Disable the base select
        if let Some(base_select) = &mut self.mod_check_base_select {
            base_select.set_disabled(true);
        }

        // Run the mod checker in a separate thread
        Concurrency::run("ModChecker", move || {
            // Get all mods and sort them
            let mut mods: Vec<&Ruleset> = RulesetCache::values()
                .iter()
                .collect();

            // Sort by name
            mods.sort_by(|a, b| a.name.cmp(&b.name));

            // Sort by whether they're in the opened expander titles
            mods.sort_by(|a, b| {
                let a_in_opened = opened_expander_titles.contains(&a.name);
                let b_in_opened = opened_expander_titles.contains(&b.name);
                b_in_opened.cmp(&a_in_opened)
            });

            // Check each mod
            for mod_ruleset in mods {
                // Skip base rulesets if we're checking against a specific base
                if base != MOD_CHECK_WITHOUT_BASE && mod_ruleset.mod_options.is_base_ruleset {
                    continue;
                }

                // Get mod links (errors)
                let mut mod_links = if base == MOD_CHECK_WITHOUT_BASE {
                    mod_ruleset.get_error_list(true)
                } else {
                    let mut linked_set = HashSet::new();
                    linked_set.insert(mod_ruleset.name.clone());
                    RulesetCache::check_combined_mod_links(linked_set, &base, true)
                };

                // Sort by error severity
                mod_links.sort_by(|a, b| b.error_severity_to_report.cmp(&a.error_severity_to_report));

                // Check if there are no problems
                let no_problem = !mod_links.is_not_ok();

                // Add separator and "No problems found" message if needed
                if !mod_links.is_empty() {
                    mod_links.push((String::new(), RulesetErrorSeverity::OK, None));
                }

                if no_problem {
                    mod_links.push((tr("No problems found."), RulesetErrorSeverity::OK, None));
                }

                // Get the final severity color
                let icon_color = mod_links.get_final_severity().color;

                // Choose the icon based on the severity
                let icon_name = match icon_color {
                    Color::RED => "OtherIcons/Stop",
                    Color::YELLOW => "OtherIcons/ExclamationMark",
                    _ => "OtherIcons/Checkmark",
                };

                // Create the icon
                let mut icon = ImageGetter::get_image(icon_name);
                icon.set_color(ImageGetter::CHARCOAL);
                let icon = icon.surround_with_circle(30.0, icon_color);

                // Create the expander tab
                let mut expander_tab = ExpanderTab::new(
                    &mod_ruleset.name,
                    Some(icon),
                    opened_expander_titles.contains(&mod_ruleset.name)
                );

                // Set up the expander tab content
                expander_tab.defaults().align_left().pad(10.0);

                // Add "Open unique builder" button
                let open_unique_builder_button = Button::new("Open unique builder");
                let ruleset = if base == MOD_CHECK_WITHOUT_BASE {
                    mod_ruleset.clone()
                } else {
                    let mut linked_set = HashSet::new();
                    linked_set.insert(mod_ruleset.name.clone());
                    RulesetCache::get_complex_ruleset(linked_set, &base)
                };

                open_unique_builder_button.on_click(Box::new(move || {
                    // In Rust, we would need to implement this functionality
                    // UncivGame::current().push_screen(UniqueBuilderScreen::new(ruleset));
                }));

                expander_tab.add(open_unique_builder_button).row();

                // Add "Autoupdate mod uniques" button if needed
                if !no_problem && mod_ruleset.folder_location.is_some() {
                    let replaceable_uniques = UniqueAutoUpdater::get_deprecated_replaceable_uniques(mod_ruleset);
                    if !replaceable_uniques.is_empty() {
                        let auto_update_button = Button::new("Autoupdate mod uniques")
                            .on_click(Box::new(move || {
                                // In Rust, we would need to implement this functionality
                                // self.auto_update_uniques(screen, mod_ruleset, replaceable_uniques);
                            }));

                        expander_tab.add(auto_update_button).row();
                    }
                }

                // Add error messages
                for (text, severity, _) in &mod_links {
                    let mut label = Text::new(text);
                    label.set_color(severity.color);
                    label.set_wrap(true);

                    expander_tab.add_label(label)
                        .width(self.screen.get_width() / 2.0)
                        .row();
                }

                // Add "Copy to clipboard" button if there are problems
                if !no_problem {
                    let copy_button = Button::new("Copy to clipboard")
                        .on_click(Box::new(move || {
                            // In Rust, we would need to implement this functionality
                            // let clipboard_text = mod_links.iter()
                            //     .map(|(text, _, _)| text)
                            //     .collect::<Vec<&str>>()
                            //     .join("\n");
                            // Gdx::app().clipboard().set_contents(clipboard_text);
                        }));

                    expander_tab.add(copy_button).row();
                }

                // Align the header to the left
                expander_tab.header_left();

                // Add the expander tab to the result table
                self.mod_check_result_table.add(expander_tab).row();
            }

            // Re-enable the base select
            if let Some(base_select) = &mut self.mod_check_base_select {
                base_select.set_disabled(false);
            }
        });
    }

    fn auto_update_uniques(&mut self, screen: &BaseScreen, mod_ruleset: &Ruleset, replaceable_uniques: HashMap<String, String>) {
        UniqueAutoUpdater::autoupdate_uniques(mod_ruleset, replaceable_uniques);

        let toast_text = "Uniques updated!";
        ToastPopup::new(toast_text, screen);

        self.run_mod_checker();
    }

    pub fn get_fixed_content(&self) -> &BaseScreen {
        &self.fixed_content
    }
}

impl TabbedPager for ModCheckTab {
    fn activated(&mut self, _index: i32, _caption: &str) {
        self.run_action();
    }

    fn deactivated(&mut self, _index: i32, _caption: &str) {
        // Nothing to do on deactivation
    }
}