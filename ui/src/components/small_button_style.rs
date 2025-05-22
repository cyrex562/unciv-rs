use ggez::graphics::{self, Color, DrawParam, Drawable, Mesh, Rect, Text};
use ggez::{Context, GameResult};
use crate::ui::screens::base_screen::BaseScreen;
use crate::ui::images::ImageGetter;

/// A custom button style that reduces the height of buttons
///
/// This style modifies the NinePatch geometry so the roundedEdgeRectangleMidShape button
/// is 38 pixels high instead of 48 pixels, making it smaller than the default style.
pub struct SmallButtonStyle {
    /// The base style inherited from TextButtonStyle
    base_style: graphics::TextStyle,

    /// The button background for the normal state
    up: Mesh,

    /// The button background for the pressed state
    down: Mesh,

    /// The button background for the hover state
    over: Mesh,

    /// The button background for the disabled state
    disabled: Mesh,

    /// The color used for disabled text
    disabled_font_color: Color,
}

impl SmallButtonStyle {
    /// Creates a new SmallButtonStyle based on the base screen's skin
    pub fn new(base_screen: &BaseScreen) -> Self {
        // Get colors from the base screen's skin
        let up_color = base_screen.skin.get_color("color");
        let down_color = base_screen.skin.get_color("pressed");
        let over_color = base_screen.skin.get_color("highlight");
        let disabled_color = base_screen.skin.get_color("disabled");

        // Get the base text style
        let base_style = base_screen.skin.get_text_style();

        // Get the button shape
        let shape = Self::get_button_shape(base_screen);

        // Create the button meshes with different tints
        let up = Self::tint_mesh(&shape, up_color);
        let down = Self::tint_mesh(&shape, down_color);
        let over = Self::tint_mesh(&shape, over_color);
        let disabled = Self::tint_mesh(&shape, disabled_color);

        Self {
            base_style,
            up,
            down,
            over,
            disabled,
            disabled_font_color: Color::GRAY,
        }
    }

    /// Gets the button shape, reducing its height if needed
    fn get_button_shape(base_screen: &BaseScreen) -> Mesh {
        // Get the skinned background
        let skinned = base_screen.skin.get_ui_background("AnimatedMenu/Button", "roundedEdgeRectangleMidShape");

        // Get the default background
        let default = ImageGetter::get_nine_patch("roundedEdgeRectangleMidShape");

        // If the skinned background is the same as the default, reduce its height
        if skinned == default {
            Self::reduce_nine_patch_height(&default)
        } else {
            skinned
        }
    }

    /// Reduces the height of a NinePatch by modifying its padding and height
    fn reduce_nine_patch_height(nine_patch: &Mesh) -> Mesh {
        // Create a new mesh with reduced height
        let mut reduced = nine_patch.clone();

        // Modify the mesh to reduce its height
        // In ggez, we need to adjust the vertices directly
        // This is a simplified approach - in a real implementation,
        // you would need to modify the actual vertices of the mesh
        reduced.scale(1.0, 0.792, 1.0); // 38/48 ≈ 0.792

        reduced
    }

    /// Applies a tint color to a mesh
    fn tint_mesh(mesh: &Mesh, color: Color) -> Mesh {
        let mut tinted = mesh.clone();
        tinted.set_color(color);
        tinted
    }

    /// Draws the button in its current state
    pub fn draw(&self, ctx: &mut Context, state: ButtonState, bounds: Rect, text: &str) -> GameResult {
        // Select the appropriate mesh based on the button state
        let mesh = match state {
            ButtonState::Normal => &self.up,
            ButtonState::Pressed => &self.down,
            ButtonState::Hover => &self.over,
            ButtonState::Disabled => &self.disabled,
        };

        // Draw the button background
        graphics::draw(ctx, mesh, DrawParam::default().dest(bounds))?;

        // Draw the text
        let text_color = if state == ButtonState::Disabled {
            self.disabled_font_color
        } else {
            Color::WHITE
        };

        let text = Text::new(text)
            .with_style(self.base_style.clone())
            .with_color(text_color);

        // Center the text in the button
        let text_bounds = text.measure(ctx)?;
        let text_x = bounds.x + (bounds.w - text_bounds.w) / 2.0;
        let text_y = bounds.y + (bounds.h - text_bounds.h) / 2.0;

        graphics::draw(
            ctx,
            &text,
            DrawParam::default().dest([text_x, text_y])
        )?;

        Ok(())
    }
}

/// Represents the different states a button can be in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    /// Normal state
    Normal,
    /// Button is being pressed
    Pressed,
    /// Mouse is hovering over the button
    Hover,
    /// Button is disabled
    Disabled,
}