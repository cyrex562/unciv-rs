use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui::{Color32, Ui, Vec2};

use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::mapeditorscreen::MapEditorScreen;
use crate::ui::components::widgets::zoomable_scroll_pane::ZoomableScrollPane;
use crate::ui::components::tilegroups::tile_group::TileGroup;
use crate::ui::components::tilegroups::tile_group_map::TileGroupMap;
use crate::ui::components::tilegroups::tile_set_strings::TileSetStrings;
use crate::logic::map::hex_math::HexMath;
use crate::logic::map::tile::Tile;
use crate::logic::map::tile_map::TileMap;
use crate::utils::log::Log;

/// This MapHolder is used both for the Map Editor and the Main Menu background!
pub struct EditorMapHolder {
    parent_screen: Rc<RefCell<BaseScreen>>,
    tile_map: Rc<RefCell<TileMap>>,
    on_tile_click: Box<dyn Fn(&Tile)>,
    editor_screen: Option<Rc<RefCell<MapEditorScreen>>>,
    tile_groups: HashMap<Tile, Rc<RefCell<TileGroup>>>,
    tile_group_map: Option<Rc<RefCell<TileGroupMap<TileGroup>>>>,
    all_tile_groups: Vec<Rc<RefCell<TileGroup>>>,
    blink_action: Option<egui::Animation>,
    saved_capture_listeners: Vec<egui::EventListener>,
    saved_listeners: Vec<egui::EventListener>,
    scroll_pane: ZoomableScrollPane,
    is_dragging: bool,
    is_painting: bool,
    touch_down_time: Instant,
}

impl EditorMapHolder {
    pub fn new(
        parent_screen: Rc<RefCell<BaseScreen>>,
        tile_map: Rc<RefCell<TileMap>>,
        on_tile_click: Box<dyn Fn(&Tile)>,
    ) -> Self {
        let editor_screen = parent_screen.borrow().as_any().downcast_ref::<MapEditorScreen>()
            .map(|screen| Rc::new(RefCell::new(screen.clone())));

        let mut scroll_pane = ZoomableScrollPane::new(20.0, 20.0);
        scroll_pane.set_continuous_scrolling_x(tile_map.borrow().map_parameters.world_wrap);

        let mut holder = Self {
            parent_screen,
            tile_map,
            on_tile_click,
            editor_screen,
            tile_groups: HashMap::new(),
            tile_group_map: None,
            all_tile_groups: Vec::new(),
            blink_action: None,
            saved_capture_listeners: Vec::new(),
            saved_listeners: Vec::new(),
            scroll_pane,
            is_dragging: false,
            is_painting: false,
            touch_down_time: Instant::now(),
        };

        holder.add_tiles();

        if holder.editor_screen.is_some() {
            holder.setup_zoom_pan_listeners();
        }

        holder.reload_max_zoom();

        holder
    }

    /// See also: WorldMapHolder.setupZoomPanListeners
    fn setup_zoom_pan_listeners(&mut self) {
        let scroll_pane = &mut self.scroll_pane;

        scroll_pane.set_on_pan_start(Box::new(|| {
            // This will be set in the render method
        }));

        scroll_pane.set_on_pan_stop(Box::new(|| {
            // This will be set in the render method
        }));

        scroll_pane.set_on_zoom_start(Box::new(|| {
            // This will be set in the render method
        }));

        scroll_pane.set_on_zoom_stop(Box::new(|| {
            // This will be set in the render method
        }));
    }

    fn add_tiles(&mut self) {
        let stage = &self.parent_screen.borrow().stage;
        let tile_map = &self.tile_map.borrow();

        let tile_set_strings = if let Some(editor) = &self.editor_screen {
            let editor = editor.borrow();
            TileSetStrings::new(&editor.ruleset, &editor.game.settings)
        } else {
            TileSetStrings::new_empty()
        };

        let da_tile_groups: Vec<Rc<RefCell<TileGroup>>> = tile_map.values.iter()
            .map(|tile| Rc::new(RefCell::new(TileGroup::new(tile.clone(), tile_set_strings.clone()))))
            .collect();

        let continuous_scrolling_x = self.scroll_pane.continuous_scrolling_x;
        let tile_group_map = Rc::new(RefCell::new(TileGroupMap::new(
            self.scroll_pane.clone(),
            da_tile_groups.clone(),
            continuous_scrolling_x
        )));

        self.tile_group_map = Some(tile_group_map.clone());
        self.scroll_pane.set_actor(tile_group_map);

        for tile_group in &da_tile_groups {
            self.all_tile_groups.push(tile_group.clone());
            self.tile_groups.insert(tile_group.borrow().tile.clone(), tile_group.clone());
        }

        for tile_group in &self.all_tile_groups {
            let mut tile_group = tile_group.borrow_mut();
            tile_group.is_force_visible = true;
            tile_group.update();

            if self.scroll_pane.touchable {
                let tile = tile_group.tile.clone();
                let on_tile_click = self.on_tile_click.clone();
                tile_group.set_on_click(Box::new(move || {
                    on_tile_click(&tile);
                }));
            }
        }

        self.scroll_pane.set_size(stage.width, stage.height);
        self.scroll_pane.layout();

        self.scroll_pane.set_scroll_percent_x(0.5);
        self.scroll_pane.set_scroll_percent_y(0.5);
        self.scroll_pane.update_visual_scroll();
    }

    pub fn update_tile_groups(&mut self) {
        for tile_group in &self.all_tile_groups {
            tile_group.borrow_mut().update();
        }
    }

    pub fn set_transients(&mut self) {
        for tile in self.tile_groups.keys() {
            tile.set_terrain_transients();
        }
    }

    /// This emulates `private TileMap.getOrNull(Int,Int)` and should really move there
    /// still more efficient than `if (rounded in tileMap) tileMap[rounded] else null`
    fn get_or_null(&self, pos: Vec2) -> Option<Tile> {
        let x = pos.x as i32;
        let y = pos.y as i32;
        let tile_map = self.tile_map.borrow();
        if tile_map.contains(x, y) {
            Some(tile_map.get(x, y))
        } else {
            None
        }
    }

    /// Copy-pasted from WorldMapHolder.setCenterPosition
    /// TODO remove code duplication
    pub fn set_center_position(&mut self, vector: Vec2, blink: bool) {
        let tile_group = self.all_tile_groups.iter()
            .find(|group| group.borrow().tile.position == vector)
            .cloned();

        if let Some(tile_group) = tile_group {
            let tile_group = tile_group.borrow();
            let x = tile_group.x + tile_group.width / 2.0;
            let y = self.scroll_pane.max_y - (tile_group.y + tile_group.width / 2.0);

            if !self.scroll_pane.scroll_to(x, y) {
                return;
            }

            if !blink {
                return;
            }

            // Remove existing blink action
            self.blink_action = None;

            // Create new blink action
            let mut animation = egui::Animation::new(3);
            animation.add_sequence(vec![
                egui::AnimationStep::new(Duration::from_millis(300), Box::new(|| {
                    // Hide highlight
                })),
                egui::AnimationStep::new(Duration::from_millis(300), Box::new(|| {
                    // Show highlight
                })),
            ]);

            self.blink_action = Some(animation);
        }
    }

    /// The ScrollPane interferes with the dragging listener of MapEditorToolsDrawer.
    /// Once the ZoomableScrollPane super is initialized, there are 3 listeners + 1 capture listener:
    /// listeners[0] = ZoomableScrollPane.getFlickScrollListener()
    /// listeners[1] = ZoomableScrollPane.addZoomListeners: override fun scrolled (MouseWheel)
    /// listeners[2] = ZoomableScrollPane.addZoomListeners: override fun zoom (Android pinch)
    /// captureListeners[0] = ScrollPane.addCaptureListener: touchDown, touchUp, touchDragged, mouseMoved
    /// Clearing and putting back the captureListener _should_ suffice, but in practice it doesn't.
    /// Therefore, save all listeners when they're hurting us, and put them back when needed.
    pub fn kill_listeners(&mut self) {
        self.saved_capture_listeners = self.scroll_pane.capture_listeners.clone();
        self.saved_listeners = self.scroll_pane.listeners.clone();
        self.scroll_pane.clear_listeners();
    }

    pub fn resurrect_listeners(&mut self) {
        let capture_listeners_to_add = std::mem::take(&mut self.saved_capture_listeners);
        let listeners_to_add = std::mem::take(&mut self.saved_listeners);

        for listener in listeners_to_add {
            self.scroll_pane.add_listener(listener);
        }

        for listener in capture_listeners_to_add {
            self.scroll_pane.add_capture_listener(listener);
        }
    }

    /// Factory to create the listener that does "paint by dragging"
    /// Should only be called if this MapHolder is used from MapEditorScreen
    fn get_drag_paint_listener(&self) -> egui::EventListener {
        let holder = self.clone();

        egui::EventListener::new()
            .on_touch_down(move |event, x, y, pointer, button| {
                holder.touch_down_time = Instant::now();
                true
            })
            .on_touch_dragged(move |event, x, y, pointer| {
                let mut holder = holder.clone();

                if !holder.is_dragging && !holder.scroll_pane.is_panning {
                    holder.is_dragging = true;
                    let delta_time = holder.touch_down_time.elapsed();

                    if delta_time > Duration::from_millis(400) {
                        holder.is_painting = true;
                        // In a real implementation, we would cancel touch focus
                    }
                }

                if !holder.is_painting {
                    return;
                }

                if let Some(editor) = &holder.editor_screen {
                    let mut editor = editor.borrow_mut();
                    editor.hide_selection();

                    if let Some(actor) = &holder.scroll_pane.actor {
                        if let Some(stage_coords) = actor.stage_to_local_coordinates(Vec2::new(event.stage_x, event.stage_y)) {
                            if let Some(center_tile) = holder.get_closest_tile_to(stage_coords) {
                                editor.tabs.edit.paint_tiles_with_brush(&center_tile);
                            }
                        }
                    }
                }
            })
            .on_touch_up(move |event, x, y, pointer, button| {
                let mut holder = holder.clone();

                // Reset the whole map
                if holder.is_painting {
                    holder.update_tile_groups();
                    holder.set_transients();
                }

                holder.is_dragging = false;
                holder.is_painting = false;
            })
    }

    pub fn get_closest_tile_to(&self, stage_coords: Vec2) -> Option<Tile> {
        if let Some(tile_group_map) = &self.tile_group_map {
            let tile_group_map = tile_group_map.borrow();
            let positional_coords = tile_group_map.get_positional_vector(stage_coords);
            let hex_position = HexMath::world2_hex_coords(positional_coords);
            let rounded = HexMath::round_hex_coords(hex_position);

            let tile_map = self.tile_map.borrow();
            if !tile_map.map_parameters.world_wrap {
                return self.get_or_null(rounded);
            }

            let wrapped = HexMath::get_unwrapped_nearest_to(rounded, Vec2::ZERO, tile_map.max_longitude);
            // This works, but means getUnwrappedNearestTo fails - on the x-y == maxLongitude vertical
            return self.get_or_null(wrapped).or(self.get_or_null(rounded));
        }

        None
    }

    pub fn render(&mut self, ui: &mut Ui) {
        self.scroll_pane.render(ui);

        // Update blink animation if active
        if let Some(animation) = &mut self.blink_action {
            animation.update();
        }
    }
}

impl Clone for EditorMapHolder {
    fn clone(&self) -> Self {
        Self {
            parent_screen: self.parent_screen.clone(),
            tile_map: self.tile_map.clone(),
            on_tile_click: self.on_tile_click.clone(),
            editor_screen: self.editor_screen.clone(),
            tile_groups: self.tile_groups.clone(),
            tile_group_map: self.tile_group_map.clone(),
            all_tile_groups: self.all_tile_groups.clone(),
            blink_action: self.blink_action.clone(),
            saved_capture_listeners: self.saved_capture_listeners.clone(),
            saved_listeners: self.saved_listeners.clone(),
            scroll_pane: self.scroll_pane.clone(),
            is_dragging: self.is_dragging,
            is_painting: self.is_painting,
            touch_down_time: self.touch_down_time,
        }
    }
}