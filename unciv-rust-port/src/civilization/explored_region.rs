use std::f32;
use serde::{Serialize, Deserialize};
use crate::models::map::{MapParameters, MapShape};
use crate::ui::components::tilegroups::TileGroupMap;
use crate::utils::math::{Rectangle, Vector2};

/// Manages the explored region of the map
#[derive(Clone, Serialize, Deserialize)]
pub struct ExploredRegion {
    #[serde(skip)]
    world_wrap: bool,
    #[serde(skip)]
    even_map_width: bool,
    #[serde(skip)]
    rectangular_map: bool,
    #[serde(skip)]
    map_radius: f32,
    #[serde(skip)]
    tile_radius: f32,
    #[serde(skip)]
    should_recalculate_coords: bool,
    #[serde(skip)]
    should_update_minimap: bool,
    #[serde(skip)]
    explored_rectangle: Rectangle,
    #[serde(skip)]
    should_restrict_x: bool,
    #[serde(skip)]
    top_left_stage: Vector2,
    #[serde(skip)]
    bottom_right_stage: Vector2,
    top_left: Vector2,
    bottom_right: Vector2,
}

impl ExploredRegion {
    pub fn new() -> Self {
        Self {
            world_wrap: false,
            even_map_width: false,
            rectangular_map: false,
            map_radius: 0.0,
            tile_radius: TileGroupMap::GROUP_SIZE as f32 * 0.8,
            should_recalculate_coords: true,
            should_update_minimap: true,
            explored_rectangle: Rectangle::new(0.0, 0.0, 0.0, 0.0),
            should_restrict_x: false,
            top_left_stage: Vector2::new(0.0, 0.0),
            bottom_right_stage: Vector2::new(0.0, 0.0),
            top_left: Vector2::new(0.0, 0.0),
            bottom_right: Vector2::new(0.0, 0.0),
        }
    }

    pub fn set_map_parameters(&mut self, map_parameters: &MapParameters) {
        self.world_wrap = map_parameters.world_wrap;
        self.even_map_width = self.world_wrap;

        if map_parameters.shape == MapShape::Rectangular {
            self.map_radius = (map_parameters.map_size.width as f32) / 2.0;
            self.even_map_width = map_parameters.map_size.width % 2 == 0 || self.even_map_width;
            self.rectangular_map = true;
        } else {
            self.map_radius = map_parameters.map_size.radius as f32;
        }
    }

    pub fn check_tile_position(&mut self, tile_position: &Vector2, explorer_position: Option<&Vector2>) {
        let mut map_explored = false;
        let mut longitude = self.get_longitude(tile_position);
        let latitude = self.get_latitude(tile_position);

        // First time call
        if self.top_left == Vector2::ZERO && self.bottom_right == Vector2::ZERO {
            self.top_left = Vector2::new(longitude, latitude);
            self.bottom_right = Vector2::new(longitude, latitude);
            return;
        }

        // Check X coord
        if self.top_left.x >= self.bottom_right.x {
            if longitude > self.top_left.x {
                if self.world_wrap && longitude == self.map_radius {
                    longitude = -self.map_radius;
                }
                self.top_left.x = longitude;
                map_explored = true;
            } else if longitude < self.bottom_right.x {
                if self.world_wrap && longitude == (-self.map_radius + 1.0) {
                    longitude = self.map_radius + 1.0;
                }
                self.bottom_right.x = longitude;
                map_explored = true;
            }
        } else {
            if longitude < self.bottom_right.x && longitude > self.top_left.x {
                let (right_side_distance, left_side_distance) = if let Some(explorer_pos) = explorer_position {
                    let explorer_longitude = self.get_longitude(explorer_pos);
                    let right_side = if explorer_longitude < 0.0 && self.bottom_right.x > 0.0 {
                        self.map_radius * 2.0 + explorer_longitude - self.bottom_right.x
                    } else {
                        (explorer_longitude - self.bottom_right.x).abs()
                    };
                    let left_side = if explorer_longitude > 0.0 && self.top_left.x < 0.0 {
                        self.map_radius * 2.0 - explorer_longitude + self.top_left.x
                    } else {
                        (self.top_left.x - explorer_longitude).abs()
                    };
                    (right_side, left_side)
                } else {
                    (self.bottom_right.x - longitude, longitude - self.top_left.x)
                };

                if right_side_distance > left_side_distance {
                    self.top_left.x = longitude;
                } else {
                    self.bottom_right.x = longitude;
                }
                map_explored = true;
            }
        }

        // Check Y coord
        if latitude > self.top_left.y {
            self.top_left.y = latitude;
            map_explored = true;
        } else if latitude < self.bottom_right.y {
            self.bottom_right.y = latitude;
            map_explored = true;
        }

        if map_explored {
            self.should_recalculate_coords = true;
            self.should_update_minimap = true;
        }
    }

    pub fn calculate_stage_coords(&mut self, map_max_x: f32, map_max_y: f32) {
        self.should_recalculate_coords = false;

        // Check if we explored the whole world wrap map horizontally
        self.should_restrict_x = self.bottom_right.x - self.top_left.x != 1.0;

        // Get world (x;y)
        let top_left_world = self.world_from_lat_long(&self.top_left);
        let bottom_right_world = self.world_from_lat_long(&self.bottom_right);

        // Convert X to the stage coords
        let map_center_x = if self.even_map_width {
            (map_max_x + TileGroupMap::GROUP_SIZE as f32 + 4.0) * 0.5
        } else {
            map_max_x * 0.5
        };
        let mut left = map_center_x + top_left_world.x;
        let mut right = map_center_x + bottom_right_world.x;

        // World wrap over edge check
        if left > map_max_x {
            left = 10.0;
        }
        if right < 0.0 {
            right = map_max_x - 10.0;
        }

        // Convert Y to the stage coords
        let map_center_y = if self.rectangular_map {
            map_max_y * 0.5 + TileGroupMap::GROUP_SIZE as f32 * 0.25
        } else {
            map_max_y * 0.5
        };
        let top = map_center_y - top_left_world.y;
        let bottom = map_center_y - bottom_right_world.y;

        self.top_left_stage = Vector2::new(left, top);
        self.bottom_right_stage = Vector2::new(right, bottom);

        // Calculate rectangle for positioning the camera viewport on the minimap
        let y_offset = self.tile_radius * f32::sqrt(3.0) * 0.5;
        self.explored_rectangle.x = left - self.tile_radius;
        self.explored_rectangle.y = map_max_y - bottom - y_offset * 0.5;
        self.explored_rectangle.width = self.get_width() as f32 * self.tile_radius * 1.5;
        self.explored_rectangle.height = self.get_height() as f32 * y_offset;
    }

    pub fn is_position_in_region(&self, position: &Vector2) -> bool {
        let long = self.get_longitude(position);
        let lat = self.get_latitude(position);
        if self.top_left.x > self.bottom_right.x {
            long <= self.top_left.x && long >= self.bottom_right.x && lat <= self.top_left.y && lat >= self.bottom_right.y
        } else {
            ((long >= self.top_left.x && long >= self.bottom_right.x) || (long <= self.top_left.x && long <= self.bottom_right.x)) && lat <= self.top_left.y && lat >= self.bottom_right.y
        }
    }

    pub fn get_width(&self) -> i32 {
        let result = if self.top_left.x > self.bottom_right.x {
            self.top_left.x - self.bottom_right.x
        } else {
            self.map_radius * 2.0 - (self.bottom_right.x - self.top_left.x)
        };
        result as i32 + 1
    }

    pub fn get_height(&self) -> i32 {
        (self.top_left.y - self.bottom_right.y) as i32 + 1
    }

    pub fn get_minimap_left(&mut self, tile_size: f32) -> f32 {
        self.should_update_minimap = false;
        (self.top_left.x + 1.0) * tile_size * -0.75
    }

    // Getters
    pub fn should_recalculate_coords(&self) -> bool {
        self.should_recalculate_coords
    }

    pub fn should_update_minimap(&self) -> bool {
        self.should_update_minimap
    }

    pub fn get_rectangle(&self) -> &Rectangle {
        &self.explored_rectangle
    }

    pub fn should_restrict_x(&self) -> bool {
        self.should_restrict_x
    }

    pub fn get_left_x(&self) -> f32 {
        self.top_left_stage.x
    }

    pub fn get_right_x(&self) -> f32 {
        self.bottom_right_stage.x
    }

    pub fn get_top_y(&self) -> f32 {
        self.top_left_stage.y
    }

    pub fn get_bottom_y(&self) -> f32 {
        self.bottom_right_stage.y
    }

    // Helper methods for coordinate conversion
    fn get_longitude(&self, position: &Vector2) -> f32 {
        position.x
    }

    fn get_latitude(&self, position: &Vector2) -> f32 {
        position.y
    }

    fn world_from_lat_long(&self, position: &Vector2) -> Vector2 {
        // TODO: Implement proper coordinate conversion
        position.clone()
    }
}

impl Default for ExploredRegion {
    fn default() -> Self {
        Self::new()
    }
}