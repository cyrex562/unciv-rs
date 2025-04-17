use std::rc::Rc;
use ggez::graphics::Color;
use ggez::mint::Point2;

use crate::logic::civilization::Civilization;
use crate::logic::map::mapunit::MapUnit;
use crate::models::counter::Counter;
use crate::models::upgrade_unit_action::UpgradeUnitAction;
use crate::ui::audio::sound_player::SoundPlayer;
use crate::ui::components::input::keyboard_binding::KeyboardBinding;
use crate::ui::components::widgets::color_markup_label::ColorMarkupLabel;
use crate::ui::object_descriptions::base_unit_descriptions::BaseUnitDescriptions;
use crate::ui::screens::worldscreen::unit::actions::unit_actions_upgrade::UnitActionsUpgrade;
use crate::ui::screens::base_screen::BaseScreen;
use crate::utils::translations::tr;

use super::scrollable_animated_menu_popup::ScrollableAnimatedMenuPopup;
use super::scrollable_animated_menu_popup::ScrollableAnimatedMenuPopupImpl;
use super::popup::Popup;

/// A popup menu showing info about an Unit upgrade, with buttons to upgrade "this" unit or _all_
/// similar units.
pub struct UnitUpgradeMenu {
    /// The base scrollable animated menu popup
    base: ScrollableAnimatedMenuPopupImpl,
    /// The unit that is ready to upgrade
    unit: Rc<MapUnit>,
    /// Holds pre-calculated info like unitToUpgradeTo, cost or resource requirements
    unit_action: Rc<UpgradeUnitAction>,
    /// Whether the buttons should be enabled
    enable: bool,
    /// If true, the callback will be delayed until the Popup is actually closed
    callback_after_animation: bool,
    /// A callback after one or several upgrades have been performed
    on_button_clicked: Box<dyn Fn()>,
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
    /// * `unit_action` - Holds pre-calculated info about the upgrade
    /// * `enable` - Whether the buttons should be enabled
    /// * `callback_after_animation` - If true, the callback will be delayed until the Popup is closed
    /// * `on_button_clicked` - A callback after upgrades have been performed
    pub fn new(
        screen: &Rc<BaseScreen>,
        position_next_to: Point2<f32>,
        unit: Rc<MapUnit>,
        unit_action: Rc<UpgradeUnitAction>,
        enable: bool,
        callback_after_animation: bool,
        on_button_clicked: Box<dyn Fn()>,
    ) -> Self {
        let mut menu = Self {
            base: ScrollableAnimatedMenuPopupImpl::new(
                screen.clone(),
                position_next_to,
                Box::new(move || Self::create_scrollable_content(&unit_action)),
                Box::new(move || Self::create_fixed_content(
                    &unit,
                    &unit_action,
                    enable,
                    &mut menu.any_button_was_clicked,
                )),
                None,
            ),
            unit,
            unit_action,
            enable,
            callback_after_animation,
            on_button_clicked,
            any_button_was_clicked: false,
        };

        // Set up the callback
        let action = Box::new(move || {
            if menu.any_button_was_clicked {
                (menu.on_button_clicked)();
            }
        });

        if callback_after_animation {
            menu.base.set_after_close_callback(Some(action));
        } else {
            menu.base.add_close_listener(action);
        }

        menu
    }

    /// Creates the scrollable content for the popup
    fn create_scrollable_content(unit_action: &UpgradeUnitAction) -> Option<Table> {
        BaseUnitDescriptions::get_upgrade_info_table(
            &unit_action.title,
            &unit_action.unit.base_unit,
            &unit_action.unit_to_upgrade_to,
        )
    }

    /// Creates the fixed content for the popup
    fn create_fixed_content(
        unit: &MapUnit,
        unit_action: &UpgradeUnitAction,
        enable: bool,
        any_button_was_clicked: &mut bool,
    ) -> Option<Table> {
        let mut table = Table::new();

        // Create the single upgrade button
        let mut single_button = Self::get_button(
            "Upgrade",
            KeyboardBinding::Upgrade,
            Box::new(move || Self::do_upgrade(unit_action, any_button_was_clicked)),
        );
        single_button.set_disabled(!enable);
        table.add(single_button).grow_x().row();

        // Get all upgradable units
        let all_upgradable_units = Self::get_all_upgradable_units(unit);
        let all_count = all_upgradable_units.len();

        if all_count <= 1 {
            return Some(table);
        }

        // Calculate the cost for upgrading all units
        let all_cost = unit_action.gold_cost_of_upgrade * all_count as i32;
        let all_resources = unit_action.new_resource_requirements.clone() * all_count as i32;

        // Create the upgrade all button
        let upgrade_all_text = format!(
            "Upgrade all [{}] [{}] ([{}] gold)",
            all_count,
            unit.name,
            all_cost
        );

        let mut upgrade_all_button = Self::get_button(
            &upgrade_all_text,
            KeyboardBinding::UpgradeAll,
            Box::new(move || Self::do_all_upgrade(&all_upgradable_units, unit_action, any_button_was_clicked)),
        );

        // Check if the player has enough resources
        let insufficient_gold = unit.civ.gold < all_cost;
        let insufficient_resources = Self::get_insufficient_resources_message(&all_resources, &unit.civ);

        upgrade_all_button.set_disabled(insufficient_gold || !insufficient_resources.is_empty());
        table.add(upgrade_all_button).pad_top(7.0).grow_x().row();

        if insufficient_resources.is_empty() {
            return Some(table);
        }

        // Add a label for insufficient resources
        let label = ColorMarkupLabel::new_with_color(&insufficient_resources, Color::new(1.0, 0.0, 0.0, 1.0));
        table.add(label).center();

        Some(table)
    }

    /// Gets all units that can be upgraded to the same unit type
    fn get_all_upgradable_units(unit: &MapUnit) -> Vec<Rc<MapUnit>> {
        let unit_to_upgrade_to = &unit.upgrade.unit_to_upgrade_to;

        unit.civ.units.get_civ_units()
            .into_iter()
            .filter(|u| {
                u.base_unit.name == unit.base_unit.name
                    && u.has_movement()
                    && u.current_tile.get_owner() == unit.civ
                    && !u.is_embarked()
                    && u.upgrade.can_upgrade(unit_to_upgrade_to, true)
            })
            .collect()
    }

    /// Gets a message about insufficient resources
    fn get_insufficient_resources_message(required_resources: &Counter<String>, civ: &Civilization) -> String {
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

    /// Performs the upgrade for a single unit
    fn do_upgrade(unit_action: &UpgradeUnitAction, any_button_was_clicked: &mut bool) {
        SoundPlayer::play(&unit_action.unciv_sound);
        *any_button_was_clicked = true;

        if let Some(action) = &unit_action.action {
            action();
        }
    }

    /// Performs the upgrade for all eligible units
    fn do_all_upgrade(
        all_upgradable_units: &[Rc<MapUnit>],
        unit_action: &UpgradeUnitAction,
        any_button_was_clicked: &mut bool,
    ) {
        SoundPlayer::play_repeated(&unit_action.unciv_sound);
        *any_button_was_clicked = true;

        let unit_to_upgrade_to = &unit_action.unit_to_upgrade_to;

        for unit in all_upgradable_units {
            let other_actions = UnitActionsUpgrade::get_upgrade_actions(unit);

            if let Some(other_action) = other_actions.into_iter().find(|a| {
                if let Some(upgrade_action) = a.downcast_ref::<UpgradeUnitAction>() {
                    upgrade_action.unit_to_upgrade_to == *unit_to_upgrade_to && upgrade_action.action.is_some()
                } else {
                    false
                }
            }) {
                if let Some(upgrade_action) = other_action.downcast_ref::<UpgradeUnitAction>() {
                    if let Some(action) = &upgrade_action.action {
                        action();
                    }
                }
            }
        }
    }

    /// Creates a button with the given text, keyboard binding, and action
    fn get_button(
        text: &str,
        binding: KeyboardBinding,
        action: Box<dyn Fn()>,
    ) -> Button {
        let mut button = Button::new(text);
        button.set_keyboard_binding(binding);
        button.on_click(action);
        button
    }
}

impl std::ops::Deref for UnitUpgradeMenu {
    type Target = ScrollableAnimatedMenuPopupImpl;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for UnitUpgradeMenu {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}