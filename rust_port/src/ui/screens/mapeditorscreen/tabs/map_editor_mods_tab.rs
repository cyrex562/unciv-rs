use bevy::prelude::*;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use crate::ui::components::*;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::mapeditorscreen::MapEditorScreen;
use crate::ui::screens::newgamescreen::ModCheckboxTable;
use crate::logic::map::tile::TileNormalizer;
use crate::models::ruleset::{Ruleset, RulesetCache};
use crate::utils::translations::tr;

pub struct MapEditorModsTab {
    editor_screen: Entity,
    mods: VecDeque<String>,
    mods_table: ModCheckboxTable,
    mods_table_cell: Cell<ModCheckboxTable>,
    apply_button: TextButton,
    revert_button: TextButton,
    base_ruleset_select_box: TranslatedSelectBox,
}

impl MapEditorModsTab {
    pub fn new(editor_screen: Entity) -> Self {
        let ruleset_name = editor_screen.new_map_parameters.base_ruleset.clone();
        // Our dirty flag `mods_tab_needs_refresh` will be true on first activation,
        // so this will be replaced and can now be minimal
        let mods_table = ModCheckboxTable::new(
            HashSet::new(),
            ruleset_name.clone(),
            editor_screen,
            false,
            Arc::new(|| {}),
        );

        let base_rulesets = RulesetCache::get_sorted_base_rulesets();
        let base_ruleset_select_box = TranslatedSelectBox::new(base_rulesets, ruleset_name.clone());

        let mut tab = Self {
            editor_screen,
            mods: editor_screen.new_map_parameters.mods.clone(),
            mods_table,
            mods_table_cell: Cell::new(),
            apply_button: TextButton::new("Change map ruleset".tr()),
            revert_button: TextButton::new("Revert to map ruleset".tr()),
            base_ruleset_select_box,
        };

        tab.init();
        tab
    }

    fn init(&mut self) {
        let tab = self;

        self.base_ruleset_select_box.on_change(Arc::new(move || {
            let new_base_ruleset = tab.base_ruleset_select_box.selected().value().clone();
            tab.editor_screen.new_map_parameters.base_ruleset = new_base_ruleset.clone();
            tab.mods_table.set_base_ruleset(new_base_ruleset);
            tab.mods_table.disable_all_checkboxes();
            tab.enable_apply_button();
        }));

        self.top();
        self.pad(5.0);

        let mut base_ruleset_table = Table::new();
        base_ruleset_table.add(Label::new("Base Ruleset:".tr()));
        base_ruleset_table.add(self.base_ruleset_select_box.clone()).fill_x();
        self.add(base_ruleset_table).fill_x().pad_bottom(10.0).row();

        let mut button_table = Table::new();
        button_table.add(self.apply_button.clone()).pad_right(10.0);
        button_table.add(self.revert_button.clone());
        self.add(button_table).fill_x().pad(10.0).row();

        self.mods_table_cell = self.add(self.mods_table.clone());
        self.row();

        self.apply_button.on_click(Arc::new(move || {
            tab.apply_controls();
        }));
        self.apply_button.add_tooltip(
            "Change the map to use the ruleset selected on this page".tr(),
            21.0,
            Align::Bottom,
        );

        self.revert_button.on_click(Arc::new(move || {
            tab.revert_controls();
        }));
        self.revert_button.add_tooltip(
            "Reset the controls to reflect the current map ruleset".tr(),
            21.0,
            Align::Bottom,
        );
    }

    fn enable_apply_button(&mut self) {
        let current_parameters = self.editor_screen.tile_map.map_parameters.clone();
        let enabled = current_parameters.mods != self.mods ||
            current_parameters.base_ruleset != self.base_ruleset_select_box.selected().value();

        self.apply_button.set_enabled(enabled);
        self.revert_button.set_enabled(enabled);
    }

    fn revert_controls(&mut self) {
        let current_parameters = self.editor_screen.tile_map.map_parameters.clone();
        self.base_ruleset_select_box.set_selected(current_parameters.base_ruleset.clone());

        self.mods.clear();
        self.mods.extend(current_parameters.mods.iter().cloned()); // clone current "into" editor_screen.new_map_parameters.mods

        let tab = self;
        self.mods_table = ModCheckboxTable::new(
            self.mods.iter().cloned().collect(),
            current_parameters.base_ruleset.clone(),
            self.editor_screen,
            false,
            Arc::new(move || {
                tab.enable_apply_button();
            }),
        );

        self.mods_table_cell.set_actor(self.mods_table.clone());
        self.enable_apply_button();
    }

    fn apply_controls(&mut self) {
        let new_ruleset = RulesetCache::get_complex_ruleset(
            &self.mods,
            self.editor_screen.new_map_parameters.base_ruleset.clone(),
        );

        let incompatibilities = self.get_incompatibilities(&new_ruleset);

        if incompatibilities.is_empty() {
            self.editor_screen.apply_ruleset(
                new_ruleset,
                self.editor_screen.new_map_parameters.base_ruleset.clone(),
                self.mods.clone(),
            );
            self.enable_apply_button();
        } else {
            let tab = self;
            AskFitMapToRulesetPopup::new(
                self.editor_screen,
                incompatibilities,
                Arc::new(move || {
                    tab.fit_map_to_ruleset(&new_ruleset);
                    tab.editor_screen.apply_ruleset(
                        new_ruleset,
                        tab.editor_screen.new_map_parameters.base_ruleset.clone(),
                        tab.mods.clone(),
                    );
                    tab.enable_apply_button();
                }),
            ).open();
        }
    }

    fn get_incompatibilities(&self, new_ruleset: &Ruleset) -> Vec<String> {
        let mut incompatibilities = HashSet::new();

        for tile in self.editor_screen.tile_map.values() {
            incompatibilities.extend(tile.get_ruleset_incompatibility(new_ruleset));
        }

        incompatibilities.remove("");
        incompatibilities.into_iter().sorted().collect()
    }

    fn fit_map_to_ruleset(&self, new_ruleset: &Ruleset) {
        for tile in self.editor_screen.tile_map.values() {
            TileNormalizer::normalize_to_ruleset(tile, new_ruleset);
        }
    }
}

// Extension trait for TabbedPager to support the IPageExtensions interface
pub trait TabbedPagerExtensions {
    fn activated(&mut self, index: usize, caption: &str, pager: &mut TabbedPager);
    fn deactivated(&mut self, index: usize, caption: &str, pager: &mut TabbedPager);
}

impl TabbedPagerExtensions for MapEditorModsTab {
    fn activated(&mut self, _index: usize, _caption: &str, _pager: &mut TabbedPager) {
        self.enable_apply_button();
        if !self.editor_screen.mods_tab_needs_refresh {
            return;
        }
        self.editor_screen.mods_tab_needs_refresh = false;
        self.revert_controls();
    }

    fn deactivated(&mut self, _index: usize, _caption: &str, _pager: &mut TabbedPager) {
        // No implementation needed
    }
}

// Helper struct for the popup that asks if the map should be fit to the ruleset
struct AskFitMapToRulesetPopup {
    popup: Popup,
}

impl AskFitMapToRulesetPopup {
    fn new(
        editor_screen: Entity,
        incompatibilities: Vec<String>,
        on_ok: Arc<() -> ()>,
    ) -> Self {
        let mut popup = Popup::new(editor_screen);

        let mut incompatibility_table = Table::new();
        for inc in incompatibilities {
            incompatibility_table.add(Label::new(inc)).row();
        }

        popup.add(ScrollPane::new(incompatibility_table))
            .colspan(2)
            .max_height(editor_screen.stage.height * 0.8)
            .row();

        popup.add_good_sized_label("Change map to fit selected ruleset?".tr(), 24)
            .colspan(2)
            .row();

        popup.add_button("Yes".tr(), 'y', Arc::new(move || {
            on_ok();
            popup.close();
        }));

        popup.add_button("No".tr(), 'n', Arc::new(move || {
            popup.close();
        }));

        popup.equalize_last_two_button_widths();

        Self { popup }
    }

    fn open(&self) {
        self.popup.open(true);
    }
}