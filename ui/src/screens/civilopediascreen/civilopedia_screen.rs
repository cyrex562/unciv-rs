use bevy::prelude::*;
use bevy_egui::egui::{self, Color32, Frame, Image, Layout, Rect, ScrollArea, Ui, Vec2};
use std::collections::{HashMap, LinkedHashMap};
use std::iter::FromIterator;

use crate::models::ruleset::{Ruleset, RulesetCache};
use crate::models::ruleset::unique::IHasUniques;
use crate::models::stats::INamed;
use crate::ui::components::input::{KeyboardBinding, KeyboardInput};
use crate::ui::images::{IconCircleGroup, ImageGetter};
use crate::ui::screens::basescreen::{BaseScreen, RecreateOnResize};
use crate::ui::screens::civilopediascreen::civilopedia_categories::CivilopediaCategories;
use crate::ui::screens::civilopediascreen::civilopedia_image_getters::CivilopediaImageGetters;
use crate::ui::screens::civilopediascreen::civilopedia_search_popup::CivilopediaSearchPopup;
use crate::ui::screens::tutorial::TutorialController;
use crate::ui::widgets::button::{Button, IconTextButton};
use crate::ui::widgets::frame::FrameBuilder;
use crate::ui::widgets::label::Label;
use crate::ui::widgets::scroll_pane::ScrollPane;
use crate::ui::widgets::split_pane::SplitPane;
use crate::ui::widgets::table::Table;
use crate::UncivGame;

/// Screen displaying the Civilopedia
///
/// # Arguments
///
/// * `ruleset` - Ruleset to display items from
/// * `category` - CivilopediaCategories key to select category
/// * `link` - Alternate selector to select category and/or entry. Can have the form `category/entry`
///           overriding the `category` parameter, or just `entry` to complement it.
pub struct CivilopediaScreen {
    ruleset: Ruleset,
    category_to_entries: LinkedHashMap<CivilopediaCategories, Vec<CivilopediaEntry>>,
    category_to_buttons: LinkedHashMap<CivilopediaCategories, CategoryButtonInfo>,
    entry_index: LinkedHashMap<String, CivilopediaEntry>,
    button_table_scroll: ScrollPane,
    entry_select_table: Table,
    entry_select_scroll: ScrollPane,
    flavour_table: Table,
    current_category: CivilopediaCategories,
    current_entry: String,
    current_entry_per_category: HashMap<CivilopediaCategories, String>,
    search_popup: Option<CivilopediaSearchPopup>,
    tutorial_controller: TutorialController,
}

/// Container collecting data per Civilopedia entry
///
/// # Arguments
///
/// * `name` - From Ruleset object INamed.name
/// * `image` - Icon for button
/// * `flavour` - ICivilopediaText
/// * `y` - Y coordinate for scrolling to
/// * `height` - Cell height
/// * `sort_by` - Optional, enabling overriding alphabetical order
struct CivilopediaEntry {
    name: String,
    image: Option<IconCircleGroup>,
    flavour: Option<Box<dyn ICivilopediaText>>,
    y: f32,              // coordinates of button cell used to scroll to entry
    height: f32,
    sort_by: i32,        // optional, enabling overriding alphabetical order
}

impl CivilopediaEntry {
    fn with_coordinates(&self, y: f32, height: f32) -> Self {
        CivilopediaEntry {
            name: self.name.clone(),
            image: self.image.clone(),
            flavour: self.flavour.clone(),
            y,
            height,
            sort_by: self.sort_by,
        }
    }
}

struct CategoryButtonInfo {
    button: Button,
    x: f32,
    width: f32,
}

impl CivilopediaScreen {
    /// Create a new CivilopediaScreen
    pub fn new(
        ruleset: Ruleset,
        category: CivilopediaCategories,
        link: String,
        tutorial_controller: TutorialController,
    ) -> Self {
        let mut screen = CivilopediaScreen {
            ruleset,
            category_to_entries: LinkedHashMap::new(),
            category_to_buttons: LinkedHashMap::new(),
            entry_index: LinkedHashMap::new(),
            button_table_scroll: ScrollPane::new(),
            entry_select_table: Table::new().with_defaults().with_padding(6.0).with_alignment(Layout::LEFT),
            entry_select_scroll: ScrollPane::new(),
            flavour_table: Table::new(),
            current_category: CivilopediaCategories::Tutorial,
            current_entry: String::new(),
            current_entry_per_category: HashMap::new(),
            search_popup: None,
            tutorial_controller,
        };

        screen.initialize(category, link);
        screen
    }

    /// Initialize the screen
    fn initialize(&mut self, category: CivilopediaCategories, link: String) {
        let image_size = 50.0;
        let religion_enabled = Self::show_religion_in_civilopedia(Some(&self.ruleset));

        // Do not confuse with IConstruction.shouldBeDisplayed - that one tests all prerequisites for building
        fn should_be_displayed(obj: &dyn ICivilopediaText, game: &UncivGame, ruleset: &Ruleset) -> bool {
            !(obj is IHasUniques) || !obj.is_hidden_from_civilopedia(&game.game_info, ruleset)
        }

        // Initialize entries for each category
        for loop_category in CivilopediaCategories::values() {
            if !religion_enabled && *loop_category == CivilopediaCategories::Belief {
                continue;
            }

            let entries = loop_category.get_category_iterator(&self.ruleset, &self.tutorial_controller)
                .into_iter()
                .filter(|obj| should_be_displayed(obj.as_ref(), &UncivGame::current(), &self.ruleset))
                .map(|obj| {
                    let name = obj.name().to_string();
                    let image = loop_category.get_image()
                        .and_then(|f| f(&obj.get_icon_name(), image_size));

                    CivilopediaEntry {
                        name,
                        image,
                        flavour: Some(obj),
                        y: 0.0,
                        height: 0.0,
                        sort_by: obj.get_sort_group(&self.ruleset),
                    }
                })
                .collect::<Vec<_>>();

            self.category_to_entries.insert(*loop_category, entries);
        }

        // Create button table
        let mut button_table = Table::new();
        button_table.with_padding(15.0);
        button_table.with_defaults().with_padding(10.0);

        let mut current_x = 10.0;  // = padLeft
        for (category_key, entries) in &self.category_to_entries {
            if entries.is_empty() {
                continue;
            }

            let icon = if !category_key.header_icon().is_empty() {
                ImageGetter::get_image(category_key.header_icon())
            } else {
                None
            };

            let mut button = IconTextButton::new(category_key.label(), icon);
            button.on_activation(category_key.binding(), move |_| {
                self.select_category(*category_key);
            });

            let cell = button_table.add(button);
            self.category_to_buttons.insert(*category_key, CategoryButtonInfo {
                button: cell.widget().clone(),
                x: current_x,
                width: cell.preferred_width(),
            });

            current_x += cell.preferred_width() + 20.0;
        }

        button_table.pack();
        self.button_table_scroll = ScrollPane::new().with_content(button_table);
        self.button_table_scroll.set_scrolling_disabled(false, true);

        // Create search button
        let mut search_button = ImageGetter::get_image_button(
            "OtherIcons/Search",
            image_size - 16.0,
            image_size,
            ImageGetter::skin_config().base_color,
            Color32::GOLD,
        );
        search_button.on_activation(KeyboardBinding::PediaSearch, move |_| {
            if let Some(popup) = &mut self.search_popup {
                popup.open(true);
            }
        });

        // Create close button
        let close_button = FrameBuilder::get_close_button(image_size, move |_| {
            UncivGame::current().pop_screen();
        });

        // Create top table
        let mut top_table = Table::new();
        top_table.add(self.button_table_scroll.clone()).grow_x();
        top_table.add(search_button).with_padding_left(10.0);
        top_table.add(close_button).with_padding_left(10.0).with_padding_right(10.0);
        top_table.set_width(UncivGame::current().stage().width());
        top_table.layout();

        // Create entry table
        let mut entry_table = Table::new();
        let mut split_pane = SplitPane::new(top_table, entry_table, true);
        split_pane.set_split_amount(top_table.preferred_height() / UncivGame::current().stage().height());
        entry_table.set_height(UncivGame::current().stage().height() - top_table.preferred_height());
        split_pane.set_fill_parent(true);

        UncivGame::current().stage().add_actor(split_pane);

        // Create entry select scroll
        self.entry_select_scroll = ScrollPane::new().with_content(self.entry_select_table.clone());
        self.entry_select_table.with_alignment(Layout::TOP);
        self.entry_select_scroll.set_overscroll(false, false);

        // Create description table
        let mut description_table = Table::new();
        description_table.add(self.flavour_table.clone()).with_padding_top(7.0).with_padding_bottom(5.0).row();

        let mut entry_split_pane = SplitPane::new(
            self.entry_select_scroll.clone(),
            ScrollPane::new().with_content(description_table),
            false,
        );
        entry_split_pane.set_split_amount(0.3);
        entry_table.add_actor(entry_split_pane);
        entry_split_pane.set_fill_parent(true);
        entry_split_pane.pack();  // ensure selectEntry has correct entrySelectScroll.height and maxY

        // Initialize search popup
        self.search_popup = Some(CivilopediaSearchPopup::new(
            self.clone(),
            self.tutorial_controller.clone(),
            move |link| {
                self.select_link(link);
            },
        ));

        // Handle link selection
        if link.is_empty() || !link.contains('/') {
            self.select_category(category);
        }

        // Show a default entry when opened without a target
        if link.is_empty() && category == CivilopediaCategories::Tutorial {
            self.select_default_entry();
        }

        if !link.is_empty() {
            if link.contains('/') {
                self.select_link(&link);
            } else {
                self.select_entry(&link, true);
            }
        }

        // Add keyboard shortcuts
        UncivGame::current().global_shortcuts().add(KeyboardInput::Left, move |_| {
            self.navigate_categories(-1);
        });
        UncivGame::current().global_shortcuts().add(KeyboardInput::Right, move |_| {
            self.navigate_categories(1);
        });
        UncivGame::current().global_shortcuts().add(KeyboardInput::Up, move |_| {
            self.navigate_entries(-1);
        });
        UncivGame::current().global_shortcuts().add(KeyboardInput::Down, move |_| {
            self.navigate_entries(1);
        });
        UncivGame::current().global_shortcuts().add(KeyboardInput::PageUp, move |_| {
            self.navigate_entries(-10);
        });
        UncivGame::current().global_shortcuts().add(KeyboardInput::PageDown, move |_| {
            self.navigate_entries(10);
        });
        UncivGame::current().global_shortcuts().add(KeyboardInput::Home, move |_| {
            self.navigate_entries(i32::MIN);
        });
        UncivGame::current().global_shortcuts().add(KeyboardInput::End, move |_| {
            self.navigate_entries(i32::MAX);
        });
    }

    /// Jump to a "link" selecting both category and entry
    ///
    /// Calls `select_category` with the substring before the first '/',
    /// and `select_entry` with the substring after the first '/'
    ///
    /// # Arguments
    ///
    /// * `link` - Link in the form Category/Entry
    fn select_link(&mut self, link: &str) {
        let parts: Vec<&str> = link.split('/').collect();
        if parts.is_empty() {
            return;
        }
        self.select_category(parts[0]);
        if parts.len() >= 2 {
            self.select_entry(parts[1], true);
        }
    }

    /// Select a specified category
    ///
    /// # Arguments
    ///
    /// * `name` - Category name or label
    fn select_category(&mut self, name: &str) {
        if let Some(category) = CivilopediaCategories::from_link(name) {
            self.select_category_by_enum(category);
        }
    }

    /// Select a specified category - unselects entry, rebuilds left side buttons.
    ///
    /// # Arguments
    ///
    /// * `category` - Category key
    fn select_category_by_enum(&mut self, category: CivilopediaCategories) {
        self.current_category = category;
        self.entry_select_table.clear();
        self.entry_index.clear();
        self.flavour_table.clear();

        // Update button colors
        for button_info in self.category_to_buttons.values() {
            button_info.button.set_color(Color32::WHITE);
        }

        let button_info = match self.category_to_buttons.get(&category) {
            Some(info) => info,
            None => return, // defense against being passed a bad selector
        };
        button_info.button.set_color(Color32::BLUE);
        self.button_table_scroll.set_scroll_x(
            button_info.x + (button_info.width - self.button_table_scroll.width()) / 2.0
        );

        // Get entries for the category
        let entries = match self.category_to_entries.get(&category) {
            Some(entries) => entries,
            None => return, // defense, allowing buggy panes to remain empty while others work
        };

        let mut sorted_entries = entries.clone();
        if category != CivilopediaCategories::Difficulty {
            // Alphabetical order of localized names, using system default locale
            sorted_entries.sort_by(|a, b| {
                a.sort_by.cmp(&b.sort_by).then(
                    UncivGame::current().settings().get_collator_from_locale().compare(
                        &a.name.tr(true, true),
                        &b.name.tr(true, true)
                    )
                )
            });
        }

        let mut current_y = -1.0;

        // Create entry buttons
        for entry in sorted_entries {
            let mut entry_button = Table::new();
            entry_button.set_background(
                UncivGame::current().skin_strings().get_ui_background(
                    "CivilopediaScreen/EntryButton",
                    Color32::from_rgb(50, 75, 125)
                )
            );
            entry_button.set_touchable(true);

            if let Some(image) = &entry.image {
                if category == CivilopediaCategories::Terrain {
                    entry_button.add(image.clone()).with_padding_left(20.0).with_padding_right(10.0);
                } else {
                    entry_button.add(image.clone()).with_padding_left(10.0);
                }
            }

            let mut label = Label::new(&entry.name)
                .with_color(Color32::WHITE)
                .with_font_size(25)
                .with_hide_icons(true);

            entry_button.with_alignment(Layout::LEFT).add(label).with_padding(10.0);
            entry_button.on_click(move |_| {
                self.select_entry_by_struct(&entry);
            });
            entry_button.set_name(&entry.name); // make button findable

            let cell = self.entry_select_table.add(entry_button).with_height(75.0).expand_x().fill_x();
            self.entry_select_table.row();

            if current_y < 0.0 {
                current_y = cell.padding_top();
            }

            self.entry_index.insert(
                entry.name.clone(),
                entry.with_coordinates(current_y, cell.preferred_height())
            );

            current_y += cell.padding_bottom() + cell.preferred_height() + cell.padding_top();
        }

        self.entry_select_scroll.layout(); // necessary for positioning in selectRow to work

        // Select the current entry for this category if it exists
        if let Some(entry) = self.current_entry_per_category.get(&category) {
            self.select_entry(entry, false);
        }
    }

    /// Select a specified entry within the current category. Unknown strings are ignored!
    ///
    /// # Arguments
    ///
    /// * `name` - Entry (Ruleset object) name
    /// * `no_scroll_animation` - Disable scroll animation
    fn select_entry(&mut self, name: &str, no_scroll_animation: bool) {
        let entry = match self.entry_index.get(name) {
            Some(entry) => entry,
            None => return,
        };

        // Scroll to the entry
        self.entry_select_scroll.set_scroll_y(
            entry.y + (entry.height - self.entry_select_scroll.height()) / 2.0
        );

        if no_scroll_animation {
            self.entry_select_scroll.update_visual_scroll(); // snap without animation on fresh pedia open
        }

        self.select_entry_by_struct(entry);
    }

    /// Select a specified entry
    ///
    /// # Arguments
    ///
    /// * `entry` - The entry to select
    fn select_entry_by_struct(&mut self, entry: &CivilopediaEntry) {
        self.current_entry = entry.name.clone();
        self.current_entry_per_category.insert(self.current_category, entry.name.clone());
        self.flavour_table.clear();

        if let Some(flavour) = &entry.flavour {
            self.flavour_table.set_visible(true);
            let text = flavour.assemble_civilopedia_text(&self.ruleset)
                .render_civilopedia_text(UncivGame::current().stage().width() * 0.5, |link| {
                    self.select_link(link);
                });
            self.flavour_table.add(text);
        } else {
            self.flavour_table.set_visible(false);
        }

        // Update button colors
        for child in self.entry_select_table.children() {
            if child.name() == entry.name {
                child.set_color(Color32::BLUE);
            } else {
                child.set_color(Color32::WHITE);
            }
        }
    }

    /// Select the default entry
    fn select_default_entry(&mut self) {
        let name = self.ruleset.mods.iter()
            .filter(|mod_name| {
                if let Some(cache) = RulesetCache::get(mod_name) {
                    cache.mod_options.is_base_ruleset
                } else {
                    false
                }
            })
            .chain(std::iter::once("Civilopedia"))
            .find(|name| self.entry_index.contains_key(*name))
            .cloned();

        if let Some(name) = name {
            self.select_entry(&name, true);
        }
    }

    /// Navigate between categories
    ///
    /// # Arguments
    ///
    /// * `direction` - Direction to navigate (-1 for previous, 1 for next)
    fn navigate_categories(&mut self, direction: i32) {
        let category_keys: Vec<CivilopediaCategories> = self.category_to_entries.keys().cloned().collect();
        let current_index = category_keys.iter().position(|&c| c == self.current_category).unwrap_or(0);
        let new_index = (current_index as i32 + category_keys.len() as i32 + direction) as usize % category_keys.len();
        self.select_category_by_enum(category_keys[new_index]);
    }

    /// Navigate between entries
    ///
    /// # Arguments
    ///
    /// * `direction` - Direction to navigate (-1 for previous, 1 for next)
    fn navigate_entries(&mut self, direction: i32) {
        // This is abusing a Map as Array - there must be a collection allowing both easy positional and associative access
        let entry_keys: Vec<String> = self.entry_index.keys().cloned().collect();
        let index = entry_keys.iter().position(|k| k == &self.current_entry).unwrap_or(0);

        let new_index = match direction {
            i32::MIN => 0,
            i32::MAX => entry_keys.len() - 1,
            _ => (index as i32 + entry_keys.len() as i32 + direction) as usize % entry_keys.len(),
        };

        if let Some(key) = entry_keys.get(new_index) {
            self.select_entry(key, false);
        }
    }

    /// Test whether to show Religion-specific items, does not require a game to be running
    /// - Do not make public - use IHasUniques.isHiddenFromCivilopedia if possible!
    fn show_religion_in_civilopedia(ruleset: Option<&Ruleset>) -> bool {
        if let Some(game_info) = UncivGame::get_game_info_or_null() {
            game_info.is_religion_enabled()
        } else if let Some(ruleset) = ruleset {
            !ruleset.beliefs.is_empty()
        } else {
            true
        }
    }
}

impl BaseScreen for CivilopediaScreen {
    fn recreate(&self) -> Box<dyn BaseScreen> {
        Box::new(CivilopediaScreen::new(
            self.ruleset.clone(),
            self.current_category,
            self.current_entry.clone(),
            self.tutorial_controller.clone(),
        ))
    }
}

impl Clone for CivilopediaScreen {
    fn clone(&self) -> Self {
        CivilopediaScreen::new(
            self.ruleset.clone(),
            self.current_category,
            self.current_entry.clone(),
            self.tutorial_controller.clone(),
        )
    }
}

/// Trait for objects that can be displayed in the Civilopedia
pub trait ICivilopediaText: Send + Sync {
    /// Get the name of the object
    fn name(&self) -> &str;

    /// Get the icon name for the object
    fn get_icon_name(&self) -> String;

    /// Get the sort group for the object
    fn get_sort_group(&self, ruleset: &Ruleset) -> i32;

    /// Assemble the civilopedia text for the object
    fn assemble_civilopedia_text(&self, ruleset: &Ruleset) -> CivilopediaText;

    /// Check if the object is hidden from the civilopedia
    fn is_hidden_from_civilopedia(&self, game_info: &crate::models::game_info::GameInfo, ruleset: &Ruleset) -> bool;
}

/// Represents formatted text for the Civilopedia
pub struct CivilopediaText {
    text: String,
}

impl CivilopediaText {
    /// Render the civilopedia text
    pub fn render_civilopedia_text(&self, width: f32, link_handler: impl Fn(&str) + Send + Sync + 'static) -> egui::Widget {
        // Implementation would go here
        // This is a placeholder for the actual implementation
        egui::Label::new(&self.text).into()
    }
}