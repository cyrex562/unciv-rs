// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/worldmap/WorldMapHolder.kt

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::cell::RefCell;
use std::time::{Duration, Instant};
use egui::{Color32, Response, Ui, Vec2, Rect, Pos2, Sense};
use crate::game::unit::Unit;
use crate::game::tile::Tile;
use crate::game::tile_map::TileMap;
use crate::game::civilization::Civilization;
use crate::game::city::City;
use crate::game::spy::Spy;
use crate::game::battle::{Battle, MapUnitCombatant, TargetHelper};
use crate::game::map_pathing::MapPathing;
use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::screens::worldscreen::worldmap::overlay_button_data::{OverlayButtonData, BUTTON_SIZE};
use crate::ui::audio::SoundPlayer;
use crate::ui::components::map_arrow_type::MapArrowType;
use crate::ui::components::misc_arrow_types::MiscArrowTypes;
use crate::ui::components::tile_groups::{TileGroup, TileGroupMap, TileSetStrings, WorldTileGroup};
use crate::ui::components::unit_icon_group::UnitIconGroup;
use crate::ui::components::zoomable_scroll_pane::ZoomableScrollPane;
use crate::utils::concurrency::Concurrency;

/// Holds the world map and manages its display and interaction
pub struct WorldMapHolder {
    /// Reference to the world screen
    pub world_screen: Rc<RefCell<WorldScreen>>,

    /// The tile map
    pub tile_map: Rc<RefCell<TileMap>>,

    /// The currently selected tile
    pub selected_tile: Option<Rc<RefCell<Tile>>>,

    /// Map of tiles to their visual representations
    pub tile_groups: HashMap<Rc<RefCell<Tile>>, Rc<RefCell<WorldTileGroup>>>,

    /// Holds buttons created by OverlayButtonData implementations
    pub unit_action_overlays: Vec<Response>,

    /// Map of units to their movement paths
    pub unit_movement_paths: HashMap<Rc<RefCell<Unit>>, Vec<Rc<RefCell<Tile>>>>,

    /// Map of units to their road connection paths
    pub unit_connect_road_paths: HashMap<Rc<RefCell<Unit>>, Vec<Rc<RefCell<Tile>>>>,

    /// The tile group map
    tile_group_map: Option<Rc<RefCell<TileGroupMap<WorldTileGroup>>>>,

    /// The current tile set strings
    current_tile_set_strings: Option<TileSetStrings>,

    /// Whether continuous scrolling is enabled on the X axis
    continuous_scrolling_x: bool,

    /// The minimum zoom level
    min_zoom: f32,

    /// The maximum zoom level
    max_zoom: f32,

    /// The current zoom scale
    scale_x: f32,

    /// The current scroll position on the X axis
    scroll_x: f32,

    /// The current scroll position on the Y axis
    scroll_y: f32,

    /// The maximum X value
    max_x: f32,

    /// The maximum Y value
    max_y: f32,

    /// Whether panning is in progress
    is_panning: bool,

    /// Whether zooming is in progress
    is_zooming: bool,

    /// The time when panning started
    pan_start_time: Option<Instant>,

    /// The time when zooming started
    zoom_start_time: Option<Instant>,

    /// The fling time
    fling_time: f32,

    last_tile_clicked: Option<Rc<RefCell<Tile>>>,
    scroll_position: Vec2,
    zoom_level: f32,
    is_dragging: bool,
    drag_start: Option<Pos2>,
    last_mouse_pos: Option<Pos2>,
}

impl WorldMapHolder {
    /// Creates a new WorldMapHolder
    pub fn new(world_screen: Rc<RefCell<WorldScreen>>, tile_map: Rc<RefCell<TileMap>>) -> Self {
        let mut holder = Self {
            world_screen,
            tile_map,
            selected_tile: None,
            tile_groups: HashMap::new(),
            unit_action_overlays: Vec::new(),
            unit_movement_paths: HashMap::new(),
            unit_connect_road_paths: HashMap::new(),
            tile_group_map: None,
            current_tile_set_strings: None,
            continuous_scrolling_x: false,
            min_zoom: 1.0,
            max_zoom: 4.0,
            scale_x: 1.0,
            scroll_x: 0.0,
            scroll_y: 0.0,
            max_x: 0.0,
            max_y: 0.0,
            is_panning: false,
            is_zooming: false,
            pan_start_time: None,
            zoom_start_time: None,
            fling_time: 0.0,
            last_tile_clicked: None,
            scroll_position: Vec2::ZERO,
            zoom_level: 1.0,
            is_dragging: false,
            drag_start: None,
            last_mouse_pos: None,
        };

        // Set up continuous scrolling if world wrap is enabled
        holder.continuous_scrolling_x = holder.tile_map.borrow().map_parameters.world_wrap;

        // Set up zoom pan listeners
        holder.setup_zoom_pan_listeners();

        holder
    }

    /// Sets up listeners for zoom and pan events
    fn setup_zoom_pan_listeners(&mut self) {
        // TODO: Implement zoom and pan listeners
        // This is a simplified version of the original Kotlin implementation
    }

    /// Adds tiles to the map
    pub fn add_tiles(&mut self) {
        let tile_set_strings = TileSetStrings::new(
            self.world_screen.borrow().game_info.borrow().ruleset.clone(),
            self.world_screen.borrow().game.borrow().settings.clone()
        );

        self.current_tile_set_strings = Some(tile_set_strings.clone());

        let tile_groups_new: Vec<Rc<RefCell<WorldTileGroup>>> = self.tile_map.borrow()
            .values()
            .map(|tile| Rc::new(RefCell::new(WorldTileGroup::new(tile.clone(), tile_set_strings.clone()))))
            .collect();

        let tile_group_map = Rc::new(RefCell::new(TileGroupMap::new(
            self,
            tile_groups_new.clone(),
            self.continuous_scrolling_x
        )));

        self.tile_group_map = Some(tile_group_map);

        for tile_group in tile_groups_new {
            let tile = tile_group.borrow().tile.clone();
            self.tile_groups.insert(tile.clone(), tile_group.clone());

            // Set up click handlers
            let world_screen = self.world_screen.clone();
            let tile_clone = tile.clone();
            tile_group.borrow_mut().layer_city_button.on_click(Box::new(move |_| {
                world_screen.borrow_mut().on_tile_clicked(tile_clone.clone());
            }));

            let world_screen = self.world_screen.clone();
            let tile_clone = tile.clone();
            tile_group.borrow_mut().on_click(Box::new(move |_| {
                world_screen.borrow_mut().on_tile_clicked(tile_clone.clone());
            }));

            // Set up right-click handler
            let world_screen = self.world_screen.clone();
            let tile_clone = tile.clone();
            tile_group.borrow_mut().on_right_click(Box::new(move |_| {
                if !world_screen.borrow().game.borrow().settings.long_tap_move {
                    return;
                }

                let unit = world_screen.borrow().bottom_unit_table.selected_unit.clone();
                if unit.is_none() {
                    return;
                }

                Concurrency::run("WorldScreenClick", move || {
                    world_screen.borrow_mut().on_tile_right_clicked(unit.unwrap(), tile_clone.clone());
                });
            }));
        }

        // Set the size of the scroll pane
        self.set_size(
            self.world_screen.borrow().stage.width,
            self.world_screen.borrow().stage.height
        );

        // Layout the scroll pane
        self.layout();
    }

    /// Handles a tile click
    pub fn on_tile_clicked(&mut self, tile: Rc<RefCell<Tile>>) {
        let viewing_civ = self.world_screen.borrow().viewing_civ.clone();

        // Check if the tile is explored
        if !viewing_civ.borrow().has_explored(&tile.borrow()) &&
            tile.borrow().neighbors.iter().all(|neighbor| viewing_civ.borrow().has_explored(neighbor)) {
            return; // This tile doesn't exist for you
        }

        self.remove_unit_action_overlay();
        self.selected_tile = Some(tile.clone());
        self.unit_movement_paths.clear();
        self.unit_connect_road_paths.clear();

        let unit_table = self.world_screen.borrow().bottom_unit_table.clone();
        let previous_selected_units = unit_table.selected_units.clone();
        let previous_selected_city = unit_table.selected_city.clone();
        let previous_selected_unit_is_swapping = unit_table.selected_unit_is_swapping;
        let previous_selected_unit_is_connecting_road = unit_table.selected_unit_is_connecting_road;
        let moving_spy_on_map = unit_table.selected_spy.is_some();

        if !moving_spy_on_map {
            unit_table.tile_selected(tile.clone());
        }

        let new_selected_unit = unit_table.selected_unit.clone();

        // Handle city selection
        if let Some(previous_city) = previous_selected_city {
            if tile.borrow().position != previous_city.borrow().get_center_tile().borrow().position && !moving_spy_on_map {
                if let Some(tile_group) = self.tile_groups.get(&previous_city.borrow().get_center_tile()) {
                    tile_group.borrow_mut().layer_city_button.move_up();
                }
            }
        }

        // Handle unit selection
        if !previous_selected_units.is_empty() {
            let is_tile_different = previous_selected_units.iter().any(|unit| unit.borrow().get_tile().borrow().position != tile.borrow().position);
            let is_player_turn = self.world_screen.borrow().is_players_turn;
            let exists_unit_not_preparing_air_sweep = previous_selected_units.iter().any(|unit| !unit.borrow().is_preparing_air_sweep());

            // Check if we can perform actions on the tile
            let can_perform_actions_on_tile = if previous_selected_unit_is_swapping {
                if let Some(unit) = previous_selected_units.first() {
                    unit.borrow().movement.can_unit_swap_to(&tile.borrow())
                } else {
                    false
                }
            } else if previous_selected_unit_is_connecting_road {
                true
            } else {
                previous_selected_units.iter().any(|unit| {
                    unit.borrow().movement.can_move_to(&tile.borrow()) ||
                    (unit.borrow().movement.is_unknown_tile_we_should_assume_to_be_passable(&tile.borrow()) && !unit.borrow().base_unit.moves_like_air_units)
                })
            }

            if is_tile_different && is_player_turn && can_perform_actions_on_tile && exists_unit_not_preparing_air_sweep {
                if previous_selected_unit_is_swapping {
                    if let Some(unit) = previous_selected_units.first() {
                        self.add_tile_overlays_with_unit_swapping(unit.clone(), tile.clone());
                    }
                } else if previous_selected_unit_is_connecting_road {
                    if let Some(unit) = previous_selected_units.first() {
                        self.add_tile_overlays_with_unit_road_connecting(unit.clone(), tile.clone());
                    }
                } else {
                    self.add_tile_overlays_with_unit_movement(previous_selected_units.clone(), tile.clone());
                }
            }
        } else if moving_spy_on_map {
            if let Some(spy) = unit_table.selected_spy.clone() {
                self.add_moving_spy_overlay(spy, tile.clone());
            }
        } else {
            self.add_tile_overlays(tile.clone());
        }

        // Handle city bombardment
        if new_selected_unit.is_none() || (new_selected_unit.is_some() && new_selected_unit.unwrap().borrow().is_civilian()) {
            let units_in_tile = tile.borrow().get_units();
            if let Some(previous_city) = previous_selected_city {
                if previous_city.borrow().can_bombard() &&
                    tile.borrow().get_tiles_in_distance(2).contains(&previous_city.borrow().get_center_tile()) &&
                    !units_in_tile.is_empty() &&
                    units_in_tile[0].borrow().civ.is_at_war_with(&viewing_civ.borrow()) {
                    // Try to select the closest city to bombard this guy
                    unit_table.city_selected(previous_city);
                }
            }
        }

        self.world_screen.borrow_mut().should_update = true;
    }

    /// Handles a right-click on a tile
    pub fn on_tile_right_clicked(&mut self, unit: Rc<RefCell<Unit>>, tile: Rc<RefCell<Tile>>) {
        self.remove_unit_action_overlay();
        self.selected_tile = Some(tile.clone());
        self.unit_movement_paths.clear();
        self.unit_connect_road_paths.clear();

        if !self.world_screen.borrow().can_change_state {
            return;
        }

        let mut local_should_update = self.world_screen.borrow().should_update;
        self.world_screen.borrow_mut().should_update = false;

        if self.world_screen.borrow().bottom_unit_table.selected_unit_is_swapping {
            // Right-click Swap
            if unit.borrow().movement.can_unit_swap_to(&tile.borrow()) {
                self.swap_move_unit_to_target_tile(unit.clone(), tile.clone());
                local_should_update = true;
            }
            // If we are in unit-swapping mode and didn't find a swap partner, we don't want to move or attack
        } else {
            // Check if we can attack
            let attackable_tile = TargetHelper::get_attackable_enemies(&unit.borrow(), &unit.borrow().movement.get_distance_to_tiles())
                .into_iter()
                .find(|attackable| attackable.tile_to_attack.borrow().position == tile.borrow().position);

            if unit.borrow().can_attack() && attackable_tile.is_some() {
                // Right-click Attack
                let attacker = MapUnitCombatant::new(unit.clone());
                if !Battle::move_preparing_attack(&attacker, &attackable_tile.unwrap()) {
                    return;
                }

                SoundPlayer::play(attacker.get_attack_sound());

                let (damage_to_defender, damage_to_attacker) = Battle::attack_or_nuke(&attacker, &attackable_tile.unwrap());

                if attackable_tile.unwrap().combatant.is_some() {
                    self.world_screen.borrow_mut().battle_animation_deferred(
                        attacker,
                        damage_to_attacker,
                        attackable_tile.unwrap().combatant.unwrap(),
                        damage_to_defender
                    );
                }

                local_should_update = true;
            } else if unit.borrow().movement.can_reach(&tile.borrow()) {
                // Right-click Move
                self.move_unit_to_target_tile(vec![unit.clone()], tile.clone());
                local_should_update = true;
            }
        }

        self.world_screen.borrow_mut().should_update = local_should_update;
    }

    /// Marks a unit move tutorial as complete
    fn mark_unit_move_tutorial_complete(&self, unit: &Rc<RefCell<Unit>>) {
        let key = if unit.borrow().base_unit.moves_like_air_units {
            "Move an air unit"
        } else {
            "Move unit"
        };

        self.world_screen.borrow().game.borrow_mut().settings.add_completed_tutorial_task(key);
    }

    /// Moves a unit to a target tile
    pub fn move_unit_to_target_tile(&mut self, selected_units: Vec<Rc<RefCell<Unit>>>, target_tile: Rc<RefCell<Tile>>) {
        if selected_units.is_empty() {
            return;
        }

        let selected_unit = selected_units[0].clone();
        self.mark_unit_move_tutorial_complete(&selected_unit);

        Concurrency::run("TileToMoveTo", move || {
            // These are the heavy parts, finding where we want to go
            let tile_to_move_to;
            let mut path_to_tile = None;

            match selected_unit.borrow().movement.get_tile_to_move_to_this_turn(&target_tile.borrow()) {
                Ok(tile) => {
                    tile_to_move_to = tile;

                    if !selected_unit.borrow().type_is_air_unit() && !selected_unit.borrow().is_preparing_paradrop() {
                        path_to_tile = Some(selected_unit.borrow().movement.get_distance_to_tiles().get_path_to_tile(&tile_to_move_to.borrow()));
                    }
                },
                Err(e) => {
                    // This is normal e.g. when selecting an air unit then right-clicking on an empty tile
                    // Or telling a ship to run onto a coastal land tile.
                    // Do nothing
                    return;
                }
            }

            self.world_screen.borrow_mut().record_undo_checkpoint();

            // Launch on GL thread
            let world_screen = self.world_screen.clone();
            let selected_unit_clone = selected_unit.clone();
            let tile_to_move_to_clone = tile_to_move_to.clone();
            let path_to_tile_clone = path_to_tile.clone();
            let target_tile_clone = target_tile.clone();
            let selected_units_clone = selected_units.clone();

            Concurrency::launch_on_gl_thread(move || {
                let previous_tile = selected_unit_clone.borrow().current_tile.clone();

                match selected_unit_clone.borrow_mut().movement.move_to_tile(tile_to_move_to_clone.clone()) {
                    Ok(_) => {
                        if selected_unit_clone.borrow().is_exploring() || selected_unit_clone.borrow().is_moving() {
                            selected_unit_clone.borrow_mut().action = None; // remove explore on manual move
                        }

                        SoundPlayer::play("Whoosh");

                        if selected_unit_clone.borrow().current_tile.borrow().position != target_tile_clone.borrow().position {
                            selected_unit_clone.borrow_mut().action = Some(format!(
                                "moveTo {},{},{}",
                                target_tile_clone.borrow().position.x,
                                target_tile_clone.borrow().position.y,
                                target_tile_clone.borrow().position.z
                            ));
                        }

                        if selected_unit_clone.borrow().has_movement() {
                            world_screen.borrow_mut().bottom_unit_table.select_unit(selected_unit_clone.clone());
                        }

                        world_screen.borrow_mut().should_update = true;

                        if let Some(path) = path_to_tile_clone {
                            self.animate_movement(previous_tile, selected_unit_clone.clone(), tile_to_move_to_clone.clone(), path);

                            if selected_unit_clone.borrow().is_escorting() {
                                if let Some(other_escort_unit) = selected_unit_clone.borrow().get_other_escort_unit() {
                                    self.animate_movement(previous_tile, other_escort_unit, tile_to_move_to_clone.clone(), path);
                                }
                            }
                        }

                        if selected_units_clone.len() > 1 {
                            // We have more tiles to move
                            let remaining_units = selected_units_clone[1..].to_vec();
                            self.move_unit_to_target_tile(remaining_units, target_tile_clone);
                        } else {
                            self.remove_unit_action_overlay(); // we're done here
                        }

                        if world_screen.borrow().game.borrow().settings.auto_unit_cycle && !selected_unit_clone.borrow().has_movement() {
                            world_screen.borrow_mut().switch_to_next_unit();
                        }
                    },
                    Err(e) => {
                        // Log error
                        println!("Exception in moveUnitToTargetTile: {:?}", e);
                    }
                }
            });
        });
    }

    /// Animates unit movement
    fn animate_movement(
        &self,
        previous_tile: Rc<RefCell<Tile>>,
        selected_unit: Rc<RefCell<Unit>>,
        target_tile: Rc<RefCell<Tile>>,
        path_to_tile: Vec<Rc<RefCell<Tile>>>
    ) {
        // TODO: Implement animation
        // This is a simplified version of the original Kotlin implementation
    }

    /// Swaps a unit to a target tile
    pub fn swap_move_unit_to_target_tile(&mut self, selected_unit: Rc<RefCell<Unit>>, target_tile: Rc<RefCell<Tile>>) {
        self.mark_unit_move_tutorial_complete(&selected_unit);

        match selected_unit.borrow_mut().movement.swap_move_to_tile(target_tile.clone()) {
            Ok(_) => {
                if selected_unit.borrow().is_exploring() || selected_unit.borrow().is_moving() {
                    selected_unit.borrow_mut().action = None; // remove explore on manual swap-move
                }

                // Play something like a swish-swoosh
                SoundPlayer::play("Swap");

                if selected_unit.borrow().has_movement() {
                    self.world_screen.borrow_mut().bottom_unit_table.select_unit(selected_unit.clone());
                }

                self.world_screen.borrow_mut().should_update = true;
                self.remove_unit_action_overlay();
            },
            Err(e) => {
                // Log error
                println!("Exception in swapMoveUnitToTargetTile: {:?}", e);
            }
        }
    }

    /// Adds tile overlays with unit movement
    fn add_tile_overlays_with_unit_movement(&mut self, selected_units: Vec<Rc<RefCell<Unit>>>, tile: Rc<RefCell<Tile>>) {
        Concurrency::run("TurnsToGetThere", move || {
            let mut unit_to_turns_to_tile = HashMap::new();

            for unit in selected_units {
                let mut shortest_path = Vec::new();
                let turns_to_get_there = if unit.borrow().base_unit.moves_like_air_units {
                    if unit.borrow().movement.can_reach(&tile.borrow()) {
                        1
                    } else {
                        0
                    }
                } else if unit.borrow().is_preparing_paradrop() {
                    if unit.borrow().movement.can_reach(&tile.borrow()) {
                        1
                    } else {
                        0
                    }
                } else {
                    // This is the most time-consuming call
                    shortest_path = unit.borrow().movement.get_shortest_path(&tile.borrow());
                    shortest_path.len() as i32
                };

                self.unit_movement_paths.insert(unit.clone(), shortest_path);
                unit_to_turns_to_tile.insert(unit, turns_to_get_there);
            }

            // Launch on GL thread
            let world_screen = self.world_screen.clone();
            let unit_to_turns_to_tile_clone = unit_to_turns_to_tile.clone();
            let tile_clone = tile.clone();

            Concurrency::launch_on_gl_thread(move || {
                let units_who_can_move_there: HashMap<_, _> = unit_to_turns_to_tile_clone.iter()
                    .filter(|(_, turns)| *turns > 0)
                    .map(|(unit, turns)| (unit.clone(), *turns))
                    .collect();

                if units_who_can_move_there.is_empty() {
                    // Give the regular tile overlays with no unit movement
                    self.add_tile_overlays(tile_clone.clone());
                    world_screen.borrow_mut().should_update = true;
                    return;
                }

                let turns_to_get_there = units_who_can_move_there.values().max().unwrap();

                if world_screen.borrow().game.borrow().settings.single_tap_move && *turns_to_get_there == 1 {
                    // Single turn instant move
                    let selected_unit = units_who_can_move_there.keys().next().unwrap().clone();

                    for unit in units_who_can_move_there.keys() {
                        unit.borrow_mut().movement.head_towards(tile_clone.clone());
                    }

                    world_screen.borrow_mut().bottom_unit_table.select_unit(selected_unit.clone());
                } else {
                    // Add "move to" button if there is a path to tile
                    let move_here_button_dto = MoveHereOverlayButtonData::new(units_who_can_move_there, tile_clone.clone());
                    self.add_tile_overlays_with_button(tile_clone.clone(), Some(Box::new(move_here_button_dto)));
                }

                world_screen.borrow_mut().should_update = true;
            });
        });
    }

    /// Adds tile overlays with unit swapping
    fn add_tile_overlays_with_unit_swapping(&mut self, selected_unit: Rc<RefCell<Unit>>, tile: Rc<RefCell<Tile>>) {
        if !selected_unit.borrow().movement.can_unit_swap_to(&tile.borrow()) {
            // Give the regular tile overlays with no unit swapping
            self.add_tile_overlays(tile.clone());
            self.world_screen.borrow_mut().should_update = true;
            return;
        }

        if self.world_screen.borrow().game.borrow().settings.single_tap_move {
            self.swap_move_unit_to_target_tile(selected_unit.clone(), tile.clone());
        } else {
            // Add "swap with" button
            let swap_with_button_dto = SwapWithOverlayButtonData::new(selected_unit.clone(), tile.clone());
            self.add_tile_overlays_with_button(tile.clone(), Some(Box::new(swap_with_button_dto)));
        }

        self.world_screen.borrow_mut().should_update = true;
    }

    /// Adds tile overlays with unit road connecting
    fn add_tile_overlays_with_unit_road_connecting(&mut self, selected_unit: Rc<RefCell<Unit>>, tile: Rc<RefCell<Tile>>) {
        Concurrency::run("ConnectRoad", move || {
            let valid_tile = tile.borrow().is_land &&
                !tile.borrow().is_impassible() &&
                selected_unit.borrow().civ.borrow().has_explored(&tile.borrow());

            if valid_tile {
                let road_path = MapPathing::get_road_path(&selected_unit.borrow(), &selected_unit.borrow().current_tile, &tile.borrow());

                // Launch on GL thread
                let world_screen = self.world_screen.clone();
                let selected_unit_clone = selected_unit.clone();
                let tile_clone = tile.clone();
                let road_path_clone = road_path.clone();

                Concurrency::launch_on_gl_thread(move || {
                    if road_path_clone.is_none() {
                        // Give the regular tile overlays with no road connection
                        self.add_tile_overlays(tile_clone.clone());
                        world_screen.borrow_mut().should_update = true;
                        return;
                    }

                    self.unit_connect_road_paths.insert(selected_unit_clone.clone(), road_path_clone.unwrap());
                    let connect_road_button_dto = ConnectRoadOverlayButtonData::new(selected_unit_clone.clone(), tile_clone.clone());
                    self.add_tile_overlays_with_button(tile_clone.clone(), Some(Box::new(connect_road_button_dto)));
                    world_screen.borrow_mut().should_update = true;
                });
            }
        });
    }

    /// Adds moving spy overlay
    fn add_moving_spy_overlay(&mut self, spy: Rc<RefCell<Spy>>, tile: Rc<RefCell<Tile>>) {
        let city = if tile.borrow().is_city_center() && spy.borrow().can_move_to(&tile.borrow().get_city().unwrap()) {
            Some(tile.borrow().get_city().unwrap())
        } else {
            None
        };

        let move_spy_button_dto = MoveSpyOverlayButtonData::new(spy, city);
        self.add_tile_overlays_with_button(tile, Some(Box::new(move_spy_button_dto)));
        self.world_screen.borrow_mut().should_update = true;
    }

    /// Adds tile overlays
    fn add_tile_overlays(&mut self, tile: Rc<RefCell<Tile>>) {
        self.add_tile_overlays_with_button(tile, None);
    }

    /// Adds tile overlays with a button
    fn add_tile_overlays_with_button(&mut self, tile: Rc<RefCell<Tile>>, button_dto: Option<Box<dyn OverlayButtonData>>) {
        // TODO: Implement the full UI with egui
        // This is a simplified version of the original Kotlin implementation
    }

    /// Adds an overlay on a tile group
    fn add_overlay_on_tile_group(&mut self, group: &Rc<RefCell<WorldTileGroup>>, response: Response) {
        // TODO: Implement the full UI with egui
        // This is a simplified version of the original Kotlin implementation
    }

    /// Returns true when the civ is a human player defeated in singleplayer game
    pub fn is_map_reveal_enabled(&self, viewing_civ: &Rc<RefCell<Civilization>>) -> bool {
        !viewing_civ.borrow().game_info.borrow().game_parameters.is_online_multiplayer &&
            viewing_civ.borrow().is_current_player() &&
            viewing_civ.borrow().is_defeated()
    }

    /// Clears all arrows to be drawn on the next update
    pub fn reset_arrows(&mut self) {
        for tile_group in self.tile_groups.values() {
            tile_group.borrow_mut().layer_misc.reset_arrows();
        }
    }

    /// Adds an arrow to draw on the next update
    pub fn add_arrow(&mut self, from_tile: &Rc<RefCell<Tile>>, to_tile: &Rc<RefCell<Tile>>, arrow_type: MapArrowType) {
        if let Some(tile_group) = self.tile_groups.get(from_tile) {
            tile_group.borrow_mut().layer_misc.add_arrow(to_tile.clone(), arrow_type);
        }
    }

    /// Updates the movement overlay
    pub fn update_movement_overlay(
        &mut self,
        past_visible_units: &[Rc<RefCell<Unit>>],
        target_visible_units: &[Rc<RefCell<Unit>>],
        visible_attacks: &[(Rc<RefCell<Tile>>, Rc<RefCell<Tile>>)]
    ) {
        let selected_unit = self.world_screen.borrow().bottom_unit_table.selected_unit.clone();

        for unit in past_visible_units {
            if unit.borrow().movement_memories.is_empty() {
                continue;
            }

            if selected_unit.is_some() && selected_unit.as_ref().unwrap() != unit {
                continue; // When selecting a unit, show only arrows of that unit
            }

            let mut step_iter = unit.borrow().movement_memories.iter();
            let mut previous = step_iter.next().unwrap();

            while let Some(next) = step_iter.next() {
                self.add_arrow(
                    &self.tile_map.borrow().get_tile_at_position(previous.position),
                    &self.tile_map.borrow().get_tile_at_position(next.position),
                    next.type_
                );
                previous = next;
            }

            self.add_arrow(
                &self.tile_map.borrow().get_tile_at_position(previous.position),
                &unit.borrow().get_tile(),
                unit.borrow().most_recent_move_type
            );
        }

        for unit in target_visible_units {
            if !unit.borrow().is_moving() {
                continue;
            }

            let to_tile = unit.borrow().get_movement_destination();
            self.add_arrow(unit.borrow().get_tile(), &to_tile, MiscArrowTypes::UnitMoving);
        }

        for (from, to) in visible_attacks {
            if selected_unit.is_some() &&
                selected_unit.as_ref().unwrap().borrow().current_tile.borrow().position != from.borrow().position &&
                selected_unit.as_ref().unwrap().borrow().current_tile.borrow().position != to.borrow().position {
                continue;
            }

            self.add_arrow(from, to, MiscArrowTypes::UnitHasAttacked);
        }
    }

    /// Sets the center position of the map
    pub fn set_center_position(&mut self, vector: (i32, i32, i32), immediately: bool, select_unit: bool, force_select_unit: Option<Rc<RefCell<Unit>>>) -> bool {
        let tile_group = self.tile_groups.values().find(|group| group.borrow().tile.borrow().position == vector);

        if tile_group.is_none() {
            return false;
        }

        let tile_group = tile_group.unwrap();
        self.selected_tile = Some(tile_group.borrow().tile.clone());

        if select_unit || force_select_unit.is_some() {
            self.world_screen.borrow_mut().bottom_unit_table.tile_selected(self.selected_tile.clone().unwrap(), force_select_unit);
        }

        // The Y axis of scrollY is inverted - when at 0 we're at the top, not bottom - so we invert it back
        let success = self.scroll_to(
            tile_group.borrow().x + tile_group.borrow().width / 2.0,
            self.max_y - (tile_group.borrow().y + tile_group.borrow().width / 2.0),
            immediately
        );

        if !success {
            return false;
        }

        // TODO: Implement blinking
        // This is a simplified version of the original Kotlin implementation

        self.world_screen.borrow_mut().should_update = true;
        true
    }

    /// Zooms the map
    pub fn zoom(&mut self, zoom_scale: f32) {
        // TODO: Implement zooming
        // This is a simplified version of the original Kotlin implementation

        self.clamp_city_button_size();
    }

    /// Clamps the city button size
    fn clamp_city_button_size(&mut self) {
        // TODO: Implement city button size clamping
        // This is a simplified version of the original Kotlin implementation
    }

    /// Removes the unit action overlay
    pub fn remove_unit_action_overlay(&mut self) {
        self.unit_action_overlays.clear();
    }

    /// Reloads the maximum zoom
    pub fn reload_max_zoom(&mut self) {
        let max_world_zoom_out = self.world_screen.borrow().game.borrow().settings.max_world_zoom_out;
        let map_radius = self.tile_map.borrow().map_parameters.map_size.radius;

        // Limit max zoom out by the map width
        let enable_zoom_limit = (map_radius < 21 && max_world_zoom_out < 3.0) || (map_radius > 20 && max_world_zoom_out < 4.0);

        if enable_zoom_limit {
            // For world-wrap we limit minimal possible zoom to content width + some extra offset
            // to hide one column of tiles so that the player doesn't see it teleporting from side to side
            let pad = if self.continuous_scrolling_x {
                self.width / map_radius as f32 * 0.7
            } else {
                0.0
            };

            self.min_zoom = (self.width + pad) * self.scale_x / self.max_x.max(1.0 / max_world_zoom_out);

            // If the window becomes too wide and minZoom > maxZoom, we cannot zoom
            self.max_zoom = (2.0 * self.min_zoom).max(max_world_zoom_out);
        } else {
            // TODO: Implement super.reloadMaxZoom()
            // This is a simplified version of the original Kotlin implementation
        }
    }

    /// Restricts the X scroll
    pub fn restrict_x(&self, delta_x: f32) -> f32 {
        let mut result = self.scroll_x - delta_x;

        if self.world_screen.borrow().viewing_civ.borrow().is_spectator() {
            return result;
        }

        let explored_region = self.world_screen.borrow().viewing_civ.borrow().explored_region.clone();

        if explored_region.borrow().should_recalculate_coords() {
            explored_region.borrow_mut().calculate_stage_coords(self.max_x, self.max_y);
        }

        if !explored_region.borrow().should_restrict_x() {
            return result;
        }

        let left_x = explored_region.borrow().get_left_x();
        let right_x = explored_region.borrow().get_right_x();

        if delta_x < 0.0 && self.scroll_x <= right_x && result > right_x {
            result = right_x;
        } else if delta_x > 0.0 && self.scroll_x >= left_x && result < left_x {
            result = left_x;
        }

        result
    }

    /// Restricts the Y scroll
    pub fn restrict_y(&self, delta_y: f32) -> f32 {
        let mut result = self.scroll_y + delta_y;

        if self.world_screen.borrow().viewing_civ.borrow().is_spectator() {
            return result;
        }

        let explored_region = self.world_screen.borrow().viewing_civ.borrow().explored_region.clone();

        if explored_region.borrow().should_recalculate_coords() {
            explored_region.borrow_mut().calculate_stage_coords(self.max_x, self.max_y);
        }

        let top_y = explored_region.borrow().get_top_y();
        let bottom_y = explored_region.borrow().get_bottom_y();

        if result < top_y {
            result = top_y;
        } else if result > bottom_y {
            result = bottom_y;
        }

        result
    }

    /// Sets the size of the map
    pub fn set_size(&mut self, width: f32, height: f32) {
        // TODO: Implement set_size
        // This is a simplified version of the original Kotlin implementation
    }

    /// Layouts the map
    pub fn layout(&mut self) {
        // TODO: Implement layout
        // This is a simplified version of the original Kotlin implementation
    }

    /// Scrolls to a position
    pub fn scroll_to(&mut self, x: f32, y: f32, immediately: bool) -> bool {
        // TODO: Implement scroll_to
        // This is a simplified version of the original Kotlin implementation
        true
    }

    /// Gets the width of the map
    pub fn get_width(&self) -> f32 {
        // TODO: Implement get_width
        // This is a simplified version of the original Kotlin implementation
        0.0
    }

    /// Gets the height of the map
    pub fn get_height(&self) -> f32 {
        // TODO: Implement get_height
        // This is a simplified version of the original Kotlin implementation
        0.0
    }

    /// Gets the width of the map
    pub fn width(&self) -> f32 {
        self.get_width()
    }

    /// Gets the height of the map
    pub fn height(&self) -> f32 {
        self.get_height()
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size(),
            Sense::click_and_drag()
        );

        self.handle_input(&response);
        self.update_map();
        self.draw_map(ui, rect);
    }

    fn handle_input(&mut self, response: &Response) {
        // Handle zooming
        if let Some(scroll_delta) = response.scroll_delta.y {
            self.zoom_level = (self.zoom_level + scroll_delta * 0.001).clamp(0.5, 2.0);
        }

        // Handle dragging
        if response.dragged() {
            if !self.is_dragging {
                self.drag_start = Some(response.hover_pos().unwrap_or_default());
                self.is_dragging = true;
            }
            let delta = response.drag_delta();
            self.scroll_position += delta;
        } else {
            self.is_dragging = false;
            self.drag_start = None;
        }

        // Handle clicking
        if response.clicked() {
            if let Some(pos) = response.hover_pos() {
                self.handle_click(pos);
            }
        }

        self.last_mouse_pos = response.hover_pos();
    }

    fn handle_click(&mut self, pos: Pos2) {
        // Convert screen position to map coordinates and find clicked tile
        if let Some(tile) = self.get_tile_at_position(pos) {
            self.select_tile(tile);
        }
    }

    fn select_tile(&mut self, tile: Rc<RefCell<Tile>>) {
        self.selected_tile = Some(Rc::clone(&tile));
        self.last_tile_clicked = Some(Rc::clone(&tile));

        // Update UI and game state based on selected tile
        let world_screen = self.world_screen.borrow();
        let viewing_civ = &world_screen.viewing_civ;

        // Handle unit selection, city interaction, etc.
        // TODO: Implement tile selection logic
    }

    fn update_map(&mut self) {
        let world_screen = self.world_screen.borrow();
        let viewing_civ = &world_screen.viewing_civ;

        // Update tile visibility and fog of war
        for tile_group in self.tile_groups.values() {
            tile_group.borrow_mut().update(viewing_civ);
        }

        // Update unit positions and animations
        // TODO: Implement unit updates
    }

    fn draw_map(&self, ui: &mut Ui, rect: Rect) {
        let transform = self.get_map_transform(rect);

        // Draw tiles
        for tile_group in self.tile_groups.values() {
            tile_group.borrow().draw(ui, &transform);
        }

        // Draw units
        // TODO: Implement unit drawing

        // Draw overlays (borders, resources, etc.)
        // TODO: Implement overlay drawing
    }

    fn get_map_transform(&self, rect: Rect) -> MapTransform {
        MapTransform {
            scale: self.zoom_level,
            offset: self.scroll_position,
            viewport: rect,
        }
    }

    fn get_tile_at_position(&self, screen_pos: Pos2) -> Option<Rc<RefCell<Tile>>> {
        // TODO: Implement tile hit testing
        None
    }
}

struct MapTransform {
    scale: f32,
    offset: Vec2,
    viewport: Rect,
}

// Additional helper structs and implementations
// TODO: Add more functionality as needed