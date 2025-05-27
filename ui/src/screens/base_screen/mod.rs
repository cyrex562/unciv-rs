pub mod stage_mouse_over_debug;
pub mod tutorial_controller;
pub mod unciv_stage;

pub mod screen;


use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use std::collections::HashSet;

use crate::game::UncivGame;
use crate::models::TutorialTrigger;
use crate::ui::components::keyboard::KeyShortcutDispatcher;
use crate::ui::popups::Popup;
use crate::ui::skin::SkinStrings;

/// Trait for screens that need to be recreated on resize
pub trait RecreateOnResize {
    fn recreate(&self) -> Box<dyn BaseScreen>;
}

/// Base screen implementation that other screens inherit from
pub trait BaseScreen: Send + Sync {
    /// Get the game instance
    fn game(&self) -> &UncivGame;

    /// Get the screen dimensions
    fn dimensions(&self) -> Vec2;

    /// Get the tutorial controller
    fn tutorial_controller(&self) -> &TutorialController;

    /// Get the global keyboard shortcuts
    fn global_shortcuts(&self) -> &KeyShortcutDispatcher;

    /// Show the screen
    fn show(&mut self) {}

    /// Render the screen
    fn render(&mut self, ctx: &mut EguiContexts) {
        // Clear the screen with the background color
        let clear_color = egui::Color32::from_rgba_premultiplied(0, 0, 51, 255);
        ctx.ctx_mut().set_visuals(egui::Visuals::dark());

        // Draw the UI
        egui::CentralPanel::default().show(ctx.ctx_mut(), |ui| {
            // UI rendering will be handled by derived screens
        });
    }

    /// Handle screen resize
    fn resize(&mut self, width: f32, height: f32) {
        if !self.is_recreate_on_resize() {
            // Update viewport
        } else if self.dimensions() != Vec2::new(width, height) {
            // Recreate screen if dimensions changed
            if let Some(game) = self.game().as_mut() {
                game.replace_current_screen(self.recreate());
            }
        }
    }

    /// Pause the screen
    fn pause(&mut self) {}

    /// Resume the screen
    fn resume(&mut self) {}

    /// Hide the screen
    fn hide(&mut self) {}

    /// Dispose of screen resources
    fn dispose(&mut self) {}

    /// Display a tutorial
    fn display_tutorial(&mut self, tutorial: TutorialTrigger, test: Option<Box<dyn Fn() -> bool>>) {
        if !self.game().settings.show_tutorials {
            return;
        }
        if self.game().settings.tutorials_shown.contains(&tutorial.name) {
            return;
        }
        if let Some(test_fn) = test {
            if !test_fn() {
                return;
            }
        }
        self.tutorial_controller().show_tutorial(tutorial);
    }

    /// Check if screen is in portrait orientation
    fn is_portrait(&self) -> bool {
        let dims = self.dimensions();
        dims.y > dims.x
    }

    /// Check if screen is in cramped portrait mode
    fn is_cramped_portrait(&self) -> bool {
        self.is_portrait() && self.dimensions().y <= 700.0
    }

    /// Check if screen is narrower than 4:3
    fn is_narrower_than_4to3(&self) -> bool {
        let dims = self.dimensions();
        dims.x / dims.y < 4.0 / 3.0
    }

    /// Open the options popup
    fn open_options_popup(&mut self, starting_page: i32, with_debug: bool, on_close: Box<dyn Fn()>) {
        // TODO: Implement options popup
    }

    /// Get the civilopedia ruleset
    fn get_civilopedia_ruleset(&self) -> Ruleset {
        if let Some(world_screen) = self.game().world_screen.as_ref() {
            return world_screen.game_info.ruleset.clone();
        }

        // Try to get ruleset from main menu screen
        if let Some(main_menu) = self.game().get_screens_of_type::<MainMenuScreen>().first() {
            return main_menu.get_civilopedia_ruleset();
        }

        // Default to Civ V G&K ruleset
        RulesetCache::get(BaseRuleset::CivV_GnK.full_name()).unwrap()
    }

    /// Open the civilopedia
    fn open_civilopedia(&mut self, link: String) {
        self.open_civilopedia_with_ruleset(self.get_civilopedia_ruleset(), link);
    }

    /// Open the civilopedia with a specific ruleset
    fn open_civilopedia_with_ruleset(&mut self, ruleset: Ruleset, link: String) {
        if let Some(game) = self.game().as_mut() {
            game.push_screen(Box::new(CivilopediaScreen::new(ruleset, link)));
        }
    }

    /// Check if screen needs recreation on resize
    fn is_recreate_on_resize(&self) -> bool {
        false
    }

    /// Recreate the screen (for RecreateOnResize trait)
    fn recreate(&self) -> Box<dyn BaseScreen> {
        unimplemented!("Screen does not implement RecreateOnResize")
    }
}

/// Static settings for BaseScreen
pub struct BaseScreenSettings {
    pub enable_scene_debug: bool,
    pub clear_color: Color,
    pub skin: Skin,
    pub skin_strings: SkinStrings,
}

impl Default for BaseScreenSettings {
    fn default() -> Self {
        Self {
            enable_scene_debug: false,
            clear_color: Color::rgb(0.0, 0.0, 0.2),
            skin: Skin::default(),
            skin_strings: SkinStrings::default(),
        }
    }
}

impl BaseScreenSettings {
    /// Set up the skin and related settings
    pub fn set_skin(&mut self) {
        // Reset fonts
        Fonts::reset();

        // Create new skin strings
        self.skin_strings = SkinStrings::new();

        // Set up skin
        self.skin = Skin::new();

        // Add default styles
        self.skin.add_style("default-clear", self.clear_color);
        self.skin.add_font("native", Fonts::default());

        // Add UI elements
        self.skin.add_drawable("rounded_edge_rectangle",
                               self.skin_strings.get_ui_background("", self.skin_strings.rounded_edge_rectangle_shape));
        self.skin.add_drawable("rectangle", ImageGetter::get_drawable(""));
        self.skin.add_drawable("circle", ImageGetter::get_circle_drawable().with_min_size(20.0, 20.0));
        self.skin.add_drawable("scrollbar",
                               ImageGetter::get_drawable("").with_min_size(10.0, 10.0));
        self.skin.add_drawable("rectangle_with_outline",
                               self.skin_strings.get_ui_background("", self.skin_strings.rectangle_with_outline_shape));
        self.skin.add_drawable("select_box",
                               self.skin_strings.get_ui_background("", self.skin_strings.select_box_shape));
        self.skin.add_drawable("select_box_pressed",
                               self.skin_strings.get_ui_background("", self.skin_strings.select_box_pressed_shape));
        self.skin.add_drawable("checkbox",
                               self.skin_strings.get_ui_background("", self.skin_strings.checkbox_shape));
        self.skin.add_drawable("checkbox_pressed",
                               self.skin_strings.get_ui_background("", self.skin_strings.checkbox_pressed_shape));

        // Load skin configuration
        self.skin.load("assets/skin.json");

        // Set up text styles
        self.skin.set_text_button_style(Fonts::default());
        self.skin.set_checkbox_style(Fonts::default(), Color::WHITE);
        self.skin.set_label_style(Fonts::default(), Color::WHITE);
        self.skin.set_text_field_style(Fonts::default());
        self.skin.set_select_box_style(Fonts::default());

        // Update clear color from skin config
        self.clear_color = self.skin_strings.skin_config.clear_color;
    }
}