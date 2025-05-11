use ggez::graphics::{Color, DrawParam, Text};
use ggez::{Context, GameResult};
use std::collections::HashMap;

use crate::constants::Constants;
use crate::game::UncivGame;
use crate::gui::GUI;
use crate::models::metadata::{GameSettings, ScreenSize};
use crate::models::skins::SkinCache;
use crate::models::tilesets::TileSetCache;
use crate::ui::components::widgets::{Checkbox, Slider, SelectBox, TranslatedSelectBox, WrappableLabel};
use crate::ui::images::ImageGetter;
use crate::ui::popups::options::OptionsPopup;
use crate::ui::popups::ConfirmPopup;
use crate::ui::screens::base_screen::BaseScreen;
use crate::ui::screens::world_screen::NotificationsScroll;
use crate::utils::display::{Display, ScreenMode, ScreenOrientation};

pub struct DisplayTab {
    options_popup: OptionsPopup,
}

impl DisplayTab {
    pub fn new(options_popup: OptionsPopup) -> Self {
        Self { options_popup }
    }

    pub fn render(&self, ctx: &mut Context, screen: &BaseScreen) -> GameResult<()> {
        let mut table = screen.create_table();
        table.pad(10.0);
        table.defaults().pad(2.5);

        let settings = &mut self.options_popup.settings;

        // Add Screen section
        let screen_heading = Text::new("Screen")
            .set_font_size(Constants::HEADING_FONT_SIZE)
            .set_color(Color::WHITE);
        table.add_label(screen_heading).colspan(2).row();

        // Add screen settings
        self.add_screen_size_select_box(&mut table, settings);
        self.add_screen_orientation_select_box(&mut table, settings);
        self.add_screen_mode_select_box(&mut table, settings);

        // Add map auto-scroll settings for desktop
        if Display::is_desktop() {
            self.add_map_auto_scroll_settings(&mut table, settings);
        }

        // Add separator
        table.add_separator();

        // Add Graphics section
        let graphics_heading = Text::new("Graphics")
            .set_font_size(Constants::HEADING_FONT_SIZE)
            .set_color(Color::WHITE);
        table.add_label(graphics_heading).colspan(2).row();

        // Add graphics settings
        self.add_tile_set_select_box(&mut table, settings);
        self.add_unit_set_select_box(&mut table, settings);
        self.add_skin_select_box(&mut table, settings);

        // Add separator
        table.add_separator();

        // Add UI section
        let ui_heading = Text::new("UI")
            .set_font_size(Constants::HEADING_FONT_SIZE)
            .set_color(Color::WHITE);
        table.add_label(ui_heading).colspan(2).row();

        // Add UI settings
        self.add_notification_scroll_select(&mut table, settings);
        self.add_ui_checkboxes(&mut table, settings);
        self.add_pedia_unit_art_size_slider(&mut table, settings);

        // Add separator
        table.add_separator();

        // Add Visual Hints section
        let visual_hints_heading = Text::new("Visual Hints")
            .set_font_size(Constants::HEADING_FONT_SIZE)
            .set_color(Color::WHITE);
        table.add_label(visual_hints_heading).colspan(2).row();

        // Add visual hints settings
        self.add_visual_hints_checkboxes(&mut table, settings);
        self.add_unit_icon_alpha_slider(&mut table, settings);

        // Add separator
        table.add_separator();

        // Add Performance section
        let performance_heading = Text::new("Performance")
            .set_font_size(Constants::HEADING_FONT_SIZE)
            .set_color(Color::WHITE);
        table.add_label(performance_heading).colspan(2).row();

        // Add performance settings
        self.add_continuous_rendering_checkbox(&mut table, settings);

        // Add continuous rendering description
        let continuous_rendering_description = "When disabled, saves battery life but certain animations will be suspended";
        let continuous_rendering_label = WrappableLabel::new(
            continuous_rendering_description,
            self.options_popup.tabs.pref_width,
            Color::ORANGE.brighten(0.7),
            14
        );
        continuous_rendering_label.set_wrap(true);
        table.add(continuous_rendering_label).colspan(2).pad_top(10.0).row();

        // Render the table
        table.render(ctx, screen)?;

        Ok(())
    }

    fn add_screen_size_select_box(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        table.add_label("Screen Size").left().fill_x();

        let screen_size_select_box = TranslatedSelectBox::new(
            ScreenSize::entries().iter().map(|s| s.name().to_string()).collect(),
            settings.screen_size.name().to_string()
        );

        screen_size_select_box.on_change(Box::new(move |value| {
            settings.screen_size = ScreenSize::from_name(&value);
            // Call the onChange callback to rebuild the UI
            self.options_popup.on_major_change();
        }));

        table.add(screen_size_select_box)
            .min_width(self.options_popup.select_box_min_width)
            .pad(10.0)
            .row();
    }

    fn add_screen_orientation_select_box(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        if !Display::has_orientation() {
            return;
        }

        table.add_label("Screen orientation").left().fill_x();

        let select_box = SelectBox::<ScreenOrientation>::new(screen.skin());
        let orientations: Vec<ScreenOrientation> = ScreenOrientation::entries().to_vec();
        select_box.set_items(orientations);
        select_box.set_selected(settings.display_orientation);

        select_box.on_change(Box::new(move |value| {
            settings.display_orientation = value;
            Display::set_orientation(value);
            // Call the onChange callback to rebuild the UI
            self.options_popup.on_major_change();
        }));

        table.add(select_box)
            .min_width(self.options_popup.select_box_min_width)
            .pad(10.0)
            .row();
    }

    fn add_screen_mode_select_box(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        table.add_label("Screen Mode").left().fill_x();

        let modes = Display::get_screen_modes();
        let current = modes.get(&settings.screen_mode).cloned();

        let select_box = SelectBox::<ScreenMode>::new(screen.skin());
        let mode_values: Vec<ScreenMode> = modes.values().cloned().collect();
        select_box.set_items(mode_values);
        select_box.set_selected(current);

        select_box.on_change(Box::new(move |value| {
            settings.refresh_window_size();
            let mode = value;
            settings.screen_mode = mode.get_id();
            Display::set_screen_mode(mode.get_id(), settings);
        }));

        table.add(select_box)
            .min_width(self.options_popup.select_box_min_width)
            .pad(10.0)
            .row();
    }

    fn add_map_auto_scroll_settings(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        // Add map auto-scroll checkbox
        let map_auto_scroll_checkbox = Checkbox::new("Map mouse auto-scroll", settings.map_auto_scroll);
        map_auto_scroll_checkbox.on_change(Box::new(move |value| {
            settings.map_auto_scroll = value;
            if GUI::is_world_loaded() {
                GUI::get_map().is_auto_scroll_enabled = settings.map_auto_scroll;
            }
        }));
        table.add(map_auto_scroll_checkbox);

        // Add scroll speed slider
        self.add_scroll_speed_slider(table, settings);
    }

    fn add_scroll_speed_slider(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        table.add_label("Map panning speed").left().fill_x();

        let scroll_speed_slider = Slider::new(
            0.2, 25.0, 0.2,
            settings.map_panning_speed
        );

        scroll_speed_slider.on_change(Box::new(move |value| {
            settings.map_panning_speed = value;
            settings.save();
            if GUI::is_world_loaded() {
                GUI::get_map().map_panning_speed = settings.map_panning_speed;
            }
        }));

        table.add(scroll_speed_slider)
            .min_width(self.options_popup.select_box_min_width)
            .pad(10.0)
            .row();
    }

    fn add_tile_set_select_box(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        table.add_label("Tileset").left().fill_x();

        let tile_set_select_box = SelectBox::<String>::new(screen.skin());
        let tile_sets = ImageGetter::get_available_tilesets();
        tile_set_select_box.set_items(tile_sets);
        tile_set_select_box.set_selected(settings.tile_set.clone());

        let unit_sets = ImageGetter::get_available_unitsets();

        tile_set_select_box.on_change(Box::new(move |value| {
            // Switch unitSet together with tileSet as long as one with the same name exists and both are selected
            if settings.tile_set == settings.unit_set && unit_sets.contains(&value) {
                settings.unit_set = Some(value.clone());
            }
            settings.tile_set = value;
            // ImageGetter ruleset should be correct no matter what screen we're on
            TileSetCache::assemble_tile_set_configs(ImageGetter::ruleset().mods);
            // Call the onChange callback to rebuild the UI
            self.options_popup.on_major_change();
        }));

        table.add(tile_set_select_box)
            .min_width(self.options_popup.select_box_min_width)
            .pad(10.0)
            .row();
    }

    fn add_unit_set_select_box(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        table.add_label("Unitset").left().fill_x();

        let unit_set_select_box = SelectBox::<String>::new(screen.skin());
        let mut unit_set_items = vec!["None".to_string()];
        let unit_sets = ImageGetter::get_available_unitsets();
        unit_set_items.extend(unit_sets);
        unit_set_select_box.set_items(unit_set_items);
        unit_set_select_box.set_selected(settings.unit_set.clone().unwrap_or_else(|| "None".to_string()));

        unit_set_select_box.on_change(Box::new(move |value| {
            settings.unit_set = if value != "None" { Some(value) } else { None };
            // ImageGetter ruleset should be correct no matter what screen we're on
            TileSetCache::assemble_tile_set_configs(ImageGetter::ruleset().mods);
            // Call the onChange callback to rebuild the UI
            self.options_popup.on_major_change();
        }));

        table.add(unit_set_select_box)
            .min_width(self.options_popup.select_box_min_width)
            .pad(10.0)
            .row();
    }

    fn add_skin_select_box(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        table.add_label("UI Skin").left().fill_x();

        let skin_select_box = SelectBox::<String>::new(screen.skin());
        let skins = ImageGetter::get_available_skins();
        skin_select_box.set_items(skins);
        skin_select_box.set_selected(settings.skin.clone());

        skin_select_box.on_change(Box::new(move |value| {
            settings.skin = value;
            // ImageGetter ruleset should be correct no matter what screen we're on
            SkinCache::assemble_skin_configs(ImageGetter::ruleset().mods);
            // Call the onChange callback to rebuild the UI
            self.options_popup.on_major_change();
        }));

        table.add(skin_select_box)
            .min_width(self.options_popup.select_box_min_width)
            .pad(10.0)
            .row();
    }

    fn add_notification_scroll_select(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        table.add_label("Notifications on world screen").left().fill_x();

        let select_box = TranslatedSelectBox::new(
            NotificationsScroll::UserSetting::entries().iter().map(|s| s.name().to_string()).collect(),
            settings.notification_scroll.clone()
        );

        select_box.on_change(Box::new(move |value| {
            settings.notification_scroll = value;
            GUI::set_update_world_on_next_render();
        }));

        table.add(select_box)
            .min_width(self.options_popup.select_box_min_width)
            .pad(10.0)
            .row();
    }

    fn add_ui_checkboxes(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        // Add show minimap checkbox
        let show_minimap_checkbox = Checkbox::new("Show minimap", settings.show_minimap);
        show_minimap_checkbox.on_change(Box::new(move |value| {
            settings.show_minimap = value;
            GUI::set_update_world_on_next_render();
        }));
        table.add(show_minimap_checkbox);

        // Add show tutorials checkbox
        let show_tutorials_checkbox = Checkbox::new("Show tutorials", settings.show_tutorials);
        show_tutorials_checkbox.on_change(Box::new(move |value| {
            settings.show_tutorials = value;
            GUI::set_update_world_on_next_render();
        }));
        table.add(show_tutorials_checkbox);

        // Add reset tutorials button
        self.add_reset_tutorials(table, settings);

        // Add show zoom buttons checkbox
        let show_zoom_buttons_checkbox = Checkbox::new("Show zoom buttons in world screen", settings.show_zoom_buttons);
        show_zoom_buttons_checkbox.on_change(Box::new(move |value| {
            settings.show_zoom_buttons = value;
        }));
        table.add(show_zoom_buttons_checkbox);

        // Add demographics scoreboard checkbox
        let use_demographics_checkbox = Checkbox::new("Experimental Demographics scoreboard", settings.use_demographics);
        use_demographics_checkbox.on_change(Box::new(move |value| {
            settings.use_demographics = value;
            GUI::set_update_world_on_next_render();
        }));
        table.add(use_demographics_checkbox);

        // Add forbid popup click behind checkbox
        let forbid_popup_click_checkbox = Checkbox::new("Never close popups by clicking outside", settings.forbid_popup_click_behind_to_close);
        forbid_popup_click_checkbox.on_change(Box::new(move |value| {
            settings.forbid_popup_click_behind_to_close = value;
        }));
        table.add(forbid_popup_click_checkbox);
    }

    fn add_reset_tutorials(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        let reset_tutorials_button = screen.create_text_button("Reset tutorials");

        reset_tutorials_button.on_click(Box::new(move || {
            let confirm_popup = ConfirmPopup::new(
                screen.stage(),
                "Do you want to reset completed tutorials?",
                "Reset"
            );

            confirm_popup.on_confirm(Box::new(move || {
                settings.tutorials_shown.clear();
                settings.tutorial_tasks_completed.clear();
                reset_tutorials_button.set_text("Done!");
                reset_tutorials_button.clear_listeners();
            }));

            confirm_popup.open(true);
        }));

        table.add(reset_tutorials_button).center().row();
    }

    fn add_pedia_unit_art_size_slider(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        table.add_label("Size of Unitset art in Civilopedia").left().fill_x();

        let unit_art_size_slider = Slider::new(
            0.0, 360.0, 1.0,
            settings.pedia_unit_art_size
        );

        unit_art_size_slider.on_change(Box::new(move |value| {
            settings.pedia_unit_art_size = value;
            GUI::set_update_world_on_next_render();
        }));

        unit_art_size_slider.set_snap_to_values(60.0, vec![0.0, 32.0, 48.0, 64.0, 96.0, 120.0, 180.0, 240.0, 360.0]);

        table.add(unit_art_size_slider)
            .min_width(self.options_popup.select_box_min_width)
            .pad(10.0)
            .row();
    }

    fn add_visual_hints_checkboxes(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        // Add show unit movements checkbox
        let show_unit_movements_checkbox = Checkbox::new("Show unit movement arrows", settings.show_unit_movements);
        show_unit_movements_checkbox.on_change(Box::new(move |value| {
            settings.show_unit_movements = value;
            GUI::set_update_world_on_next_render();
        }));
        table.add(show_unit_movements_checkbox);

        // Add show settlers suggested city locations checkbox
        let show_settlers_locations_checkbox = Checkbox::new(
            "Show suggested city locations for units that can found cities",
            settings.show_settlers_suggested_city_locations
        );
        show_settlers_locations_checkbox.on_change(Box::new(move |value| {
            settings.show_settlers_suggested_city_locations = value;
            GUI::set_update_world_on_next_render();
        }));
        table.add(show_settlers_locations_checkbox);

        // Add show tile yields checkbox
        let show_tile_yields_checkbox = Checkbox::new("Show tile yields", settings.show_tile_yields);
        show_tile_yields_checkbox.on_change(Box::new(move |value| {
            settings.show_tile_yields = value;
            GUI::set_update_world_on_next_render();
        }));
        table.add(show_tile_yields_checkbox);

        // Add show worked tiles checkbox
        let show_worked_tiles_checkbox = Checkbox::new("Show worked tiles", settings.show_worked_tiles);
        show_worked_tiles_checkbox.on_change(Box::new(move |value| {
            settings.show_worked_tiles = value;
            GUI::set_update_world_on_next_render();
        }));
        table.add(show_worked_tiles_checkbox);

        // Add show resources and improvements checkbox
        let show_resources_checkbox = Checkbox::new(
            "Show resources and improvements",
            settings.show_resources_and_improvements
        );
        show_resources_checkbox.on_change(Box::new(move |value| {
            settings.show_resources_and_improvements = value;
            GUI::set_update_world_on_next_render();
        }));
        table.add(show_resources_checkbox);

        // Add show pixel improvements checkbox
        let show_pixel_improvements_checkbox = Checkbox::new("Show pixel improvements", settings.show_pixel_improvements);
        show_pixel_improvements_checkbox.on_change(Box::new(move |value| {
            settings.show_pixel_improvements = value;
            GUI::set_update_world_on_next_render();
        }));
        table.add(show_pixel_improvements_checkbox);
    }

    fn add_unit_icon_alpha_slider(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        table.add_label("Unit icon opacity").left().fill_x();

        let get_tip_text = |value: f32| -> String {
            format!("{:.0}%", value * 100.0)
        };

        let unit_icon_alpha_slider = Slider::new(
            0.0, 1.0, 0.1,
            settings.unit_icon_opacity,
            Some(get_tip_text)
        );

        unit_icon_alpha_slider.on_change(Box::new(move |value| {
            settings.unit_icon_opacity = value;
            GUI::set_update_world_on_next_render();
        }));

        table.add(unit_icon_alpha_slider)
            .min_width(self.options_popup.select_box_min_width)
            .pad(10.0)
            .row();
    }

    fn add_continuous_rendering_checkbox(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        let continuous_rendering_checkbox = Checkbox::new("Continuous rendering", settings.continuous_rendering);
        continuous_rendering_checkbox.on_change(Box::new(move |value| {
            settings.continuous_rendering = value;
            // Set continuous rendering in ggez
            // Note: This would need to be implemented in the ggez context
            // For now, we'll just update the setting
        }));
        table.add(continuous_rendering_checkbox);
    }
}