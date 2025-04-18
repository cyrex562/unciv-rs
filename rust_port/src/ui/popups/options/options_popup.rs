use std::collections::HashMap;
use std::time::Duration;
use ggez::graphics::{Color, DrawParam, Text};
use ggez::{Context, GameResult};

use crate::constants::Constants;
use crate::models::metadata::{GameSettings, GameSetting};
use crate::ui::components::widgets::{Button, Checkbox, SettingsSelect, TabbedPager, UncivTextField};
use crate::ui::popups::{AuthPopup, Popup};
use crate::ui::screens::base_screen::BaseScreen;
use crate::ui::screens::main_menu_screen::MainMenuScreen;
use crate::ui::screens::world_screen::WorldScreen;
use crate::utils::concurrency::Concurrency;
use crate::utils::media_finder::IMediaFinder;
use crate::utils::multiplayer::{Multiplayer, MultiplayerAuthException, FileStorageRateLimitReached};
use crate::utils::translation::tr;
use crate::utils::with_gl_context;
use crate::UncivGame;

/// The Options (Settings) Popup
pub struct OptionsPopup {
    game: UncivGame,
    settings: GameSettings,
    tabs: TabbedPager,
    select_box_min_width: f32,
    tab_min_width: f32,
    select_page: i32,
    on_close: Box<dyn Fn()>,
}

impl OptionsPopup {
    /// Default page to show when opening the options popup
    pub const DEFAULT_PAGE: i32 = 2; // Gameplay

    /// Create a new OptionsPopup
    pub fn new(
        screen: &mut BaseScreen,
        select_page: i32,
        with_debug: bool,
        on_close: Box<dyn Fn()>
    ) -> Self {
        let game = screen.get_game();
        let settings = game.settings.clone();

        let mut popup = Self {
            game,
            settings,
            tabs: TabbedPager::new(),
            select_box_min_width: 0.0,
            tab_min_width: 0.0,
            select_page,
            on_close,
        };

        popup.init(screen, with_debug);
        popup
    }

    /// Initialize the popup
    fn init(&mut self, screen: &mut BaseScreen, with_debug: bool) {
        // Set click behind to close
        self.set_click_behind_to_close(true);

        // Add completed tutorial task
        if self.settings.add_completed_tutorial_task("Open the options table") {
            if let Some(world_screen) = screen.downcast_ref::<WorldScreen>() {
                world_screen.set_should_update(true);
            }
        }

        // Set up dimensions
        let stage_width = screen.get_stage_width();
        let stage_height = screen.get_stage_height();

        self.select_box_min_width = if stage_width < 600.0 { 200.0 } else { 240.0 };
        let tab_max_width = if self.is_portrait() { stage_width - 10.0 } else { 0.8 * stage_width };
        self.tab_min_width = 0.6 * stage_width;
        let tab_max_height = 0.8 * stage_height;

        // Create tabbed pager
        self.tabs = TabbedPager::new_with_params(
            self.tab_min_width,
            tab_max_width,
            0.0,
            tab_max_height,
            21, // header font size
            Color::CLEAR,
            8 // capacity
        );

        // Add tabs
        self.add_tabs(screen, with_debug);

        // Pack and center
        self.pack();
        self.center(screen.get_stage());
    }

    /// Add all tabs to the popup
    fn add_tabs(&mut self, screen: &mut BaseScreen, with_debug: bool) {
        // About tab
        self.tabs.add_page(
            "About",
            self.about_tab(),
            self.get_external_image("Icons/Unciv128.png"),
            24.0
        );

        // Display tab
        self.tabs.add_page(
            "Display",
            self.display_tab(Box::new(move || self.reload_world_and_options())),
            self.get_image("UnitPromotionIcons/Scouting"),
            24.0
        );

        // Gameplay tab
        self.tabs.add_page(
            "Gameplay",
            self.gameplay_tab(),
            self.get_image("OtherIcons/Options"),
            24.0
        );

        // Automation tab
        self.tabs.add_page(
            "Automation",
            self.automation_tab(),
            self.get_image("OtherIcons/NationSwap"),
            24.0
        );

        // Language tab
        self.tabs.add_page(
            "Language",
            self.language_tab(Box::new(move || self.reload_world_and_options())),
            self.get_image(&format!("FlagIcons/{}", self.settings.language)),
            24.0
        );

        // Sound tab
        self.tabs.add_page(
            "Sound",
            self.sound_tab(),
            self.get_image("OtherIcons/Speaker"),
            24.0
        );

        // Multiplayer tab
        self.tabs.add_page(
            "Multiplayer",
            self.multiplayer_tab(),
            self.get_image("OtherIcons/Multiplayer"),
            24.0
        );

        // Keys tab (if keyboard available)
        if self.is_keyboard_available() {
            self.tabs.add_page(
                "Keys",
                self.key_bindings_tab(self.tab_min_width - 40.0), // 40 = padding
                self.get_image("OtherIcons/Keyboard"),
                24.0
            );
        }

        // Advanced tab
        self.tabs.add_page(
            "Advanced",
            self.advanced_tab(Box::new(move || self.reload_world_and_options())),
            self.get_image("OtherIcons/Settings"),
            24.0
        );

        // Mod check tab (if there are mods)
        if self.ruleset_cache_size() > self.base_ruleset_entries_size() {
            self.tabs.add_page(
                "Locate mod errors",
                self.mod_check_tab(screen),
                self.get_image("OtherIcons/Mods"),
                24.0
            );
        }

        // Debug tab (if debug or secret keys pressed)
        if with_debug || self.are_secret_keys_pressed() {
            self.tabs.add_page(
                "Debug",
                self.debug_tab(),
                self.get_image("OtherIcons/SecretOptions"),
                24.0
            );
        }

        // Add close button
        self.tabs.decorate_header(self.get_close_button(Box::new(move || {
            self.game.music_controller.on_change(None);
            self.center(self.get_stage());
            self.tabs.select_page(-1, false);
            self.settings.save();
            (self.on_close)(); // activate the passed 'on close' callback
            self.close(); // close this popup
        })));
    }

    /// Set visibility of the popup
    pub fn set_visible(&mut self, visible: bool) {
        super::set_visible(visible);
        if !visible {
            return;
        }
        if self.tabs.get_active_page() < 0 {
            self.tabs.select_page(self.select_page);
        }
    }

    /// Reload this Popup after major changes (resolution, tileset, language, font)
    fn reload_world_and_options(&self) {
        Concurrency::run("Reload from options", move || {
            with_gl_context(|| {
                // We have to run set_skin before the screen is rebuild else changing skins
                // would only load the new SkinConfig after the next rebuild
                BaseScreen::set_skin();
            });

            let screen = UncivGame::current().get_screen();
            if let Some(world_screen) = screen.downcast_ref::<WorldScreen>() {
                UncivGame::current().reload_world_screen();
            } else if screen.is::<MainMenuScreen>() {
                with_gl_context(|| {
                    UncivGame::current().replace_current_screen(Box::new(MainMenuScreen::new()));
                });
            }

            with_gl_context(|| {
                if let Some(screen) = UncivGame::current().get_screen() {
                    screen.open_options_popup(self.tabs.get_active_page());
                }
            });
        });
    }

    /// Call if an option change might trigger a Screen.resize
    ///
    /// Does nothing if any Popup (which can only be this one) is still open after a short delay and context yield.
    /// Reason: A resize might relaunch the parent screen ([MainMenuScreen] is [RecreateOnResize]) and thus close this Popup.
    pub fn reopen_after_display_layout_change(&self) {
        Concurrency::run("Reload from options", move || {
            std::thread::sleep(Duration::from_millis(100));
            with_gl_context(|| {
                if let Some(screen) = UncivGame::current().get_screen() {
                    if screen.has_open_popups() {
                        return; // e.g. Orientation auto to fixed while auto is already the new orientation
                    }
                    screen.open_options_popup(self.tabs.get_active_page());
                }
            });
        });
    }

    /// Add a checkbox to a table
    pub fn add_checkbox(
        &self,
        table: &mut BaseScreen,
        text: &str,
        initial_state: bool,
        update_world: bool,
        new_row: bool,
        action: Box<dyn Fn(bool)>
    ) {
        let mut checkbox = Checkbox::new(text, initial_state);
        checkbox.set_on_change(Box::new(move |checked| {
            action(checked);
            if let Some(world_screen) = self.get_world_screen_if_active() {
                if update_world {
                    world_screen.set_should_update(true);
                }
            }
        }));

        if new_row {
            table.add(checkbox).colspan(2).left().row();
        } else {
            table.add(checkbox).left();
        }
    }

    /// Add a checkbox to a table with a settings property
    pub fn add_checkbox_with_settings(
        &self,
        table: &mut BaseScreen,
        text: &str,
        settings_property: &mut bool,
        update_world: bool,
        action: Box<dyn Fn(bool)>
    ) {
        self.add_checkbox(
            table,
            text,
            *settings_property,
            update_world,
            true,
            Box::new(move |checked| {
                action(checked);
                *settings_property = checked;
            })
        );
    }

    // Helper methods that would be implemented in the actual code
    fn is_portrait(&self) -> bool {
        // Implementation would check if the screen is in portrait mode
        false
    }

    fn get_external_image(&self, path: &str) -> ggez::graphics::Image {
        // Implementation would load an external image
        ggez::graphics::Image::new_empty()
    }

    fn get_image(&self, path: &str) -> ggez::graphics::Image {
        // Implementation would load an image
        ggez::graphics::Image::new_empty()
    }

    fn is_keyboard_available(&self) -> bool {
        // Implementation would check if keyboard is available
        true
    }

    fn ruleset_cache_size(&self) -> usize {
        // Implementation would return the ruleset cache size
        0
    }

    fn base_ruleset_entries_size(&self) -> usize {
        // Implementation would return the base ruleset entries size
        0
    }

    fn are_secret_keys_pressed(&self) -> bool {
        // Implementation would check if secret keys are pressed
        false
    }

    fn get_close_button(&self, action: Box<dyn Fn()>) -> Button {
        // Implementation would create a close button
        Button::new("Close")
    }

    fn get_world_screen_if_active(&self) -> Option<&WorldScreen> {
        // Implementation would get the world screen if active
        None
    }

    // Tab creation methods that would be implemented in the actual code
    fn about_tab(&self) -> BaseScreen {
        // Implementation would create the about tab
        BaseScreen::new()
    }

    fn display_tab(&self, reload_callback: Box<dyn Fn()>) -> BaseScreen {
        // Implementation would create the display tab
        BaseScreen::new()
    }

    fn gameplay_tab(&self) -> BaseScreen {
        // Implementation would create the gameplay tab
        BaseScreen::new()
    }

    fn automation_tab(&self) -> BaseScreen {
        // Implementation would create the automation tab
        BaseScreen::new()
    }

    fn language_tab(&self, reload_callback: Box<dyn Fn()>) -> BaseScreen {
        // Implementation would create the language tab
        BaseScreen::new()
    }

    fn sound_tab(&self) -> BaseScreen {
        // Implementation would create the sound tab
        BaseScreen::new()
    }

    fn multiplayer_tab(&self) -> BaseScreen {
        // Implementation would create the multiplayer tab
        BaseScreen::new()
    }

    fn key_bindings_tab(&self, width: f32) -> BaseScreen {
        // Implementation would create the key bindings tab
        BaseScreen::new()
    }

    fn advanced_tab(&self, reload_callback: Box<dyn Fn()>) -> BaseScreen {
        // Implementation would create the advanced tab
        BaseScreen::new()
    }

    fn mod_check_tab(&self, screen: &mut BaseScreen) -> BaseScreen {
        // Implementation would create the mod check tab
        BaseScreen::new()
    }

    fn debug_tab(&self) -> BaseScreen {
        // Implementation would create the debug tab
        BaseScreen::new()
    }
}

// Implement Popup trait for OptionsPopup
impl Popup for OptionsPopup {
    fn get_stage(&self) -> &ggez::graphics::Stage {
        // Implementation would return the stage
        &self.game.get_stage()
    }

    fn get_stage_mut(&mut self) -> &mut ggez::graphics::Stage {
        // Implementation would return the mutable stage
        self.game.get_stage_mut()
    }

    fn get_stage_width(&self) -> f32 {
        // Implementation would return the stage width
        self.game.get_stage_width()
    }

    fn get_stage_height(&self) -> f32 {
        // Implementation would return the stage height
        self.game.get_stage_height()
    }

    fn center(&self, parent: &ggez::graphics::Actor) {
        // Implementation would center the popup
    }

    fn pack(&mut self) {
        // Implementation would pack the popup
    }

    fn close(&mut self) {
        // Implementation would close the popup
    }

    fn set_click_behind_to_close(&mut self, value: bool) {
        // Implementation would set click behind to close
    }
}