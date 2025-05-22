use ggez::graphics::{self, Color, Context, DrawParam, GameResult, Image, Point2, Rect};
use ggez::mint::Point2 as MintPoint2;
use std::collections::HashMap;

use crate::logic::ruleset::Ruleset;
use crate::models::ruleset::unit::Promotion;
use crate::models::stats::Stats;
use crate::ui::components::label::Label;
use crate::ui::components::non_transform_group::NonTransformGroup;
use crate::ui::components::table::Table;
use crate::ui::components::table_cell::TableCell;
use crate::ui::images::image_getter::IMAGE_GETTER;

/// Type of portrait
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortraitType {
    Unit,
    Building,
    Tech,
    Resource,
    Improvement,
    Promotion,
    Unique,
    Nation,
    Religion,
    UnitAction,
}

impl PortraitType {
    /// Get the directory name for this portrait type
    pub fn directory(&self) -> &'static str {
        match self {
            PortraitType::Unit => "Unit",
            PortraitType::Building => "Building",
            PortraitType::Tech => "Tech",
            PortraitType::Resource => "Resource",
            PortraitType::Improvement => "Improvement",
            PortraitType::Promotion => "UnitPromotion",
            PortraitType::Unique => "Unique",
            PortraitType::Nation => "Nation",
            PortraitType::Religion => "Religion",
            PortraitType::UnitAction => "UnitAction",
        }
    }
}

/// Base portrait class that manages "portraits" for a subset of RulesetObjects
///
/// A Portrait will be a classic circular Icon in vanilla
/// Mods can supply portraits in separate texture paths that can fill a square
/// Instantiate through ImageGetter.get_<type>_portrait() methods
///
/// Caveat:
/// - This is a Group and does not support Layout
/// - It sets its own size but paints outside these bounds - by border_size
/// - Typically, if you want one in a Table Cell, add an extra border_size padding to avoid surprises
pub struct Portrait {
    /// The type of portrait
    portrait_type: PortraitType,
    /// The name of the image
    image_name: String,
    /// The size of the portrait
    size: f32,
    /// The size of the border
    border_size: f32,
    /// The main image
    image: Image,
    /// The background group
    background: NonTransformGroup,
    /// Reference to the ruleset
    ruleset: Ruleset,
    /// Whether this is a portrait (vs an icon)
    is_portrait: bool,
    /// The path to the portrait
    path_portrait: String,
    /// The path to the portrait fallback
    path_portrait_fallback: String,
    /// The path to the icon
    path_icon: String,
    /// The path to the icon fallback
    path_icon_fallback: String,
}

impl Portrait {
    /// Creates a new portrait
    pub fn new(
        portrait_type: PortraitType,
        image_name: String,
        size: f32,
        border_size: f32,
    ) -> Self {
        let ruleset = IMAGE_GETTER.ruleset.clone();
        let path_portrait = format!("{}Portraits/{}", portrait_type.directory(), image_name);
        let path_portrait_fallback = format!("{}Portraits/Fallback", portrait_type.directory());
        let path_icon = format!("{}Icons/{}", portrait_type.directory(), image_name);
        let path_icon_fallback = format!("{}Icons/Fallback", portrait_type.directory());

        let mut portrait = Self {
            portrait_type,
            image_name,
            size,
            border_size,
            image: Image::default(),
            background: NonTransformGroup::new(),
            ruleset,
            is_portrait: false,
            path_portrait,
            path_portrait_fallback,
            path_icon,
            path_icon_fallback,
        };

        // Initialize the portrait
        portrait.image = portrait.get_main_image();
        portrait.background = portrait.get_main_background();

        // Set the size
        portrait
            .background
            .set_size(size + border_size, size + border_size);
        portrait.image.set_size(size * 0.75, size * 0.75);

        // Center the image and background
        portrait.center_image();
        portrait.center_background();

        portrait
    }

    /// Gets the default inner background tint color
    pub fn get_default_inner_background_tint(&self) -> Color {
        Color::WHITE
    }

    /// Gets the default outer background tint color
    pub fn get_default_outer_background_tint(&self) -> Color {
        Color::BLACK
    }

    /// Gets the default image tint color
    pub fn get_default_image_tint(&self) -> Color {
        Color::WHITE
    }

    /// Gets the default image
    pub fn get_default_image(&self) -> Image {
        if IMAGE_GETTER.image_exists(&self.path_icon) {
            IMAGE_GETTER.get_image(Some(&self.path_icon), None)
        } else if IMAGE_GETTER.image_exists(&self.path_icon_fallback) {
            IMAGE_GETTER.get_image(Some(&self.path_icon_fallback), None)
        } else {
            IMAGE_GETTER.get_circle()
        }
    }

    /// Gets the main image
    pub fn get_main_image(&self) -> Image {
        if IMAGE_GETTER.image_exists(&self.path_portrait) {
            let mut image = IMAGE_GETTER.get_image(Some(&self.path_portrait), None);
            image.set_color(self.get_default_image_tint());
            image
        } else if IMAGE_GETTER.image_exists(&self.path_portrait_fallback) {
            let mut image = IMAGE_GETTER.get_image(Some(&self.path_portrait_fallback), None);
            image.set_color(self.get_default_image_tint());
            image
        } else {
            let mut image = self.get_default_image();
            image.set_color(self.get_default_image_tint());
            image
        }
    }

    /// Gets the circle image
    pub fn get_circle_image(&self) -> Image {
        IMAGE_GETTER.get_circle()
    }

    /// Gets the main background
    pub fn get_main_background(&self) -> NonTransformGroup {
        if self.is_portrait
            && IMAGE_GETTER.image_exists(&format!(
                "{}Portraits/Background",
                self.portrait_type.directory()
            ))
        {
            let background_image = IMAGE_GETTER.get_image(
                Some(&format!(
                    "{}Portraits/Background",
                    self.portrait_type.directory()
                )),
                None,
            );

            let ratio_w = self.image.width() / background_image.width();
            let ratio_h = self.image.height() / background_image.height();

            self.image.set_size(
                (self.size + self.border_size) * ratio_w,
                (self.size + self.border_size) * ratio_h,
            );

            let mut group = NonTransformGroup::new();
            group.set_size(self.size + self.border_size, self.size + self.border_size);
            group.add_child(Box::new(background_image));
            group
        } else {
            let mut bg = NonTransformGroup::new();
            bg.set_size(self.size + self.border_size, self.size + self.border_size);

            let circle_inner = self.get_circle_image();
            let circle_outer = self.get_circle_image();

            circle_inner.set_size(self.size, self.size);
            circle_outer.set_size(self.size + self.border_size, self.size + self.border_size);

            circle_inner.set_color(self.get_default_inner_background_tint());
            circle_outer.set_color(self.get_default_outer_background_tint());

            // Center the circles
            circle_outer.set_position(
                (self.size + self.border_size - circle_outer.width()) / 2.0,
                (self.size + self.border_size - circle_outer.height()) / 2.0,
            );

            circle_inner.set_position(
                (self.size + self.border_size - circle_inner.width()) / 2.0,
                (self.size + self.border_size - circle_inner.height()) / 2.0,
            );

            bg.add_child(Box::new(circle_outer));
            bg.add_child(Box::new(circle_inner));

            bg
        }
    }

    /// Centers the image in the portrait
    pub fn center_image(&mut self) {
        self.image.set_position(
            (self.size + self.border_size - self.image.width()) / 2.0,
            (self.size + self.border_size - self.image.height()) / 2.0,
        );
    }

    /// Centers the background in the portrait
    pub fn center_background(&mut self) {
        self.background.set_position(0.0, 0.0);
    }

    /// Draws the portrait
    pub fn draw(&self, ctx: &mut Context, param: DrawParam) -> GameResult {
        // Draw the background first
        self.background.draw(ctx, param)?;

        // Then draw the image
        self.image.draw(ctx, param)
    }

    /// Gets the width of the portrait
    pub fn width(&self) -> f32 {
        self.size + self.border_size
    }

    /// Gets the height of the portrait
    pub fn height(&self) -> f32 {
        self.size + self.border_size
    }

    /// Sets the position of the portrait
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.background.set_position(x, y);
        self.image.set_position(
            x + (self.size + self.border_size - self.image.width()) / 2.0,
            y + (self.size + self.border_size - self.image.height()) / 2.0,
        );
    }

    /// Sets the size of the portrait
    pub fn set_size(&mut self, width: f32, height: f32) {
        let scale_x = width / (self.size + self.border_size);
        let scale_y = height / (self.size + self.border_size);

        self.background.set_size(width, height);
        self.image
            .set_size(self.size * 0.75 * scale_x, self.size * 0.75 * scale_y);

        self.center_image();
    }
}

/// Resource portrait
pub struct PortraitResource {
    /// The base portrait
    base: Portrait,
    /// The amount of the resource
    amount: i32,
}

impl PortraitResource {
    /// Creates a new resource portrait
    pub fn new(name: String, size: f32, amount: i32) -> Self {
        let mut base = Portrait::new(PortraitType::Resource, name, size, 2.0);

        // Override the circle image
        let circle_image = IMAGE_GETTER.get_image(Some("ResourceIcons/Circle"), None);
        base.background = base.get_main_background();

        // Create the portrait
        let mut portrait = Self { base, amount };

        // Add the amount label if needed
        if amount > 0 {
            let label = Label::new(
                amount.to_string(),
                8.0,
                Color::WHITE,
                graphics::TextAlignment::Center,
            );

            let mut amount_group = NonTransformGroup::new();
            amount_group.set_size(portrait.base.size / 2.0, portrait.base.size / 2.0);

            // Add a circle background
            let mut circle = IMAGE_GETTER.get_circle();
            circle.set_size(portrait.base.size / 2.0, portrait.base.size / 2.0);
            circle.set_color(IMAGE_GETTER::CHARCOAL);
            circle.set_position(0.0, 0.0);

            // Position the label
            let mut label_clone = label.clone();
            label_clone.set_position(
                (portrait.base.size / 2.0 - label.width()) / 2.0,
                (portrait.base.size / 2.0 - label.height()) / 2.0 - 0.5,
            );

            amount_group.add_child(Box::new(circle));
            amount_group.add_child(Box::new(label_clone));

            // Position the amount group
            amount_group.set_position(
                portrait.base.width() - amount_group.width() * 3.0 / 4.0,
                -amount_group.height() / 4.0,
            );

            portrait.base.background.add_child(Box::new(amount_group));
        }

        portrait
    }

    /// Gets the default inner background tint color
    pub fn get_default_inner_background_tint(&self) -> Color {
        if let Some(resource) = self.base.ruleset.tile_resources.get(&self.base.image_name) {
            if let Some(resource_type) = &resource.resource_type {
                resource_type.get_color()
            } else {
                Color::WHITE
            }
        } else {
            Color::WHITE
        }
    }

    /// Draws the resource portrait
    pub fn draw(&self, ctx: &mut Context, param: DrawParam) -> GameResult {
        self.base.draw(ctx, param)
    }

    /// Gets the width of the portrait
    pub fn width(&self) -> f32 {
        self.base.width()
    }

    /// Gets the height of the portrait
    pub fn height(&self) -> f32 {
        self.base.height()
    }

    /// Sets the position of the portrait
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.base.set_position(x, y);
    }

    /// Sets the size of the portrait
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.base.set_size(width, height);
    }
}

/// Tech portrait
pub struct PortraitTech {
    /// The base portrait
    base: Portrait,
}

impl PortraitTech {
    /// Creates a new tech portrait
    pub fn new(name: String, size: f32) -> Self {
        let base = Portrait::new(PortraitType::Tech, name, size, 2.0);

        Self { base }
    }

    /// Gets the default outer background tint color
    pub fn get_default_outer_background_tint(&self) -> Color {
        self.get_default_image_tint()
    }

    /// Gets the default image tint color
    pub fn get_default_image_tint(&self) -> Color {
        if let Some(tech) = self.base.ruleset.technologies.get(&self.base.image_name) {
            if let Some(era) = tech.era() {
                if let Some(era_obj) = self.base.ruleset.eras.get(&era) {
                    if let Some(color) = era_obj.get_color() {
                        return color.darken(0.6);
                    }
                }
            }
        }

        IMAGE_GETTER::CHARCOAL
    }

    /// Gets the circle image
    pub fn get_circle_image(&self) -> Image {
        IMAGE_GETTER.get_image(Some("TechIcons/Circle"), None)
    }

    /// Draws the tech portrait
    pub fn draw(&self, ctx: &mut Context, param: DrawParam) -> GameResult {
        self.base.draw(ctx, param)
    }

    /// Gets the width of the portrait
    pub fn width(&self) -> f32 {
        self.base.width()
    }

    /// Gets the height of the portrait
    pub fn height(&self) -> f32 {
        self.base.height()
    }

    /// Sets the position of the portrait
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.base.set_position(x, y);
    }

    /// Sets the size of the portrait
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.base.set_size(width, height);
    }
}

/// Unit portrait
pub struct PortraitUnit {
    /// The base portrait
    base: Portrait,
}

impl PortraitUnit {
    /// Creates a new unit portrait
    pub fn new(name: String, size: f32) -> Self {
        let base = Portrait::new(PortraitType::Unit, name, size, 2.0);

        Self { base }
    }

    /// Gets the default image tint color
    pub fn get_default_image_tint(&self) -> Color {
        Color::BLACK
    }

    /// Gets the circle image
    pub fn get_circle_image(&self) -> Image {
        IMAGE_GETTER.get_image(Some("OtherIcons/ConstructionCircle"), None)
    }

    /// Draws the unit portrait
    pub fn draw(&self, ctx: &mut Context, param: DrawParam) -> GameResult {
        self.base.draw(ctx, param)
    }

    /// Gets the width of the portrait
    pub fn width(&self) -> f32 {
        self.base.width()
    }

    /// Gets the height of the portrait
    pub fn height(&self) -> f32 {
        self.base.height()
    }

    /// Sets the position of the portrait
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.base.set_position(x, y);
    }

    /// Sets the size of the portrait
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.base.set_size(width, height);
    }
}

/// Building portrait
pub struct PortraitBuilding {
    /// The base portrait
    base: Portrait,
}

impl PortraitBuilding {
    /// Creates a new building portrait
    pub fn new(name: String, size: f32) -> Self {
        let base = Portrait::new(PortraitType::Building, name, size, 2.0);

        Self { base }
    }

    /// Gets the default image tint color
    pub fn get_default_image_tint(&self) -> Color {
        Color::BLACK
    }

    /// Gets the circle image
    pub fn get_circle_image(&self) -> Image {
        IMAGE_GETTER.get_image(Some("OtherIcons/ConstructionCircle"), None)
    }

    /// Draws the building portrait
    pub fn draw(&self, ctx: &mut Context, param: DrawParam) -> GameResult {
        self.base.draw(ctx, param)
    }

    /// Gets the width of the portrait
    pub fn width(&self) -> f32 {
        self.base.width()
    }

    /// Gets the height of the portrait
    pub fn height(&self) -> f32 {
        self.base.height()
    }

    /// Sets the position of the portrait
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.base.set_position(x, y);
    }

    /// Sets the size of the portrait
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.base.set_size(width, height);
    }
}

/// Unavailable wonder for tech tree portrait
pub struct PortraitUnavailableWonderForTechTree {
    /// The base portrait
    base: Portrait,
}

impl PortraitUnavailableWonderForTechTree {
    /// Creates a new unavailable wonder for tech tree portrait
    pub fn new(name: String, size: f32) -> Self {
        let base = Portrait::new(PortraitType::Building, name, size, 2.0);

        Self { base }
    }

    /// Gets the default outer background tint color
    pub fn get_default_outer_background_tint(&self) -> Color {
        Color::RED
    }

    /// Draws the unavailable wonder for tech tree portrait
    pub fn draw(&self, ctx: &mut Context, param: DrawParam) -> GameResult {
        self.base.draw(ctx, param)
    }

    /// Gets the width of the portrait
    pub fn width(&self) -> f32 {
        self.base.width()
    }

    /// Gets the height of the portrait
    pub fn height(&self) -> f32 {
        self.base.height()
    }

    /// Sets the position of the portrait
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.base.set_position(x, y);
    }

    /// Sets the size of the portrait
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.base.set_size(width, height);
    }
}

/// Unique portrait
pub struct PortraitUnique {
    /// The base portrait
    base: Portrait,
}

impl PortraitUnique {
    /// Creates a new unique portrait
    pub fn new(name: String, size: f32) -> Self {
        let base = Portrait::new(PortraitType::Unique, name, size, 2.0);

        Self { base }
    }

    /// Gets the default image tint color
    pub fn get_default_image_tint(&self) -> Color {
        IMAGE_GETTER::CHARCOAL
    }

    /// Draws the unique portrait
    pub fn draw(&self, ctx: &mut Context, param: DrawParam) -> GameResult {
        self.base.draw(ctx, param)
    }

    /// Gets the width of the portrait
    pub fn width(&self) -> f32 {
        self.base.width()
    }

    /// Gets the height of the portrait
    pub fn height(&self) -> f32 {
        self.base.height()
    }

    /// Sets the position of the portrait
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.base.set_position(x, y);
    }

    /// Sets the size of the portrait
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.base.set_size(width, height);
    }
}

/// Religion portrait
pub struct PortraitReligion {
    /// The base portrait
    base: Portrait,
}

impl PortraitReligion {
    /// Creates a new religion portrait
    pub fn new(name: String, size: f32) -> Self {
        let base = Portrait::new(PortraitType::Religion, name, size, 2.0);

        Self { base }
    }

    /// Gets the default image tint color
    pub fn get_default_image_tint(&self) -> Color {
        IMAGE_GETTER::CHARCOAL
    }

    /// Draws the religion portrait
    pub fn draw(&self, ctx: &mut Context, param: DrawParam) -> GameResult {
        self.base.draw(ctx, param)
    }

    /// Gets the width of the portrait
    pub fn width(&self) -> f32 {
        self.base.width()
    }

    /// Gets the height of the portrait
    pub fn height(&self) -> f32 {
        self.base.height()
    }

    /// Sets the position of the portrait
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.base.set_position(x, y);
    }

    /// Sets the size of the portrait
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.base.set_size(width, height);
    }
}

/// Unit action portrait
pub struct PortraitUnitAction {
    /// The base portrait
    base: Portrait,
}

impl PortraitUnitAction {
    /// Creates a new unit action portrait
    pub fn new(name: String, size: f32) -> Self {
        let base = Portrait::new(PortraitType::UnitAction, name, size, 2.0);

        Self { base }
    }

    /// Gets the default image tint color
    pub fn get_default_image_tint(&self) -> Color {
        IMAGE_GETTER::CHARCOAL
    }

    /// Draws the unit action portrait
    pub fn draw(&self, ctx: &mut Context, param: DrawParam) -> GameResult {
        self.base.draw(ctx, param)
    }

    /// Gets the width of the portrait
    pub fn width(&self) -> f32 {
        self.base.width()
    }

    /// Gets the height of the portrait
    pub fn height(&self) -> f32 {
        self.base.height()
    }

    /// Sets the position of the portrait
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.base.set_position(x, y);
    }

    /// Sets the size of the portrait
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.base.set_size(width, height);
    }
}

/// Improvement portrait
pub struct PortraitImprovement {
    /// The base portrait
    base: Portrait,
    /// Whether the portrait is dimmed
    dim: bool,
    /// Whether the improvement is pillaged
    is_pillaged: bool,
}

impl PortraitImprovement {
    /// Creates a new improvement portrait
    pub fn new(name: String, size: f32, dim: bool, is_pillaged: bool) -> Self {
        let mut base = Portrait::new(PortraitType::Improvement, name, size, 2.0);

        // Create the portrait
        let mut portrait = Self {
            base,
            dim,
            is_pillaged,
        };

        // Apply dimming if needed
        if dim {
            let mut image_color = portrait.base.image.color();
            image_color.a = 0.7;
            portrait.base.image.set_color(image_color);

            // Apply to background as well
            for child in &mut portrait.base.background.children {
                if let Some(image) = child.downcast_mut::<Image>() {
                    let mut color = image.color();
                    color.a = 0.7;
                    image.set_color(color);
                }
            }
        }

        // Add pillaged icon if needed
        if is_pillaged {
            let mut pillaged_icon = IMAGE_GETTER.get_image(Some("OtherIcons/Fire"), None);
            pillaged_icon.set_size(portrait.base.width() / 2.0, portrait.base.height() / 2.0);
            pillaged_icon.set_position(portrait.base.width() - pillaged_icon.width(), 0.0);

            portrait.base.background.add_child(Box::new(pillaged_icon));
        }

        portrait
    }

    /// Gets the circle image
    pub fn get_circle_image(&self) -> Image {
        IMAGE_GETTER.get_image(Some("ImprovementIcons/Circle"), None)
    }

    /// Gets the color from stats
    fn get_color_from_stats(&self, stats: &Stats) -> Color {
        let mut max_value = 0.0;
        let mut max_key = None;

        for (key, value) in stats.iter() {
            if *value > max_value {
                max_value = *value;
                max_key = Some(key.clone());
            }
        }

        if max_value > 0.0 {
            if let Some(key) = max_key {
                key.color()
            } else {
                Color::WHITE
            }
        } else {
            Color::WHITE
        }
    }

    /// Gets the default inner background tint color
    pub fn get_default_inner_background_tint(&self) -> Color {
        if let Some(improvement) = self
            .base
            .ruleset
            .tile_improvements
            .get(&self.base.image_name)
        {
            self.get_color_from_stats(improvement)
        } else {
            Color::WHITE
        }
    }

    /// Draws the improvement portrait
    pub fn draw(&self, ctx: &mut Context, param: DrawParam) -> GameResult {
        self.base.draw(ctx, param)
    }

    /// Gets the width of the portrait
    pub fn width(&self) -> f32 {
        self.base.width()
    }

    /// Gets the height of the portrait
    pub fn height(&self) -> f32 {
        self.base.height()
    }

    /// Sets the position of the portrait
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.base.set_position(x, y);
    }

    /// Sets the size of the portrait
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.base.set_size(width, height);
    }
}

/// Nation portrait
pub struct PortraitNation {
    /// The base portrait
    base: Portrait,
}

impl PortraitNation {
    /// Creates a new nation portrait
    pub fn new(name: String, size: f32) -> Self {
        let border_size = size * 0.1;
        let base = Portrait::new(PortraitType::Nation, name, size, border_size);

        Self { base }
    }

    /// Gets the default image
    pub fn get_default_image(&self) -> Image {
        let nation = self.base.ruleset.nations.get(&self.base.image_name);
        let is_city_state = nation.map_or(false, |n| n.is_city_state);
        let path_city_state = "NationIcons/CityState";

        if IMAGE_GETTER.image_exists(&self.base.path_icon) {
            IMAGE_GETTER.get_image(Some(&self.base.path_icon), None)
        } else if is_city_state && IMAGE_GETTER.image_exists(path_city_state) {
            IMAGE_GETTER.get_image(Some(path_city_state), None)
        } else if IMAGE_GETTER.image_exists(&self.base.path_icon_fallback) {
            IMAGE_GETTER.get_image(Some(&self.base.path_icon_fallback), None)
        } else {
            IMAGE_GETTER.get_circle()
        }
    }

    /// Gets the default inner background tint color
    pub fn get_default_inner_background_tint(&self) -> Color {
        if let Some(nation) = self.base.ruleset.nations.get(&self.base.image_name) {
            nation.get_outer_color()
        } else {
            IMAGE_GETTER::CHARCOAL
        }
    }

    /// Gets the default outer background tint color
    pub fn get_default_outer_background_tint(&self) -> Color {
        self.get_default_image_tint()
    }

    /// Gets the default image tint color
    pub fn get_default_image_tint(&self) -> Color {
        if let Some(nation) = self.base.ruleset.nations.get(&self.base.image_name) {
            nation.get_inner_color()
        } else {
            Color::WHITE
        }
    }

    /// Draws the nation portrait
    pub fn draw(&self, ctx: &mut Context, param: DrawParam) -> GameResult {
        self.base.draw(ctx, param)
    }

    /// Gets the width of the portrait
    pub fn width(&self) -> f32 {
        self.base.width()
    }

    /// Gets the height of the portrait
    pub fn height(&self) -> f32 {
        self.base.height()
    }

    /// Sets the position of the portrait
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.base.set_position(x, y);
    }

    /// Sets the size of the portrait
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.base.set_size(width, height);
    }
}

/// Promotion portrait
pub struct PortraitPromotion {
    /// The base portrait
    base: Portrait,
    /// The level of the promotion
    level: i32,
}

impl PortraitPromotion {
    /// Default inner color
    pub const DEFAULT_INNER_COLOR: Color = Color {
        r: 1.0,
        g: 0.886,
        b: 0.0,
        a: 1.0,
    };

    /// Default outer color
    pub const DEFAULT_OUTER_COLOR: Color = Color {
        r: 0.0,
        g: 0.047,
        b: 0.192,
        a: 1.0,
    };

    /// Creates a new promotion portrait
    pub fn new(name: String, size: f32) -> Self {
        let base = Portrait::new(PortraitType::Promotion, name, size, 2.0);

        let mut portrait = Self { base, level: 0 };

        // Get the base name and level
        let (name_without_brackets, level, base_promotion_name) =
            Promotion::get_base_name_and_level(&portrait.base.image_name);
        portrait.level = level;

        // Add stars if needed
        if portrait.level > 0 {
            let padding = if portrait.level == 3 { 0.5 } else { 2.0 };

            let mut star_table = Table::new();
            star_table.set_padding(padding);

            for _ in 0..portrait.level {
                let mut star = IMAGE_GETTER.get_image(Some("OtherIcons/Star"), None);
                star.set_size(portrait.base.size / 4.0, portrait.base.size / 4.0);

                let mut cell = TableCell::new(Some(Box::new(star)));
                cell.set_size(portrait.base.size / 4.0, portrait.base.size / 4.0);

                star_table.add_cell(cell);
            }

            // Center the star table
            star_table.set_position(
                (portrait.base.width() - star_table.width()) / 2.0,
                portrait.base.size / 6.0,
            );

            portrait.base.background.add_child(Box::new(star_table));
        }

        portrait
    }

    /// Gets the default image
    pub fn get_default_image(&self) -> Image {
        let (name_without_brackets, level, base_promotion_name) =
            Promotion::get_base_name_and_level(&self.base.image_name);

        let path_without_brackets = format!("UnitPromotionIcons/{}", name_without_brackets);
        let path_base = format!("UnitPromotionIcons/{}", base_promotion_name);
        let path_unit = format!("UnitIcons/{}", base_promotion_name.replace(" ability", ""));

        if IMAGE_GETTER.image_exists(&path_without_brackets) {
            IMAGE_GETTER.get_image(Some(&path_without_brackets), None)
        } else if IMAGE_GETTER.image_exists(&path_base) {
            IMAGE_GETTER.get_image(Some(&path_base), None)
        } else if IMAGE_GETTER.image_exists(&path_unit) {
            IMAGE_GETTER.get_image(Some(&path_unit), None)
        } else {
            IMAGE_GETTER.get_image(Some(&self.base.path_icon_fallback), None)
        }
    }

    /// Gets the default image tint color
    pub fn get_default_image_tint(&self) -> Color {
        if let Some(promotion) = self.base.ruleset.unit_promotions.get(&self.base.image_name) {
            promotion.inner_color_object
        } else {
            Self::DEFAULT_INNER_COLOR
        }
    }

    /// Gets the default outer background tint color
    pub fn get_default_outer_background_tint(&self) -> Color {
        self.get_default_image_tint()
    }

    /// Gets the default inner background tint color
    pub fn get_default_inner_background_tint(&self) -> Color {
        if let Some(promotion) = self.base.ruleset.unit_promotions.get(&self.base.image_name) {
            promotion.outer_color_object
        } else {
            Self::DEFAULT_OUTER_COLOR
        }
    }

    /// Draws the promotion portrait
    pub fn draw(&self, ctx: &mut Context, param: DrawParam) -> GameResult {
        self.base.draw(ctx, param)
    }

    /// Gets the width of the portrait
    pub fn width(&self) -> f32 {
        self.base.width()
    }

    /// Gets the height of the portrait
    pub fn height(&self) -> f32 {
        self.base.height()
    }

    /// Sets the position of the portrait
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.base.set_position(x, y);
    }

    /// Sets the size of the portrait
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.base.set_size(width, height);
    }
}

// Helper trait for downcasting
trait Downcast {
    fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static;
}

impl Downcast for Box<dyn std::any::Any> {
    fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static,
    {
        self.as_mut().downcast_mut::<T>()
    }
}

// Helper extension for Color
trait ColorExt {
    fn darken(&self, factor: f32) -> Color;
}

impl ColorExt for Color {
    fn darken(&self, factor: f32) -> Color {
        Color {
            r: self.r * (1.0 - factor),
            g: self.g * (1.0 - factor),
            b: self.b * (1.0 - factor),
            a: self.a,
        }
    }
}
