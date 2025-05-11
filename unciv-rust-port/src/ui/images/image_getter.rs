use std::collections::{HashMap, HashSet};
use std::f32::consts::PI;
use std::path::Path;
use std::sync::Arc;

use ggez::filesystem::Filesystem;
use ggez::graphics::{self, Color, DrawParam, Image, Rect, Text, TextureFilter};
use ggez::mint::Point2;
use ggez::Context;
use ggez::GameResult;
use serde_json;

use crate::constants::DEFAULT_FONT_SIZE;
use crate::logic::civilization::Civilization;
use crate::logic::ruleset::Ruleset;
use crate::models::ruleset::nation::Nation;
use crate::models::ruleset::unit::BaseUnit;
use crate::models::ruleset::PerpetualConstruction;
use crate::models::skins::SkinCache;
use crate::models::tilesets::TileSetCache;
use crate::ui::components::fonts::FontRulesetIcons;
use crate::ui::components::non_transform_group::NonTransformGroup;
use crate::ui::components::progress_bar::ProgressBar;
use crate::ui::components::table::Table;
use crate::ui::components::table_cell::TableCell;
use crate::ui::screens::base_screen::BaseScreen;
use crate::ui::screens::game_screen::GameScreen;
use crate::utils::debug;

/// A singleton class that manages image loading, caching, and various image-related utilities.
pub struct ImageGetter {
    /// Location of the white dot image used as a fallback
    pub const WHITE_DOT_LOCATION: &'static str = "OtherIcons/whiteDot";
    /// Location of the circle image
    pub const CIRCLE_LOCATION: &'static str = "OtherIcons/Circle";
    /// A dark gray color used for various UI elements
    pub const CHARCOAL: Color = Color {
        r: 0.067, // 0x11
        g: 0.067, // 0x11
        b: 0.067, // 0x11
        a: 1.0,   // 0xFF
    };

    /// The main texture atlas
    atlas: Option<graphics::Texture>,
    /// Map of texture atlases by name
    atlases: HashMap<String, graphics::Texture>,
    /// The current ruleset
    ruleset: Ruleset,
    /// Map of texture region drawables by name
    texture_region_drawables: HashMap<String, graphics::Image>,
    /// Map of nine-patch drawables by name
    nine_patch_drawables: HashMap<String, graphics::Image>,
}

impl ImageGetter {
    /// Creates a new ImageGetter instance
    pub fn new() -> Self {
        Self {
            atlas: None,
            atlases: HashMap::new(),
            ruleset: Ruleset::new(),
            texture_region_drawables: HashMap::new(),
            nine_patch_drawables: HashMap::new(),
        }
    }

    /// Gets a specific atlas by name
    pub fn get_specific_atlas(&self, name: &str) -> Option<&graphics::Texture> {
        self.atlases.get(name)
    }

    /// Resets all atlases, disposing of them
    pub fn reset_atlases(&mut self) {
        for atlas in self.atlases.values() {
            // In ggez, textures are automatically disposed when dropped
        }
        self.atlases.clear();
    }

    /// Reloads all images
    pub fn reload_images(&mut self) {
        self.set_new_ruleset(self.ruleset.clone(), false);
    }

    /// Required every time the ruleset changes, in order to load mod-specific images
    pub fn set_new_ruleset(&mut self, ruleset: Ruleset, ignore_if_mods_are_equal: bool) {
        if ignore_if_mods_are_equal && ruleset.mods == self.ruleset.mods {
            return;
        }

        self.ruleset = ruleset;
        self.texture_region_drawables.clear();

        // Load base
        self.load_mod_atlases("", &GameScreen::get_filesystem());

        // These are from the mods
        let visual_mods: Vec<String> = GameScreen::get_settings()
            .visual_mods
            .iter()
            .chain(self.ruleset.mods.iter())
            .cloned()
            .collect();

        for mod_name in visual_mods {
            if let Some(mod_folder) = GameScreen::get_filesystem().get_mod_folder(&mod_name) {
                self.load_mod_atlases(&mod_name, mod_folder);
            }
        }

        TileSetCache::assemble_tile_set_configs(&self.ruleset.mods);
        SkinCache::assemble_skin_configs(&self.ruleset.mods);

        BaseScreen::set_skin();
        FontRulesetIcons::add_ruleset_images(&self.ruleset);
    }

    /// Loads all atlas/texture files from a folder, as controlled by an Atlases.json
    fn load_mod_atlases(&mut self, mod_name: &str, folder: &Filesystem) {
        // See #4993 - you can't .list() on a jar file, so the ImagePacker leaves us the list of actual atlases.
        let control_file = folder.child("Atlases.json");
        let mut file_names: HashSet<String> = if control_file.exists() {
            if let Ok(content) = control_file.read_to_string() {
                serde_json::from_str(&content).unwrap_or_default()
            } else {
                HashSet::new()
            }
        } else {
            HashSet::new()
        };

        if !mod_name.is_empty() {
            file_names.insert("game".to_string()); // Backwards compatibility
        }

        for file_name in file_names {
            let file = folder.child(&format!("{}.atlas", file_name));
            if !file.exists() {
                continue;
            }

            let extra_atlas = if mod_name.is_empty() {
                file_name.clone()
            } else if file_name == "game" {
                mod_name.to_string()
            } else {
                format!("{}/{}", mod_name, file_name)
            };

            let temp_atlas = self.atlases.get(&extra_atlas).cloned();
            if temp_atlas.is_none() {
                match file.read_to_string() {
                    Ok(atlas_content) => {
                        debug!("Loading {} = {}", extra_atlas, file.path().display());
                        // In a real implementation, we would parse the atlas file and create textures
                        // For now, we'll just create a placeholder texture
                        if let Ok(texture) = graphics::Image::new_color(
                            &mut GameScreen::get_context(),
                            1,
                            1,
                            Color::WHITE,
                        ) {
                            self.atlases.insert(extra_atlas.clone(), texture);
                        }
                    }
                    Err(e) => {
                        debug!("Could not load file {}: {}", file.path().display(), e);
                        continue;
                    }
                }
            }

            // In a real implementation, we would process the atlas regions
            // For now, we'll just create placeholder images
            if let Some(atlas) = self.atlases.get(&extra_atlas) {
                // Process regions from the atlas
                // This is a simplified version - in a real implementation, we would parse the atlas file
                // and create proper texture regions
                if let Ok(image) = graphics::Image::new_color(
                    &mut GameScreen::get_context(),
                    1,
                    1,
                    Color::WHITE,
                ) {
                    self.texture_region_drawables.insert(extra_atlas.clone(), image);
                }
            }
        }
    }

    /// Colors a multilayer image and returns it as a list of layers (Image).
    ///
    /// # Arguments
    ///
    /// * `base_file_name` - The filename of the base image.
    ///   For example: "TileSets/FantasyHex/Units/Warrior"
    /// * `colors` - The list of colors, one per layer. No coloring is applied to layers
    ///   whose color is None.
    ///
    /// # Returns
    ///
    /// The list of layers colored. The layers are sorted by NUMBER (see example below) order
    /// and colors are applied, one per layer, in the same order. If a color is None, no
    /// coloring is performed on such layer (it stays as it is). If there are less colors
    /// than layers, the last layers are not colored. Defaults to an empty list if there
    /// is no layer corresponding to base_file_name.
    pub fn get_layered_image_colored(
        &self,
        base_file_name: &str,
        colors: &[Option<Color>],
    ) -> Vec<graphics::Image> {
        if !self.image_exists(base_file_name) {
            return Vec::new();
        }

        let mut layer_names = vec![base_file_name.to_string()];
        let mut layer_list = Vec::new();

        let mut number = 1;
        while self.image_exists(&format!("{}-{}", base_file_name, number)) {
            layer_names.push(format!("{}-{}", base_file_name, number));
            number += 1;
        }

        for (i, layer_name) in layer_names.iter().enumerate() {
            let mut image = self.get_image(layer_name, None);
            if i < colors.len() {
                if let Some(color) = colors[i] {
                    image.set_color(color);
                }
            }
            layer_list.push(image);
        }

        layer_list
    }

    /// Gets a white dot image
    pub fn get_white_dot(&self) -> graphics::Image {
        let mut image = self.get_image(Self::WHITE_DOT_LOCATION, None);
        image.set_size(1.0);
        image
    }

    /// Gets a white dot drawable
    pub fn get_white_dot_drawable(&self) -> graphics::Image {
        self.texture_region_drawables
            .get(Self::WHITE_DOT_LOCATION)
            .cloned()
            .unwrap_or_else(|| self.get_white_dot())
    }

    /// Gets a dot with the specified color
    pub fn get_dot(&self, dot_color: Color) -> graphics::Image {
        let mut image = self.get_white_dot();
        image.set_color(dot_color);
        image
    }

    /// Finds an image file under /ExtraImages/, including Mods (which can override builtin).
    /// Extension can be included or is guessed as png/jpg.
    /// Returns None if no match found.
    pub fn find_external_image(&self, name: &str) -> Option<PathBuf> {
        let folders = match self.ruleset.mods.iter().map(|mod_name| {
            GameScreen::get_filesystem().get_local_file(&format!("mods/{}/ExtraImages", mod_name))
        }).chain(std::iter::once(GameScreen::get_filesystem().get_internal_file("ExtraImages"))) {
            Some(folders) => folders,
            None => {
                debug!("Error loading mods");
                return None;
            }
        };

        let extensions = vec!["", ".png", ".jpg"];
        for folder in folders {
            for extension in &extensions {
                let path = folder.join(format!("{}{}", name, extension));
                if path.exists() {
                    return Some(path);
                }
            }
        }
        None
    }

    /// Loads an image on the fly - uncached Texture, not too fast.
    pub fn get_external_image(&self, file: &Path) -> graphics::Image {
        // Since these are not packed in an atlas, they have no scaling filter metadata and
        // default to Nearest filter, anisotropic level 1. Use Linear instead, helps
        // loading screen and Tutorial.WorldScreen quite a bit. More anisotropy barely helps.
        match graphics::Image::new(&mut GameScreen::get_context(), file) {
            Ok(mut texture) => {
                texture.set_filter(TextureFilter::Linear);
                texture
            }
            Err(_) => self.get_white_dot(),
        }
    }

    /// Loads an image from (assets)/ExtraImages, from the jar if Unciv runs packaged.
    /// Cannot load ExtraImages from a Mod - use find_external_image and the get_external_image(Path) overload instead.
    pub fn get_external_image_by_name(&self, file_name: &str) -> graphics::Image {
        let path = GameScreen::get_filesystem().get_internal_file("ExtraImages").join(file_name);
        self.get_external_image(&path)
    }

    /// Gets an image by filename, with optional tint color
    pub fn get_image(&self, file_name: Option<&str>, tint_color: Option<Color>) -> graphics::Image {
        let drawable = self.get_drawable(file_name);
        let mut image = drawable;
        image.set_color(tint_color.unwrap_or(Color::WHITE));
        image
    }

    /// Gets a drawable by filename
    pub fn get_drawable(&self, file_name: Option<&str>) -> graphics::Image {
        if let Some(name) = file_name {
            self.texture_region_drawables
                .get(name)
                .cloned()
                .unwrap_or_else(|| self.texture_region_drawables[Self::WHITE_DOT_LOCATION].clone())
        } else {
            self.texture_region_drawables[Self::WHITE_DOT_LOCATION].clone()
        }
    }

    /// Gets a drawable by filename, or None if not found
    pub fn get_drawable_or_null(&self, file_name: Option<&str>) -> Option<graphics::Image> {
        if let Some(name) = file_name {
            self.texture_region_drawables.get(name).cloned()
        } else {
            None
        }
    }

    /// Gets a nine-patch drawable by filename, with optional tint color
    pub fn get_nine_patch(&self, file_name: Option<&str>, tint_color: Option<Color>) -> graphics::Image {
        let drawable = if let Some(name) = file_name {
            self.nine_patch_drawables.get(name).cloned()
        } else {
            None
        }.unwrap_or_else(|| {
            let mut image = graphics::Image::new_color(
                &mut GameScreen::get_context(),
                1,
                1,
                Color::WHITE,
            ).unwrap();
            image.set_size(0.0, 0.0);
            image
        });

        if let Some(color) = tint_color {
            let mut image = drawable;
            image.set_color(color);
            image
        } else {
            drawable
        }
    }

    /// Checks if an image exists
    pub fn image_exists(&self, file_name: &str) -> bool {
        self.texture_region_drawables.contains_key(file_name)
    }

    /// Checks if a nine-patch image exists
    pub fn nine_patch_image_exists(&self, file_name: &str) -> bool {
        self.nine_patch_drawables.contains_key(file_name)
    }

    /// Gets a stat icon
    pub fn get_stat_icon(&self, stat_name: &str) -> graphics::Image {
        let mut image = self.get_image(Some(&format!("StatIcons/{}", stat_name)), None);
        image.set_size(20.0, 20.0);
        image
    }

    /// Checks if a wonder image exists
    pub fn wonder_image_exists(&self, wonder_name: &str) -> bool {
        self.image_exists(&format!("WonderImages/{}", wonder_name))
    }

    /// Gets a wonder image
    pub fn get_wonder_image(&self, wonder_name: &str) -> graphics::Image {
        self.get_image(Some(&format!("WonderImages/{}", wonder_name)), None)
    }

    /// Gets a nation icon
    pub fn get_nation_icon(&self, nation: &str) -> graphics::Image {
        self.get_image(Some(&format!("NationIcons/{}", nation)), None)
    }

    /// Gets a nation portrait
    pub fn get_nation_portrait(&self, nation: &Nation, size: f32) -> NonTransformGroup {
        // In a real implementation, this would create a proper portrait
        // For now, we'll just return a placeholder
        let mut group = NonTransformGroup::new();
        group.set_size(size, size);
        group
    }

    /// Gets a random nation portrait
    pub fn get_random_nation_portrait(&self, size: f32) -> NonTransformGroup {
        // In a real implementation, this would create a proper portrait
        // For now, we'll just return a placeholder
        let mut group = NonTransformGroup::new();
        group.set_size(size, size);
        group
    }

    /// Gets a unit icon
    pub fn get_unit_icon(&self, unit: &BaseUnit, color: Option<Color>) -> graphics::Image {
        let color = color.unwrap_or(Self::CHARCOAL);
        if self.image_exists(&format!("UnitIcons/{}", unit.name)) {
            let mut image = self.get_image(Some(&format!("UnitIcons/{}", unit.name)), None);
            image.set_color(color);
            image
        } else {
            let mut image = self.get_image(Some(&format!("UnitTypeIcons/{}", unit.unit_type)), None);
            image.set_color(color);
            image
        }
    }

    /// Gets a construction portrait
    pub fn get_construction_portrait(&self, construction: &str, size: f32) -> NonTransformGroup {
        if self.ruleset.buildings.contains_key(construction) {
            // In a real implementation, this would create a proper portrait
            // For now, we'll just return a placeholder
            let mut group = NonTransformGroup::new();
            group.set_size(size, size);
            group
        } else if self.ruleset.units.contains_key(construction) {
            // In a real implementation, this would create a proper portrait
            // For now, we'll just return a placeholder
            let mut group = NonTransformGroup::new();
            group.set_size(size, size);
            group
        } else if PerpetualConstruction::perpetual_constructions_map().contains_key(construction) {
            let image = self.get_image(Some(&format!("OtherIcons/Convert{}", construction)), None);
            let mut group = NonTransformGroup::new();
            group.set_size(size, size);
            group.add_child(Box::new(image));
            group
        } else {
            let icon = self.get_stat_icon(construction);
            let mut group = NonTransformGroup::new();
            group.set_size(size, size);
            group.add_child(Box::new(icon));
            group
        }
    }

    /// Gets a unique portrait
    pub fn get_unique_portrait(&self, unique_name: &str, size: f32) -> NonTransformGroup {
        // In a real implementation, this would create a proper portrait
        // For now, we'll just return a placeholder
        let mut group = NonTransformGroup::new();
        group.set_size(size, size);
        group
    }

    /// Gets a promotion portrait
    pub fn get_promotion_portrait(&self, promotion_name: &str, size: f32) -> NonTransformGroup {
        // In a real implementation, this would create a proper portrait
        // For now, we'll just return a placeholder
        let mut group = NonTransformGroup::new();
        group.set_size(size, size);
        group
    }

    /// Gets a resource portrait
    pub fn get_resource_portrait(&self, resource_name: &str, size: f32, amount: i32) -> NonTransformGroup {
        // In a real implementation, this would create a proper portrait
        // For now, we'll just return a placeholder
        let mut group = NonTransformGroup::new();
        group.set_size(size, size);
        group
    }

    /// Gets a tech icon portrait
    pub fn get_tech_icon_portrait(&self, tech_name: &str, circle_size: f32) -> NonTransformGroup {
        // In a real implementation, this would create a proper portrait
        // For now, we'll just return a placeholder
        let mut group = NonTransformGroup::new();
        group.set_size(circle_size, circle_size);
        group
    }

    /// Gets an improvement portrait
    pub fn get_improvement_portrait(
        &self,
        improvement_name: &str,
        size: f32,
        is_pillaged: bool,
    ) -> NonTransformGroup {
        // In a real implementation, this would create a proper portrait
        // For now, we'll just return a placeholder
        let mut group = NonTransformGroup::new();
        group.set_size(size, size);
        group
    }

    /// Gets a unit action portrait
    pub fn get_unit_action_portrait(&self, action_name: &str, size: f32) -> NonTransformGroup {
        // In a real implementation, this would create a proper portrait
        // For now, we'll just return a placeholder
        let mut group = NonTransformGroup::new();
        group.set_size(size, size);
        group
    }

    /// Gets a religion icon
    pub fn get_religion_icon(&self, icon_name: &str) -> graphics::Image {
        self.get_image(Some(&format!("ReligionIcons/{}", icon_name)), None)
    }

    /// Gets a religion portrait
    pub fn get_religion_portrait(&self, icon_name: &str, size: f32) -> NonTransformGroup {
        if self.religion_icon_exists(icon_name) {
            // In a real implementation, this would create a proper portrait
            // For now, we'll just return a placeholder
            let mut group = NonTransformGroup::new();
            group.set_size(size, size);
            group
        } else if let Some(belief) = self.ruleset.beliefs.get(icon_name) {
            if let Some(type_name) = &belief.belief_type {
                if let Some(type_name_str) = &type_name.name {
                    if self.religion_icon_exists(type_name_str) {
                        // In a real implementation, this would create a proper portrait
                        // For now, we'll just return a placeholder
                        let mut group = NonTransformGroup::new();
                        group.set_size(size, size);
                        group
                    } else {
                        // In a real implementation, this would create a proper portrait
                        // For now, we'll just return a placeholder
                        let mut group = NonTransformGroup::new();
                        group.set_size(size, size);
                        group
                    }
                } else {
                    // In a real implementation, this would create a proper portrait
                    // For now, we'll just return a placeholder
                    let mut group = NonTransformGroup::new();
                    group.set_size(size, size);
                    group
                }
            } else {
                // In a real implementation, this would create a proper portrait
                // For now, we'll just return a placeholder
                let mut group = NonTransformGroup::new();
                group.set_size(size, size);
                group
            }
        } else {
            // In a real implementation, this would create a proper portrait
            // For now, we'll just return a placeholder
            let mut group = NonTransformGroup::new();
            group.set_size(size, size);
            group
        }
    }

    /// Checks if a religion icon exists
    pub fn religion_icon_exists(&self, icon_name: &str) -> bool {
        self.image_exists(&format!("ReligionIcons/{}", icon_name))
    }

    /// Gets a circle drawable
    pub fn get_circle_drawable(&self) -> graphics::Image {
        self.get_drawable(Some(Self::CIRCLE_LOCATION))
    }

    /// Gets a circle image
    pub fn get_circle(&self) -> graphics::Image {
        self.get_image(Some(Self::CIRCLE_LOCATION), None)
    }

    /// Gets a circle image with the specified color and size
    pub fn get_circle_with_color(&self, color: Option<Color>, size: Option<f32>) -> graphics::Image {
        let mut image = self.get_circle();
        image.set_color(color.unwrap_or(Color::WHITE));
        if let Some(s) = size {
            image.set_size(s, s);
        }
        image
    }

    /// Gets a triangle image
    pub fn get_triangle(&self) -> graphics::Image {
        self.get_image(Some("OtherIcons/Triangle"), None)
    }

    /// Gets a red cross image
    pub fn get_red_cross(&self, size: f32, alpha: f32) -> graphics::Image {
        let mut red_cross = self.get_image(Some("OtherIcons/Close"), None);
        red_cross.set_size(size, size);
        let mut color = Color::RED;
        color.a = alpha;
        red_cross.set_color(color);
        red_cross
    }

    /// Gets a crossed image
    pub fn get_crossed_image(&self, image: graphics::Image, icon_size: f32) -> NonTransformGroup {
        let mut group = NonTransformGroup::new();
        group.set_size(icon_size, icon_size);

        // Center the image in the group
        let mut image_clone = image.clone();
        image_clone.set_position(
            (icon_size - image_clone.width()) / 2.0,
            (icon_size - image_clone.height()) / 2.0,
        );
        group.add_child(Box::new(image_clone));

        // Add the cross
        let cross = self.get_red_cross(icon_size * 0.7, 0.7);
        let mut cross_clone = cross.clone();
        cross_clone.set_position(
            (icon_size - cross_clone.width()) / 2.0,
            (icon_size - cross_clone.height()) / 2.0,
        );
        group.add_child(Box::new(cross_clone));

        group
    }

    /// Gets an arrow image
    pub fn get_arrow_image(&self, align: i32) -> graphics::Image {
        let mut image = self.get_image(Some("OtherIcons/ArrowRight"), None);
        image.set_origin(0.5, 0.5); // Center origin

        // In ggez, rotation is in radians, and positive is clockwise
        match align {
            1 => image.set_rotation(PI), // left
            2 => image.set_rotation(-PI / 2.0), // bottom
            3 => image.set_rotation(PI / 2.0), // top
            _ => {} // right (default)
        }

        image
    }

    /// Gets a vertical progress bar
    pub fn get_progress_bar_vertical(
        &self,
        width: f32,
        height: f32,
        percent_complete: f32,
        progress_color: Color,
        background_color: Color,
        progress_padding: f32,
    ) -> ProgressBar {
        let mut progress_bar = ProgressBar::new(width, height, true);
        progress_bar.set_background(background_color);
        progress_bar.set_progress(progress_color, percent_complete, progress_padding);
        progress_bar
    }

    /// Gets a health bar
    pub fn get_health_bar(&self, current_health: f32, max_health: f32, health_bar_size: f32, height: f32) -> Table {
        let health_percent = current_health / max_health;
        let mut health_bar = Table::new();

        let mut health_part_of_bar = self.get_white_dot();
        let health_color = if health_percent > 2.0 / 3.0 {
            Color::GREEN
        } else if health_percent > 1.0 / 3.0 {
            Color::ORANGE
        } else {
            Color::RED
        };
        health_part_of_bar.set_color(health_color);
        health_bar.add_cell(TableCell::new(Some(Box::new(health_part_of_bar))).size(
            health_bar_size * health_percent,
            height,
        ));

        let mut empty_part_of_bar = self.get_dot(Self::CHARCOAL);
        health_bar.add_cell(TableCell::new(Some(Box::new(empty_part_of_bar))).size(
            health_bar_size * (1.0 - health_percent),
            height,
        ));

        health_bar.set_padding(1.0);
        health_bar.pack();
        health_bar.set_background(BaseScreen::get_skin().get_ui_background("General/HealthBar", Some(Self::CHARCOAL)));
        health_bar
    }

    /// Gets a line between two points
    pub fn get_line(&self, start_x: f32, start_y: f32, end_x: f32, end_y: f32, width: f32) -> graphics::Image {
        // The simplest way to draw a line between 2 points seems to be:
        // A. Get a pixel dot, set its width to the required length (hypotenuse)
        // B. Set its rotational center, and set its rotation
        // C. Center it on the point where you want its center to be

        // A
        let mut line = self.get_white_dot();
        let delta_x = (start_x - end_x) as f64;
        let delta_y = (start_y - end_y) as f64;
        line.set_size((delta_x * delta_x + delta_y * delta_y).sqrt() as f32, width);

        // B
        line.set_origin(0.5, 0.5); // Center origin
        let radians_to_degrees = 180.0 / PI as f64;
        line.set_rotation((delta_y.atan2(delta_x) * radians_to_degrees) as f32);

        // C
        line.set_position(
            (start_x + end_x) / 2.0 - line.width() / 2.0,
            (start_y + end_y) / 2.0 - line.height() / 2.0,
        );

        line
    }

    /// Gets a specialist icon
    pub fn get_specialist_icon(&self, color: Color) -> graphics::Image {
        let mut specialist = self.get_image(Some("StatIcons/Specialist"), None);
        specialist.set_color(color);
        specialist
    }

    /// Gets all image names
    pub fn get_all_image_names(&self) -> Vec<String> {
        self.texture_region_drawables.keys().cloned().collect()
    }

    /// Gets available skins
    pub fn get_available_skins(&self) -> Vec<String> {
        self.nine_patch_drawables
            .keys()
            .filter_map(|key| {
                if key.starts_with("Skins/") {
                    key.split('/').nth(1).map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Determines available TileSets from the currently loaded Texture paths.
    ///
    /// Note TileSetCache will not necessarily load all of them, e.g. if a Mod fails
    /// to provide a config json for a graphic with a Tileset path.
    ///
    /// Intersect with TileSetCache.get_available_tilesets for a more reliable answer
    pub fn get_available_tilesets(&self) -> Vec<String> {
        self.texture_region_drawables
            .keys()
            .filter(|key| key.starts_with("TileSets") && !key.contains("/Units/"))
            .filter_map(|key| key.split('/').nth(1).map(|s| s.to_string()))
            .collect()
    }

    /// Gets available unitsets
    pub fn get_available_unitsets(&self) -> Vec<String> {
        self.texture_region_drawables
            .keys()
            .filter(|key| key.starts_with("TileSets") && key.contains("/Units/"))
            .filter_map(|key| key.split('/').nth(1).map(|s| s.to_string()))
            .collect()
    }
}

// Implement Default for ImageGetter
impl Default for ImageGetter {
    fn default() -> Self {
        Self::new()
    }
}

// Make ImageGetter a singleton
lazy_static::lazy_static! {
    pub static ref IMAGE_GETTER: Arc<ImageGetter> = Arc::new(ImageGetter::new());
}