// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/unit/presenter/UnitPresenter.kt

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::VecDeque;
use egui::{Vec2, Align, Color32};
use crate::game::unit::MapUnit;
use crate::game::ruleset::unique::UniqueType;
use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::screens::pickerscreens::{PromotionPickerScreen, UnitRenamePopup};
use crate::ui::screens::worldscreen::unit::UnitTable;
use crate::ui::components::fonts::Fonts;
use crate::ui::components::extensions::{surround_with_circle, to_label};
use crate::ui::images::ImageGetter;
use crate::utils::translations::tr;

/// Presenter for unit information in the unit table
pub struct UnitPresenter {
    unit_table: Rc<RefCell<UnitTable>>,
    world_screen: Rc<RefCell<WorldScreen>>,
    selected_units: VecDeque<Rc<RefCell<MapUnit>>>,
    selected_unit_is_swapping: bool,
    selected_unit_is_connecting_road: bool,
}

impl UnitPresenter {
    /// Creates a new UnitPresenter
    pub fn new(unit_table: Rc<RefCell<UnitTable>>, world_screen: Rc<RefCell<WorldScreen>>) -> Self {
        Self {
            unit_table,
            world_screen,
            selected_units: VecDeque::new(),
            selected_unit_is_swapping: false,
            selected_unit_is_connecting_road: false,
        }
    }

    /// Gets the first selected unit
    pub fn selected_unit(&self) -> Option<Rc<RefCell<MapUnit>>> {
        self.selected_units.front().cloned()
    }

    /// Gets the position of the selected unit
    pub fn position(&self) -> Option<Vec2> {
        self.selected_unit().map(|unit| unit.borrow().current_tile.borrow().position)
    }

    /// Selects a unit
    pub fn select_unit(&mut self, unit: Option<Rc<RefCell<MapUnit>>>, append: bool) {
        if !append {
            self.selected_units.clear();
        }

        if let Some(unit) = unit {
            self.selected_units.push_back(unit.clone());
            unit.borrow_mut().actions_on_deselect();
        }

        self.selected_unit_is_swapping = false;
        self.selected_unit_is_connecting_road = false;
    }

    /// Updates the UI
    pub fn update(&self) {
        if let Some(unit) = self.selected_unit() {
            let unit = unit.borrow();
            let world_screen = self.world_screen.borrow();

            // Check if unit was captured or disappeared
            let captured = unit.civ.id != world_screen.viewing_civ.borrow().id &&
                           !world_screen.viewing_civ.borrow().is_spectator();

            let disappeared = !unit.get_tile().borrow().get_units().contains(&unit);

            if captured || disappeared {
                let mut unit_table = self.unit_table.borrow_mut();
                unit_table.select_unit(None);
                world_screen.borrow_mut().should_update = true;
                return;
            }

            // Update UI for single selected unit
            if self.selected_units.len() == 1 {
                let mut unit_table = self.unit_table.borrow_mut();
                unit_table.separator_visible = true;

                let name_label_text = self.build_name_label_text(&unit);
                unit_table.unit_name_label.clear_listeners();

                // Add click listener for renaming
                let world_screen = self.world_screen.clone();
                let unit_clone = unit.clone();
                unit_table.unit_name_label.add_click_listener(move |_| {
                    if !world_screen.borrow().can_change_state {
                        return;
                    }

                    let popup = UnitRenamePopup::new(
                        world_screen.clone(),
                        unit_clone.clone(),
                        Box::new(move |_| {
                            let mut unit_table = unit_table.borrow_mut();
                            unit_table.unit_name_label.set_text(self.build_name_label_text(&unit_clone.borrow()));
                            world_screen.borrow_mut().should_update = true;
                        })
                    );

                    world_screen.borrow_mut().push_screen(Box::new(popup));
                });

                // Clear and update description table
                unit_table.description_table.clear();
                unit_table.description_table.defaults().pad(2.0);

                // Add movement information
                unit_table.description_table.add(format!("{} {}", Fonts::MOVEMENT, unit.get_movement_string())).pad_right(10.0);

                // Add strength information for non-civilian units
                if !unit.is_civilian() {
                    unit_table.description_table.add(format!("{} {}", Fonts::STRENGTH, tr(&unit.base_unit.strength.to_string()))).pad_right(10.0);
                }

                // Add ranged strength if applicable
                if unit.base_unit.ranged_strength != 0 {
                    unit_table.description_table.add(format!("{} {}", Fonts::RANGED_STRENGTH, tr(&unit.base_unit.ranged_strength.to_string()))).pad_right(10.0);
                }

                // Add range if applicable
                if unit.base_unit.is_ranged() {
                    unit_table.description_table.add(format!("{} {}", Fonts::RANGE, tr(&unit.get_range().to_string()))).pad_right(10.0);
                }

                // Add interception range if applicable
                let interception_range = unit.get_interception_range();
                if interception_range > 0 {
                    let icon = ImageGetter::get_stat_icon("InterceptRange");
                    icon.set_size(20.0);
                    unit_table.description_table.add(icon);
                    unit_table.description_table.add(tr(&interception_range.to_string())).pad_right(10.0);
                }

                // Add XP information for non-civilian units
                if !unit.is_civilian() {
                    let xp_label = to_label("XP");
                    xp_label.add_click_listener(move |_| {
                        if self.selected_unit().is_none() {
                            return;
                        }
                        world_screen.borrow_mut().push_screen(Box::new(PromotionPickerScreen::new(unit.clone())));
                    });
                    unit_table.description_table.add(xp_label);
                    unit_table.description_table.add(format!("{}/{}",
                        tr(&unit.promotions.xp.to_string()),
                        tr(&unit.promotions.xp_for_next_promotion().to_string())));
                }

                // Add religious strength if applicable
                if unit.base_unit.religious_strength > 0 {
                    let icon = ImageGetter::get_stat_icon("ReligiousStrength");
                    icon.set_size(20.0);
                    unit_table.description_table.add(icon);
                    unit_table.description_table.add(tr(&(unit.base_unit.religious_strength - unit.religious_strength_lost).to_string()));
                }

                // Check if promotions have changed
                if unit.promotions.promotions.len() != unit_table.promotions_table.children.len() {
                    world_screen.borrow_mut().should_update = true;
                }
            } else {
                // Multiple selected units
                let mut unit_table = self.unit_table.borrow_mut();
                unit_table.name_label_text = String::new();
                unit_table.description_table.clear();
            }
        }
    }

    /// Updates the UI when needed
    pub fn update_when_needed(&self) {
        if let Some(unit) = self.selected_unit() {
            let unit = unit.borrow();

            // Update UI for single selected unit
            if self.selected_units.len() == 1 {
                let mut unit_table = self.unit_table.borrow_mut();

                // Add unit icon
                let unit_icon_group = UnitIconGroup::new(unit.clone(), 30.0);
                unit_table.unit_icon_holder.add(unit_icon_group).pad(5.0);

                // Add promotion icons
                for promotion in unit.promotions.get_promotions(true) {
                    if !promotion.has_unique(UniqueType::NotShownOnWorldScreen) {
                        let promotion_portrait = ImageGetter::get_promotion_portrait(&promotion.name, 20.0);
                        unit_table.promotions_table.add(promotion_portrait).pad_bottom(2.0);
                    }
                }

                // Add status icons
                for status in unit.status_map.values() {
                    if status.uniques.iter().any(|unique| unique.type_ == UniqueType::NotShownOnWorldScreen) {
                        continue;
                    }

                    let group = ImageGetter::get_promotion_portrait(&status.name, 20.0);
                    let turns_left = to_label(&format!("{}{}", status.turns_left, Fonts::TURN))
                        .with_font_size(8.0)
                        .surround_with_circle(15.0, ImageGetter::CHARCOAL);

                    group.add_actor(turns_left.clone());
                    turns_left.set_position(group.width, 0.0, Align::BottomRight);
                    unit_table.promotions_table.add(group).pad_bottom(2.0);
                }

                // Add click listener for promotions
                let world_screen = self.world_screen.clone();
                let unit_clone = unit.clone();
                unit_table.promotions_table.add_click_listener(move |_| {
                    if self.selected_unit().is_none() || unit_clone.borrow().promotions.promotions.is_empty() {
                        return;
                    }
                    world_screen.borrow_mut().push_screen(Box::new(PromotionPickerScreen::new(unit_clone.clone())));
                });

                // Add click listener for unit icon
                let world_screen = self.world_screen.clone();
                let unit_clone = unit.clone();
                unit_table.unit_icon_holder.add_click_listener(move |_| {
                    world_screen.borrow_mut().open_civilopedia(unit_clone.borrow().base_unit.make_link());
                });
            } else {
                // Multiple selected units
                let mut unit_table = self.unit_table.borrow_mut();
                for selected_unit in &self.selected_units {
                    let unit_icon_group = UnitIconGroup::new(selected_unit.clone(), 30.0);
                    unit_table.unit_icon_holder.add(unit_icon_group).pad(5.0);
                }
            }
        }
    }

    /// Builds the name label text for a unit
    fn build_name_label_text(&self, unit: &MapUnit) -> String {
        let mut name_label_text = unit.display_name(true);
        if unit.health < 100 {
            name_label_text.push_str(&format!(" ({})", tr(&unit.health.to_string())));
        }
        name_label_text
    }
}