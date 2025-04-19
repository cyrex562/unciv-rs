// Source: orig_src/core/src/com/unciv/ui/screens/pickerscreens/PromotionPickerScreen.kt

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use egui::{Ui, Color32, Align, Response, Button, Image, ScrollArea, RichText, Vec2};
use crate::models::civilization::Civilization;
use crate::models::map_unit::MapUnit;
use crate::models::tutorial_trigger::TutorialTrigger;
use crate::models::unciv_sound::UncivSound;
use crate::models::ruleset::unit::Promotion;
use crate::ui::screens::basescreen::{BaseScreen, RecreateOnResize};
use crate::ui::screens::pickerscreen::PickerScreen;
use crate::ui::images::ImageGetter;
use crate::ui::popups::unit_rename_popup::UnitRenamePopup;
use crate::utils::translation::tr;
use super::promotion_button::PromotionButton;
use super::promotion_tree::PromotionTree;
use super::promotion_screen_colors::PromotionScreenColors;

pub struct PromotionPickerScreen {
    unit: Rc<RefCell<MapUnit>>,
    close_on_pick: bool,
    original_name: Option<String>,
    on_change: Option<Box<dyn Fn()>>,
    picker_screen: PickerScreen,
    colors: Rc<PromotionScreenColors>,
    promoted_label_style: egui::TextStyle,
    button_cell_max_width: f32,
    button_cell_min_width: f32,
    promotions_table: egui::Grid,
    promotion_to_button: HashMap<String, PromotionButton>,
    selected_promotion: Option<PromotionButton>,
    lines: Vec<Image>,
    can_change_state: bool,
    can_promote_now: bool,
    tree: PromotionTree,
    save_unit_type_promotion: bool,
}

impl PromotionPickerScreen {
    pub fn new(
        unit: Rc<RefCell<MapUnit>>,
        close_on_pick: bool,
        on_change: Option<Box<dyn Fn()>>,
    ) -> Self {
        let original_name = unit.borrow().instance_name.clone();
        Self::new_with_name(unit, close_on_pick, Some(original_name), on_change)
    }

    pub fn new_with_name(
        unit: Rc<RefCell<MapUnit>>,
        close_on_pick: bool,
        original_name: Option<String>,
        on_change: Option<Box<dyn Fn()>>,
    ) -> Self {
        let mut screen = Self {
            unit,
            close_on_pick,
            original_name,
            on_change,
            picker_screen: PickerScreen::new(false),
            colors: Rc::new(PromotionScreenColors::default()),
            promoted_label_style: egui::TextStyle::Body,
            button_cell_max_width: 0.0,
            button_cell_min_width: 0.0,
            promotions_table: egui::Grid::new("promotions_grid"),
            promotion_to_button: HashMap::new(),
            selected_promotion: None,
            lines: Vec::new(),
            can_change_state: false,
            can_promote_now: false,
            tree: PromotionTree::new(Rc::clone(&unit)),
            save_unit_type_promotion: false,
        };

        screen.init();
        screen
    }

    fn init(&mut self) {
        // Set up close button action
        self.picker_screen.set_default_close_action();

        // Set up the right side button text based on promotion state
        let unit = self.unit.borrow();
        self.can_change_state = true; // TODO: Replace with actual check
        self.can_promote_now = self.can_change_state &&
            unit.promotions.can_be_promoted() &&
            unit.has_movement() && unit.attacks_this_turn == 0;

        if self.can_promote_now {
            self.picker_screen.set_right_side_button_text(&tr("Pick promotion"));
            self.picker_screen.set_right_side_button_on_click(UncivSound::Silent, || {
                if let Some(selected) = &self.selected_promotion {
                    self.accept_promotion(Some(selected.clone()));
                    self.check_save_unit_type_promotion();
                }
            });
        } else {
            self.picker_screen.set_right_side_button_enabled(false);
        }

        // Update description label
        self.update_description_label();

        // Add rename button if can change state
        if self.can_change_state {
            let unit = self.unit.borrow();
            let rename_button_text = format!("Choose name for [{}]", unit.name);
            let rename_button = Button::new(tr(&rename_button_text));

            rename_button.on_click(|| {
                // TODO: Implement UnitRenamePopup
                // UnitRenamePopup::new(self, Rc::clone(&self.unit), || {
                //     // Recreate screen
                // });
            });

            self.picker_screen.add_to_top_table(rename_button);
        }

        // Create all buttons without placing them yet, measure
        let stage_width = 800.0; // TODO: Get actual stage width
        self.button_cell_max_width = ((stage_width - 80.0) / self.tree.get_max_columns() as f32)
            .clamp(190.0, 300.0);

        for node in self.tree.all_nodes() {
            self.promotion_to_button.insert(
                node.promotion.name.clone(),
                self.get_button(&self.tree, node)
            );
        }

        // Calculate minimum button width
        let max_pref_width = self.promotion_to_button.values()
            .map(|b| b.pref_width() + 10.0)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        self.button_cell_min_width = max_pref_width.clamp(190.0, self.button_cell_max_width);

        // Fill the table with buttons
        self.fill_table();

        // Display tutorial
        // TODO: Implement tutorial display
    }

    fn accept_promotion(&mut self, button: Option<&PromotionButton>) {
        // If user managed to click disabled button, still do nothing
        if button.is_none() || !button.unwrap().is_pickable() {
            return;
        }

        let path = self.tree.get_path_to(&button.unwrap().node.promotion);

        // Play sound
        // TODO: Implement sound playing

        // Add promotions
        let mut unit = self.unit.borrow_mut();
        for promotion in path {
            unit.promotions.add_promotion(&promotion.name);
        }

        // Call onChange callback if provided
        if let Some(on_change) = &self.on_change {
            on_change();
        }

        // Close or recreate screen
        if !self.close_on_pick || unit.promotions.can_be_promoted() {
            // TODO: Implement screen recreation
        } else {
            // TODO: Implement screen closing
        }
    }

    fn fill_table(&mut self) {
        let mut placed_buttons = HashSet::new();

        // Create cell matrix
        let max_columns = self.tree.get_max_columns();
        let max_rows = self.tree.get_max_rows();

        // TODO: Implement cell matrix creation and button placement
        // This is a complex part of the original code that needs careful adaptation

        // Add statuses if unit has any
        let unit = self.unit.borrow();
        if !unit.status_map.is_empty() {
            self.add_statuses();
        }

        // Add connecting lines
        self.add_connecting_lines(&HashSet::new());
    }

    fn check_save_unit_type_promotion(&self) {
        if !self.save_unit_type_promotion {
            return;
        }

        let unit = self.unit.borrow();
        if let Some(current_city) = unit.current_tile.get_city() {
            // If you clicked the save baseUnit promotion, you want the next baseUnit to have the same promotion
            current_city.unit_should_use_saved_promotion.insert(unit.base_unit.name.clone());
            current_city.unit_to_promotions.insert(unit.base_unit.name.clone(), unit.promotions.clone());
        }
    }

    fn get_button(&mut self, tree: &PromotionTree, node: Rc<PromotionTree::PromotionNode>) -> PromotionButton {
        let is_pickable = self.can_promote_now &&
            (!node.path_is_ambiguous || node.distance_to_adopted == 1) &&
            tree.can_buy_up_to(&node.promotion);

        let button = PromotionButton::new(
            node,
            is_pickable,
            self.promoted_label_style.clone(),
            self.button_cell_max_width - 60.0
        );

        // Set up button click handler
        let mut button_clone = button.clone();
        button_clone.on_click(|| {
            self.selected_promotion = Some(button_clone.clone());

            let path = tree.get_path_to(&button_clone.node.promotion);
            let path_as_set: HashSet<&Promotion> = path.iter().collect();
            let prerequisites = &button_clone.node.parents;

            // Update button colors
            for btn in self.promotion_to_button.values_mut() {
                btn.update_color(
                    btn == &button_clone,
                    &path,
                    prerequisites
                );
            }

            // Enable/disable right side button
            self.picker_screen.set_right_side_button_enabled(is_pickable);
            self.picker_screen.set_right_side_button_text(&tr(&button_clone.node.promotion.name));

            // Update description label
            self.update_description_label_with_path(is_pickable, tree, &button_clone.node, &path);

            // Add connecting lines
            self.add_connecting_lines(&path_as_set);
        });

        // Set up double click handler for pickable buttons
        if is_pickable {
            let mut button_clone = button.clone();
            button_clone.on_double_click(UncivSound::Silent, || {
                self.accept_promotion(Some(&button_clone));
                self.check_save_unit_type_promotion();
            });
        }

        button
    }

    fn add_statuses(&mut self) {
        let unit = self.unit.borrow();
        let mut status_table = egui::Grid::new("status_grid");

        // Sort statuses by turns left
        let mut statuses: Vec<_> = unit.status_map.values().collect();
        statuses.sort_by(|a, b| a.turns_left.cmp(&b.turns_left));

        for status in statuses {
            let status_button_text = format!("{}: {}{}", status.name, status.turns_left, "t");
            let status_button = Button::new(tr(&status_button_text));

            let description = format!("{}: {}{}\n{}",
                status.name,
                status.turns_left,
                "t",
                unit.civ.game_info.ruleset.unit_promotions.get(&status.name)
                    .map(|p| p.get_description(&HashSet::new()))
                    .unwrap_or_default()
            );

            status_button.on_click(|| {
                self.picker_screen.set_description_label_text(&description);
            });

            status_table.add(status_button);
        }

        self.picker_screen.add_to_top_table(status_table);
    }

    fn add_connecting_lines(&mut self, path: &HashSet<&Promotion>) {
        // TODO: Implement connecting lines
        // This is a complex part of the original code that needs careful adaptation
    }

    fn update_description_label(&mut self) {
        // Default implementation
        self.picker_screen.set_description_label_text("");
    }

    fn update_description_label_with_path(
        &mut self,
        is_pickable: bool,
        tree: &PromotionTree,
        node: &PromotionTree::PromotionNode,
        path: &Vec<Promotion>
    ) {
        // TODO: Implement description label update with path
    }

    pub fn show(&self, ui: &mut Ui) {
        self.picker_screen.show(ui);
    }
}

impl RecreateOnResize for PromotionPickerScreen {
    fn recreate(&self) -> Box<dyn BaseScreen> {
        Box::new(PromotionPickerScreen::new(
            Rc::clone(&self.unit),
            self.close_on_pick,
            self.on_change.clone()
        ))
    }
}