use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;
use std::uuid::Uuid;

use ggez::graphics::{Color, DrawParam, Image, Text};
use ggez::input::keyboard::KeyCode;
use ggez::mint::Point2;
use ggez::{Context, GameResult};

use crate::constants::Constants;
use crate::display::Display;
use crate::game::UncivGame;
use crate::gui::GUI;
use crate::models::metadata::{GameSettings, ModCategories, ScreenSize};
use crate::models::translations::TranslationFileWriter;
use crate::ui::components::font::{FontFamilyData, Fonts};
use crate::ui::components::input::KeyShortcuts;
use crate::ui::components::widgets::{Checkbox, SelectBox, Slider, TextButton};
use crate::ui::popups::confirm_popup::ConfirmPopup;
use crate::ui::popups::options::OptionsPopup;
use crate::ui::screens::base_screen::BaseScreen;
use crate::utils::concurrency::Concurrency;
use crate::utils::display::launch_on_gl_thread;

pub struct AdvancedTab {
    options_popup: OptionsPopup,
    on_font_change: Box<dyn Fn()>,
}

impl AdvancedTab {
    pub fn new(options_popup: OptionsPopup, on_font_change: Box<dyn Fn()>) -> Self {
        Self {
            options_popup,
            on_font_change,
        }
    }

    pub fn render(&self, ctx: &mut Context, screen: &BaseScreen) -> GameResult<()> {
        let mut table = screen.create_table();
        table.pad(10.0);
        table.defaults().pad(5.0);

        // Add autosave settings
        self.add_max_autosaves_stored(&mut table);
        self.add_autosave_turns_select_box(&mut table);
        table.add_separator();

        // Add display settings
        if Display::has_cutout() {
            self.add_cutout_checkbox(&mut table);
        }

        if Display::has_system_ui_visibility() {
            self.add_hide_system_ui_checkbox(&mut table);
        }

        // Add font settings
        self.add_font_family_select(&mut table);
        self.add_font_size_multiplier(&mut table);
        table.add_separator();

        // Add zoom settings
        self.add_max_zoom_slider(&mut table);

        // Add miscellaneous settings
        self.add_easter_eggs_checkbox(&mut table);
        self.add_enlarge_notifications_checkbox(&mut table);
        table.add_separator();

        // Add user ID settings
        self.add_set_user_id(&mut table);

        // Add translation generation
        self.add_translation_generation(&mut table);

        // Render the table
        table.render(ctx, screen)?;

        Ok(())
    }

    fn add_cutout_checkbox(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;
        let checkbox = Checkbox::new("Enable using display cutout areas", settings.android_cutout);

        checkbox.on_change(Box::new(move |value| {
            settings.android_cutout = value;
            Display::set_cutout(value);
            self.options_popup.reopen_after_display_layout_change();
        }));

        table.add(checkbox);
    }

    fn add_hide_system_ui_checkbox(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;
        let checkbox = Checkbox::new("Hide system status and navigation bars", settings.android_hide_system_ui);

        checkbox.on_change(Box::new(move |value| {
            settings.android_hide_system_ui = value;
            Display::set_system_ui_visibility(value);
            self.options_popup.reopen_after_display_layout_change();
        }));

        table.add(checkbox);
    }

    fn add_max_autosaves_stored(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;

        table.add_label("Number of autosave files stored");

        let options = vec![1, 2, 5, 10, 15, 20, 35, 50, 100, 150, 200, 250];
        let mut select_box = SelectBox::new(options);
        select_box.set_selected(settings.max_autosaves_stored);

        select_box.on_change(Box::new(move |value| {
            settings.max_autosaves_stored = value;
        }));

        table.add(select_box).pad(10.0);
        table.row();
    }

    fn add_autosave_turns_select_box(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;

        table.add_label("Turns between autosaves");

        let options = vec![1, 2, 5, 10];
        let mut select_box = SelectBox::new(options);
        select_box.set_selected(settings.turns_between_autosaves);

        select_box.on_change(Box::new(move |value| {
            settings.turns_between_autosaves = value;
        }));

        table.add(select_box).pad(10.0);
        table.row();
    }

    fn add_font_family_select(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;
        let on_font_change = self.on_font_change.clone();

        table.add_label("Font family");

        // Create a placeholder for the select box
        let select_cell = table.add_empty_cell();
        table.row();

        // Load fonts in a background thread
        Concurrency::run("Add Font Select", move || {
            let mut fonts = Vec::new();

            // Add default font
            fonts.push(FontFamilyData::default());

            // Add mods fonts
            let mods_dir = UncivGame::current().files.get_mods_folder();
            if let Ok(entries) = fs::read_dir(mods_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_dir() {
                        continue;
                    }

                    let mod_fonts_dir = path.join("fonts");
                    if !mod_fonts_dir.exists() {
                        continue;
                    }

                    // Find .ttf files and add construct FontFamilyData
                    if let Ok(font_entries) = fs::read_dir(mod_fonts_dir) {
                        for font_entry in font_entries.flatten() {
                            let font_path = font_entry.path();
                            if let Some(extension) = font_path.extension() {
                                if extension.to_string_lossy().to_lowercase() == "ttf" {
                                    let mod_name = path.file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("Unknown");

                                    let font_name = font_path.file_stem()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("Unknown");

                                    fonts.push(FontFamilyData::new(
                                        format!("{} ({})", font_name, mod_name),
                                        font_name.to_string(),
                                        font_path.to_string_lossy().to_string()
                                    ));
                                }
                            }
                        }
                    }
                }
            }

            // Add system fonts
            fonts.extend(Fonts::get_system_fonts());

            // Update UI on the main thread
            launch_on_gl_thread(move || {
                self.load_font_select(fonts, select_cell, settings, on_font_change);
            });
        });
    }

    fn load_font_select(
        &self,
        fonts: Vec<FontFamilyData>,
        select_cell: &mut BaseScreen,
        settings: &mut GameSettings,
        on_font_change: Box<dyn Fn()>
    ) {
        if fonts.is_empty() {
            return;
        }

        let mut font_select_box = SelectBox::new(fonts.clone());

        // Find the font to select based on invariant name
        let font_to_select = &settings.font_family_data;
        if let Some(selected_font) = fonts.iter().find(|f| f.invariant_name == font_to_select.invariant_name) {
            font_select_box.set_selected(selected_font.clone());
        }

        font_select_box.on_change(Box::new(move |value| {
            settings.font_family_data = value;
            on_font_change();
        }));

        select_cell.set_actor(font_select_box)
            .min_width(self.options_popup.select_box_min_width)
            .pad(10.0);
    }

    fn add_font_size_multiplier(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;
        let on_font_change = self.on_font_change.clone();

        table.add_label("Font size multiplier").pad_top(5.0);

        let mut slider = Slider::new(0.7, 1.5, 0.05, settings.font_size_multiplier);

        slider.on_change(Box::new(move |value| {
            settings.font_size_multiplier = value;
        }));

        slider.on_drag_end(Box::new(move || {
            on_font_change();
        }));

        table.add(slider).pad(5.0).pad_top(10.0);
        table.row();
    }

    fn add_max_zoom_slider(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;

        table.add_label("Max zoom out").pad_top(5.0);

        let mut slider = Slider::new(2.0, 6.0, 1.0, settings.max_world_zoom_out);

        slider.on_change(Box::new(move |value| {
            settings.max_world_zoom_out = value;
            if GUI::is_world_loaded() {
                GUI::get_map().reload_max_zoom();
            }
        }));

        table.add(slider).pad(5.0).pad_top(10.0);
        table.row();
    }

    fn add_translation_generation(&self, table: &mut BaseScreen) {
        // Only show on desktop
        if !cfg!(target_os = "windows") && !cfg!(target_os = "linux") && !cfg!(target_os = "macos") {
            return;
        }

        let settings = &mut self.options_popup.settings;

        // Generate translations button
        let mut generate_translations_button = TextButton::new("Generate translation files");

        generate_translations_button.on_activation(Box::new(move || {
            self.options_popup.tabs.select_page("Advanced"); // Only because key F12 works from any page
            generate_translations_button.set_text(Constants::WORKING);

            Concurrency::run("WriteTranslations", move || {
                let result = TranslationFileWriter::write_new_translation_files();

                launch_on_gl_thread(move || {
                    // Notify about completion
                    generate_translations_button.set_text(result);
                    generate_translations_button.disable();
                });
            });
        }));

        // Add F12 shortcut
        let mut shortcuts = KeyShortcuts::new();
        shortcuts.add(KeyCode::F12);
        generate_translations_button.set_shortcuts(shortcuts);
        generate_translations_button.add_tooltip("F12", 18.0);

        table.add(generate_translations_button).colspan(2).row();

        // Update mod categories button
        let mut update_mod_categories_button = TextButton::new("Update Mod categories");

        update_mod_categories_button.on_activation(Box::new(move || {
            update_mod_categories_button.set_text(Constants::WORKING);

            Concurrency::run("GithubTopicQuery", move || {
                let result = ModCategories::merge_online();

                launch_on_gl_thread(move || {
                    update_mod_categories_button.set_text(result);
                });
            });
        }));

        table.add(update_mod_categories_button).colspan(2).row();

        // Screenshot generation (only if the game exists)
        let screenshot_game_path = UncivGame::current().files.get_save("ScreenshotGenerationGame");
        if !screenshot_game_path.exists() {
            return;
        }

        let mut generate_screenshots_button = TextButton::new("Generate screenshots");

        generate_screenshots_button.on_activation(Box::new(move || {
            generate_screenshots_button.set_text(Constants::WORKING);

            Concurrency::run("GenerateScreenshot", move || {
                let extra_images_location = "../../extraImages";

                // Create screenshot configs
                let configs = vec![
                    ScreenshotConfig::new(630, 500, ScreenSize::Medium,
                        format!("{}/itch.io image.png", extra_images_location),
                        Point2 { x: -2.0, y: 2.0 }, false),
                    ScreenshotConfig::new(1280, 640, ScreenSize::Medium,
                        format!("{}/GithubPreviewImage.png", extra_images_location),
                        Point2 { x: -2.0, y: 4.0 }, true),
                    ScreenshotConfig::new(1024, 500, ScreenSize::Medium,
                        format!("{}/Feature graphic - Google Play.png", extra_images_location),
                        Point2 { x: -2.0, y: 6.0 }, true),
                    ScreenshotConfig::new(1024, 500, ScreenSize::Medium,
                        "../../fastlane/metadata/android/en-US/images/featureGraphic.png".to_string(),
                        Point2 { x: -2.0, y: 8.0 }, true),
                ];

                self.generate_screenshots(settings.clone(), configs);
            });
        }));

        table.add(generate_screenshots_button).colspan(2).row();
    }

    fn generate_screenshots(&self, settings: GameSettings, configs: Vec<ScreenshotConfig>) {
        if configs.is_empty() {
            return;
        }

        let current_config = configs[0].clone();
        let remaining_configs = configs[1..].to_vec();

        launch_on_gl_thread(move || {
            let screenshot_game = UncivGame::current().files.load_game_by_name("ScreenshotGenerationGame");
            let mut settings = settings;
            settings.screen_size = current_config.screen_size;

            let new_screen = UncivGame::current().load_game(screenshot_game);
            new_screen.stage.viewport.update(current_config.width, current_config.height, true);

            // Reposition mapholder and minimap whose position was based on the previous stage size
            new_screen.map_holder.set_size(new_screen.stage.width, new_screen.stage.height);
            new_screen.map_holder.layout();
            new_screen.minimap_wrapper.x = new_screen.stage.width - new_screen.minimap_wrapper.width;

            // Center on the city
            new_screen.map_holder.set_center_position(
                current_config.center_tile,
                true,
                true
            );

            // Click on Keshik
            new_screen.map_holder.on_tile_clicked(new_screen.map_holder.tile_map.get(-2, 3));

            // Click city again for attack table if needed
            if current_config.attack_city {
                new_screen.map_holder.on_tile_clicked(new_screen.map_holder.tile_map.get(-2, 2));
            }

            new_screen.map_holder.zoom_in(true);

            // Wait a bit for the UI to update
            thread::sleep(Duration::from_millis(300));

            // Take screenshot
            let pixmap = new_screen.stage.viewport.capture_screenshot(
                0, 0, current_config.width, current_config.height
            );

            // Save the screenshot
            if let Some(pixmap) = pixmap {
                if let Some(parent) = Path::new(&current_config.file_location).parent() {
                    fs::create_dir_all(parent).ok();
                }

                pixmap.save_png(&current_config.file_location);
            }

            // Process remaining configs
            if !remaining_configs.is_empty() {
                self.generate_screenshots(settings, remaining_configs);
            }
        });
    }

    fn add_set_user_id(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;

        let id_set_label = table.add_label("");
        let mut take_user_id_from_clipboard_button = TextButton::new("Take user ID from clipboard");

        take_user_id_from_clipboard_button.on_click(Box::new(move || {
            let clipboard_contents = Display::get_clipboard_contents().trim().to_string();

            // Validate UUID
            match Uuid::parse_str(&clipboard_contents) {
                Ok(_) => {
                    let settings = settings.clone();
                    let id_set_label = id_set_label.clone();

                    let confirm = ConfirmPopup::new(
                        "Doing this will reset your current user ID to the clipboard contents - are you sure?",
                        "Take user ID from clipboard",
                        Box::new(move || {
                            settings.multiplayer.user_id = clipboard_contents;
                            id_set_label.set_font_color(Color::WHITE).set_text("ID successfully set!");
                        })
                    );

                    confirm.open(true);
                    id_set_label.set_visible(true);
                },
                Err(_) => {
                    id_set_label.set_visible(true);
                    id_set_label.set_font_color(Color::RED).set_text("Invalid ID!");
                }
            }
        }));

        table.add(take_user_id_from_clipboard_button).pad(5.0).colspan(2).row();
        table.add(id_set_label).colspan(2).row();
    }

    fn add_easter_eggs_checkbox(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;
        let checkbox = Checkbox::new("Enable Easter Eggs", settings.enable_easter_eggs);

        checkbox.on_change(Box::new(move |value| {
            settings.enable_easter_eggs = value;
        }));

        table.add(checkbox);
    }

    fn add_enlarge_notifications_checkbox(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;
        let checkbox = Checkbox::new("Enlarge selected notifications", settings.enlarge_selected_notification);

        checkbox.on_change(Box::new(move |value| {
            settings.enlarge_selected_notification = value;
        }));

        table.add(checkbox);
    }
}

#[derive(Clone)]
pub struct ScreenshotConfig {
    pub width: i32,
    pub height: i32,
    pub screen_size: ScreenSize,
    pub file_location: String,
    pub center_tile: Point2<f32>,
    pub attack_city: bool,
}

impl ScreenshotConfig {
    pub fn new(
        width: i32,
        height: i32,
        screen_size: ScreenSize,
        file_location: String,
        center_tile: Point2<f32>,
        attack_city: bool
    ) -> Self {
        Self {
            width,
            height,
            screen_size,
            file_location,
            center_tile,
            attack_city,
        }
    }
}