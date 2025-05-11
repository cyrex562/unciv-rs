use bevy::prelude::*;
use bevy_egui::egui::{self, Align, Color32, Frame, Layout, Rect, ScrollArea, Ui, Vec2};
use std::collections::HashSet;
use std::sync::Arc;
use regex::Regex;

use crate::models::ruleset::{Ruleset, RulesetCache};
use crate::models::stats::INamed;
use crate::ui::components::input::{KeyboardBinding, KeyboardInput};
use crate::ui::components::widgets::{Button, ExpanderTab, SelectBox, TextField};
use crate::ui::popups::{Popup, ToastPopup};
use crate::ui::screens::basescreen::{BaseScreen, TutorialController};
use crate::ui::screens::civilopediascreen::civilopedia_categories::CivilopediaCategories;
use crate::ui::screens::civilopediascreen::civilopedia_screen::CivilopediaScreen;
use crate::ui::widgets::label::Label;
use crate::utils::concurrency::{Concurrency, Job};
use crate::utils::formatted_line::FormattedLine;
use crate::utils::launch_on_gl_thread;
use crate::UncivGame;

/// Popup for searching the Civilopedia
pub struct CivilopediaSearchPopup {
    pedia_screen: Arc<CivilopediaScreen>,
    tutorial_controller: Arc<TutorialController>,
    link_action: Arc<dyn Fn(String) + Send + Sync>,
    ruleset: Ruleset,
    search_text: TextField,
    mod_select: ModSelectBox,
    result_expander: Option<ExpanderTab>,
    result_cell: Option<egui::Frame>,
    search_button: Button,
    search_job: Option<Job>,
    check_line: Arc<dyn Fn(&str) -> bool + Send + Sync>,
}

impl CivilopediaSearchPopup {
    /// Create a new CivilopediaSearchPopup
    pub fn new(
        pedia_screen: Arc<CivilopediaScreen>,
        tutorial_controller: Arc<TutorialController>,
        link_action: Arc<dyn Fn(String) + Send + Sync>,
    ) -> Self {
        let mut popup = CivilopediaSearchPopup {
            pedia_screen: pedia_screen.clone(),
            tutorial_controller,
            link_action,
            ruleset: pedia_screen.ruleset.clone(),
            search_text: TextField::new(""),
            mod_select: ModSelectBox::new(pedia_screen.clone()),
            result_expander: None,
            result_cell: None,
            search_button: Button::new("Search!", KeyboardBinding::Return),
            search_job: None,
            check_line: Arc::new(|_| false),
        };

        popup.initialize();
        popup
    }

    /// Initialize the popup
    fn initialize(&mut self) {
        self.search_text.set_max_length(100);

        // Create the UI layout
        let mut frame = Frame::new();
        frame.set_fill(true);
        frame.set_padding(10.0);

        // Add search text field
        frame.add(Label::new("Search text:"));
        frame.add(self.search_text.clone()).grow_x().row();

        // Add mod filter
        frame.add(Label::new("Mod filter:"));
        frame.add(self.mod_select.clone()).grow_x().row();

        // Add result area
        let result_frame = Frame::new();
        result_frame.set_fill(true);
        self.result_cell = Some(result_frame);
        frame.add(result_frame).grow_x().row();

        // Add search button
        self.search_button.on_click(move |_| {
            self.start_search(self.search_text.text());
        });
        frame.add(self.search_button.clone());

        // Add close button
        let close_button = Button::new("Close", KeyboardBinding::Escape);
        close_button.on_click(move |_| {
            self.close();
        });
        frame.add(close_button);

        // Set up show and close listeners
        self.show_listeners.push(Box::new(move || {
            self.keyboard_focus = Some(self.search_text.clone());
            self.search_text.select_all();
        }));

        self.close_listeners.push(Box::new(move || {
            if self.is_search_running() {
                if let Some(job) = &self.search_job {
                    job.cancel();
                }
            }
        }));
    }

    /// Check if a search is currently running
    fn is_search_running(&self) -> bool {
        self.search_job.as_ref().map_or(false, |job| job.is_active())
    }

    /// Start a search with the given text
    fn start_search(&mut self, text: String) {
        self.search_button.set_enabled(false);

        // Set up the check_line function based on the search text
        if text.is_empty() {
            self.check_line = Arc::new(|_| true);
        } else if text.contains(".*") || text.contains('\\') || text.contains('|') {
            match Regex::new(&format!("(?i){}", text)) {
                Ok(regex) => {
                    let regex = Arc::new(regex);
                    self.check_line = Arc::new(move |line| regex.is_match(line));
                }
                Err(_) => {
                    ToastPopup::new("Invalid regular expression", self.pedia_screen.clone(), 4000);
                    self.search_button.set_enabled(true);
                    return;
                }
            }
        } else {
            let words: HashSet<String> = text.split(' ')
                .map(|s| s.to_lowercase())
                .collect();

            self.check_line = Arc::new(move |line| {
                let line_lower = line.to_lowercase();
                words.iter().all(|word| line_lower.contains(word))
            });
        }

        // Update ruleset based on selected mod
        self.ruleset = self.mod_select.selected_ruleset();

        // Clear or create result expander
        if let Some(expander) = &mut self.result_expander {
            expander.inner_table.clear();
        } else {
            let mut expander = ExpanderTab::new("Results");
            expander.inner_table.defaults().grow_x().pad(2.0);
            self.result_expander = Some(expander);

            if let Some(cell) = &mut self.result_cell {
                cell.set_content(expander.clone());
            }
        }

        // Start search job
        let check_line = self.check_line.clone();
        let pedia_screen = self.pedia_screen.clone();
        let tutorial_controller = self.tutorial_controller.clone();
        let result_expander = self.result_expander.clone();
        let link_action = self.link_action.clone();

        self.search_job = Some(Concurrency::run("PediaSearch", move || {
            Self::search_loop(
                &pedia_screen,
                &tutorial_controller,
                &check_line,
                &result_expander,
                &link_action,
            );
        }));

        // Set up completion handler
        if let Some(job) = &self.search_job {
            job.invoke_on_completion(move || {
                self.search_job = None;
                launch_on_gl_thread(move || {
                    self.finish_search();
                });
            });
        }
    }

    /// Search loop that iterates through categories and entries
    fn search_loop(
        pedia_screen: &Arc<CivilopediaScreen>,
        tutorial_controller: &Arc<TutorialController>,
        check_line: &Arc<dyn Fn(&str) -> bool + Send + Sync>,
        result_expander: &Option<ExpanderTab>,
        link_action: &Arc<dyn Fn(String) + Send + Sync>,
    ) {
        for category in CivilopediaCategories::values() {
            if !Job::is_active() {
                break;
            }

            if !pedia_screen.ruleset.mod_options.is_base_ruleset && category == CivilopediaCategories::Tutorial {
                continue; // Search tutorials only when the mod filter is a base ruleset
            }

            for entry in category.get_category_iterator(&pedia_screen.ruleset, tutorial_controller) {
                if !Job::is_active() {
                    break;
                }

                if !entry.is_named() {
                    continue;
                }

                if !pedia_screen.ruleset.mod_options.is_base_ruleset {
                    let sort = entry.get_sort_group(&pedia_screen.ruleset);
                    if category == CivilopediaCategories::UnitType && sort < 2 {
                        continue; // Search "Domain:" entries only when the mod filter is a base ruleset
                    }
                    if category == CivilopediaCategories::Belief && sort == 0 {
                        continue; // Search "Religions" from `get_civilopedia_religion_entry` only when the mod filter is a base ruleset
                    }
                }

                Self::search_entry(entry, check_line, result_expander, link_action);
            }
        }
    }

    /// Search a single entry for matches
    fn search_entry(
        entry: Box<dyn ICivilopediaText>,
        check_line: &Arc<dyn Fn(&str) -> bool + Send + Sync>,
        result_expander: &Option<ExpanderTab>,
        link_action: &Arc<dyn Fn(String) + Send + Sync>,
    ) {
        // Get all text lines from the entry
        let mut lines = Vec::new();

        if let Some(header) = entry.get_civilopedia_text_header() {
            lines.push(header);
        }

        lines.extend(entry.civilopedia_text());
        lines.extend(entry.get_civilopedia_text_lines(&pedia_screen.ruleset));

        // Check each line for matches
        for line in lines {
            if !Job::is_active() {
                break;
            }

            let line_text = line.text().tr(true);
            if !check_line(&line_text) {
                continue;
            }

            Self::add_result(entry.clone(), result_expander, link_action);
            break;
        }
    }

    /// Add a result to the result expander
    fn add_result(
        entry: Box<dyn ICivilopediaText>,
        result_expander: &Option<ExpanderTab>,
        link_action: &Arc<dyn Fn(String) + Send + Sync>,
    ) {
        launch_on_gl_thread(move || {
            if let Some(expander) = result_expander {
                let icon_name = entry.get_icon_name();
                let link = entry.make_link();

                let mut label = Label::new(&icon_name);
                label.set_alignment(Align::LEFT);

                expander.inner_table.add(label).row();

                label.on_click(move |_| {
                    link_action(link.clone());
                    self.close();
                });
            }
        });
    }

    /// Finish the search and update the UI
    fn finish_search(&mut self) {
        self.search_button.set_enabled(true);

        if let Some(expander) = &self.result_expander {
            if expander.inner_table.is_empty() {
                let nothing_found = FormattedLine::new("Nothing found!")
                    .with_color(Color32::from_rgb(245, 51, 51))
                    .with_header(3)
                    .with_centered(true)
                    .render(0.0);

                expander.inner_table.add(nothing_found);
            }
        }
    }
}

/// Entry for the mod select box
struct ModSelectEntry {
    key: String,
    translate: bool,
}

impl ModSelectEntry {
    /// Create a new ModSelectEntry
    fn new(key: String, translate: bool) -> Self {
        ModSelectEntry { key, translate }
    }

    /// Get the display string for the entry
    fn to_string(&self) -> String {
        if self.translate {
            self.key.tr()
        } else {
            self.key.clone()
        }
    }
}

/// Select box for choosing a mod
struct ModSelectBox {
    pedia_screen: Arc<CivilopediaScreen>,
    entries: Vec<ModSelectEntry>,
    selected_index: usize,
}

impl ModSelectBox {
    /// Create a new ModSelectBox
    fn new(pedia_screen: Arc<CivilopediaScreen>) -> Self {
        let mut entries = Vec::new();
        entries.push(ModSelectEntry::new("-Combined-".to_string(), true));

        // Add entries for each mod
        let mods = pedia_screen.ruleset.mods.clone();
        for mod_name in mods.intersect(RulesetCache::keys()) {
            entries.push(ModSelectEntry::new(mod_name, false));
        }

        ModSelectBox {
            pedia_screen,
            entries,
            selected_index: 0,
        }
    }

    /// Get the selected ruleset
    fn selected_ruleset(&self) -> Ruleset {
        if self.selected_index == 0 {
            self.pedia_screen.ruleset.clone()
        } else {
            let selected = &self.entries[self.selected_index];
            RulesetCache::get(&selected.key).unwrap()
        }
    }
}

impl SelectBox for ModSelectBox {
    fn items(&self) -> &[ModSelectEntry] {
        &self.entries
    }

    fn selected_index(&self) -> usize {
        self.selected_index
    }

    fn set_selected_index(&mut self, index: usize) {
        self.selected_index = index;
    }

    fn selected(&self) -> &ModSelectEntry {
        &self.entries[self.selected_index]
    }
}