use std::collections::HashMap;
use std::sync::Arc;

use crate::models::civilization::Civilization;
use crate::models::map::neighbor_direction::NeighborDirection;
use crate::models::map::road_status::RoadStatus;
use crate::models::map::unit::MapUnit;
use crate::models::metadata::GameSettings;
use crate::models::ruleset::Ruleset;
use crate::models::tilesets::{TileSetCache, TileSetConfig};
use crate::ui::components::tilegroups::layers::EdgeTileImage;
use crate::ui::images::{ImageAttempter, ImageGetter};
use crate::UncivGame;

/// Resolver translating more abstract tile data to paint on a map into actual texture names.
///
/// Deals with variants, e.g. there could be a "City center-asian-Ancient era.png" that would be chosen
/// for a "City center"-containing Tile when it is to be drawn for a Nation defining it's style as "asian"
/// and whose techs say it's still in the first vanilla Era.
///
/// Instantiated once per [TileGroupMap] and per [TileSet] -
/// typically once for HexaRealm and once for FantasyHex (fallback) at the start of a player turn,
/// and the same two every time they enter a CityScreen.
pub struct TileSetStrings {
    /// Separator used to mark variants, e.g. nation style or era specific
    tag: String,

    /// Name of the tileset
    tile_set_name: String,

    /// Name of the unitset
    unit_set_name: Option<String>,

    /// Location of the tileset
    tile_set_location: String,

    /// Location of the unitset
    unit_set_location: String,

    /// Configuration for the tileset
    tile_set_config: Arc<TileSetConfig>,

    /// Hexagon image path
    hexagon: String,

    /// List containing the hexagon image path
    hexagon_list: Vec<String>,

    /// Crosshatch hexagon image path
    crosshatch_hexagon: String,

    /// Unexplored tile image path
    unexplored_tile: String,

    /// Crosshair image path
    crosshair: String,

    /// Highlight image path
    highlight: String,

    /// Map of road status to image paths
    roads_map: HashMap<RoadStatus, String>,

    /// Natural wonder image path
    natural_wonder: String,

    /// Location of tiles
    tiles_location: String,

    /// Bottom right river image path
    bottom_right_river: String,

    /// Bottom river image path
    bottom_river: String,

    /// Bottom left river image path
    bottom_left_river: String,

    /// Edge images by position
    edge_images_by_position: HashMap<NeighborDirection, Vec<EdgeTileImage>>,

    /// Location of units
    units_location: String,

    /// Location of borders
    borders_location: String,

    /// Hashmap for string concatenation caching
    string_concat_hashmap: HashMap<(String, String), String>,

    /// Fallback TileSetStrings to use when the currently chosen tileset is missing an image
    fallback: Option<Box<TileSetStrings>>,

    /// For caching image locations based on given parameters (era, style, etc)
    image_params_to_image_location: HashMap<String, String>,

    /// Embarked military unit location
    embarked_military_unit_location: String,

    /// Whether the embarked military unit image exists
    has_embarked_military_unit_image: bool,

    /// Embarked civilian unit location
    embarked_civilian_unit_location: String,

    /// Whether the embarked civilian unit image exists
    has_embarked_civilian_unit_image: bool,
}

impl TileSetStrings {
    /// Creates a new TileSetStrings with the given tileset and unitset
    pub fn new(
        tile_set: String,
        unit_set: Option<String>,
        fallback_depth: i32,
    ) -> Self {
        let tile_set_location = format!("TileSets/{}/", tile_set);
        let unit_set_location = unit_set.as_ref()
            .map(|u| format!("TileSets/{}/", u))
            .unwrap_or_default();

        let tile_set_config = TileSetCache::get(&tile_set)
            .map(|ts| ts.config.clone())
            .unwrap_or_else(|| Arc::new(TileSetConfig::default()));

        let mut tile_set_strings = Self {
            tag: "-".to_string(),
            tile_set_name: tile_set,
            unit_set_name: unit_set,
            tile_set_location: tile_set_location.clone(),
            unit_set_location: unit_set_location.clone(),
            tile_set_config: tile_set_config.clone(),
            hexagon: String::new(),
            hexagon_list: Vec::new(),
            crosshatch_hexagon: String::new(),
            unexplored_tile: String::new(),
            crosshair: String::new(),
            highlight: String::new(),
            roads_map: HashMap::new(),
            natural_wonder: format!("{}Tiles/NaturalWonder", tile_set_location),
            tiles_location: format!("{}Tiles/", tile_set_location),
            bottom_right_river: String::new(),
            bottom_river: String::new(),
            bottom_left_river: String::new(),
            edge_images_by_position: HashMap::new(),
            units_location: format!("{}Units/", unit_set_location),
            borders_location: format!("{}Borders/", tile_set_location),
            string_concat_hashmap: HashMap::new(),
            fallback: None,
            image_params_to_image_location: HashMap::new(),
            embarked_military_unit_location: String::new(),
            has_embarked_military_unit_image: false,
            embarked_civilian_unit_location: String::new(),
            has_embarked_civilian_unit_image: false,
        };

        // Initialize lazy-loaded values
        tile_set_strings.hexagon = tile_set_strings.or_fallback(|| format!("{}Hexagon", tile_set_location));
        tile_set_strings.hexagon_list = vec![tile_set_strings.hexagon.clone()];
        tile_set_strings.crosshatch_hexagon = tile_set_strings.or_fallback(|| format!("{}CrosshatchHexagon", tile_set_location));
        tile_set_strings.unexplored_tile = tile_set_strings.or_fallback(|| format!("{}UnexploredTile", tile_set_location));
        tile_set_strings.crosshair = tile_set_strings.or_fallback(|| tile_set_strings.get_string(&[tile_set_location.clone(), "Crosshair".to_string()]));
        tile_set_strings.highlight = tile_set_strings.or_fallback(|| tile_set_strings.get_string(&[tile_set_location.clone(), "Highlight".to_string()]));

        // Initialize roads map
        for road_status in RoadStatus::iter() {
            if road_status != RoadStatus::None {
                tile_set_strings.roads_map.insert(
                    road_status,
                    format!("{}{}", tile_set_location, road_status.to_string())
                );
            }
        }

        // Initialize river images
        tile_set_strings.bottom_right_river = tile_set_strings.or_fallback(|| format!("{}River-BottomRight", tile_set_strings.tiles_location));
        tile_set_strings.bottom_river = tile_set_strings.or_fallback(|| format!("{}River-Bottom", tile_set_strings.tiles_location));
        tile_set_strings.bottom_left_river = tile_set_strings.or_fallback(|| format!("{}River-BottomLeft", tile_set_strings.tiles_location));

        // Initialize edge images
        let mut edge_images_by_position: HashMap<NeighborDirection, Vec<EdgeTileImage>> = HashMap::new();

        for image_name in ImageGetter::get_all_image_names() {
            if image_name.starts_with(&format!("{}Edges/", tile_set_location)) {
                let parts: Vec<&str> = image_name.split('/').collect();
                if let Some(filename) = parts.last() {
                    let split: Vec<&str> = filename.split('-').collect();

                    // Comprised of 3 parts: origin tilefilter, destination tilefilter,
                    // and edge type: Bottom, BottomLeft or BottomRight
                    if split.len() != 4 {
                        continue;
                    }

                    // split[0] is name and is unused
                    let origin_tile_filter = split[1].to_string();
                    let destination_tile_filter = split[2].to_string();
                    let neighbor_direction_str = split[3];

                    if let Some(neighbor_direction) = NeighborDirection::from_str(neighbor_direction_str) {
                        let edge_image = EdgeTileImage::new(
                            image_name.clone(),
                            origin_tile_filter,
                            destination_tile_filter,
                            neighbor_direction,
                        );

                        edge_images_by_position
                            .entry(neighbor_direction)
                            .or_insert_with(Vec::new)
                            .push(edge_image);
                    }
                }
            }
        }

        tile_set_strings.edge_images_by_position = edge_images_by_position;

        // Initialize embarked unit locations
        tile_set_strings.embarked_military_unit_location = tile_set_strings.get_string(&[
            tile_set_strings.units_location.clone(),
            "EmbarkedUnit-Military".to_string(),
        ]);

        tile_set_strings.has_embarked_military_unit_image = ImageGetter::image_exists(&tile_set_strings.embarked_military_unit_location);

        tile_set_strings.embarked_civilian_unit_location = tile_set_strings.get_string(&[
            tile_set_strings.units_location.clone(),
            "EmbarkedUnit-Civilian".to_string(),
        ]);

        tile_set_strings.has_embarked_civilian_unit_image = ImageGetter::image_exists(&tile_set_strings.embarked_civilian_unit_location);

        // Initialize fallback
        if fallback_depth > 0 {
            if let Some(fallback_tile_set) = &tile_set_config.fallback_tile_set {
                tile_set_strings.fallback = Some(Box::new(TileSetStrings::new(
                    fallback_tile_set.clone(),
                    Some(fallback_tile_set.clone()),
                    fallback_depth - 1,
                )));
            }
        }

        tile_set_strings
    }

    /// Creates a new TileSetStrings with the given ruleset and settings
    pub fn from_ruleset(ruleset: &Ruleset, settings: &GameSettings) -> Self {
        let tile_set = ruleset.mod_options.tileset.clone()
            .unwrap_or_else(|| settings.tile_set.clone());

        let unit_set = ruleset.mod_options.unitset.clone()
            .or_else(|| Some(settings.unit_set.clone()));

        Self::new(tile_set, unit_set, 1)
    }

    /// Gets a string by concatenating the given strings, using a cache to avoid duplicates
    pub fn get_string(&mut self, strings: &[String]) -> String {
        if strings.is_empty() {
            return String::new();
        }

        let mut current_string = strings[0].clone();

        for str in strings.iter().skip(1) {
            let pair = (current_string.clone(), str.clone());

            if let Some(cached) = self.string_concat_hashmap.get(&pair) {
                current_string = cached.clone();
            } else {
                let new_string = format!("{}{}", current_string, str);
                self.string_concat_hashmap.insert(pair, new_string.clone());
                current_string = new_string;
            }
        }

        current_string
    }

    /// Gets a tile image path for the given base terrain
    pub fn get_tile(&mut self, base_terrain: &str) -> String {
        self.get_string(&[self.tiles_location.clone(), base_terrain.to_string()])
    }

    /// Gets a border image path for the given border shape and inner/outer type
    pub fn get_border(&mut self, border_shape_string: &str, inner_or_outer: &str) -> String {
        self.get_string(&[
            self.borders_location.clone(),
            border_shape_string.to_string(),
            inner_or_outer.to_string(),
        ])
    }

    /// Gets an image path, falling back to the fallback tileset if the image doesn't exist
    pub fn or_fallback<F>(&self, image_fn: F) -> String
    where
        F: FnOnce() -> String,
    {
        let image = image_fn();

        if self.fallback.is_none() || ImageGetter::image_exists(&image) {
            image
        } else {
            self.fallback.as_ref().unwrap().or_fallback(image_fn)
        }
    }

    /// Gets an image path, falling back to the fallback tileset if the image doesn't exist
    pub fn or_fallback_with_image<F>(&self, image: String, fallback_fn: F) -> String
    where
        F: FnOnce(&TileSetStrings) -> String,
    {
        if self.fallback.is_none() || ImageGetter::image_exists(&image) {
            image
        } else {
            fallback_fn(self.fallback.as_ref().unwrap())
        }
    }

    /// Tries to get a unit image location for the given unit
    fn try_get_unit_image_location(&self, unit: &MapUnit) -> Option<String> {
        let mut base_unit_icon_location = self.get_string(&[self.units_location.clone(), unit.name().to_string()]);

        if unit.is_embarked() {
            let unit_specific_embarked_unit_location = self.get_string(&[
                self.units_location.clone(),
                format!("EmbarkedUnit-{}", unit.name()),
            ]);

            base_unit_icon_location = if ImageGetter::image_exists(&unit_specific_embarked_unit_location) {
                unit_specific_embarked_unit_location
            } else if unit.is_civilian() && self.has_embarked_civilian_unit_image {
                self.embarked_civilian_unit_location.clone()
            } else if unit.is_military() && self.has_embarked_military_unit_image {
                self.embarked_military_unit_location.clone()
            } else {
                base_unit_icon_location // no change
            };
        }

        let civ_info = unit.civ();
        let style = civ_info.nation().get_style_or_civ_name();

        let mut image_attempter = ImageAttempter::new(base_unit_icon_location.clone())
            // Era+style image: looks like "pikeman-France-Medieval era"
            // More advanced eras default to older eras
            .try_era_image(civ_info, &base_unit_icon_location, Some(&style), self)
            // Era-only image: looks like "pikeman-Medieval era"
            .try_era_image(civ_info, &base_unit_icon_location, None, self)
            // Style era: looks like "pikeman-France" or "pikeman-European"
            .try_image(|| self.get_string(&[base_unit_icon_location.clone(), self.tag.clone(), style.clone()]))
            .try_image(|| base_unit_icon_location.clone());

        if let Some(replaces) = unit.base_unit().replaces() {
            image_attempter = image_attempter.try_image(|| self.get_string(&[self.units_location.clone(), replaces.to_string()]));
        }

        image_attempter.get_path_or_null()
    }

    /// Gets a unit image location for the given unit
    pub fn get_unit_image_location(&mut self, unit: &MapUnit) -> String {
        let image_key = self.get_string(&[
            unit.name().to_string(),
            self.tag.clone(),
            unit.civ().get_era().name().to_string(),
            self.tag.clone(),
            unit.civ().nation().get_style_or_civ_name().to_string(),
            self.tag.clone(),
            unit.is_embarked().to_string(),
        ]);

        // If in cache return that
        if let Some(current_image_mapping) = self.image_params_to_image_location.get(&image_key) {
            return current_image_mapping.clone();
        }

        let image_location = self.try_get_unit_image_location(unit)
            .or_else(|| {
                self.fallback.as_ref()
                    .and_then(|fallback| fallback.try_get_unit_image_location(unit))
            })
            .unwrap_or_default();

        self.image_params_to_image_location.insert(image_key, image_location.clone());
        image_location
    }

    /// Tries to get an owned tile image location for the given base location and owner
    fn try_get_owned_tile_image_location(&self, base_location: &str, owner: &Civilization) -> Option<String> {
        let owners_style = owner.nation().get_style_or_civ_name();

        ImageAttempter::new(base_location.to_string())
            .try_era_image(owner, base_location, Some(&owners_style), self)
            .try_era_image(owner, base_location, None, self)
            .try_image(|| self.get_string(&[base_location.to_string(), self.tag.clone(), owners_style.clone()]))
            .get_path_or_null()
    }

    /// Gets an owned tile image location for the given base location and owner
    pub fn get_owned_tile_image_location(&mut self, base_location: &str, owner: &Civilization) -> String {
        let image_key = self.get_string(&[
            base_location.to_string(),
            self.tag.clone(),
            owner.get_era().name().to_string(),
            self.tag.clone(),
            owner.nation().get_style_or_civ_name().to_string(),
        ]);

        if let Some(current_image_mapping) = self.image_params_to_image_location.get(&image_key) {
            return current_image_mapping.clone();
        }

        let image_location = self.try_get_owned_tile_image_location(base_location, owner)
            .unwrap_or_else(|| base_location.to_string());

        self.image_params_to_image_location.insert(image_key, image_location.clone());
        image_location
    }

    /// Gets the hexagon image path
    pub fn hexagon(&self) -> &str {
        &self.hexagon
    }

    /// Gets the hexagon list
    pub fn hexagon_list(&self) -> &[String] {
        &self.hexagon_list
    }

    /// Gets the crosshatch hexagon image path
    pub fn crosshatch_hexagon(&self) -> &str {
        &self.crosshatch_hexagon
    }

    /// Gets the unexplored tile image path
    pub fn unexplored_tile(&self) -> &str {
        &self.unexplored_tile
    }

    /// Gets the crosshair image path
    pub fn crosshair(&self) -> &str {
        &self.crosshair
    }

    /// Gets the highlight image path
    pub fn highlight(&self) -> &str {
        &self.highlight
    }

    /// Gets the roads map
    pub fn roads_map(&self) -> &HashMap<RoadStatus, String> {
        &self.roads_map
    }

    /// Gets the natural wonder image path
    pub fn natural_wonder(&self) -> &str {
        &self.natural_wonder
    }

    /// Gets the tiles location
    pub fn tiles_location(&self) -> &str {
        &self.tiles_location
    }

    /// Gets the bottom right river image path
    pub fn bottom_right_river(&self) -> &str {
        &self.bottom_right_river
    }

    /// Gets the bottom river image path
    pub fn bottom_river(&self) -> &str {
        &self.bottom_river
    }

    /// Gets the bottom left river image path
    pub fn bottom_left_river(&self) -> &str {
        &self.bottom_left_river
    }

    /// Gets the edge images by position
    pub fn edge_images_by_position(&self) -> &HashMap<NeighborDirection, Vec<EdgeTileImage>> {
        &self.edge_images_by_position
    }

    /// Gets the units location
    pub fn units_location(&self) -> &str {
        &self.units_location
    }

    /// Gets the borders location
    pub fn borders_location(&self) -> &str {
        &self.borders_location
    }

    /// Gets the tag
    pub fn tag(&self) -> &str {
        &self.tag
    }
}