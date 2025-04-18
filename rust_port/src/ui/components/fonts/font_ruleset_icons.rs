use std::collections::HashMap;
use std::f32;
use gdx::graphics::{Pixmap, GL20};
use gdx::graphics::g2d::SpriteBatch;
use gdx::graphics::glutils::FrameBuffer;
use gdx::math::Matrix4;
use gdx::scenes::scene2d::{Actor, Group};
use crate::models::ruleset::Ruleset;
use crate::models::tilesets::TileSetCache;
use crate::ui::components::extensions::{center, set_size};
use crate::ui::components::fonts::fonts::ORIGINAL_FONT_SIZE;
use crate::ui::components::tilegroups::TileSetStrings;
use crate::ui::images::{ImageGetter, Portrait};
use crate::ui::screens::civilopediascreen::CivilopediaImageGetters;
use crate::UncivGame;

/// Map all or most Ruleset icons as Actors to unused Char codepoints,
/// `NativeBitmapFontData.get_glyph` can then paint them onto a PixMap,
/// on demand, by calling `get_pixmap_from_actor`.
pub struct FontRulesetIcons {
    /// Maps ruleset object names to character codes
    pub ruleset_object_name_to_char: HashMap<String, char>,
    /// Maps character codes to ruleset image actors
    pub char_to_ruleset_image_actor: HashMap<char, Box<dyn Actor>>,
    /// Next unused character number to assign
    next_unused_character_number: u32,
    /// Frame buffer for rendering
    frame_buffer: Option<FrameBuffer>,
    /// Sprite batch for rendering
    sprite_batch: Option<SpriteBatch>,
    /// Transform matrix for rendering
    transform: Matrix4,
}

impl FontRulesetIcons {
    /// Creates a new FontRulesetIcons instance
    pub fn new() -> Self {
        Self {
            ruleset_object_name_to_char: HashMap::new(),
            char_to_ruleset_image_actor: HashMap::new(),
            next_unused_character_number: Self::UNUSED_CHARACTER_CODES_START,
            frame_buffer: None,
            sprite_batch: None,
            transform: Matrix4::new(),
        }
    }

    /// See https://en.wikipedia.org/wiki/Private_Use_Areas
    /// char encodings 57344 to 63743 (U+E000-U+F8FF) are not assigned
    pub const UNUSED_CHARACTER_CODES_START: u32 = 57344;
    const UNUSED_CHARACTER_CODES_END: u32 = 63743;

    /// Adds ruleset images to the font
    pub fn add_ruleset_images(&mut self, ruleset: &Ruleset) {
        self.ruleset_object_name_to_char.clear();
        self.char_to_ruleset_image_actor.clear();
        self.next_unused_character_number = Self::UNUSED_CHARACTER_CODES_START;

        // Helper function to add a character mapping
        let add_char = |object_name: String, object_actor: Box<dyn Actor>| {
            if self.next_unused_character_number > Self::UNUSED_CHARACTER_CODES_END {
                return;
            }
            let char_code = char::from_u32(self.next_unused_character_number).unwrap_or('\u{FFFD}');
            self.next_unused_character_number += 1;
            self.ruleset_object_name_to_char.insert(object_name, char_code);
            self.char_to_ruleset_image_actor.insert(char_code, object_actor);
        };

        // Note: If an image is missing, these will insert a white square in the font - acceptable in
        // most cases as these white squares will be visible elsewhere anyway. "Policy branch Complete"
        // is an exception, and therefore gets an existence test.

        // Add resource icons
        for resource_name in ruleset.tile_resources.keys() {
            add_char(
                resource_name.clone(),
                ImageGetter::get_resource_portrait(resource_name, ORIGINAL_FONT_SIZE),
            );
        }

        // Add building icons
        for building_name in ruleset.buildings.keys() {
            add_char(
                building_name.clone(),
                ImageGetter::get_construction_portrait(building_name, ORIGINAL_FONT_SIZE),
            );
        }

        // Add unit icons
        for unit_name in ruleset.units.keys() {
            add_char(
                unit_name.clone(),
                ImageGetter::get_construction_portrait(unit_name, ORIGINAL_FONT_SIZE),
            );
        }

        // Add promotion icons
        for promotion_name in ruleset.unit_promotions.keys() {
            add_char(
                promotion_name.clone(),
                ImageGetter::get_promotion_portrait(promotion_name, ORIGINAL_FONT_SIZE),
            );
        }

        // Add improvement icons
        for improvement_name in ruleset.tile_improvements.keys() {
            add_char(
                improvement_name.clone(),
                ImageGetter::get_improvement_portrait(improvement_name, ORIGINAL_FONT_SIZE),
            );
        }

        // Add technology icons
        for tech_name in ruleset.technologies.keys() {
            add_char(
                tech_name.clone(),
                ImageGetter::get_tech_icon_portrait(tech_name, ORIGINAL_FONT_SIZE),
            );
        }

        // Add nation icons
        for nation in ruleset.nations.values() {
            add_char(
                nation.name.clone(),
                ImageGetter::get_nation_portrait(nation, ORIGINAL_FONT_SIZE),
            );
        }

        // Add policy icons
        for policy in ruleset.policies.values() {
            let file_location = if ruleset.policy_branches.contains_key(&policy.name) {
                format!("PolicyBranchIcons/{}", policy.name)
            } else {
                format!("PolicyIcons/{}", policy.name)
            };

            if !ImageGetter::image_exists(&file_location) {
                continue;
            }

            let mut image = ImageGetter::get_image(&file_location);
            set_size(&mut image, ORIGINAL_FONT_SIZE);
            add_char(policy.name.clone(), image);
        }

        // Upon *game initialization* we can get here without the tileset being loaded yet
        // in which case we can't add terrain icons
        if TileSetCache::contains_key(&UncivGame::current().settings.tile_set) {
            let tile_set_strings = TileSetStrings::new(ruleset, &UncivGame::current().settings);

            for terrain in ruleset.terrains.values() {
                // These ensure that the font icons are correctly sized - tilegroup rendering works differently than others, to account for clickability vs rendered areas
                let mut tile_group = CivilopediaImageGetters::terrain_image(terrain, ruleset, ORIGINAL_FONT_SIZE, &tile_set_strings);

                tile_group.width *= 1.5;
                tile_group.height *= 1.5;

                for layer in tile_group.children.iter_mut() {
                    center(layer, &tile_group);
                }

                add_char(terrain.name.clone(), Box::new(tile_group));
            }
        }
    }

    /// Gets the frame buffer for rendering
    fn get_frame_buffer(&mut self) -> &mut FrameBuffer {
        if self.frame_buffer.is_none() {
            // Size here is way too big, but it's hard to know in advance how big it needs to be.
            // Gdx world coords, not pixels.
            self.frame_buffer = Some(FrameBuffer::new(
                Pixmap::Format::RGBA8888,
                gdx::graphics::Gdx::graphics().width,
                gdx::graphics::Gdx::graphics().height,
                false,
            ));
        }
        self.frame_buffer.as_mut().unwrap()
    }

    /// Gets the sprite batch for rendering
    fn get_sprite_batch(&mut self) -> &mut SpriteBatch {
        if self.sprite_batch.is_none() {
            self.sprite_batch = Some(SpriteBatch::new());
        }
        self.sprite_batch.as_mut().unwrap()
    }

    /// Get a Pixmap for a "show ruleset icons as part of text" actor.
    ///
    /// Draws onto an offscreen frame buffer and copies the pixels.
    /// Caller becomes owner of the returned Pixmap and is responsible for disposing it.
    ///
    /// Size is such that the actor's height is mapped to the font's ascent (close to
    /// ORIGINAL_FONT_SIZE * GameSettings.fontSizeMultiplier), the actor is placed like a letter into
    /// the total height as given by the font's metrics, and width scaled to maintain aspect ratio.
    pub fn get_pixmap_from_actor(&mut self, actor: &dyn Actor) -> Pixmap {
        let (box_width, box_height) = self.scale_and_position_actor(actor);
        self.get_pixmap_from_actor_base(actor, box_width, box_height)
    }

    /// Also required for dynamically generating pixmaps for pixmappacker
    pub fn get_pixmap_from_actor_base(&mut self, actor: &dyn Actor, box_width: i32, box_height: i32) -> Pixmap {
        let pixmap = Pixmap::new(box_width, box_height, Pixmap::Format::RGBA8888);
        let frame_buffer = self.get_frame_buffer();
        frame_buffer.begin();

        gdx::graphics::Gdx::gl().gl_clear_color(0.0, 0.0, 0.0, 0.0);
        gdx::graphics::Gdx::gl().gl_clear(GL20::GL_COLOR_BUFFER_BIT);

        let sprite_batch = self.get_sprite_batch();
        sprite_batch.begin();
        actor.draw(sprite_batch, 1.0);
        sprite_batch.end();

        gdx::graphics::Gdx::gl().gl_read_pixels(
            0, 0, box_width, box_height,
            GL20::GL_RGBA, GL20::GL_UNSIGNED_BYTE,
            pixmap.pixels(),
        );

        frame_buffer.end();
        pixmap
    }

    /// Does the Actor scaling and positioning using metrics for `get_pixmap_from_actor`
    /// Returns (box_width, box_height)
    fn scale_and_position_actor(&mut self, actor: &dyn Actor) -> (i32, i32) {
        // We want our - mostly circular - icon to match a typical large uppercase letter in height
        // The drawing bounding box should have room, however for the font's leading and descent
        let metrics = crate::ui::components::fonts::fonts::FONTS.font_implementation.get_metrics();

        // Empiric slight size reduction - "correctly calculated" they just look a bit too big
        let scaled_actor_height = metrics.ascent * 0.93;
        let scaled_actor_width = actor.width() * (scaled_actor_height / actor.height());
        let box_height = f32::ceil(metrics.height) as i32;
        let box_width = f32::ceil(scaled_actor_width) as i32;

        // Nudge down by the border size if it's a Portrait having one, so the "core" sits on the baseline
        let border = if let Some(portrait) = actor.downcast_ref::<Portrait>() {
            portrait.border_size
        } else {
            0.0
        };

        // Scale to desired font dimensions - modifying the actor this way is OK as the decisions are
        // the same each repetition, and size in the Group case or aspect ratio otherwise is preserved
        if let Some(group) = actor.downcast_ref::<Group>() {
            // We can't just actor.set_size - a Group won't scale its children that way
            group.set_transform(true);
            let scale = scaled_actor_width / group.width();
            group.set_scale(scale, -scale);

            // Now the Actor is scaled, we need to position it at the baseline, Y from top of the box
            // The +1.0 is empirical because the result still looked off.
            group.set_position(0.0, metrics.leading + metrics.ascent + border * scale + 1.0);
        } else {
            // Assume it's an Image obeying Actor size, but needing explicit Y flipping
            // place actor from top of bounding box down
            // (don't think the Gdx (Y is upwards) way - due to the transformMatrix below)
            actor.set_position(0.0, metrics.leading + border);
            set_size(actor, scaled_actor_width);
            actor.set_size(scaled_actor_width, scaled_actor_height);

            self.transform.identity()
                .scale(1.0, -1.0, 1.0)
                .translate(0.0, box_height as f32, 0.0);

            self.get_sprite_batch().set_transform_matrix(self.transform);
            // (copies matrix, not a set-by-reference, ignored when actor isTransform is on)
        }

        (box_width, box_height)
    }
}

// Implement singleton pattern
lazy_static::lazy_static! {
    pub static ref FONT_RULESET_ICONS: std::sync::Mutex<FontRulesetIcons> = std::sync::Mutex::new(FontRulesetIcons::new());
}