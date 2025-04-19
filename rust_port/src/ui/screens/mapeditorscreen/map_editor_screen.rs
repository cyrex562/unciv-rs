use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui::{Color32, Ui, Vec2};

use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::basescreen::RecreateOnResize;
use crate::ui::components::widgets::unciv_text_field::UncivTextField;
use crate::ui::components::input::key_char_and_code::KeyCharAndCode;
use crate::ui::components::input::key_shortcut_dispatcher_veto::KeyShortcutDispatcherVeto;
use crate::ui::components::input::keyboard_panning_listener::KeyboardPanningListener;
use crate::ui::components::tilegroups::tile_group::TileGroup;
use crate::ui::images::image_getter::ImageGetter;
use crate::ui::images::image_with_custom_size::ImageWithCustomSize;
use crate::ui::popups::confirm_popup::ConfirmPopup;
use crate::ui::popups::toast_popup::ToastPopup;
use crate::ui::screens::worldscreen::zoom_button_pair::ZoomButtonPair;
use crate::logic::map::map_parameters::MapParameters;
use crate::logic::map::map_shape::MapShape;
use crate::logic::map::map_size::MapSize;
use crate::logic::map::map_type::MapType;
use crate::logic::map::tile_map::TileMap;
use crate::logic::map::tile::Tile;
use crate::models::metadata::base_ruleset::BaseRuleset;
use crate::models::metadata::game_parameters::GameParameters;
use crate::models::metadata::game_setup_info::GameSetupInfo;
use crate::models::ruleset::ruleset::Ruleset;
use crate::models::ruleset::ruleset_cache::RulesetCache;
use crate::utils::concurrency::Concurrency;
use crate::utils::dispatcher::Dispatcher;
use crate::utils::log::Log;

use super::editor_map_holder::EditorMapHolder;
use super::map_editor_main_tabs::MapEditorMainTabs;
use super::map_editor_tools_drawer::MapEditorToolsDrawer;
use super::tabs::map_editor_options_tab::MapEditorOptionsTab;

/// The map editor screen
pub struct MapEditorScreen {
    /// The map being edited, with mod list for that map
    pub tile_map: Rc<RefCell<TileMap>>,
    /// Flag indicating the map should be saved
    pub is_dirty: bool,
    /// The parameters to use for new maps, and the UI-shown mod list (which can be applied to the active map)
    pub new_map_parameters: MapParameters,
    /// RuleSet corresponding to tile_map's mod list
    pub ruleset: Rc<RefCell<Ruleset>>,
    /// Set only by loading a map from file and used only by mods tab
    pub mods_tab_needs_refresh: bool,
    /// Set by loading a map or changing ruleset and used only by the edit tabs
    pub edit_tabs_need_refresh: bool,
    /// Set on load, generate or paint natural wonder - used to read nat wonders for the view tab
    pub natural_wonders_need_refresh: bool,
    /// Copy of same field in MapEditorOptionsTab
    pub tile_match_fuzziness: MapEditorOptionsTab::TileMatchFuzziness,
    /// UI
    pub map_holder: Rc<RefCell<EditorMapHolder>>,
    pub tabs: Rc<RefCell<MapEditorMainTabs>>,
    pub tile_click_handler: Option<Box<dyn Fn(&Tile)>>,
    pub zoom_controller: Option<Rc<RefCell<ZoomButtonPair>>>,
    pub description_text_field: UncivTextField,
    /// Control of background jobs - make them cancel on context changes like exit editor or resize screen
    jobs: Vec<Job>,
    /// Overlay image
    image_overlay: Option<ImageWithCustomSize>,
    /// Overlay file
    pub overlay_file: Option<PathBuf>,
    /// Overlay alpha
    pub overlay_alpha: f32,
}

impl MapEditorScreen {
    pub fn new(map: Option<TileMap>) -> Self {
        let (tile_map, ruleset) = if let Some(map) = map {
            let ruleset = map.ruleset.clone().unwrap_or_else(|| {
                RulesetCache::get_complex_ruleset(&map.map_parameters)
            });
            (Rc::new(RefCell::new(map)), Rc::new(RefCell::new(ruleset)))
        } else {
            let ruleset = RulesetCache::get(BaseRuleset::Civ_V_GnK.full_name).unwrap();
            let mut tile_map = TileMap::new(MapSize::Tiny.radius, ruleset.clone(), false);
            tile_map.map_parameters.map_size = MapSize::Tiny;
            (Rc::new(RefCell::new(tile_map)), Rc::new(RefCell::new(ruleset)))
        };

        let new_map_parameters = Self::get_default_parameters();
        let description_text_field = UncivTextField::new("Enter a description for the users of this map");

        if let Some(map) = map {
            description_text_field.set_text(map.description);
        }

        let mut screen = Self {
            tile_map,
            is_dirty: false,
            new_map_parameters,
            ruleset,
            mods_tab_needs_refresh: false,
            edit_tabs_need_refresh: false,
            natural_wonders_need_refresh: false,
            tile_match_fuzziness: MapEditorOptionsTab::TileMatchFuzziness::CompleteMatch,
            map_holder: Rc::new(RefCell::new(EditorMapHolder::new(
                Rc::new(RefCell::new(BaseScreen::new())),
                self.tile_map.clone(),
                Box::new(|_| {}),
            ))),
            tabs: Rc::new(RefCell::new(MapEditorMainTabs::new(Rc::new(RefCell::new(self.clone()))))),
            tile_click_handler: None,
            zoom_controller: None,
            description_text_field,
            jobs: Vec::new(),
            image_overlay: None,
            overlay_file: None,
            overlay_alpha: 0.33,
        };

        screen.map_holder = screen.new_map_holder();
        screen.is_dirty = false;
        screen.description_text_field.on_change(Box::new(|| {
            screen.is_dirty = true;
        }));

        screen.tabs = Rc::new(RefCell::new(MapEditorMainTabs::new(Rc::new(RefCell::new(screen.clone())))));
        MapEditorToolsDrawer::new(screen.tabs.clone(), screen.stage.clone(), screen.map_holder.clone());

        // The top level pager assigns its own key bindings, but making nested TabbedPagers bind keys
        // so all levels select to show the tab in question is too complex. Sub-Tabs need to maintain
        // the key binding here and the used key in their `addPage`s again for the tooltips.
        let tabs_clone = screen.tabs.clone();
        let select_generate_page = move |index: i32| {
            let mut tabs = tabs_clone.borrow_mut();
            tabs.select_page(1);
            tabs.generate.select_page(index);
        };

        screen.global_shortcuts.add(KeyCharAndCode::ctrl('n'), Box::new(move || {
            select_generate_page(0);
        }));
        screen.global_shortcuts.add(KeyCharAndCode::ctrl('g'), Box::new(move || {
            select_generate_page(1);
        }));
        screen.global_shortcuts.add(KeyCharAndCode::BACK, Box::new(move || {
            screen.close_editor();
        }));

        screen
    }

    fn get_default_parameters() -> MapParameters {
        let last_setup = UncivGame::current().settings.last_game_setup
            .unwrap_or_else(|| GameSetupInfo::new(GameParameters::new(), MapParameters::new()));

        let mut parameters = last_setup.map_parameters.clone();
        parameters.reseed();
        parameters.mods.retain(|mod_name| {
            !RulesetCache::get_sorted_base_rulesets().contains(mod_name)
        });

        parameters
    }

    pub fn save_default_parameters(parameters: &MapParameters) {
        let settings = &mut UncivGame::current().settings;
        let game_parameters = settings.last_game_setup
            .as_ref()
            .map(|setup| setup.game_parameters.clone())
            .unwrap_or_else(GameParameters::new);

        settings.last_game_setup = Some(GameSetupInfo::new(
            game_parameters,
            parameters.clone(),
        ));

        settings.save();
    }

    pub fn get_tools_width(&self) -> f32 {
        self.stage.width * 0.4
    }

    pub fn set_world_wrap(&mut self, new_value: bool) {
        if new_value == self.tile_map.borrow().map_parameters.world_wrap {
            return;
        }

        self.set_world_wrap_fix_odd_width(new_value);

        if new_value && self.overlay_file.is_some() {
            self.overlay_file = None;
            ToastPopup::new(
                "An overlay image is incompatible with world wrap and was deactivated.",
                self.stage.clone(),
                4000,
            ).open();
            self.tabs.borrow_mut().options.update();
        }

        self.recreate_map_holder();
    }

    fn set_world_wrap_fix_odd_width(&mut self, new_value: bool) {
        let mut map_parameters = self.tile_map.borrow_mut().map_parameters;

        // Turning *off* WW and finding an odd width means it must have been rounded
        // down by the TileMap constructor - fix so we can turn it back on later
        if map_parameters.world_wrap && map_parameters.map_size.width % 2 != 0 && map_parameters.shape == MapShape::Rectangular {
            map_parameters.map_size.width -= 1;
        }

        map_parameters.world_wrap = new_value;
    }

    fn recreate_map_holder<F>(&mut self, action_while_removed: F)
    where
        F: FnOnce(),
    {
        let saved_scale = self.map_holder.borrow().scale_x;
        self.clear_overlay_images();
        self.map_holder.borrow_mut().remove();
        action_while_removed();
        self.map_holder = self.new_map_holder();
        self.map_holder.borrow_mut().zoom(saved_scale);
    }

    fn new_map_holder(&self) -> Rc<RefCell<EditorMapHolder>> {
        ImageGetter::set_new_ruleset(&self.ruleset.borrow());
        // setNewRuleset is missing some graphics - those "EmojiIcons"&co already rendered as font characters
        // so to get the "Water" vs "Gold" icons when switching between Deciv and Vanilla to render properly,
        // we will need to ditch the already rendered font glyphs. Fonts.resetFont is not sufficient,
        // the skin seems to clone a separate copy of the Fonts singleton, proving that kotlin 'object'
        // are not really guaranteed to exist in one instance only.
        self.set_skin();

        let mut tile_map = self.tile_map.borrow_mut();
        tile_map.set_transients(&self.ruleset.borrow(), false);
        tile_map.set_starting_locations_transients();
        UncivGame::current().translations.translation_active_mods = self.ruleset.borrow().mods.clone();

        let tile_click_handler = self.tile_click_handler.clone();
        let new_holder = EditorMapHolder::new(
            Rc::new(RefCell::new(self.clone())),
            self.tile_map.clone(),
            Box::new(move |tile| {
                if let Some(handler) = &tile_click_handler {
                    handler(tile);
                }
            }),
        );

        // Remove old panning listeners
        for listener in self.stage.root.listeners.iter()
            .filter(|l| l.is::<KeyboardPanningListener>()) {
            self.stage.remove_listener(listener);
        }

        new_holder.borrow_mut().map_panning_speed = UncivGame::current().settings.map_panning_speed;
        self.stage.add_listener(KeyboardPanningListener::new(new_holder.clone(), false));

        if Gdx::app().type_() == Application::ApplicationType::Desktop {
            new_holder.borrow_mut().is_auto_scroll_enabled = UncivGame::current().settings.map_auto_scroll;
        }

        self.add_overlay_to_map_holder(new_holder.borrow().actor.clone());

        self.stage.root.add_actor_at(0, new_holder.clone());
        self.stage.scroll_focus = Some(new_holder.clone());

        self.is_dirty = true;
        self.mods_tab_needs_refresh = true;
        self.edit_tabs_need_refresh = true;
        self.natural_wonders_need_refresh = true;

        if UncivGame::current().settings.show_zoom_buttons {
            let zoom_controller = ZoomButtonPair::new(new_holder.clone());
            zoom_controller.borrow_mut().set_position(10.0, 10.0);
            self.stage.add_actor(zoom_controller.clone());
            self.zoom_controller = Some(zoom_controller);
        }

        new_holder
    }

    pub fn load_map(&mut self, map: TileMap, new_ruleset: Option<Ruleset>, select_page: i32) {
        self.clear_overlay_images();
        self.map_holder.borrow_mut().remove();
        self.tile_map = Rc::new(RefCell::new(map.clone()));
        self.description_text_field.set_text(map.description);
        self.ruleset = Rc::new(RefCell::new(
            new_ruleset.unwrap_or_else(|| RulesetCache::get_complex_ruleset(&map.map_parameters))
        ));
        self.map_holder = self.new_map_holder();
        self.is_dirty = false;
        Gdx::input().input_processor = Some(self.stage.clone());
        self.tabs.borrow_mut().select_page(select_page);  // must be done _after_ resetting inputProcessor!
    }

    pub fn get_map_clone_for_save(&self) -> TileMap {
        let mut map = self.tile_map.borrow().clone();
        map.set_transients(false);
        map
    }

    pub fn apply_ruleset(&mut self, new_ruleset: Ruleset, new_base_ruleset: String, mods: HashSet<String>) {
        self.recreate_map_holder(|| {
            let mut map_parameters = self.tile_map.borrow_mut().map_parameters;
            map_parameters.base_ruleset = new_base_ruleset;
            map_parameters.mods = mods;

            let mut tile_map = self.tile_map.borrow_mut();
            tile_map.ruleset = Some(new_ruleset.clone());

            self.ruleset = Rc::new(RefCell::new(new_ruleset));
        });

        self.mods_tab_needs_refresh = false;
    }

    pub fn close_editor(&mut self) {
        self.ask_if_dirty(
            "Do you want to leave without saving the recent changes?",
            "Leave",
            false,
            || {
                self.cancel_jobs();
                self.game.pop_screen();
            },
        );
    }

    fn ask_if_dirty<F>(&self, question: &str, confirm_text: &str, is_confirm_positive: bool, action: F)
    where
        F: FnOnce(),
    {
        if !self.is_dirty {
            action();
            return;
        }

        ConfirmPopup::new(
            self.clone(),
            question,
            confirm_text,
            is_confirm_positive,
            action,
        ).open();
    }

    pub fn ask_if_dirty_for_load<F>(&self, action: F)
    where
        F: FnOnce(),
    {
        self.ask_if_dirty(
            "Do you want to load another map without saving the recent changes?",
            "Load map",
            false,
            action,
        );
    }

    pub fn hide_selection(&mut self) {
        for group in &self.highlighted_tile_groups {
            group.borrow_mut().layer_overlay.hide_highlight();
        }
        self.highlighted_tile_groups.clear();
    }

    pub fn highlight_tile(&mut self, tile: &Tile, color: Color32) {
        if let Some(group) = self.map_holder.borrow().tile_groups.get(tile) {
            group.borrow_mut().layer_overlay.show_highlight(color);
            self.highlighted_tile_groups.push(group.clone());
        }
    }

    pub fn update_tile(&mut self, tile: &Tile) {
        if let Some(group) = self.map_holder.borrow().tile_groups.get(tile) {
            group.borrow_mut().update();
        }
    }

    pub fn update_and_highlight(&mut self, tile: &Tile, color: Color32) {
        self.update_tile(tile);
        self.highlight_tile(tile, color);
    }

    pub fn start_background_job<F>(&mut self, name: &str, is_daemon: bool, block: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let scope = if is_daemon {
            Dispatcher::DAEMON
        } else {
            Dispatcher::NON_DAEMON
        };

        let new_job = Concurrency::run(name, scope, block);
        self.jobs.push(new_job.clone());

        new_job.invoke_on_completion(Box::new(move || {
            self.jobs.retain(|j| j != &new_job);
        }));
    }

    fn cancel_jobs(&mut self) {
        for job in &self.jobs {
            job.cancel();
        }
        self.jobs.clear();
    }

    // Overlay Image methods
    fn clear_overlay_images(&mut self) {
        if let Some(old_image) = self.image_overlay.take() {
            old_image.remove();
            if let Some(drawable) = old_image.drawable.downcast_ref::<TextureRegionDrawable>() {
                if let Some(texture) = &drawable.region.texture {
                    texture.dispose();
                }
            }
        }
    }

    fn overlay_file_changed(&mut self, value: Option<PathBuf>) {
        self.clear_overlay_images();
        if value.is_none() {
            return;
        }

        if self.tile_map.borrow().map_parameters.world_wrap {
            self.set_world_wrap_fix_odd_width(false);
            ToastPopup::new(
                "World wrap is incompatible with an overlay and was deactivated.",
                self.stage.clone(),
                4000,
            ).open();
            self.tabs.borrow_mut().options.update();
        }

        self.recreate_map_holder(|| {});
    }

    fn overlay_alpha_changed(&mut self, value: f32) {
        if let Some(image) = &mut this.image_overlay {
            image.color.a = value;
        }
    }

    fn add_overlay_to_map_holder(&mut this, new_holder_content: Group) {
        this.clear_overlay_images();
        if this.overlay_file.is_none() {
            return;
        }

        match Texture::new(this.overlay_file.as_ref().unwrap()) {
            Ok(texture) => {
                texture.set_filter(TextureFilter::Linear, TextureFilter::Linear);
                let image = ImageWithCustomSize::new(TextureRegion::new(texture));

                image.touchable = Touchable::Disabled;
                image.set_fill_parent(true);
                image.color.a = this.overlay_alpha;
                new_holder_content.add_actor(image.clone());

                this.image_overlay = Some(image);
            },
            Err(ex) => {
                Log::error("Invalid overlay image", ex);
                this.overlay_file = None;
                ToastPopup::new("Invalid overlay image", this.stage.clone(), 3000).open();
                this.tabs.borrow_mut().options.update();
            }
        }
    }
}

impl RecreateOnResize for MapEditorScreen {
    fn recreate(&self) -> Box<dyn BaseScreen> {
        self.cancel_jobs();
        Box::new(MapEditorScreen::new(Some(self.tile_map.borrow().clone())))
    }
}

impl Drop for MapEditorScreen {
    fn drop(&mut self) {
        self.cancel_jobs();
    }
}