use std::rc::Rc;
use std::collections::HashMap;
use eframe::egui::{self, Ui, Color32, Response, Rect, Vec2, Align};
use log::info;

use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::popups::scrollable_animated_menu_popup::ScrollableAnimatedMenuPopup;
use crate::ui::popups::Popup;
use crate::ui::components::input::KeyboardBinding;
use crate::ui::components::widgets::Button;
use crate::ui::objectdescriptions::BaseUnitDescriptions;
use crate::logic::civilization::Civilization;
use crate::logic::map::mapunit::MapUnit;
use crate::models::counter::Counter;
use crate::models::upgrade_unit_action::UpgradeUnitAction;
use crate::ui::screens::worldscreen::unit::actions::UnitActionsUpgrade;
use crate::audio::sound_player::SoundPlayer;
use crate::utils::concurrency::Concurrency;

/// A popup menu showing info about a Unit upgrade, with buttons to upgrade "this" unit or all
/// similar units.
pub struct UnitUpgradeMenu {
    /// The base scrollable animated menu popup
    base: ScrollableAnimatedMenuPopup,

    /// The unit that is ready to upgrade
    unit: Rc<MapUnit>,

    /// The upgrade action for the unit
    unit_action: Rc<UpgradeUnitAction>,

    /// Whether the buttons should be enabled
    enable: bool,

    /// Whether to call the callback after animation
    callback_after_animation: bool,

    /// Callback to be called after one or several upgrades have been performed
    on_button_clicked: Option<Box<dyn FnOnce()>>,

    /// The unit to upgrade to
    unit_to_upgrade_to: Option<Rc<MapUnit>>,

    /// All units that can be upgraded
    all_upgradable_units: Vec<Rc<MapUnit>>,

    /// Whether any button was clicked
    any_button_was_clicked: bool,
}

impl UnitUpgradeMenu {
    /// Creates a new UnitUpgradeMenu
    ///
    /// # Arguments
    ///
    /// * `screen` - The screen to show the popup on
    /// * `position_next_to` - The position to show the popup next to
    /// * `unit` - The unit that is ready to upgrade
    /// * `unit_action` - The upgrade action for the unit
    /// * `enable` - Whether the buttons should be enabled
    /// * `callback_after_animation` - Whether to call the callback after animation
    /// * `on_button_clicked` - Callback to be called after one or several upgrades have been performed
    pub fn new(
        screen: &Rc<BaseScreen>,
        position_next_to: Vec2,
        unit: Rc<MapUnit>,
        unit_action: Rc<UpgradeUnitAction>,
        enable: bool,
        callback_after_animation: bool,
        on_button_clicked: Box<dyn FnOnce()>,
    ) -> Self {
        // Create the base popup
        let mut base = ScrollableAnimatedMenuPopup::new(screen, position_next_to);

        // Get the unit to upgrade to
        let unit_to_upgrade_to = unit_action.unit_to_upgrade_to();

        // Get all upgradable units
        let all_upgradable_units = Self::get_all_upgradable_units(&unit, &unit_to_upgrade_to);

        // Create the popup
        let mut popup = Self {
            base,
            unit: Rc::clone(&unit),
            unit_action: Rc::clone(&unit_action),
            enable,
            callback_after_animation,
            on_button_clicked: Some(on_button_clicked),
            unit_to_upgrade_to,
            all_upgradable_units,
            any_button_was_clicked: false,
        };

        // Set up the popup content
        popup.setup_content();

        popup
    }

    /// Gets all units that can be upgraded
    fn get_all_upgradable_units(unit: &Rc<MapUnit>, unit_to_upgrade_to: &Option<Rc<MapUnit>>) -> Vec<Rc<MapUnit>> {
        let civ = unit.civ();
        let base_unit_name = unit.base_unit().name();

        civ.units()
            .get_civ_units()
            .into_iter()
            .filter(|u| {
                u.base_unit().name() == base_unit_name
                    && u.has_movement()
                    && u.current_tile().get_owner() == Some(civ.clone())
                    && !u.is_embarked()
                    && u.upgrade().can_upgrade(unit_to_upgrade_to, true)
            })
            .collect()
    }

    /// Sets up the popup content
    fn setup_content(&mut self) {
        // Set the scrollable content
        self.base.set_scrollable_content(move |ui| {
            let title = self.unit_action.title();
            let base_unit = self.unit.base_unit();
            let unit_to_upgrade_to = self.unit_to_upgrade_to.as_ref().unwrap();

            BaseUnitDescriptions::get_upgrade_info_table(ui, title, base_unit, unit_to_upgrade_to);
        });

        // Set the fixed content (buttons)
        self.base.set_fixed_content(move |ui| {
            // Create the upgrade button
            let mut upgrade_button = Button::new("Upgrade", KeyboardBinding::Upgrade);
            upgrade_button.set_enabled(self.enable);
            upgrade_button.set_on_click(Box::new(move || {
                self.any_button_was_clicked = true;
                self.do_upgrade();
            }));

            ui.add(upgrade_button);
            ui.add_space(10.0);

            // Check if there are multiple units to upgrade
            let all_count = self.all_upgradable_units.len();
            if all_count <= 1 {
                return;
            }

            // Calculate the cost for upgrading all units
            let all_cost = self.unit_action.gold_cost_of_upgrade() * all_count as i32;
            let all_resources = self.unit_action.new_resource_requirements().clone() * all_count as i32;

            // Create the upgrade all button text
            let unit_name = self.unit.name();
            let upgrade_all_text = format!("Upgrade all [{}] [{}] ([{}] gold)", all_count, unit_name, all_cost);

            // Create the upgrade all button
            let mut upgrade_all_button = Button::new(&upgrade_all_text, KeyboardBinding::UpgradeAll);

            // Check if there are insufficient resources
            let insufficient_gold = self.unit.civ().gold() < all_cost;
            let insufficient_resources = self.get_insufficient_resources_message(&all_resources, self.unit.civ());

            // Disable the button if there are insufficient resources
            upgrade_all_button.set_enabled(!insufficient_gold && insufficient_resources.is_empty());

            // Set the button click handler
            upgrade_all_button.set_on_click(Box::new(move || {
                self.any_button_was_clicked = true;
                self.do_all_upgrade();
            }));

            ui.add(upgrade_all_button);

            // Show insufficient resources message if any
            if !insufficient_resources.is_empty() {
                ui.add_space(10.0);
                ui.label(egui::RichText::new(insufficient_resources).color(Color32::RED));
            }
        });

        // Set up the callback
        let action = Box::new(move || {
            if self.any_button_was_clicked {
                if let Some(callback) = self.on_button_clicked.take() {
                    callback();
                }
            }
        });

        if self.callback_after_animation {
            self.base.base_mut().set_after_close_callback(action);
        } else {
            self.base.base_mut().popup_mut().add_close_listener(action);
        }
    }

    /// Gets a message about insufficient resources
    fn get_insufficient_resources_message(&self, required_resources: &Counter<String>, civ: &Rc<Civilization>) -> String {
        if required_resources.is_empty() {
            return String::new();
        }

        let available = civ.get_civ_resources_by_name();
        let mut message = String::new();

        for (name, amount) in required_resources.iter() {
            let available_amount = available.get(name).copied().unwrap_or(0);
            let difference = amount - available_amount;

            if difference <= 0 {
                continue;
            }

            if message.is_empty() {
                message.push('\n');
            }

            message.push_str(&format!("Need [{}] more [{}]", difference, name));
        }

        message
    }

    /// Upgrades the unit
    fn do_upgrade(&self) {
        // Play the sound
        SoundPlayer::play(self.unit_action.unciv_sound());

        // Execute the action
        if let Some(action) = self.unit_action.action() {
            action();
        }
    }

    /// Upgrades all units
    fn do_all_upgrade(&self) {
        // Play the sound repeatedly
        SoundPlayer::play_repeated(self.unit_action.unciv_sound());

        // Get the unit to upgrade to
        let unit_to_upgrade_to = self.unit_to_upgrade_to.as_ref().unwrap();

        // Upgrade all units
        for unit in &self.all_upgradable_units {
            let actions = UnitActionsUpgrade::get_upgrade_actions(unit);

            // Find the matching action
            for action in actions {
                if let Some(upgrade_action) = action.downcast_ref::<UpgradeUnitAction>() {
                    if upgrade_action.unit_to_upgrade_to() == Some(unit_to_upgrade_to.clone()) {
                        if let Some(action_fn) = upgrade_action.action() {
                            action_fn();
                        }
                        break;
                    }
                }
            }
        }
    }

    /// Shows the popup
    pub fn show(&mut self, ui: &mut Ui) -> bool {
        self.base.show(ui)
    }

    /// Closes the popup
    pub fn close(&mut self) {
        self.base.close();
    }

    /// Returns a reference to the base popup
    pub fn base(&self) -> &ScrollableAnimatedMenuPopup {
        &self.base
    }

    /// Returns a mutable reference to the base popup
    pub fn base_mut(&mut self) -> &mut ScrollableAnimatedMenuPopup {
        &mut self.base
    }
}

impl Popup for UnitUpgradeMenu {
    fn show(&mut self, ui: &mut Ui) -> bool {
        self.show(ui)
    }

    fn title(&self) -> String {
        self.base.title()
    }

    fn screen(&self) -> &Rc<BaseScreen> {
        self.base.screen()
    }

    fn max_size_percentage(&self) -> f32 {
        self.base.max_size_percentage()
    }

    fn scrollability(&self) -> crate::ui::popups::Scrollability {
        self.base.scrollability()
    }
}