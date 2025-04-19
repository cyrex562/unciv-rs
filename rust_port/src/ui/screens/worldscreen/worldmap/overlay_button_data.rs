// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/worldmap/OverlayButtonData.kt

use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use egui::{Color32, Image, Response, Ui, Vec2};
use crate::game::unit::Unit;
use crate::game::tile::Tile;
use crate::game::city::City;
use crate::game::spy::Spy;
use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::image_getter::ImageGetter;
use crate::ui::screens::worldscreen::worldmap::WorldMapHolder;
use crate::game::unit_automation::UnitAutomation;
use crate::models::unit_action_type::UnitActionType;
use crate::ui::audio::SoundPlayer;
use crate::ui::screens::overviewscreen::EspionageOverviewScreen;

/// Interface for creating floating "action" buttons on tiles
pub trait OverlayButtonData {
    /// Creates a button for the overlay
    fn create_button(&self, world_map_holder: &mut WorldMapHolder) -> Response;
}

/// Constants for button sizes
pub const BUTTON_SIZE: f32 = 60.0;
pub const SMALLER_CIRCLE_SIZES: f32 = 25.0;

/// Data for creating a "Move Here" button
pub struct MoveHereOverlayButtonData {
    /// Map of units to turns needed to reach destination
    unit_to_turns_to_destination: HashMap<Rc<RefCell<Unit>>, i32>,
    /// The target tile
    tile: Rc<RefCell<Tile>>,
}

impl MoveHereOverlayButtonData {
    /// Creates a new MoveHereOverlayButtonData
    pub fn new(unit_to_turns_to_destination: HashMap<Rc<RefCell<Unit>>, i32>, tile: Rc<RefCell<Tile>>) -> Self {
        Self {
            unit_to_turns_to_destination,
            tile,
        }
    }

    /// Gets the move here button
    fn get_move_here_button(&self, world_map_holder: &mut WorldMapHolder) -> Response {
        let is_paradrop = self.unit_to_turns_to_destination.keys().all(|unit| unit.borrow().is_preparing_paradrop());

        let image = if is_paradrop {
            ImageGetter::get_unit_action_portrait("Paradrop", BUTTON_SIZE / 2.0)
        } else {
            let mut movement_icon = ImageGetter::get_stat_icon("Movement");
            movement_icon.color = Color32::from_rgb(50, 50, 50); // CHARCOAL
            movement_icon.size = Vec2::new(BUTTON_SIZE / 2.0, BUTTON_SIZE / 2.0);
            movement_icon
        };

        // Create the button
        let mut response = Response::default();

        // TODO: Implement the full button UI with egui
        // This is a simplified version of the original Kotlin implementation

        // Add click handler
        if response.clicked() {
            let units_that_can_move: Vec<_> = self.unit_to_turns_to_destination.keys()
                .filter(|unit| unit.borrow().has_movement())
                .cloned()
                .collect();

            if !units_that_can_move.is_empty() {
                world_map_holder.move_unit_to_target_tile(units_that_can_move, self.tile.clone());
            }
        }

        response
    }
}

impl OverlayButtonData for MoveHereOverlayButtonData {
    fn create_button(&self, world_map_holder: &mut WorldMapHolder) -> Response {
        self.get_move_here_button(world_map_holder)
    }
}

/// Data for creating a "Swap With" button
pub struct SwapWithOverlayButtonData {
    /// The unit to swap
    unit: Rc<RefCell<Unit>>,
    /// The target tile
    tile: Rc<RefCell<Tile>>,
}

impl SwapWithOverlayButtonData {
    /// Creates a new SwapWithOverlayButtonData
    pub fn new(unit: Rc<RefCell<Unit>>, tile: Rc<RefCell<Tile>>) -> Self {
        Self {
            unit,
            tile,
        }
    }

    /// Gets the swap with button
    fn get_swap_with_button(&self, world_map_holder: &mut WorldMapHolder) -> Response {
        // Create the button
        let mut response = Response::default();

        // TODO: Implement the full button UI with egui
        // This is a simplified version of the original Kotlin implementation

        // Add click handler
        if response.clicked() {
            world_map_holder.swap_move_unit_to_target_tile(self.unit.clone(), self.tile.clone());
        }

        response
    }
}

impl OverlayButtonData for SwapWithOverlayButtonData {
    fn create_button(&self, world_map_holder: &mut WorldMapHolder) -> Response {
        self.get_swap_with_button(world_map_holder)
    }
}

/// Data for creating a "Connect Road" button
pub struct ConnectRoadOverlayButtonData {
    /// The unit to connect road with
    unit: Rc<RefCell<Unit>>,
    /// The target tile
    tile: Rc<RefCell<Tile>>,
}

impl ConnectRoadOverlayButtonData {
    /// Creates a new ConnectRoadOverlayButtonData
    pub fn new(unit: Rc<RefCell<Unit>>, tile: Rc<RefCell<Tile>>) -> Self {
        Self {
            unit,
            tile,
        }
    }

    /// Gets the connect road button
    fn get_connect_road_button(&self, world_map_holder: &mut WorldMapHolder) -> Response {
        // Create the button
        let mut response = Response::default();

        // TODO: Implement the full button UI with egui
        // This is a simplified version of the original Kotlin implementation

        // Add click handler
        if response.clicked() {
            self.connect_road_to_target_tile(world_map_holder, self.unit.clone(), self.tile.clone());
        }

        response
    }

    /// Connects a road to the target tile
    fn connect_road_to_target_tile(&self, world_map_holder: &mut WorldMapHolder, selected_unit: Rc<RefCell<Unit>>, target_tile: Rc<RefCell<Tile>>) {
        let mut unit = selected_unit.borrow_mut();
        unit.automated_road_connection_destination = Some(target_tile.borrow().position);
        unit.automated_road_connection_path = None;
        unit.action = Some(UnitActionType::ConnectRoad.to_string());
        unit.automated = true;

        // Play sound
        SoundPlayer::play("wagon");

        // Update UI
        world_map_holder.world_screen.borrow_mut().should_update = true;
        world_map_holder.remove_unit_action_overlay();

        // Make highlighting go away
        world_map_holder.world_screen.borrow_mut().bottom_unit_table.selected_unit_is_connecting_road = false;
    }
}

impl OverlayButtonData for ConnectRoadOverlayButtonData {
    fn create_button(&self, world_map_holder: &mut WorldMapHolder) -> Response {
        self.get_connect_road_button(world_map_holder)
    }
}

/// Data for creating a "Move Spy" button
pub struct MoveSpyOverlayButtonData {
    /// The spy to move
    spy: Rc<RefCell<Spy>>,
    /// The target city, if any
    city: Option<Rc<RefCell<City>>>,
}

impl MoveSpyOverlayButtonData {
    /// Creates a new MoveSpyOverlayButtonData
    pub fn new(spy: Rc<RefCell<Spy>>, city: Option<Rc<RefCell<City>>>) -> Self {
        Self {
            spy,
            city,
        }
    }

    /// Gets the move spy button
    fn get_move_spy_button(&self, world_map_holder: &mut WorldMapHolder) -> Response {
        // Create the button
        let mut response = Response::default();

        // TODO: Implement the full button UI with egui
        // This is a simplified version of the original Kotlin implementation

        // Add click handler
        if response.clicked() {
            let world_screen = world_map_holder.world_screen.clone();
            let mut world_screen = world_screen.borrow_mut();

            if let Some(city) = &self.city {
                self.spy.borrow_mut().move_to(city.clone());
                world_screen.game.borrow_mut().push_screen(Box::new(EspionageOverviewScreen::new(
                    world_screen.selected_civ.clone(),
                    world_screen.clone()
                )));
            } else {
                world_screen.game.borrow_mut().push_screen(Box::new(EspionageOverviewScreen::new(
                    world_screen.selected_civ.clone(),
                    world_screen.clone()
                )));
                world_screen.bottom_unit_table.select_spy(None);
            }

            world_map_holder.remove_unit_action_overlay();
            world_map_holder.selected_tile = None;
            world_screen.should_update = true;
            world_screen.bottom_unit_table.select_spy(None);
        }

        response
    }
}

impl OverlayButtonData for MoveSpyOverlayButtonData {
    fn create_button(&self, world_map_holder: &mut WorldMapHolder) -> Response {
        self.get_move_spy_button(world_map_holder)
    }
}