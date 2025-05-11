use std::rc::Rc;
use std::cell::RefCell;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui::{Color32, Ui, Vec2};

use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::mapeditorscreen::MapEditorScreen;
use crate::ui::components::widgets::tabbed_pager::TabbedPager;
use crate::ui::components::widgets::tabbed_pager::PageExtensions;
use crate::ui::components::input::key_char_and_code::KeyCharAndCode;
use crate::ui::images::image_getter::ImageGetter;
use crate::ui::screens::mapeditorscreen::tabs::map_editor_edit_tab::MapEditorEditTab;
use crate::ui::screens::mapeditorscreen::tabs::map_editor_generate_tab::MapEditorGenerateTab;
use crate::ui::screens::mapeditorscreen::tabs::map_editor_load_tab::MapEditorLoadTab;
use crate::ui::screens::mapeditorscreen::tabs::map_editor_mods_tab::MapEditorModsTab;
use crate::ui::screens::mapeditorscreen::tabs::map_editor_options_tab::MapEditorOptionsTab;
use crate::ui::screens::mapeditorscreen::tabs::map_editor_save_tab::MapEditorSaveTab;
use crate::ui::screens::mapeditorscreen::tabs::map_editor_view_tab::MapEditorViewTab;

/// Main tabs for the map editor
pub struct MapEditorMainTabs {
    editor_screen: Rc<RefCell<MapEditorScreen>>,
    header_height: f32,
    view: MapEditorViewTab,
    generate: MapEditorGenerateTab,
    edit: MapEditorEditTab,
    load: MapEditorLoadTab,
    save: MapEditorSaveTab,
    mods: MapEditorModsTab,
    options: MapEditorOptionsTab,
    pager: TabbedPager,
}

impl MapEditorMainTabs {
    pub fn new(editor_screen: Rc<RefCell<MapEditorScreen>>) -> Self {
        let header_height = 24.0;
        let stage_height = editor_screen.borrow().stage.height;

        let view = MapEditorViewTab::new(editor_screen.clone());
        let generate = MapEditorGenerateTab::new(editor_screen.clone(), header_height);
        let edit = MapEditorEditTab::new(editor_screen.clone(), header_height);
        let load = MapEditorLoadTab::new(editor_screen.clone(), header_height);
        let save = MapEditorSaveTab::new(editor_screen.clone(), header_height);
        let mods = MapEditorModsTab::new(editor_screen.clone());
        let options = MapEditorOptionsTab::new(editor_screen.clone());

        let mut pager = TabbedPager::new(
            stage_height,
            stage_height,
            header_height,
            7,
        );

        pager.set_pref_width(editor_screen.borrow().get_tools_width());

        let mut tabs = Self {
            editor_screen: editor_screen.clone(),
            header_height,
            view,
            generate,
            edit,
            load,
            save,
            mods,
            options,
            pager,
        };

        tabs.setup_pages();

        tabs
    }

    fn setup_pages(&mut self) {
        // View tab
        self.pager.add_page(
            "View",
            Box::new(self.view.clone()),
            ImageGetter::get_image("OtherIcons/Search"),
            25.0,
            Some(KeyCharAndCode::ctrl('i')),
            false,
        );

        // Generate tab
        self.pager.add_page(
            "Generate",
            Box::new(self.generate.clone()),
            ImageGetter::get_image("OtherIcons/New"),
            25.0,
            Some(KeyCharAndCode::ctrl('n')),
            false,
        );

        // Edit tab
        self.pager.add_page(
            "Edit",
            Box::new(self.edit.clone()),
            ImageGetter::get_image("OtherIcons/Terrains"),
            25.0,
            Some(KeyCharAndCode::ctrl('e')),
            false,
        );

        // Load tab
        self.pager.add_page(
            "Load",
            Box::new(self.load.clone()),
            ImageGetter::get_image("OtherIcons/Load"),
            25.0,
            Some(KeyCharAndCode::ctrl('l')),
            self.load.no_maps_available(),
        );

        // Save tab
        self.pager.add_page(
            "Save",
            Box::new(self.save.clone()),
            ImageGetter::get_image("OtherIcons/Checkmark"),
            25.0,
            Some(KeyCharAndCode::ctrl('s')),
            false,
        );

        // Mods tab
        self.pager.add_page(
            "Mods",
            Box::new(self.mods.clone()),
            ImageGetter::get_image("OtherIcons/Mods"),
            25.0,
            Some(KeyCharAndCode::ctrl('d')),
            false,
        );

        // Options tab
        self.pager.add_page(
            "Options",
            Box::new(self.options.clone()),
            ImageGetter::get_image("OtherIcons/Settings"),
            25.0,
            Some(KeyCharAndCode::ctrl('o')),
            false,
        );

        self.pager.select_page(0);
        self.pager.set_header_scroll_fade_scroll_bars(false);
    }

    pub fn render(&mut self, ui: &mut Ui) {
        self.pager.render(ui);
    }
}

impl Clone for MapEditorMainTabs {
    fn clone(&self) -> Self {
        Self {
            editor_screen: self.editor_screen.clone(),
            header_height: self.header_height,
            view: self.view.clone(),
            generate: self.generate.clone(),
            edit: self.edit.clone(),
            load: self.load.clone(),
            save: self.save.clone(),
            mods: self.mods.clone(),
            options: self.options.clone(),
            pager: self.pager.clone(),
        }
    }
}