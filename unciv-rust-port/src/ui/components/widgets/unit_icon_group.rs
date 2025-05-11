use std::sync::Arc;

use ggez::graphics::{Color, DrawParam, Drawable, Mesh, MeshBuilder, Rect, Text};
use ggez::mint::Point2;
use ggez::Context;

use crate::ui::components::non_transform_group::NonTransformGroup;
use crate::ui::components::widgets::image::Image;
use crate::ui::components::widgets::widget::Widget;
use crate::ui::components::widgets::widget_group::WidgetGroup;
use crate::ui::components::extensions::add_to_center;
use crate::ui::components::extensions::center_x;
use crate::ui::components::extensions::color_from_rgb;
use crate::ui::components::extensions::set_size;
use crate::ui::components::extensions::surround_with_circle;
use crate::ui::components::extensions::surround_with_thin_circle;
use crate::ui::images::ImageGetter;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::basescreen::UncivGame;
use crate::ui::screens::basescreen::GUI;
use crate::logic::map::mapunit::MapUnit;
use crate::logic::map::tile::Tile;

/// A background for unit flags with inner and outer colors
struct FlagBackground {
    /// The base image
    image: Image,

    /// The inner drawable
    drawable_inner: Option<Image>,

    /// The inner color
    inner_color: Color,

    /// The outer color
    outer_color: Color,

    /// The outline color
    outline_color: Color,

    /// Whether to draw the outline
    draw_outline: bool,

    /// The inner multiplier
    inner_multiplier: f32,

    /// The outline multiplier
    outline_multiplier: f32,

    /// The inner width
    inner_width: f32,

    /// The inner height
    inner_height: f32,

    /// The inner offset X
    inner_offset_x: f32,

    /// The inner offset Y
    inner_offset_y: f32,

    /// The outline width
    outline_width: f32,

    /// The outline height
    outline_height: f32,

    /// The outline offset X
    outline_offset_x: f32,

    /// The outline offset Y
    outline_offset_y: f32,
}

impl FlagBackground {
    /// Creates a new FlagBackground
    fn new(drawable: Image, size: f32) -> Self {
        let ratio = drawable.height() / drawable.width();
        let width = size;
        let height = size * ratio;

        let inner_multiplier = 0.88;
        let outline_multiplier = 1.08;

        let inner_width = width * inner_multiplier;
        let inner_height = height * inner_multiplier;
        let inner_offset_x = (width - inner_width) / 2.0;
        let inner_offset_y = (height - inner_height) / 2.0;

        let outline_width = width * outline_multiplier;
        let outline_height = height * outline_multiplier;
        let outline_offset_x = (outline_width - width) / 2.0;
        let outline_offset_y = (outline_height - height) / 2.0;

        Self {
            image: drawable,
            drawable_inner: None,
            inner_color: Color::WHITE,
            outer_color: Color::RED,
            outline_color: Color::WHITE,
            draw_outline: false,
            inner_multiplier,
            outline_multiplier,
            inner_width,
            inner_height,
            inner_offset_x,
            inner_offset_y,
            outline_width,
            outline_height,
            outline_offset_x,
            outline_offset_y,
        }
    }

    /// Gets the drawable
    fn drawable(&self) -> &Image {
        &self.image
    }

    /// Draws the flag background
    fn draw(&self, ctx: &mut Context, parent_alpha: f32) -> ggez::GameResult {
        let alpha = self.image.color().a * parent_alpha;

        if self.draw_outline {
            let mut outline_color = self.outline_color;
            outline_color.a *= alpha;

            let outline_mesh = MeshBuilder::new()
                .rectangle(
                    ggez::graphics::DrawMode::fill(),
                    Rect::new(
                        self.image.x() - self.outline_offset_x,
                        self.image.y() - self.outline_offset_y,
                        self.outline_width,
                        self.outline_height
                    ),
                    outline_color
                )
                .build(ctx)?;

            outline_mesh.draw(ctx, DrawParam::default())?;
        }

        let mut outer_color = self.outer_color;
        outer_color.a *= alpha;

        let outer_mesh = MeshBuilder::new()
            .rectangle(
                ggez::graphics::DrawMode::fill(),
                Rect::new(
                    self.image.x(),
                    self.image.y(),
                    self.image.width(),
                    self.image.height()
                ),
                outer_color
            )
            .build(ctx)?;

        outer_mesh.draw(ctx, DrawParam::default())?;

        let mut inner_color = self.inner_color;
        inner_color.a *= alpha;

        if let Some(drawable_inner) = &self.drawable_inner {
            let inner_mesh = MeshBuilder::new()
                .rectangle(
                    ggez::graphics::DrawMode::fill(),
                    Rect::new(
                        self.image.x(),
                        self.image.y(),
                        self.image.width(),
                        self.image.height()
                    ),
                    inner_color
                )
                .build(ctx)?;

            inner_mesh.draw(ctx, DrawParam::default())?;
        } else {
            let inner_mesh = MeshBuilder::new()
                .rectangle(
                    ggez::graphics::DrawMode::fill(),
                    Rect::new(
                        self.image.x() + self.inner_offset_x,
                        self.image.y() + self.inner_offset_y,
                        self.inner_width,
                        self.inner_height
                    ),
                    inner_color
                )
                .build(ctx)?;

            inner_mesh.draw(ctx, DrawParam::default())?;
        }

        Ok(())
    }
}

/// Displays the unit's icon and action
pub struct UnitIconGroup {
    /// The base NonTransformGroup that this UnitIconGroup extends
    base: NonTransformGroup,

    /// The unit
    unit: Arc<MapUnit>,

    /// The size
    size: f32,

    /// The action group
    action_group: Option<Box<dyn WidgetGroup>>,

    /// The flag icon
    flag_icon: Image,

    /// The flag background
    flag_bg: FlagBackground,

    /// The flag selection
    flag_selection: Image,

    /// The flag mask
    flag_mask: Option<Image>,
}

impl UnitIconGroup {
    /// Creates a new UnitIconGroup
    pub fn new(unit: Arc<MapUnit>, size: f32) -> Self {
        let base_screen = BaseScreen::get_instance();
        let skin = base_screen.skin();

        let flag_icon = ImageGetter::get_unit_icon(unit.base_unit(), unit.civ().nation().get_inner_color());
        let flag_bg = FlagBackground::new(self.get_background_drawable_for_unit(), size);
        let flag_selection = self.get_background_selection_for_unit();
        let flag_mask = self.get_background_mask_for_unit();

        let mut icon_group = Self {
            base: NonTransformGroup::new(),
            unit: unit.clone(),
            size,
            action_group: None,
            flag_icon: flag_icon.clone(),
            flag_bg,
            flag_selection: flag_selection.clone(),
            flag_mask,
        };

        // Set opacity
        icon_group.base.set_color(Color::new(1.0, 1.0, 1.0, UncivGame::current().settings().unit_icon_opacity()));

        let size_selection_x = size * 1.6;
        let size_selection_y = size_selection_x * flag_selection.height() / flag_selection.width();

        icon_group.base.set_size(icon_group.flag_bg.image.width(), icon_group.flag_bg.image.height());

        icon_group.flag_selection.set_color(Color::new(1.0, 1.0, 0.9, 0.0));
        icon_group.flag_selection.set_alignment(ggez::graphics::Align::Center);
        icon_group.flag_selection.set_size(size_selection_x, size_selection_y);

        icon_group.flag_bg.inner_color = unit.civ().nation().get_outer_color();
        icon_group.flag_bg.outer_color = unit.civ().nation().get_inner_color();
        icon_group.flag_bg.outline_color = icon_group.flag_bg.inner_color;
        icon_group.flag_bg.drawable_inner = self.get_background_inner_drawable_for_unit();

        if let Some(flag_mask) = &mut icon_group.flag_mask {
            flag_mask.set_size(size * 0.88, size * 0.88 * flag_mask.height() / flag_mask.width());
        }

        let flag_icon_size_multiplier = if unit.is_civilian() { 0.5 } else { 0.65 };
        icon_group.flag_icon.set_size(size * flag_icon_size_multiplier);

        icon_group.base.add_to_center(icon_group.flag_selection.clone());
        icon_group.base.add_to_center(icon_group.flag_bg.image.clone());

        if let Some(flag_mask) = &icon_group.flag_mask {
            icon_group.base.add_to_center(flag_mask.clone());
        }

        icon_group.base.add_to_center(icon_group.flag_icon.clone());

        let action_image = self.get_action_image(&unit);
        if let Some(action_image) = action_image {
            let action_group = action_image
                .surround_with_circle(size / 2.0 * 0.9)
                .surround_with_thin_circle();

            action_group.set_position(size / 2.0, 0.0);
            icon_group.base.add_actor(action_group.clone());
            icon_group.action_group = Some(action_group);
        }

        if unit.health() < 100 {
            // Add health bar
            let hp = ImageGetter::get_health_bar(unit.health() as f32, 100.0, size * 0.78);
            icon_group.base.add_actor(hp.clone());
            hp.center_x(&icon_group.base);
        }

        icon_group
    }

    /// Gets the background drawable for the unit
    fn get_background_drawable_for_unit(&self) -> Image {
        if self.unit.is_embarked() {
            ImageGetter::get_drawable("UnitFlagIcons/UnitFlagEmbark")
        } else if self.unit.is_fortified() {
            ImageGetter::get_drawable("UnitFlagIcons/UnitFlagFortify")
        } else if self.unit.is_guarding() {
            ImageGetter::get_drawable("UnitFlagIcons/UnitFlagFortify")
        } else if self.unit.is_civilian() {
            ImageGetter::get_drawable("UnitFlagIcons/UnitFlagCivilian")
        } else {
            ImageGetter::get_drawable("UnitFlagIcons/UnitFlag")
        }
    }

    /// Gets the background inner drawable for the unit
    fn get_background_inner_drawable_for_unit(&self) -> Option<Image> {
        if self.unit.is_embarked() {
            ImageGetter::get_drawable_or_null("UnitFlagIcons/UnitFlagEmbarkInner")
        } else if self.unit.is_fortified() {
            ImageGetter::get_drawable_or_null("UnitFlagIcons/UnitFlagFortifyInner")
        } else if self.unit.is_guarding() {
            ImageGetter::get_drawable_or_null("UnitFlagIcons/UnitFlagFortifyInner")
        } else if self.unit.is_civilian() {
            ImageGetter::get_drawable_or_null("UnitFlagIcons/UnitFlagCivilianInner")
        } else {
            ImageGetter::get_drawable_or_null("UnitFlagIcons/UnitFlagInner")
        }
    }

    /// Gets the background mask for the unit
    fn get_background_mask_for_unit(&self) -> Option<Image> {
        let filename = if self.unit.is_embarked() {
            "UnitFlagIcons/UnitFlagMaskEmbark"
        } else if self.unit.is_fortified() {
            "UnitFlagIcons/UnitFlagMaskFortify"
        } else if self.unit.is_guarding() {
            "UnitFlagIcons/UnitFlagMaskFortify"
        } else if self.unit.is_civilian() {
            "UnitFlagIcons/UnitFlagMaskCivilian"
        } else {
            "UnitFlagIcons/UnitFlagMask"
        };

        if ImageGetter::image_exists(filename) {
            Some(ImageGetter::get_image(filename))
        } else {
            None
        }
    }

    /// Gets the background selection for the unit
    fn get_background_selection_for_unit(&self) -> Image {
        if self.unit.is_embarked() {
            ImageGetter::get_image("UnitFlagIcons/UnitFlagSelectionEmbark")
        } else if self.unit.is_fortified() {
            ImageGetter::get_image("UnitFlagIcons/UnitFlagSelectionFortify")
        } else if self.unit.is_guarding() {
            ImageGetter::get_image("UnitFlagIcons/UnitFlagSelectionFortify")
        } else if self.unit.is_civilian() {
            ImageGetter::get_image("UnitFlagIcons/UnitFlagSelectionCivilian")
        } else {
            ImageGetter::get_image("UnitFlagIcons/UnitFlagSelection")
        }
    }

    /// Gets the action image for the unit
    fn get_action_image(&self, unit: &Arc<MapUnit>) -> Option<Image> {
        if unit.is_sleeping() {
            Some(ImageGetter::get_image("UnitActionIcons/Sleep"))
        } else if unit.get_tile().improvement_in_progress().is_some() && unit.can_build_improvement(unit.get_tile().get_tile_improvement_in_progress().unwrap()) {
            Some(ImageGetter::get_image(&format!("ImprovementIcons/{}", unit.get_tile().improvement_in_progress().unwrap())))
        } else if unit.is_escorting() {
            Some(ImageGetter::get_image("UnitActionIcons/Escort"))
        } else if unit.is_moving() {
            Some(ImageGetter::get_image("UnitActionIcons/MoveTo"))
        } else if unit.is_exploring() {
            Some(ImageGetter::get_image("UnitActionIcons/Explore"))
        } else if unit.is_automated() {
            Some(ImageGetter::get_image("UnitActionIcons/Automate"))
        } else if unit.is_set_up_for_siege() {
            Some(ImageGetter::get_image("UnitActionIcons/SetUp"))
        } else {
            None
        }
    }

    /// Highlights the unit in red
    pub fn highlight_red(&mut self) {
        self.flag_selection.set_color(color_from_rgb(230, 0, 0));
        self.flag_bg.draw_outline = true;
    }

    /// Selects the unit
    pub fn select_unit(&mut self) {
        let opacity = 1.0;

        self.base.set_color(Color::new(1.0, 1.0, 1.0, opacity));

        // If unit is idle, leave actionGroup at 50% opacity when selected
        if self.unit.is_idle() {
            if let Some(action_group) = &mut self.action_group {
                action_group.set_color(Color::new(1.0, 1.0, 1.0, opacity * 0.5));
            }
        } else {
            // Else set to 100% opacity when selected
            if let Some(action_group) = &mut self.action_group {
                action_group.set_color(Color::new(1.0, 1.0, 1.0, opacity));
            }
        }

        // Unit base icon is faded out only if out of moves
        // Foreign unit icons are never faded!
        let should_be_faded = (self.unit.owner() == GUI::get_selected_player().civ_name()
                && !self.unit.has_movement() && GUI::get_settings().unit_icon_opacity() == 1.0);

        let alpha = if should_be_faded { opacity * 0.5 } else { opacity };

        self.flag_icon.set_color(Color::new(1.0, 1.0, 1.0, alpha));
        self.flag_bg.image.set_color(Color::new(1.0, 1.0, 1.0, alpha));
        self.flag_selection.set_color(Color::new(1.0, 1.0, 1.0, opacity));

        if GUI::get_settings().continuous_rendering() {
            self.flag_selection.set_color(Color::new(1.0, 1.0, 1.0, opacity));

            // Add pulsing animation
            self.flag_selection.add_action(
                ggez::graphics::Action::repeat(
                    ggez::graphics::RepeatAction::Forever,
                    ggez::graphics::Action::sequence(
                        ggez::graphics::Action::alpha(opacity * 0.7, 1.0),
                        ggez::graphics::Action::alpha(opacity, 1.0)
                    )
                )
            );
        } else {
            self.flag_selection.set_color(Color::new(1.0, 1.0, 1.0, opacity * 0.8));
        }
    }
}

// Implement the necessary traits for UnitIconGroup
impl std::ops::Deref for UnitIconGroup {
    type Target = NonTransformGroup;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for UnitIconGroup {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Clone for UnitIconGroup {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            unit: self.unit.clone(),
            size: self.size,
            action_group: self.action_group.clone(),
            flag_icon: self.flag_icon.clone(),
            flag_bg: FlagBackground {
                image: self.flag_bg.image.clone(),
                drawable_inner: self.flag_bg.drawable_inner.clone(),
                inner_color: self.flag_bg.inner_color,
                outer_color: self.flag_bg.outer_color,
                outline_color: self.flag_bg.outline_color,
                draw_outline: self.flag_bg.draw_outline,
                inner_multiplier: self.flag_bg.inner_multiplier,
                outline_multiplier: self.flag_bg.outline_multiplier,
                inner_width: self.flag_bg.inner_width,
                inner_height: self.flag_bg.inner_height,
                inner_offset_x: self.flag_bg.inner_offset_x,
                inner_offset_y: self.flag_bg.inner_offset_y,
                outline_width: self.flag_bg.outline_width,
                outline_height: self.flag_bg.outline_height,
                outline_offset_x: self.flag_bg.outline_offset_x,
                outline_offset_y: self.flag_bg.outline_offset_y,
            },
            flag_selection: self.flag_selection.clone(),
            flag_mask: self.flag_mask.clone(),
        }
    }
}