use std::collections::{HashMap, HashSet, LinkedHashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use egui::{Color32, Ui, Align};
use egui_extras::Size;
use uuid::Uuid;

use crate::game::UncivGame;
use crate::logic::civilization::PlayerType;
use crate::files::MapSaver;
use crate::logic::map::{MapParameters, TileMap};
use crate::models::metadata::Player;
use crate::models::ruleset::{Ruleset, RulesetCache};
use crate::models::ruleset::nation::Nation;
use crate::models::ruleset::unique::UniqueType;
use crate::ui::components::widgets::{LoadingImage, WrappableLabel};
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::newgamescreen::NewGameScreen;
use crate::ui::screens::victoryscreen::LoadMapPreview;
use crate::utils::concurrency::Concurrency;
use crate::utils::translations::tr;

/// Wrapper for a map file to display in the UI
struct MapWrapper {
    file_path: PathBuf,
    map_preview: TileMap::Preview,
}

impl MapWrapper {
    /// Create a new MapWrapper
    fn new(file_path: PathBuf, map_preview: TileMap::Preview) -> Self {
        Self {
            file_path,
            map_preview,
        }
    }

    /// Get the file name
    fn name(&self) -> String {
        self.file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    }

    /// Get the category name (mod name or base ruleset)
    fn get_category_name(&self) -> String {
        if let Some(parent) = self.file_path.parent().and_then(|p| p.parent()) {
            if let Some(name) = parent.file_name().and_then(|n| n.to_str()) {
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
        self.map_preview.map_parameters.base_ruleset.clone()
    }
}

impl std::fmt::Display for MapWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Table for selecting map files
pub struct MapFileSelectTable {
    new_game_screen: Arc<NewGameScreen>,
    map_parameters: MapParameters,
    map_category_select_box: egui::ComboBox,
    map_file_select_box: egui::ComboBox,
    loading_icon: LoadingImage,
    use_nations_from_map_button: egui::Button,
    map_nations: Vec<Nation>,
    map_human_pick: Option<String>,
    mini_map_wrapper: egui::Frame,
    map_preview_job: Option<Uuid>,
    preselected_name: String,
    description_label: String,
    map_wrappers: Vec<MapWrapper>,
    column_width: f32,
    is_activated: bool,
}

impl MapFileSelectTable {
    /// Create a new MapFileSelectTable
    pub fn new(new_game_screen: Arc<NewGameScreen>, map_parameters: MapParameters) -> Self {
        let column_width = new_game_screen.get_column_width();

        Self {
            new_game_screen,
            map_parameters,
            map_category_select_box: egui::ComboBox::new("map_category", ""),
            map_file_select_box: egui::ComboBox::new("map_file", ""),
            loading_icon: LoadingImage::new(30.0, Color32::RED),
            use_nations_from_map_button: egui::Button::new("Select players from starting locations"),
            map_nations: Vec::new(),
            map_human_pick: None,
            mini_map_wrapper: egui::Frame::none(),
            map_preview_job: None,
            preselected_name: map_parameters.name.clone(),
            description_label: String::new(),
            map_wrappers: Vec::new(),
            column_width,
            is_activated: false,
        }
    }

    /// Get the sequence of map files
    fn get_map_files_sequence(&self) -> Vec<PathBuf> {
        let mut map_files = MapSaver::get_maps();

        for ruleset in RulesetCache::values() {
            if let Some(folder_location) = &ruleset.folder_location {
                let maps_folder = folder_location.join(MapSaver::MAPS_FOLDER);
                if maps_folder.exists() {
                    if let Ok(entries) = std::fs::read_dir(&maps_folder) {
                        for entry in entries.flatten() {
                            if let Ok(path) = entry.path().canonicalize() {
                                map_files.push(path);
                            }
                        }
                    }
                }
            }
        }

        // Sort by last modified time (descending)
        map_files.sort_by(|a, b| {
            let a_time = a.metadata().and_then(|m| m.modified()).unwrap_or(UNIX_EPOCH);
            let b_time = b.metadata().and_then(|m| m.modified()).unwrap_or(UNIX_EPOCH);
            b_time.cmp(&a_time)
        });

        map_files
    }

    /// Add map wrappers asynchronously
    fn add_map_wrappers_async(&mut this) {
        let map_files = this.get_map_files_sequence();

        this.loading_icon.show();

        let new_game_screen = this.new_game_screen.clone();
        let map_wrappers = &mut this.map_wrappers;
        let map_category_select_box = &mut this.map_category_select_box;
        let map_file_select_box = &mut this.map_file_select_box;
        let loading_icon = &mut this.loading_icon;
        let column_width = this.column_width;
        let preselected_name = this.preselected_name.clone();
        let base_ruleset = this.new_game_screen.game_setup_info().game_parameters.base_ruleset.clone();

        Concurrency::run(move || {
            for file_path in map_files {
                if let Ok(map_preview) = MapSaver::load_map_preview(&file_path) {
                    let map_wrapper = MapWrapper::new(file_path, map_preview);
                    map_wrappers.push(map_wrapper.clone());

                    Concurrency::run_on_gl_thread(move || {
                        Self::add_async_entry_to_select_boxes(
                            map_wrapper,
                            map_category_select_box,
                            map_file_select_box,
                            &base_ruleset,
                            &preselected_name,
                            column_width,
                        );
                    });
                }
            }

            Concurrency::run_on_gl_thread(move || {
                loading_icon.hide();
                // Re-sort lower SelectBox, and trigger map selection
                Self::on_category_select_box_change(
                    map_category_select_box,
                    map_file_select_box,
                    map_wrappers,
                    &base_ruleset,
                    &preselected_name,
                    column_width,
                );
            });
        });
    }

    /// Add an async entry to the select boxes
    fn add_async_entry_to_select_boxes(
        map_wrapper: MapWrapper,
        map_category_select_box: &mut egui::ComboBox,
        map_file_select_box: &mut egui::ComboBox,
        base_ruleset: &str,
        preselected_name: &str,
        column_width: f32,
    ) {
        let category_name = map_wrapper.get_category_name();

        if !map_category_select_box.items().contains(&category_name) {
            let mut items = map_category_select_box.items().clone();
            items.push(category_name.clone());

            // Sort items
            items.sort_by(|a, b| {
                if a == base_ruleset {
                    std::cmp::Ordering::Less
                } else if b == base_ruleset {
                    std::cmp::Ordering::Greater
                } else {
                    a.cmp(b)
                }
            });

            map_category_select_box.set_items(items);

            if map_category_select_box.selected().is_empty() {
                map_category_select_box.set_selected(category_name.clone());
            }
        }

        if map_category_select_box.selected() == category_name {
            let mut items = map_file_select_box.items().clone();
            items.push(map_wrapper.clone());
            map_file_select_box.set_items(items);
        }
    }

    /// Get the first map
    fn first_map(&self) -> Option<PathBuf> {
        for file_path in self.get_map_files_sequence() {
            if let Ok(_) = MapSaver::load_map_parameters(&file_path) {
                return Some(file_path);
            }
        }
        None
    }

    /// Check if a file was recently modified
    fn is_recently_modified(file_path: &PathBuf) -> bool {
        if let Ok(metadata) = file_path.metadata() {
            if let Ok(modified) = metadata.modified() {
                if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) {
                    let modified_secs = modified.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                    let now_secs = now.as_secs();
                    return now_secs - modified_secs < 900; // 900s = quarter hour
                }
            }
        }
        false
    }

    /// Check if the table is not empty
    pub fn is_not_empty(&self) -> bool {
        self.first_map().is_some()
    }

    /// Activate custom maps
    pub fn activate_custom_maps(&mut this) {
        if this.is_activated {
            if this.loading_icon.is_showing() {
                return; // Default map selection will be handled when background loading finishes
            }
            Self::on_category_select_box_change(
                &mut this.map_category_select_box,
                &mut this.map_file_select_box,
                &mut this.map_wrappers,
                &this.new_game_screen.game_setup_info().game_parameters.base_ruleset,
                &this.preselected_name,
                this.column_width,
            );
        }

        // Code to only run once per NewGameScreen lifetime
        this.is_activated = true;
        this.preselected_name = this.map_parameters.name.clone();

        this.add_map_wrappers_async();
    }

    /// Handle category select box change
    fn on_category_select_box_change(
        map_category_select_box: &mut egui::ComboBox,
        map_file_select_box: &mut egui::ComboBox,
        map_wrappers: &mut Vec<MapWrapper>,
        base_ruleset: &str,
        preselected_name: &str,
        column_width: f32,
    ) {
        let selected_ruleset = map_category_select_box.selected();

        let mut map_files: Vec<MapWrapper> = map_wrappers
            .iter()
            .filter(|w| w.get_category_name() == selected_ruleset)
            .cloned()
            .collect();

        map_files.sort_by(|a, b| a.name().cmp(&b.name()));

        fn get_preselect(
            map_files: &[MapWrapper],
            selected_ruleset: &str,
            preselected_name: &str,
        ) -> Option<MapWrapper> {
            if map_files.is_empty() {
                return None;
            }

            // Check for mod option preselect
            if let Some(ruleset) = RulesetCache::get(selected_ruleset) {
                if let Some(unique) = ruleset.mod_options.get_matching_uniques(UniqueType::ModMapPreselection).first() {
                    if let Some(param) = unique.params.get(0) {
                        if let Some(preselect_file) = map_files.iter().find(|w| w.name() == *param) {
                            return Some(preselect_file.clone());
                        }
                    }
                }
            }

            // Check for recently modified files
            let recent = map_files.iter()
                .filter(|w| Self::is_recently_modified(&w.file_path))
                .max_by_key(|w| {
                    w.file_path.metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(UNIX_EPOCH)
                });

            let oldest_timestamp = map_files.iter()
                .filter_map(|w| {
                    w.file_path.metadata()
                        .and_then(|m| m.modified())
                        .ok()
                })
                .min()
                .unwrap_or(UNIX_EPOCH);

            if let Some(recent) = recent {
                let recent_time = recent.file_path.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(UNIX_EPOCH);

                if recent_time.duration_since(oldest_timestamp).unwrap_or_default().as_secs() > 0
                    || map_files.len() == 1 {
                    return Some(recent.clone());
                }
            }

            // Check for named file
            if let Some(named) = map_files.iter().find(|w| w.name() == preselected_name) {
                return Some(named.clone());
            }

            // Default to first file
            map_files.first().cloned()
        }

        let selected_item = get_preselect(&map_files, &selected_ruleset, preselected_name);

        map_file_select_box.set_items(map_files);
        if let Some(item) = selected_item {
            map_file_select_box.set_selected(item);
        }

        // Always run on_file_select_box_change
        Self::on_file_select_box_change(
            map_file_select_box,
            map_wrappers,
            &this.new_game_screen,
            &mut this.map_parameters,
            &mut this.map_nations,
            &mut this.map_human_pick,
            &mut this.use_nations_from_map_button,
            &mut this.description_label,
            &mut this.mini_map_wrapper,
            this.column_width,
        );
    }

    /// Handle file select box change
    fn on_file_select_box_change(
        map_file_select_box: &mut egui::ComboBox,
        map_wrappers: &mut Vec<MapWrapper>,
        new_game_screen: &Arc<NewGameScreen>,
        map_parameters: &mut MapParameters,
        map_nations: &mut Vec<Nation>,
        map_human_pick: &mut Option<String>,
        use_nations_from_map_button: &mut egui::Button,
        description_label: &mut String,
        mini_map_wrapper: &mut egui::Frame,
        column_width: f32,
    ) {
        Self::cancel_background_jobs(map_preview_job);

        if map_file_select_box.selected().is_empty() {
            return;
        }

        let selection = map_file_select_box.selected();

        let map_mods = selection.map_preview.map_parameters.mods
            .iter()
            .partition::<Vec<_>, _>(|m| {
                RulesetCache::get(m)
                    .map(|r| r.mod_options.is_base_ruleset)
                    .unwrap_or(false)
            });

        let mut game_parameters = new_game_screen.game_setup_info().game_parameters.clone();
        game_parameters.mods = LinkedHashSet::from_iter(map_mods.1.iter().cloned());
        game_parameters.base_ruleset = map_mods.0.first()
            .cloned()
            .unwrap_or_else(|| selection.map_preview.map_parameters.base_ruleset.clone());

        let success = new_game_screen.try_update_ruleset(true);

        if success {
            *map_nations = selection.map_preview.get_declared_nations()
                .iter()
                .filter_map(|n| new_game_screen.ruleset().nations.get(n))
                .filter(|n| n.is_major_civ)
                .cloned()
                .collect();

            let human_nations: Vec<String> = selection.map_preview.get_nations_for_human_player()
                .iter()
                .filter(|n| {
                    new_game_screen.ruleset().nations.get(n)
                        .map(|n| n.is_major_civ)
                        .unwrap_or(false)
                })
                .cloned()
                .collect();

            *map_human_pick = human_nations.choose(&mut rand::thread_rng()).cloned();
        } else {
            *map_nations = Vec::new();
            *map_human_pick = None;
        }

        if map_nations.is_empty() {
            use_nations_from_map_button.set_enabled(false);
        } else {
            use_nations_from_map_button.set_enabled(true);
        }

        let map_file = &selection.file_path;
        map_parameters.name = selection.name();
        new_game_screen.set_map_file(map_file.clone());

        new_game_screen.update_tables();
        Self::hide_mini_map(mini_map_wrapper);

        if success {
            Self::start_map_preview(
                map_file,
                new_game_screen,
                description_label,
                mini_map_wrapper,
                column_width,
            );
        } else {
            // Mod error - the options have been reset by update_ruleset
            let mut items = map_file_select_box.items().clone();
            if let Some(index) = map_file_select_box.selected_index() {
                items.remove(index);
            }
            map_file_select_box.set_items(items);
            // This will have triggered a nested on_file_select_box_change!
        }
    }

    /// Start map preview
    fn start_map_preview(
        map_file: &PathBuf,
        new_game_screen: &Arc<NewGameScreen>,
        description_label: &mut String,
        mini_map_wrapper: &mut egui::Frame,
        column_width: f32,
    ) {
        let map_file = map_file.clone();
        let new_game_screen = new_game_screen.clone();
        let description_label = description_label.clone();
        let mini_map_wrapper = mini_map_wrapper.clone();

        let job_id = Concurrency::run(move || {
            if let Ok(map) = MapSaver::load_map(&map_file) {
                map.set_transients(new_game_screen.ruleset(), false);

                // ReplayMap still paints outside its bounds - so we subtract padding and a little extra
                let size = (column_width - 40.0).min(500.0);
                let mini_map = LoadMapPreview::new(&map, size, size);
                let description = map.description.clone();

                Concurrency::run_on_gl_thread(move || {
                    *description_label = description;
                    Self::show_minimap(mini_map_wrapper, mini_map);
                });
            }
        });

        *map_preview_job = Some(job_id);
    }

    /// Cancel background jobs
    pub fn cancel_background_jobs(map_preview_job: &mut Option<Uuid>) {
        if let Some(job_id) = *map_preview_job {
            Concurrency::cancel(job_id);
            *map_preview_job = None;
        }
    }

    /// Show minimap
    fn show_minimap(mini_map_wrapper: &mut egui::Frame, mini_map: LoadMapPreview) {
        mini_map_wrapper.set_visible(true);
        mini_map_wrapper.set_content(mini_map);
    }

    /// Show description
    fn show_description(description_label: &mut String, text: String) {
        *description_label = text;
    }

    /// Hide minimap
    fn hide_mini_map(mini_map_wrapper: &mut egui::Frame) {
        mini_map_wrapper.set_visible(false);
    }

    /// Handle use nations from map
    fn on_use_nations_from_map(
        &mut this,
        use_nations_from_map_button: &mut egui::Button,
    ) {
        use_nations_from_map_button.set_enabled(false);

        let mut players = this.new_game_screen.player_picker_table().game_parameters().players.clone();
        players.clear();

        let mut nation_pairs: Vec<(String, String)> = this.map_nations
            .iter()
            .map(|n| (n.name.clone(), tr(&n.name)))
            .collect();

        // Sort by translation but keep untranslated name
        nation_pairs.sort_by(|a, b| {
            if a.0 == this.map_human_pick {
                std::cmp::Ordering::Less
            } else if b.0 == this.map_human_pick {
                std::cmp::Ordering::Greater
            } else {
                a.1.cmp(&b.1)
            }
        });

        for (name, _) in nation_pairs {
            let player_type = if Some(name.clone()) == this.map_human_pick {
                PlayerType::Human
            } else {
                PlayerType::AI
            };

            players.push(Player::new(name, player_type));
        }

        this.new_game_screen.player_picker_table().update();
    }

    /// Show the table
    pub fn show(&mut this, ui: &mut Ui) {
        ui.add_space(5.0);

        // Map category and file selection
        ui.horizontal(|ui| {
            ui.label("{Map Mod}:");
            this.map_category_select_box.show(ui);
        });

        ui.horizontal(|ui| {
            ui.label("{Map file}:");
            this.map_file_select_box.show(ui);
        });

        // Loading icon
        if this.loading_icon.is_showing() {
            this.loading_icon.show(ui);
        }

        // Use nations from map button
        if !this.map_nations.is_empty() {
            if ui.add(this.use_nations_from_map_button).clicked() {
                Self::on_use_nations_from_map(&mut this, &mut this.use_nations_from_map_button);
            }
        }

        // Description
        if !this.description_label.is_empty() {
            ui.add_space(10.0);
            ui.label(&this.description_label);
        }

        // Minimap
        this.mini_map_wrapper.show(ui);
    }
}