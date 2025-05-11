use bevy::prelude::*;
use bevy_egui::egui::{self, Color32, Layout, Rect, Ui};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::constants::Constants;
use crate::gui::GUI;
use crate::logic::automation::Automation;
use crate::logic::city::City;
use crate::logic::civilization::Civilization;
use crate::logic::map::tile::Tile;
use crate::models::tutorial::TutorialTrigger;
use crate::models::unciv_sound::UncivSound;
use crate::models::ruleset::building::Building;
use crate::models::ruleset::construction::IConstruction;
use crate::models::ruleset::tile_improvement::TileImprovement;
use crate::models::ruleset::unique::{LocalUniqueCache, UniqueType};
use crate::models::stats::Stat;
use crate::ui::audio::{CityAmbiencePlayer, SoundPlayer};
use crate::ui::components::particle_effect::ParticleEffectMapFireworks;
use crate::ui::components::extensions::{color_from_rgb, disable, pack_if_needed, to_text_button};
use crate::ui::components::input::{KeyCharAndCode, KeyShortcutDispatcherVeto, KeyboardBinding, key_shortcuts, on_activation, onClick, onDoubleClick};
use crate::ui::components::tilegroups::{CityTileGroup, CityTileState, TileGroupMap, TileSetStrings};
use crate::ui::images::ImageGetter;
use crate::ui::popups::{ConfirmPopup, ToastPopup, close_all_popups};
use crate::ui::screens::basescreen::{BaseScreen, RecreateOnResize};
use crate::ui::screens::worldscreen::WorldScreen;
use crate::utils::translations::tr;

/// Distance from stage edges to floating widgets
const POS_FROM_EDGE: f32 = 5.0;

/// Size of the decoration icons shown besides the raze button
const WLTK_ICON_SIZE: f32 = 40.0;

/// Data for picking a tile for improvement
struct PickTileForImprovementData {
    building: Building,
    improvement: TileImprovement,
    is_buying: bool,
    buy_stat: Stat,
}

/// Main screen for city management
pub struct CityScreen {
    city: City,
    selected_civ: Civilization,
    is_spying: bool,
    viewable_cities: Vec<City>,
    can_change_state: bool,

    // UI Components
    constructions_table: CityConstructionsTable,
    raze_city_button_holder: egui::Frame,
    city_stats_table: CityStatsTable,
    tile_table: CityScreenTileTable,
    selected_construction_table: ConstructionInfoTable,
    city_picker_table: CityScreenCityPickerTable,
    exit_city_button: egui::Button,

    // Map and tiles
    tile_groups: Vec<CityTileGroup>,
    map_scroll_pane: CityMapHolder,

    // Selection state
    selected_construction: Option<Box<dyn IConstruction>>,
    selected_tile: Option<Tile>,
    pick_tile_data: Option<PickTileForImprovementData>,
    selected_queue_entry_target_tile: Option<Tile>,
    selected_queue_entry: Option<usize>,
    next_tile_to_own: Option<Tile>,

    // Audio and effects
    city_ambience_player: Option<CityAmbiencePlayer>,
    is_wltk_day: bool,
    fireworks: Option<ParticleEffectMapFireworks>,
}

impl CityScreen {
    /// Create a new CityScreen
    pub fn new(
        city: City,
        init_selected_construction: Option<Box<dyn IConstruction>>,
        init_selected_tile: Option<Tile>,
        ambience_player: Option<CityAmbiencePlayer>,
    ) -> Self {
        let selected_civ = GUI::get_world_screen().selected_civ;
        let is_spying = selected_civ.game_info.is_espionage_enabled() && selected_civ != city.civ;

        // Determine viewable cities based on spying status
        let viewable_cities = if is_spying {
            selected_civ.espionage_manager.get_cities_with_our_spies()
                .into_iter()
                .filter(|c| c.civ != GUI::get_world_screen().selected_civ)
                .collect()
        } else {
            city.civ.cities.clone()
        };

        let can_change_state = GUI::is_allowed_change_state() && !is_spying;
        let is_wltk_day = city.is_we_love_the_king_day_active();

        let mut screen = Self {
            city,
            selected_civ,
            is_spying,
            viewable_cities,
            can_change_state,

            constructions_table: CityConstructionsTable::new(),
            raze_city_button_holder: egui::Frame::none(),
            city_stats_table: CityStatsTable::new(),
            tile_table: CityScreenTileTable::new(),
            selected_construction_table: ConstructionInfoTable::new(),
            city_picker_table: CityScreenCityPickerTable::new(),
            exit_city_button: egui::Button::new("Exit city"),

            tile_groups: Vec::new(),
            map_scroll_pane: CityMapHolder::new(),

            selected_construction: init_selected_construction,
            selected_tile: init_selected_tile,
            pick_tile_data: None,
            selected_queue_entry_target_tile: None,
            selected_queue_entry: None,
            next_tile_to_own: None,

            city_ambience_player: ambience_player,
            is_wltk_day,
            fireworks: None,
        };

        // Initialize the screen
        screen.init();

        screen
    }

    /// Initialize the screen
    fn init(&mut self) {
        // Play WLTK sound if applicable
        if self.is_wltk_day && GUI::get_settings().city_sounds_volume > 0.0 {
            SoundPlayer::play(UncivSound::WLTK);
        }

        // Create fireworks effect if applicable
        if self.is_wltk_day {
            self.fireworks = Some(ParticleEffectMapFireworks::create(&self.map_scroll_pane));
        }

        // Mark tutorial task as completed
        GUI::get_settings().add_completed_tutorial_task("Enter city screen");

        // Add tiles to the map
        self.add_tiles();

        // Add UI components to the stage
        if !self.is_spying {
            self.constructions_table.add_actors_to_stage();
        }

        // Update the screen
        self.update();

        // Add keyboard shortcuts
        self.add_global_shortcut(KeyboardBinding::PreviousCity, move || self.page(-1));
        self.add_global_shortcut(KeyboardBinding::NextCity, move || self.page(1));

        // Center the map in portrait mode
        if self.is_portrait() {
            let max_x = self.get_stage_width();
            let max_y = self.get_stage_height();

            self.map_scroll_pane.set_scroll_x(
                (max_x - self.constructions_table.get_lower_width() - POS_FROM_EDGE) / 2.0
            );

            self.map_scroll_pane.set_scroll_y(
                (max_y - self.city_stats_table.pack_if_needed().height - POS_FROM_EDGE + self.city_picker_table.top) / 2.0
            );

            self.map_scroll_pane.update_visual_scroll();
        }
    }

    /// Update the screen
    pub fn update(&mut self) {
        // Recalculate city stats
        self.city.city_stats.update();

        // Update construction table
        self.constructions_table.set_visible(!self.is_spying);
        self.constructions_table.update(self.selected_construction.as_deref());

        // Update other UI components
        self.update_without_construction_and_map();

        // Update tile groups
        self.update_tile_groups();
    }

    /// Update UI components except construction table and map
    fn update_without_construction_and_map(&mut self) {
        // Update tile or construction info tables
        self.tile_table.update(self.selected_tile.as_ref());
        self.tile_table.set_position(
            self.get_stage_width() - POS_FROM_EDGE,
            POS_FROM_EDGE,
            egui::Align::BOTTOM_RIGHT
        );

        self.selected_construction_table.update(self.selected_construction.as_deref());
        self.selected_construction_table.set_position(
            self.get_stage_width() - POS_FROM_EDGE,
            POS_FROM_EDGE,
            egui::Align::BOTTOM_RIGHT
        );

        // Calculate margins for portrait mode
        let right_margin = if !self.is_portrait() || self.is_cramped_portrait() {
            0.0
        } else if self.selected_tile.is_some() {
            self.tile_table.pack_if_needed().width
        } else if self.selected_construction.is_some() {
            self.selected_construction_table.pack_if_needed().width
        } else {
            POS_FROM_EDGE
        };

        let left_margin = if !self.is_portrait() {
            0.0
        } else {
            self.constructions_table.get_lower_width()
        };

        // Position city picker and exit button
        let centered_x = (self.get_stage_width() - left_margin - right_margin) / 2.0 + left_margin;

        self.exit_city_button.set_position(centered_x, 10.0, egui::Align::BOTTOM);
        self.city_picker_table.update();
        self.city_picker_table.set_position(
            centered_x,
            self.exit_city_button.top + 10.0,
            egui::Align::BOTTOM
        );

        // Update city stats
        self.update_city_stats();

        // Update annex/raze button
        self.update_annex_and_raze_city_button();
    }

    /// Update city stats table
    fn update_city_stats(&mut self) {
        let mut stats_height = self.get_stage_height() - POS_FROM_EDGE * 2.0;

        if self.selected_tile.is_some() {
            stats_height -= self.tile_table.top + 10.0;
        }

        if self.selected_construction.is_some() {
            stats_height -= self.selected_construction_table.top + 10.0;
        }

        self.city_stats_table.update(stats_height);
        self.city_stats_table.set_position(
            self.get_stage_width() - POS_FROM_EDGE,
            self.get_stage_height() - POS_FROM_EDGE,
            egui::Align::TOP_RIGHT
        );
    }

    /// Check if the city can be changed
    pub fn can_city_be_changed(&self) -> bool {
        self.can_change_state && !self.city.is_puppet
    }

    /// Update tile groups
    fn update_tile_groups(&mut self) {
        let city_unique_cache = LocalUniqueCache::new();

        // Helper function to check if an existing improvement is valuable
        let is_existing_improvement_valuable = |tile: &Tile| -> bool {
            if tile.improvement.is_none() {
                return false;
            }

            let civ_info = &self.city.civ;
            let stat_diff_for_new_improvement = tile.stats.get_stat_diff_for_improvement(
                tile.get_tile_improvement().unwrap(),
                civ_info,
                &self.city,
                &city_unique_cache
            );

            // If stat diff for new improvement is negative/zero utility, current improvement is valuable
            Automation::rank_stats_value(stat_diff_for_new_improvement, civ_info) <= 0.0
        };

        // Helper function to get color for improvement placement
        let get_pick_improvement_color = |tile: &Tile| -> (Color32, f32) {
            let improvement_to_place = &self.pick_tile_data.as_ref().unwrap().improvement;

            if tile.is_marked_for_creates_one_improvement() {
                (Color32::from_rgb(139, 69, 19), 0.7) // BROWN
            } else if !tile.improvement_functions.can_build_improvement(improvement_to_place, &self.city.civ) {
                (Color32::RED, 0.4)
            } else if is_existing_improvement_valuable(tile) {
                (Color32::from_rgb(255, 165, 0), 0.5) // ORANGE
            } else if tile.improvement.is_some() {
                (Color32::YELLOW, 0.6)
            } else if tile.turns_to_improvement > 0 {
                (Color32::YELLOW, 0.6)
            } else {
                (Color32::GREEN, 0.5)
            }
        };

        // Update each tile group
        for tile_group in &mut self.tile_groups {
            tile_group.update();
            tile_group.layer_misc.remove_hex_outline();

            if tile_group.tile_state == CityTileState::BLOCKADED {
                self.display_tutorial(TutorialTrigger::CityTileBlockade);
            }

            // Add outlines based on tile state
            if let Some(next_tile) = &self.next_tile_to_own {
                if tile_group.tile == *next_tile {
                    tile_group.layer_misc.add_hex_outline(color_from_rgb(200, 20, 220));
                }
            }

            if let Some(target_tile) = &self.selected_queue_entry_target_tile {
                if tile_group.tile == *target_tile {
                    tile_group.layer_misc.add_hex_outline(Color32::from_rgb(139, 69, 19)); // BROWN
                }
            }

            if let Some(pick_data) = &self.pick_tile_data {
                if self.city.tiles.contains(&tile_group.tile.position) {
                    let (color, alpha) = get_pick_improvement_color(&tile_group.tile);
                    let mut color_with_alpha = color;
                    color_with_alpha.a = (alpha * 255.0) as u8;
                    tile_group.layer_misc.add_hex_outline(color_with_alpha);
                }
            }

            // Set fireworks bounds if applicable
            if let Some(fireworks) = &self.fireworks {
                if tile_group.tile.position == self.city.location {
                    fireworks.set_actor_bounds(tile_group);
                }
            }
        }
    }

    /// Update annex and raze city button
    fn update_annex_and_raze_city_button(&mut self) {
        self.raze_city_button_holder.clear();

        // Helper function to add WLTK icons
        let add_wltk_icon = |name: &str, color: Color32, pad_right: bool| {
            let mut image = ImageGetter::get_image(name);
            image.color = color;

            if pad_right {
                image.pad_right(10.0);
            }

            self.raze_city_button_holder.add(image).size(WLTK_ICON_SIZE);
        };

        // Add WLTK icons if applicable
        if self.is_wltk_day && self.fireworks.is_none() {
            add_wltk_icon("OtherIcons/WLTK LR", Color32::from_rgb(255, 215, 0), false); // GOLD
            add_wltk_icon("OtherIcons/WLTK 1", Color32::from_rgb(178, 34, 34), true); // FIREBRICK
        }

        // Check if city can be annexed
        let can_annex = !self.city.civ.has_unique(UniqueType::MayNotAnnexCities);

        // Add appropriate button based on city state
        if self.city.is_puppet && can_annex {
            let mut annex_city_button = to_text_button("Annex city");
            annex_city_button.pad(10.0);
            annex_city_button.on_click(move || {
                self.city.annex_city();
                self.update();
            });

            if !self.can_change_state {
                annex_city_button.disable();
            }

            self.raze_city_button_holder.add(annex_city_button);
        } else if !self.city.is_being_razed {
            let mut raze_city_button = to_text_button("Raze city");
            raze_city_button.pad(10.0);
            raze_city_button.on_click(move || {
                self.city.is_being_razed = true;
                self.update();
            });

            if !self.can_change_state || !self.city.can_be_destroyed() || !can_annex {
                raze_city_button.disable();
            }

            self.raze_city_button_holder.add(raze_city_button);
        } else {
            let mut stop_razing_city_button = to_text_button("Stop razing city");
            stop_razing_city_button.pad(10.0);
            stop_razing_city_button.on_click(move || {
                self.city.is_being_razed = false;
                self.update();
            });

            if !self.can_change_state {
                stop_razing_city_button.disable();
            }

            self.raze_city_button_holder.add(stop_razing_city_button);
        }

        // Add WLTK icons if applicable
        if self.is_wltk_day && self.fireworks.is_none() {
            add_wltk_icon("OtherIcons/WLTK 2", Color32::from_rgb(178, 34, 34), false); // FIREBRICK
            add_wltk_icon("OtherIcons/WLTK LR", Color32::from_rgb(255, 215, 0), false); // GOLD
        }

        // Position the button holder
        self.raze_city_button_holder.pack();

        if self.is_cramped_portrait() {
            // In cramped portrait mode, move raze button down to city picker
            let center_x = self.city_picker_table.x + self.city_picker_table.width / 2.0 - self.raze_city_button_holder.width / 2.0;
            self.raze_city_button_holder.set_position(
                center_x,
                self.city_picker_table.y + self.city_picker_table.height + 10.0,
                egui::Align::TOP
            );

            // Reposition tooltips
            self.tile_table.set_position(
                self.get_stage_width() - POS_FROM_EDGE,
                self.raze_city_button_holder.top + 10.0,
                egui::Align::BOTTOM_RIGHT
            );

            self.selected_construction_table.set_position(
                self.get_stage_width() - POS_FROM_EDGE,
                self.raze_city_button_holder.top + 10.0,
                egui::Align::BOTTOM_RIGHT
            );

            // Update city stats
            self.update_city_stats();
        } else {
            // Position in normal mode
            let center_x = if self.is_portrait() {
                let upper_width = self.constructions_table.get_upper_width();
                upper_width + (self.get_stage_width() - self.city_stats_table.width - upper_width) / 2.0
            } else {
                self.get_stage_width() / 2.0
            };

            self.raze_city_button_holder.set_position(
                center_x,
                self.get_stage_height() - 20.0,
                egui::Align::TOP
            );
        }
    }

    /// Add tiles to the map
    fn add_tiles(&mut self) {
        let view_range = self.city.get_expand_range().max(self.city.get_work_range());
        let tile_set_strings = TileSetStrings::new(&self.city.civ.game_info.ruleset, &GUI::get_settings());

        // Get tiles in range
        let city_tile_groups: Vec<CityTileGroup> = self.city.get_center_tile()
            .get_tiles_in_distance(view_range)
            .into_iter()
            .filter(|t| self.selected_civ.has_explored(t))
            .map(|t| CityTileGroup::new(&self.city, t, &tile_set_strings, false))
            .collect();

        // Set up tile groups
        for mut tile_group in city_tile_groups {
            tile_group.on_click(move |tile_group, city| self.tile_group_on_click(tile_group, city));
            tile_group.layer_misc.on_click(move |tile_group, city| self.tile_worked_icon_on_click(tile_group, city));
            tile_group.layer_misc.on_double_click(move |tile_group, city| self.tile_worked_icon_double_click(tile_group, city));
            self.tile_groups.push(tile_group);
        }

        // Find tiles to unwrap (on the other side of the map)
        let mut tiles_to_unwrap = HashSet::new();
        for tile_group in &self.tile_groups {
            let x_difference = self.city.get_center_tile().position.x - tile_group.tile.position.x;
            let y_difference = self.city.get_center_tile().position.y - tile_group.tile.position.y;

            if x_difference > view_range || x_difference < -view_range || y_difference > view_range || y_difference < -view_range {
                tiles_to_unwrap.insert(tile_group.clone());
            }
        }

        // Create tile map group
        let tile_map_group = TileGroupMap::new(&self.map_scroll_pane, &self.tile_groups, tiles_to_unwrap);
        self.map_scroll_pane.set_actor(tile_map_group);
        self.map_scroll_pane.set_size(self.get_stage_width(), self.get_stage_height());

        // Center the map
        self.map_scroll_pane.layout();
        self.map_scroll_pane.set_scroll_percent_x(0.5);
        self.map_scroll_pane.set_scroll_percent_y(0.5);
        self.map_scroll_pane.update_visual_scroll();
    }

    /// Handle tile worked icon click
    fn tile_worked_icon_on_click(&mut self, tile_group: &mut CityTileGroup, city: &mut City) {
        if !self.can_change_state || city.is_puppet {
            return;
        }

        let tile = &tile_group.tile;

        // Cycling as: Not-worked -> Worked -> Not-worked
        if tile_group.tile_state == CityTileState::WORKABLE {
            if !tile.provides_yield() && city.population.get_free_population() > 0 {
                city.worked_tiles.insert(tile.position);
                GUI::get_settings().add_completed_tutorial_task("Reassign worked tiles");
            } else {
                city.worked_tiles.remove(&tile.position);
                city.locked_tiles.remove(&tile.position);
            }

            city.city_stats.update();
            self.update();
        } else if tile_group.tile_state == CityTileState::PURCHASABLE {
            self.ask_to_buy_tile(tile);
        }
    }

    /// Ask whether user wants to buy a tile for gold
    pub fn ask_to_buy_tile(&mut self, selected_tile: &Tile) {
        // Check if tile can be bought
        if !self.can_change_state || !self.city.expansion.can_buy_tile(selected_tile) {
            return;
        }

        let gold_cost_of_tile = self.city.expansion.get_gold_cost_of_tile(selected_tile);
        if !self.city.civ.has_stat_to_buy(Stat::Gold, gold_cost_of_tile) {
            return;
        }

        close_all_popups();

        // Create purchase prompt
        let purchase_prompt = format!(
            "{}Currently you have [{}] [Gold].\n\nWould you like to purchase [Tile] for [{}] [{}]?",
            tr("Currently you have [{}] [Gold]."),
            self.city.civ.gold,
            gold_cost_of_tile,
            Stat::Gold.character
        );

        // Show confirmation popup
        let mut popup = ConfirmPopup::new(
            self,
            purchase_prompt,
            "Purchase".to_string(),
            true,
            Some(Box::new(move || self.update())),
        );

        popup.set_action(Box::new(move || {
            SoundPlayer::play(UncivSound::Coin);
            self.city.expansion.buy_tile(selected_tile);

            // Preselect the next tile on city screen rebuild
            let next_tile = self.city.expansion.choose_new_tile_to_own();
            let new_screen = CityScreen::new(self.city.clone(), None, next_tile, None);
            GUI::replace_current_screen(Box::new(new_screen));
        }));

        popup.open();
    }

    /// Handle tile worked icon double click
    fn tile_worked_icon_double_click(&mut self, tile_group: &mut CityTileGroup, city: &mut City) {
        if !self.can_change_state || city.is_puppet || tile_group.tile_state != CityTileState::WORKABLE {
            return;
        }

        let tile = &tile_group.tile;

        // Double-click should lead to locked tiles - both for unworked AND worked tiles
        if !tile.is_worked() {
            // If not worked, try to work it first
            self.tile_worked_icon_on_click(tile_group, city);
        }

        if tile.is_worked() {
            city.locked_tiles.insert(tile.position);
        }

        self.update();
    }

    /// Handle tile group click
    fn tile_group_on_click(&mut self, tile_group: &mut CityTileGroup, city: &mut City) {
        if city.is_puppet {
            return;
        }

        let tile_info = &tile_group.tile;

        // Handle tile selection for improvement
        if let Some(pick_data) = &self.pick_tile_data {
            let pick_data = pick_data.clone();
            self.pick_tile_data = None;

            let improvement = &pick_data.improvement;
            if tile_info.improvement_functions.can_build_improvement(improvement, &city.civ) {
                if pick_data.is_buying {
                    // Buy the construction
                    BuyButtonFactory::new(self).ask_to_buy_construction(
                        &pick_data.building,
                        pick_data.buy_stat,
                        tile_info
                    );
                } else {
                    // Mark the tile for improvement
                    tile_info.improvement_functions.mark_for_creates_one_improvement(&improvement.name);
                    city.city_constructions.add_to_queue(&pick_data.building.name);
                }
            }

            self.update();
            return;
        }

        // Select the tile
        self.select_tile(Some(tile_info.clone()));
        self.update();
    }

    /// Check if the city has a free building
    pub fn has_free_building(&self, building: &Building) -> bool {
        self.city.civ.civ_constructions.has_free_building(&self.city, building)
    }

    /// Select a construction from the queue
    pub fn select_construction_from_queue(&mut self, index: usize) {
        if let Some(construction) = self.city.city_constructions.construction_queue.get(index) {
            self.select_construction(Some(construction.clone()));
        }
    }

    /// Select a construction by name
    pub fn select_construction_by_name(&mut self, name: &str) {
        if let Some(construction) = self.city.city_constructions.get_construction(name) {
            self.select_construction(Some(construction));
        }
    }

    /// Select a construction
    pub fn select_construction(&mut self, new_construction: Option<Box<dyn IConstruction>>) {
        self.selected_construction = new_construction.clone();

        if let Some(construction) = new_construction.as_ref() {
            if let Some(building) = construction.as_any().downcast_ref::<Building>() {
                if building.has_create_one_improvement_unique() {
                    let improvement = building.get_improvement_to_create(&self.city.get_ruleset(), &self.city.civ);

                    if let Some(improvement) = improvement {
                        self.selected_queue_entry_target_tile = self.city.city_constructions.get_tile_for_improvement(&improvement.name);
                    } else {
                        self.selected_queue_entry_target_tile = None;
                    }
                } else {
                    self.selected_queue_entry_target_tile = None;
                    self.pick_tile_data = None;
                }
            }
        } else {
            self.selected_queue_entry_target_tile = None;
            self.pick_tile_data = None;
        }

        self.selected_tile = None;
    }

    /// Select a tile
    fn select_tile(&mut self, new_tile: Option<Tile>) {
        self.selected_construction = None;
        self.selected_queue_entry_target_tile = None;
        self.pick_tile_data = None;
        self.selected_tile = new_tile;
    }

    /// Clear the current selection
    pub fn clear_selection(&mut self) {
        self.select_tile(None);
    }

    /// Start picking a tile for a building that creates an improvement
    pub fn start_pick_tile_for_creates_one_improvement(&mut self, construction: &Building, stat: Stat, is_buying: bool) {
        if let Some(improvement) = construction.get_improvement_to_create(&self.city.get_ruleset(), &self.city.civ) {
            self.pick_tile_data = Some(PickTileForImprovementData {
                building: construction.clone(),
                improvement,
                is_buying,
                buy_stat: stat,
            });

            self.update_tile_groups();

            let message = format!("Please select a tile for this building's [{}]", improvement.name);
            ToastPopup::new(message, self).open();
        }
    }

    /// Stop picking a tile for improvement
    pub fn stop_pick_tile_for_creates_one_improvement(&mut self) {
        if self.pick_tile_data.is_some() {
            self.pick_tile_data = None;
            self.update_tile_groups();
        }
    }

    /// Exit the city screen
    pub fn exit(&mut self) {
        let new_screen = GUI::pop_screen();

        if let Some(world_screen) = new_screen.downcast_ref::<WorldScreen>() {
            world_screen.map_holder.set_center_position(self.city.location, true);
            world_screen.bottom_unit_table.select_unit();
        }
    }

    /// Pass on the city ambience player
    fn pass_on_city_ambience_player(&mut self) -> Option<CityAmbiencePlayer> {
        let player = self.city_ambience_player.take();
        player
    }

    /// Page to the next/previous city
    pub fn page(&mut self, delta: i32) {
        let num_cities = self.viewable_cities.len();
        if num_cities == 0 {
            return;
        }

        let index_of_city = self.viewable_cities.iter().position(|c| c == &self.city).unwrap_or(0);
        let index_of_next_city = (index_of_city as i32 + delta + num_cities as i32) % num_cities as i32;

        let new_city_screen = CityScreen::new(
            self.viewable_cities[index_of_next_city as usize].clone(),
            None,
            None,
            self.pass_on_city_ambience_player(),
        );

        // Retain zoom level
        new_city_screen.map_scroll_pane.zoom(self.map_scroll_pane.scale_x);
        new_city_screen.update();

        GUI::replace_current_screen(Box::new(new_city_screen));
    }
}

impl RecreateOnResize for CityScreen {
    fn recreate(&self) -> Box<dyn BaseScreen> {
        Box::new(CityScreen::new(
            self.city.clone(),
            self.selected_construction.clone(),
            self.selected_tile.clone(),
            None,
        ))
    }
}

impl Drop for CityScreen {
    fn drop(&mut self) {
        if let Some(player) = &mut self.city_ambience_player {
            player.dispose();
        }

        if let Some(fireworks) = &mut self.fireworks {
            fireworks.dispose();
        }
    }
}

impl BaseScreen for CityScreen {
    fn render(&mut self, delta: f32) {
        // Render fireworks if applicable
        if let Some(fireworks) = &mut self.fireworks {
            fireworks.render(delta);
        }
    }

    fn get_shortcut_dispatcher_vetoer(&self) -> Option<Box<dyn KeyShortcutDispatcherVeto>> {
        Some(Box::new(KeyShortcutDispatcherVeto::create_tile_group_map_dispatcher_vetoer()))
    }
}