use ggez::graphics::{DrawParam, Rect};
use ggez::mint::Point2;
use ggez::Context;
use std::collections::{HashMap, HashSet};
use std::f32;
use std::sync::Arc;

use crate::models::map::tile_map::TileMap;
use crate::ui::components::tilegroups::TileGroup;
use crate::ui::components::widgets::ZoomableScrollPane;
use crate::utils::hex_math::HexMath;

/// A (potentially partial) map view
///
/// # Type Parameters
///
/// * `T` - TileGroup or a subclass (WorldTileGroup, CityTileGroup)
///
/// # Fields
///
/// * `map_holder` - The scroll pane that contains the map
/// * `world_wrap` - Whether the map wraps around the world
/// * `tile_groups` - Source of TileGroups to include
/// * `tile_groups_to_unwrap` - For these, coordinates will be unwrapped using TileMap.getUnWrappedPosition
pub struct TileGroupMap<T: TileGroup> {
    pub map_holder: Arc<ZoomableScrollPane>,
    pub world_wrap: bool,
    pub tile_groups: Vec<T>,
    pub tile_groups_to_unwrap: Option<HashSet<T>>,
    pub should_act: bool,
    pub should_hit: bool,
    top_x: f32,
    top_y: f32,
    bottom_x: f32,
    bottom_y: f32,
    draw_top_x: f32,
    draw_bottom_x: f32,
    max_visible_map_width: f32,
    children: Vec<Box<dyn Drawable>>,
    culling_area: Rect,
    width: f32,
    height: f32,
}

impl<T: TileGroup> TileGroupMap<T> {
    /// Vertical size of a hex in world coordinates, or the distance between the centers of any two opposing edges
    /// (the hex is oriented so it has corners to the left and right of the center and its upper and lower bounds are horizontal edges)
    pub const GROUP_SIZE: f32 = 50.0;

    /// Length of the diagonal of a hex, or distance between two opposing corners
    pub const GROUP_SIZE_DIAGONAL: f32 = Self::GROUP_SIZE * 1.1547005; // groupSize * sqrt(4/3)

    /// Horizontal displacement per hex, meaning the increase in overall map size (in world coordinates) when adding a column.
    /// On the hex, this can be visualized as the horizontal distance between the leftmost corner and the
    /// line connecting the two corners at 2 and 4 o'clock.
    pub const GROUP_HORIZONTAL_ADVANCE: f32 = Self::GROUP_SIZE_DIAGONAL * 3.0 / 4.0;

    /// Creates a new TileGroupMap
    pub fn new(
        map_holder: Arc<ZoomableScrollPane>,
        tile_groups: Vec<T>,
        world_wrap: bool,
        tile_groups_to_unwrap: Option<HashSet<T>>,
    ) -> Self {
        let mut map = Self {
            map_holder,
            world_wrap,
            tile_groups: tile_groups.clone(),
            tile_groups_to_unwrap,
            should_act: true,
            should_hit: true,
            top_x: -f32::MAX,
            top_y: -f32::MAX,
            bottom_x: f32::MAX,
            bottom_y: f32::MAX,
            draw_top_x: 0.0,
            draw_bottom_x: 0.0,
            max_visible_map_width: 0.0,
            children: Vec::new(),
            culling_area: Rect::new(0.0, 0.0, 0.0, 0.0),
            width: 0.0,
            height: 0.0,
        };

        map.init(tile_groups);
        map
    }

    /// Initializes the TileGroupMap
    fn init(&mut self, tile_groups: Vec<T>) {
        // Position all tile groups
        for tile_group in &tile_groups {
            let positional_vector = if let Some(groups_to_unwrap) = &self.tile_groups_to_unwrap {
                if groups_to_unwrap.contains(tile_group) {
                    HexMath::hex2_world_coords(
                        tile_group.tile().tile_map().get_unwrapped_position(tile_group.tile().position())
                    )
                } else {
                    HexMath::hex2_world_coords(tile_group.tile().position())
                }
            } else {
                HexMath::hex2_world_coords(tile_group.tile().position())
            };

            tile_group.set_position(
                positional_vector.x * 0.8 * Self::GROUP_SIZE,
                positional_vector.y * 0.8 * Self::GROUP_SIZE
            );

            // Update bounds
            self.top_x = if self.world_wrap {
                // Well it's not pretty but it works
                // The resulting topX was always missing 1.2 * groupSize in every possible
                // combination of map size and shape
                f32::max(self.top_x, tile_group.x() + Self::GROUP_SIZE * 1.2)
            } else {
                f32::max(self.top_x, tile_group.x() + Self::GROUP_SIZE + 4.0)
            };

            self.top_y = f32::max(self.top_y, tile_group.y() + Self::GROUP_SIZE);
            self.bottom_x = f32::min(self.bottom_x, tile_group.x());
            self.bottom_y = f32::min(self.bottom_y, tile_group.y());
        }

        // Adjust positions to start from (0,0)
        for tile_group in &mut self.tile_groups {
            tile_group.move_by(-self.bottom_x, -self.bottom_y);
        }

        self.draw_top_x = self.top_x - self.bottom_x;
        self.draw_bottom_x = self.bottom_x - self.bottom_x;

        let number_of_tilegroups = self.tile_groups.len();

        // Create layer collections
        let mut base_layers = Vec::with_capacity(number_of_tilegroups);
        let mut feature_layers = Vec::with_capacity(number_of_tilegroups);
        let mut border_layers = Vec::with_capacity(number_of_tilegroups);
        let mut resource_layers = Vec::with_capacity(number_of_tilegroups);
        let mut improvement_layers = Vec::with_capacity(number_of_tilegroups);
        let mut misc_layers = Vec::with_capacity(number_of_tilegroups);
        let mut yield_layers = Vec::with_capacity(number_of_tilegroups);
        let mut pixel_unit_layers = Vec::with_capacity(number_of_tilegroups);
        let mut circle_fog_crosshair_layers = Vec::with_capacity(number_of_tilegroups);
        let mut unit_layers = Vec::with_capacity(number_of_tilegroups);
        let mut city_button_layers = Vec::with_capacity(number_of_tilegroups);

        // Group tile groups by position and sort by descending order
        let mut position_groups: HashMap<i32, Vec<&T>> = HashMap::new();
        for group in &self.tile_groups {
            let key = group.tile().position().x + group.tile().position().y;
            position_groups.entry(key).or_insert_with(Vec::new).push(group);
        }

        // Sort by position key in descending order
        let mut sorted_keys: Vec<i32> = position_groups.keys().cloned().collect();
        sorted_keys.sort_by(|a, b| b.cmp(a));

        // Process each group in sorted order
        for key in sorted_keys {
            if let Some(groups) = position_groups.get(&key) {
                for group in groups {
                    // Add each layer to its collection
                    base_layers.push(group.layer_terrain().clone());
                    feature_layers.push(group.layer_features().clone());
                    border_layers.push(group.layer_borders().clone());
                    resource_layers.push(group.layer_resource().clone());
                    improvement_layers.push(group.layer_improvement().clone());
                    misc_layers.push(group.layer_misc().clone());
                    yield_layers.push(group.layer_yield().clone());
                    pixel_unit_layers.push(group.layer_unit_art().clone());
                    circle_fog_crosshair_layers.push(group.layer_overlay().clone());
                    unit_layers.push(group.layer_unit_flag().clone());
                    city_button_layers.push(group.layer_city_button().clone());
                }
            }
        }

        // Combine all layers into a single list
        let layer_lists = vec![
            base_layers,
            feature_layers,
            border_layers,
            resource_layers,
            improvement_layers,
            misc_layers,
            yield_layers,
            pixel_unit_layers,
            circle_fog_crosshair_layers,
            // The above layers are for the visual layers, this is for the clickability of the tile
            self.tile_groups.iter().map(|t| t.clone() as Box<dyn Drawable>).collect(),
            unit_layers,
            city_button_layers,
        ];

        // Add all layers to children
        for layer in layer_lists {
            for item in layer {
                self.children.push(item);
            }
        }

        // Set size based on bounds
        self.width = self.top_x - self.bottom_x;
        self.height = self.top_y - self.bottom_y;

        // Set culling area
        self.culling_area = Rect::new(0.0, 0.0, self.width, self.height);

        // Set max visible map width
        self.max_visible_map_width = self.width - Self::GROUP_SIZE * 1.5;
    }

    /// Returns the positional coordinates of the TileGroupMap center.
    pub fn get_positional_vector(&self, stage_coords: Point2<f32>) -> Point2<f32> {
        let true_group_size = 0.8 * Self::GROUP_SIZE;
        let mut result = Point2 {
            x: self.bottom_x + stage_coords.x - Self::GROUP_SIZE / 2.0,
            y: self.bottom_y + stage_coords.y - Self::GROUP_SIZE / 2.0,
        };
        result.x /= true_group_size;
        result.y /= true_group_size;
        result
    }

    /// Updates the map
    pub fn act(&mut self, delta: f32) {
        if self.should_act {
            // Update all children
            for child in &mut self.children {
                if let Some(actor) = child.as_any().downcast_mut::<dyn Actor>() {
                    actor.act(delta);
                }
            }
        }
    }

    /// Checks if a point hits any actor
    pub fn hit(&self, x: f32, y: f32, touchable: bool) -> Option<&dyn Actor> {
        if self.should_hit {
            // Check each child for hits
            for child in &self.children {
                if let Some(actor) = child.as_any().downcast_ref::<dyn Actor>() {
                    if actor.hit(x, y, touchable) {
                        return Some(actor);
                    }
                }
            }
        }
        None
    }

    /// Draws the map
    pub fn draw(&mut self, ctx: &mut Context, parent_alpha: f32) {
        if self.world_wrap {
            // Prevent flickering when zoomed out so you can see entire map
            let visible_map_width = if self.map_holder.width() > self.max_visible_map_width {
                self.max_visible_map_width
            } else {
                self.map_holder.width()
            };

            // Where is viewport's boundaries
            let right_side = self.map_holder.scroll_x() + visible_map_width / 2.0;
            let left_side = self.map_holder.scroll_x() - visible_map_width / 2.0;

            // Have we looked beyond map?
            let diff_right = right_side - self.draw_top_x;
            let diff_left = left_side - self.draw_bottom_x;

            let beyond_right = diff_right >= 0.0;
            let beyond_left = diff_left <= 0.0;

            if beyond_right || beyond_left {
                // If we looked beyond - reposition needed tiles from the other side
                // and update topX and bottomX accordingly.

                let mut new_bottom_x = f32::MAX;
                let mut new_top_x = -f32::MAX;

                for child in &mut self.children {
                    if beyond_right {
                        // Move from left to right
                        if child.x() - self.draw_bottom_x <= diff_right {
                            child.set_x(child.x() + self.width);
                        }
                    } else if beyond_left {
                        // Move from right to left
                        if child.x() + Self::GROUP_SIZE + 4.0 >= self.draw_top_x + diff_left {
                            child.set_x(child.x() - self.width);
                        }
                    }
                    new_bottom_x = f32::min(new_bottom_x, child.x());
                    new_top_x = f32::max(new_top_x, child.x() + Self::GROUP_SIZE + 4.0);
                }

                self.draw_bottom_x = new_bottom_x;
                self.draw_top_x = new_top_x;
            }
        }

        // Draw all children
        for child in &self.children {
            child.draw(ctx, parent_alpha);
        }
    }

    /// Gets the width of the map
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Gets the height of the map
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Sets the size of the map
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.culling_area = Rect::new(0.0, 0.0, width, height);
    }
}

/// Trait for drawable objects
pub trait Drawable: std::any::Any {
    fn draw(&self, ctx: &mut Context, parent_alpha: f32);
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Trait for actors that can be updated and hit
pub trait Actor: std::any::Any {
    fn act(&mut self, delta: f32);
    fn hit(&self, x: f32, y: f32, touchable: bool) -> bool;
    fn x(&self) -> f32;
    fn y(&self) -> f32;
    fn set_x(&mut self, x: f32);
    fn set_y(&mut self, y: f32);
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Extension trait for Drawable to implement Actor
impl<T: Drawable + 'static> Actor for T {
    fn act(&mut self, _delta: f32) {
        // Default implementation does nothing
    }

    fn hit(&self, _x: f32, _y: f32, _touchable: bool) -> bool {
        // Default implementation returns false
        false
    }

    fn x(&self) -> f32 {
        // Default implementation returns 0
        0.0
    }

    fn y(&self) -> f32 {
        // Default implementation returns 0
        0.0
    }

    fn set_x(&mut self, _x: f32) {
        // Default implementation does nothing
    }

    fn set_y(&mut self, _y: f32) {
        // Default implementation does nothing
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}