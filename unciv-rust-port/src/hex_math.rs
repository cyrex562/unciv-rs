use std::collections::HashMap;
use std::f32::consts::PI;
use crate::math::{Vector2, Vector3};
use crate::map::map_parameters::{MapParameters, MapShape};

/// A collection of utility functions for working with hexagonal grids
pub struct HexMath;

impl HexMath {
    /// Returns a vector for a given angle in radians
    pub fn get_vector_for_angle(angle: f32) -> Vector2 {
        Vector2::new(angle.sin(), angle.cos())
    }

    /// Returns a vector for a given clock hour (0-12)
    fn get_vector_by_clock_hour(hour: i32) -> Vector2 {
        Self::get_vector_for_angle(2.0 * PI * (hour as f32 / 12.0))
    }

    /// Returns the number of tiles in a hexagonal map of radius size
    pub fn get_number_of_tiles_in_hexagon(size: i32) -> i32 {
        if size < 0 {
            return 0;
        }
        1 + 6 * size * (size + 1) / 2
    }

    /// Almost inverse of get_number_of_tiles_in_hexagon - get equivalent fractional Hexagon radius for an Area
    pub fn get_hexagonal_radius_for_area(number_of_tiles: i32) -> f32 {
        if numberOfTiles < 1 {
            0.0
        } else {
            ((12.0 * numberOfTiles as f32 - 3.0).sqrt() - 3.0) / 6.0
        }
    }

    /// In our reference system latitude, i.e. how distant from equator we are, is proportional to x + y
    pub fn get_latitude(vector: &Vector2) -> f32 {
        vector.x + vector.y
    }

    /// Returns the longitude of a vector
    pub fn get_longitude(vector: &Vector2) -> f32 {
        vector.x - vector.y
    }

    /// Convert a latitude and longitude back into a hex coordinate.
    /// Inverse function of get_latitude and get_longitude.
    ///
    /// # Arguments
    /// * `latitude` - As from get_latitude.
    /// * `longitude` - As from get_longitude.
    ///
    /// # Returns
    /// Hex coordinate. May need to be passed through round_hex_coords for further use.
    pub fn hex_from_lat_long(latitude: f32, longitude: f32) -> Vector2 {
        let y = (latitude - longitude) / 2.0;
        let x = longitude + y;
        Vector2::new(x, y)
    }

    /// Convert hex latitude and longitude into world coordinates.
    pub fn world_from_lat_long(vector: &Vector2, tile_radius: f32) -> Vector2 {
        let x = vector.x * tile_radius * 1.5 * -1.0;
        let y = vector.y * tile_radius * 3.0_f32.sqrt() * 0.5;
        Vector2::new(x, y)
    }

    /// Returns a vector containing width and height a rectangular map should have to have
    /// approximately the same number of tiles as an hexagonal map given a height/width ratio
    pub fn get_equivalent_rectangular_size(size: i32, ratio: f32) -> Vector2 {
        if size < 0 {
            return Vector2::zero();
        }

        let n_tiles = Self::get_number_of_tiles_in_hexagon(size) as f32;
        let width = (n_tiles / ratio).sqrt().round();
        let height = (width * ratio).round();
        Vector2::new(width, height)
    }

    /// Returns a radius of a hexagonal map that has approximately the same number of
    /// tiles as a rectangular map of a given width/height
    pub fn get_equivalent_hexagonal_radius(width: i32, height: i32) -> i32 {
        Self::get_hexagonal_radius_for_area(width * height).round() as i32
    }

    /// Returns the adjacent vectors for a given origin
    pub fn get_adjacent_vectors(origin: &Vector2) -> Vec<Vector2> {
        let mut vectors = vec![
            Vector2::new(1.0, 0.0),
            Vector2::new(1.0, 1.0),
            Vector2::new(0.0, 1.0),
            Vector2::new(-1.0, 0.0),
            Vector2::new(-1.0, -1.0),
            Vector2::new(0.0, -1.0),
        ];

        for vector in &mut vectors {
            *vector = *vector + *origin;
        }

        vectors
    }

    /// Returns the unwrapped nearest hex coordinate to a given hex coordinate
    ///
    /// # Arguments
    /// * `unwrap_hex_coord` - Hex coordinate to unwrap.
    /// * `static_hex_coord` - Reference hex coordinate.
    /// * `longitudinal_radius` - Maximum longitudinal absolute value of world tiles.
    ///
    /// # Returns
    /// The closest hex coordinate to static_hex_coord that is equivalent to unwrap_hex_coord.
    /// THIS MAY NOT BE A VALID TILE COORDINATE. It may also require rounding for further use.
    pub fn get_unwrapped_nearest_to(unwrap_hex_coord: &Vector2, static_hex_coord: &Vector2, longitudinal_radius: f32) -> Vector2 {
        let reference_long = Self::get_longitude(static_hex_coord);
        let to_wrap_lat = Self::get_latitude(unwrap_hex_coord);
        let to_wrap_long = Self::get_longitude(unwrap_hex_coord);
        let long_radius = longitudinal_radius;

        Self::hex_from_lat_long(
            to_wrap_lat,
            ((to_wrap_long - reference_long + long_radius) % (long_radius * 2.0)) - long_radius + reference_long
        )
    }

    /// Converts hex coordinates to world coordinates
    pub fn hex2_world_coords(hex_coord: &Vector2) -> Vector2 {
        // Distance between cells = 2* normal of triangle = 2* (sqrt(3)/2) = sqrt(3)
        let x_vector = Self::get_vector_by_clock_hour(10).scale(3.0_f32.sqrt());
        let y_vector = Self::get_vector_by_clock_hour(2).scale(3.0_f32.sqrt());
        x_vector.scale(hex_coord.x) + y_vector.scale(hex_coord.y)
    }

    /// Converts world coordinates to hex coordinates
    pub fn world2_hex_coords(world_coord: &Vector2) -> Vector2 {
        // D: diagonal, A: antidiagonal versors
        let d = Self::get_vector_by_clock_hour(10).scale(3.0_f32.sqrt());
        let a = Self::get_vector_by_clock_hour(2).scale(3.0_f32.sqrt());
        let den = d.x * a.y - d.y * a.x;
        let x = (world_coord.x * a.y - world_coord.y * a.x) / den;
        let y = (world_coord.y * d.x - world_coord.x * d.y) / den;
        Vector2::new(x, y)
    }

    /// Returns the row of a hex coordinate
    pub fn get_row(hex_coord: &Vector2) -> i32 {
        (hex_coord.x / 2.0 + hex_coord.y / 2.0) as i32
    }

    /// Returns the column of a hex coordinate
    pub fn get_column(hex_coord: &Vector2) -> i32 {
        (hex_coord.y - hex_coord.x) as i32
    }

    /// Returns the tile coordinates from a column and row
    pub fn get_tile_coords_from_column_row(column: i32, row: i32) -> Vector2 {
        // we know that column = y-x in hex coords
        // And we know that row = (y+x)/2 in hex coords
        // Therefore, 2row+column = 2y, 2row-column=2x

        // However, these row numbers only apear on alternating columns.
        // So column 0 will have rows 0,1,2, etc, and column 1 will have rows 0.5,1.5,2.5 etc.
        // you'll need to see a hexmap to see it, and then it will be obvious

        // So for even columns, the row is incremented by half
        let mut two_rows = row * 2;
        if (column.abs() % 2) == 1 {
            two_rows += 1;
        }

        Vector2::new(
            ((two_rows - column) / 2) as f32,
            ((two_rows + column) / 2) as f32
        )
    }

    /// Rounds hex coordinates to the nearest valid hex
    pub fn round_hex_coords(hex_coord: &Vector2) -> Vector2 {
        /// Rounds cubic coordinates to the nearest valid cubic coordinate
        fn round_cubic_coords(cubic_coords: &Vector3) -> Vector3 {
            let mut rx = cubic_coords.x.round();
            let mut ry = cubic_coords.y.round();
            let mut rz = cubic_coords.z.round();

            let delta_x = (rx - cubic_coords.x).abs();
            let delta_y = (ry - cubic_coords.y).abs();
            let delta_z = (rz - cubic_coords.z).abs();

            if delta_x > delta_y && delta_x > delta_z {
                rx = -ry - rz;
            } else if delta_y > delta_z {
                ry = -rx - rz;
            } else {
                rz = -rx - ry;
            }

            Vector3::new(rx, ry, rz)
        }

        /// Converts hex coordinates to cubic coordinates
        fn hex2_cubic_coords(hex_coord: &Vector2) -> Vector3 {
            Vector3::new(hex_coord.y - hex_coord.x, hex_coord.x, -hex_coord.y)
        }

        /// Converts cubic coordinates to hex coordinates
        fn cubic2_hex_coords(cubic_coord: &Vector3) -> Vector2 {
            Vector2::new(cubic_coord.y, -cubic_coord.z)
        }

        cubic2_hex_coords(&round_cubic_coords(&hex2_cubic_coords(hex_coord)))
    }

    /// Returns vectors at a specific distance from the origin
    pub fn get_vectors_at_distance(origin: &Vector2, distance: i32, max_distance: i32, world_wrap: bool) -> Vec<Vector2> {
        let mut vectors = Vec::new();

        if distance == 0 {
            vectors.push(*origin);
            return vectors;
        }

        let mut current = *origin - Vector2::new(distance as f32, distance as f32); // start at 6 o clock

        for _ in 0..distance { // From 6 to 8
            vectors.push(current);
            vectors.push(*origin * 2.0 - current); // Get vector on other side of clock
            current = current + Vector2::new(1.0, 0.0);
        }

        for i in 0..distance { // 8 to 10
            vectors.push(current);
            if !world_wrap || distance != max_distance {
                vectors.push(*origin * 2.0 - current); // Get vector on other side of clock
            }
            current = current + Vector2::new(1.0, 1.0);
        }

        for i in 0..distance { // 10 to 12
            vectors.push(current);
            if !world_wrap || distance != max_distance || i != 0 {
                vectors.push(*origin * 2.0 - current); // Get vector on other side of clock
            }
            current = current + Vector2::new(0.0, 1.0);
        }

        vectors
    }

    /// Returns all vectors within a specific distance from the origin
    pub fn get_vectors_in_distance(origin: &Vector2, distance: i32, world_wrap: bool) -> Vec<Vector2> {
        let mut hexes_to_return = Vec::new();

        for i in 0..=distance {
            hexes_to_return.extend(Self::get_vectors_at_distance(origin, i, distance, world_wrap));
        }

        hexes_to_return
    }

    /// Get number of hexes from origin to destination _without respecting world-wrap_
    pub fn get_distance(origin: &Vector2, destination: &Vector2) -> i32 {
        let relative_x = origin.x - destination.x;
        let relative_y = origin.y - destination.y;

        if relative_x * relative_y >= 0.0 {
            relative_x.abs().max(relative_y.abs()) as i32
        } else {
            (relative_x.abs() + relative_y.abs()) as i32
        }
    }

    /// Returns the hex-space distance corresponding to clock_position, or a zero vector if clock_position is invalid
    pub fn get_clock_position_to_hex_vector(clock_position: i32) -> Vector2 {
        lazy_static! {
            static ref CLOCK_POSITION_TO_HEX_VECTOR_MAP: HashMap<i32, Vector2> = {
                let mut map = HashMap::new();
                map.insert(0, Vector2::new(1.0, 1.0)); // This alias of 12 makes clock modulo logic easier
                map.insert(12, Vector2::new(1.0, 1.0));
                map.insert(2, Vector2::new(0.0, 1.0));
                map.insert(4, Vector2::new(-1.0, 0.0));
                map.insert(6, Vector2::new(-1.0, -1.0));
                map.insert(8, Vector2::new(0.0, -1.0));
                map.insert(10, Vector2::new(1.0, 0.0));
                map
            };
        }

        CLOCK_POSITION_TO_HEX_VECTOR_MAP.get(&clock_position).cloned().unwrap_or(Vector2::zero())
    }

    /// Returns the world/screen-space distance corresponding to clock_position, or a zero vector if clock_position is invalid
    pub fn get_clock_position_to_world_vector(clock_position: i32) -> Vector2 {
        lazy_static! {
            static ref CLOCK_POSITION_TO_WORLD_VECTOR_MAP: HashMap<i32, Vector2> = {
                let mut map = HashMap::new();
                map.insert(2, Self::hex2_world_coords(&Vector2::new(0.0, -1.0)));
                map.insert(4, Self::hex2_world_coords(&Vector2::new(1.0, 0.0)));
                map.insert(6, Self::hex2_world_coords(&Vector2::new(1.0, 1.0)));
                map.insert(8, Self::hex2_world_coords(&Vector2::new(0.0, 1.0)));
                map.insert(10, Self::hex2_world_coords(&Vector2::new(-1.0, 0.0)));
                map.insert(12, Self::hex2_world_coords(&Vector2::new(-1.0, -1.0)));
                map
            };
        }

        CLOCK_POSITION_TO_WORLD_VECTOR_MAP.get(&clock_position).cloned().unwrap_or(Vector2::zero())
    }

    /// Returns the distance from the edge of the map
    pub fn get_distance_from_edge(vector: &Vector2, map_parameters: &MapParameters) -> i32 {
        let x = vector.x as i32;
        let y = vector.y as i32;

        if map_parameters.shape == MapShape::Rectangular {
            let height = map_parameters.map_size.height;
            let width = map_parameters.map_size.width;
            let left = if map_parameters.world_wrap { i32::MAX } else { width / 2 - (x - y) };
            let right = if map_parameters.world_wrap { i32::MAX } else { (width - 1) / 2 - (y - x) };
            let top = height / 2 - (x + y) / 2;
            // Rust's Int division rounds in different directions depending on sign! Thus 1 extra `-1`
            let bottom = (x + y - 1) / 2 + (height - 1) / 2;

            left.min(right).min(top).min(bottom)
        } else {
            let radius = map_parameters.map_size.radius;

            if !map_parameters.world_wrap {
                return radius - Self::get_distance(vector, &Vector2::zero());
            }

            // The non-wrapping method holds in the upper two and lower two 'triangles' of the hexagon
            // but needs special casing for left and right 'wedges', where only distance from the
            // 'choke points' counts (upper and lower hex at the 'seam' where height is smallest).
            // These are at (radius,0) and (0,-radius)
            if x.signum() == y.signum() {
                return radius - Self::get_distance(vector, &Vector2::zero());
            }

            // left wedge - the 'choke points' are not wrapped relative to us
            if x > 0 {
                return Self::get_distance(vector, &Vector2::new(radius as f32, 0.0))
                    .min(Self::get_distance(vector, &Vector2::new(0.0, -radius as f32)));
            }

            // right wedge - compensate wrap by using a hex 1 off along the edge - same result
            Self::get_distance(vector, &Vector2::new(1.0, radius as f32))
                .min(Self::get_distance(vector, &Vector2::new(-radius as f32, -1.0)))
        }
    }
}