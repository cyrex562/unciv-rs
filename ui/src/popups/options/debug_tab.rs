use ggez::graphics::{Color, DrawParam, Text};
use ggez::{Context, GameResult};
use clipboard::{ClipboardContext, ClipboardProvider};

use crate::constants::Constants;
use crate::game::UncivGame;
use crate::gui::GUI;
use crate::models::civilization::PlayerType;
use crate::models::metadata::GameSettings;
use crate::models::ruleset::ResourceType;
use crate::ui::components::widgets::{Checkbox, Slider, TextField};
use crate::ui::popups::options::OptionsPopup;
use crate::ui::popups::ToastPopup;
use crate::ui::screens::base_screen::BaseScreen;
use crate::utils::debug_utils::DebugUtils;
use crate::utils::files::{MapSaver, UncivFiles};
use crate::utils::ruleset_cache::RulesetCache;

pub struct DebugTab {
    options_popup: OptionsPopup,
    simulate_until_turn: i32,
    invalid_input_visible: bool,
}

impl DebugTab {
    pub fn new(options_popup: OptionsPopup) -> Self {
        Self {
            options_popup,
            simulate_until_turn: DebugUtils::SIMULATE_UNTIL_TURN,
            invalid_input_visible: false,
        }
    }

    pub fn render(&self, ctx: &mut Context, screen: &BaseScreen) -> GameResult<()> {
        let mut table = screen.create_table();
        table.pad(10.0);
        table.defaults().pad(5.0);

        // Add simulation controls if world is loaded
        if GUI::is_world_loaded() {
            self.add_simulation_controls(&mut table);
        }

        // Add debug toggles
        self.add_debug_toggles(&mut table);

        // Add game-specific toggles if game is loaded
        if let Some(game_info) = &UncivGame::current().game_info {
            self.add_game_specific_toggles(&mut table, game_info);
        }

        // Add file compression toggles
        self.add_compression_toggles(&mut table);

        // Add unique misspelling threshold slider
        self.add_misspelling_threshold_slider(&mut table);

        // Add unlock techs button
        self.add_unlock_techs_button(&mut table);

        // Add give resources button
        self.add_give_resources_button(&mut table);

        // Add load from clipboard button
        self.add_load_from_clipboard_button(&mut table);

        // Add separator
        table.add_separator();

        // Add crash button
        self.add_crash_button(&mut table);

        // Add separator
        table.add_separator();

        // Render the table
        table.render(ctx, screen)?;

        Ok(())
    }

    fn add_simulation_controls(&self, table: &mut BaseScreen) {
        // Add simulate button
        let simulate_button = screen.create_text_button("Simulate until turn:");

        // Add text field for turn input
        let mut simulate_text_field = TextField::new("Turn", self.simulate_until_turn.to_string());

        // Add invalid input label
        let invalid_input_label = screen.create_label("This is not a valid integer!");
        invalid_input_label.set_visible(self.invalid_input_visible);

        // Set up button click handler
        simulate_button.on_click(Box::new(move || {
            if let Ok(turns) = simulate_text_field.text().parse::<i32>() {
                DebugUtils::SIMULATE_UNTIL_TURN = turns;
                invalid_input_label.set_visible(false);
                GUI::get_world_screen().next_turn();
            } else {
                invalid_input_label.set_visible(true);
            }
        }));

        table.add(simulate_button);
        table.add(simulate_text_field).row();
        table.add(invalid_input_label).colspan(2).row();
    }

    fn add_debug_toggles(&self, table: &mut BaseScreen) {
        // Add supercharged toggle
        let supercharged_checkbox = Checkbox::new("Supercharged", DebugUtils::SUPERCHARGED);
        supercharged_checkbox.on_change(Box::new(move |value| {
            DebugUtils::SUPERCHARGED = value;
        }));
        table.add(supercharged_checkbox);

        // Add view entire map toggle
        let visible_map_checkbox = Checkbox::new("View entire map", DebugUtils::VISIBLE_MAP);
        visible_map_checkbox.on_change(Box::new(move |value| {
            DebugUtils::VISIBLE_MAP = value;
        }));
        table.add(visible_map_checkbox);

        // Add show coordinates toggle
        let show_coords_checkbox = Checkbox::new("Show coordinates on tiles", DebugUtils::SHOW_TILE_COORDS);
        show_coords_checkbox.on_change(Box::new(move |value| {
            DebugUtils::SHOW_TILE_COORDS = value;
        }));
        table.add(show_coords_checkbox);

        // Add show tile image locations toggle
        let show_locations_checkbox = Checkbox::new("Show tile image locations", DebugUtils::SHOW_TILE_IMAGE_LOCATIONS);
        show_locations_checkbox.on_change(Box::new(move |value| {
            DebugUtils::SHOW_TILE_IMAGE_LOCATIONS = value;
        }));
        table.add(show_locations_checkbox);
    }

    fn add_game_specific_toggles(&self, table: &mut BaseScreen, game_info: &GameInfo) {
        // Add god mode toggle
        let god_mode_checkbox = Checkbox::new("God mode (current game)", game_info.game_parameters.god_mode);
        god_mode_checkbox.on_change(Box::new(move |value| {
            game_info.game_parameters.god_mode = value;
        }));
        table.add(god_mode_checkbox);
    }

    fn add_compression_toggles(&self, table: &mut BaseScreen) {
        // Add save games compressed toggle
        let save_zipped_checkbox = Checkbox::new("Save games compressed", UncivFiles::SAVE_ZIPPED);
        save_zipped_checkbox.on_change(Box::new(move |value| {
            UncivFiles::SAVE_ZIPPED = value;
        }));
        table.add(save_zipped_checkbox);

        // Add save maps compressed toggle
        let save_maps_zipped_checkbox = Checkbox::new("Save maps compressed", MapSaver::SAVE_ZIPPED);
        save_maps_zipped_checkbox.on_change(Box::new(move |value| {
            MapSaver::SAVE_ZIPPED = value;
        }));
        table.add(save_maps_zipped_checkbox);

        // Add scene debug toggle
        let scene_debug_checkbox = Checkbox::new("Gdx Scene2D debug", BaseScreen::ENABLE_SCENE_DEBUG);
        scene_debug_checkbox.on_change(Box::new(move |value| {
            BaseScreen::ENABLE_SCENE_DEBUG = value;
        }));
        table.add(scene_debug_checkbox);
    }

    fn add_misspelling_threshold_slider(&self, table: &mut BaseScreen) {
        let mut inner_table = screen.create_table();

        // Add label
        let label = screen.create_label("Unique misspelling threshold");
        label.set_alignment(Alignment::Left);
        inner_table.add(label).fill_x();

        // Add slider
        let mut slider = Slider::new(
            0.0, 0.5, 0.05,
            RulesetCache::UNIQUE_MISSPELLING_THRESHOLD as f32
        );

        slider.on_change(Box::new(move |value| {
            RulesetCache::UNIQUE_MISSPELLING_THRESHOLD = value as f64;
        }));

        inner_table.add(slider)
            .min_width(120.0)
            .pad(5.0);

        table.add(inner_table).colspan(2).row();
    }

    fn add_unlock_techs_button(&self, table: &mut BaseScreen) {
        let unlock_techs_button = screen.create_text_button("Unlock all techs");

        unlock_techs_button.on_click(Box::new(move || {
            if let Some(game_info) = &UncivGame::current().game_info {
                let current_civ = game_info.get_current_player_civilization();

                // Unlock all technologies
                for tech_name in game_info.ruleset.technologies.keys() {
                    if !current_civ.tech.techs_researched.contains(tech_name) {
                        current_civ.tech.add_technology(tech_name);
                        current_civ.popup_alerts.remove_last_or_null();
                    }
                }

                // Update sight and resources
                current_civ.cache.update_sight_and_resources();
                GUI::set_update_world_on_next_render();
            }
        }));

        table.add(unlock_techs_button).colspan(2).row();
    }

    fn add_give_resources_button(&self, table: &mut BaseScreen) {
        let give_resources_button = screen.create_text_button("Get all strategic resources");

        give_resources_button.on_click(Box::new(move || {
            if let Some(game_info) = &UncivGame::current().game_info {
                let current_civ = game_info.get_current_player_civilization();

                // Get owned tiles
                let owned_tiles: Vec<_> = game_info.tile_map.values()
                    .filter(|tile| tile.get_owner() == current_civ)
                    .collect();

                // Get strategic resources
                let resource_types: Vec<_> = game_info.ruleset.tile_resources.values()
                    .filter(|resource| resource.resource_type == ResourceType::Strategic)
                    .collect();

                // Apply resources to tiles
                for (tile, resource) in owned_tiles.iter().zip(resource_types.iter()) {
                    tile.resource = resource.name.clone();
                    tile.resource_amount = 999;

                    // Set improvement if available
                    if let Some(improvement) = resource.get_improvements().first() {
                        tile.set_improvement(improvement.clone());
                    }
                }

                // Update sight and resources
                current_civ.cache.update_sight_and_resources();
                GUI::set_update_world_on_next_render();
            }
        }));

        table.add(give_resources_button).colspan(2).row();
    }

    fn add_load_from_clipboard_button(&self, table: &mut BaseScreen) {
        let load_button = screen.create_text_button("Load online multiplayer game as hotseat from clipboard");

        load_button.on_click(Box::new(move || {
            // Run in a separate thread to avoid blocking the UI
            std::thread::spawn(move || {
                let mut ctx = ClipboardContext::new().unwrap();
                if let Ok(clipboard_contents) = ctx.get_contents() {
                    let trimmed = clipboard_contents.trim();

                    match UncivFiles::game_info_from_string(trimmed) {
                        Ok(mut loaded_game) => {
                            loaded_game.game_parameters.is_online_multiplayer = false;
                            self.options_popup.game.load_game(loaded_game, true);
                            self.options_popup.close();
                        },
                        Err(ex) => {
                            let error_message = ex.to_string();
                            ToastPopup::new(error_message, self.options_popup.stage_to_show_on);
                        }
                    }
                }
            });
        }));

        table.add(load_button).colspan(2).row();
    }

    fn add_crash_button(&self, table: &mut BaseScreen) {
        let crash_button = screen.create_text_button("* Crash Unciv! *");
        crash_button.set_style("negative");

        crash_button.on_click(Box::new(move || {
            panic!("Intentional crash");
        }));

        table.add(crash_button).colspan(2).row();
    }
}