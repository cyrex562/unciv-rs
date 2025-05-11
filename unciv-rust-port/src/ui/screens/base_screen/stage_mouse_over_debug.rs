use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use std::collections::HashSet;
use std::fmt::Write;

use crate::ui::components::fonts::Fonts;
use crate::ui::components::images::ImageGetter;
use crate::ui::screens::base_screen::BaseScreen;

/// A debug helper drawing mouse-over info and world coordinate axes onto a Stage.
///
/// Usage: save an instance, and in your `Stage.draw` override, call `draw` (your_stage) *after* `super.draw()`.
///
/// Implementation notes:
/// * Uses the stage's batch, but its own shape renderer
/// * Tries to avoid any memory allocation in `draw`, hence for building nice Actor names,
///     the reusable StringBuilder is filled using those lambdas, and those are built trying
///     to use as few closures as possible.
pub struct StageMouseOverDebug {
    label: egui::Label,
    mouse_coords: Vec2,
    shape_renderer: Option<egui::ShapeRenderer>,
    axis_color: Color,
    sb: String,
}

impl StageMouseOverDebug {
    const PADDING: f32 = 3.0;
    const OVERLAY_ALPHA: f32 = 0.8;
    const AXIS_INTERVAL: i32 = 20;
    const AXIS_TICK_LENGTH: f32 = 6.0;
    const AXIS_TICK_WIDTH: f32 = 1.5;
    const MAX_CHILD_SCAN: usize = 10;
    const MAX_TEXT_LENGTH: usize = 20;

    pub fn new() -> Self {
        let mut label = egui::Label::new("");
        label.set_style(egui::LabelStyle {
            font: Fonts::default(),
            color: Color::GOLDENROD,
            background: Some(ImageGetter::get_white_dot_drawable()
                .tint(Color::DARK_GRAY)
                .with_padding(
                    Self::PADDING,
                    Self::PADDING,
                    Self::PADDING,
                    Self::PADDING,
                )),
            alignment: egui::Align::Center,
        });

        Self {
            label,
            mouse_coords: Vec2::ZERO,
            shape_renderer: None,
            axis_color: Color::RED.with_a(Self::OVERLAY_ALPHA),
            sb: String::with_capacity(160),
        }
    }

    pub fn draw(&mut self, ctx: &mut EguiContexts, screen: &dyn BaseScreen) {
        // Get mouse coordinates
        let mouse_pos = ctx.ctx_mut().pointer_hover_pos().unwrap_or_default();
        self.mouse_coords = Vec2::new(mouse_pos.x, mouse_pos.y);

        // Convert screen coordinates to stage coordinates
        let stage_coords = screen.screen_to_stage_coordinates(self.mouse_coords);

        // Clear string builder
        self.sb.clear();

        // Add mouse coordinates and FPS
        write!(
            self.sb,
            "{} / {} ({})\n",
            stage_coords.x as i32,
            stage_coords.y as i32,
            ctx.ctx_mut().fps()
        ).unwrap();

        // Add actor label
        if let Some(actor) = screen.hit(stage_coords.x, stage_coords.y, false) {
            self.add_actor_label(actor);
        }

        // Update label text
        self.label.set_text(&self.sb);

        // Layout label
        self.layout_label(ctx, screen);

        // Draw label
        egui::Window::new("Debug Info")
            .fixed_pos([screen.dimensions().x - self.label.min_size().x, 0.0])
            .show(ctx.ctx_mut(), |ui| {
                ui.add(self.label.clone());
            });

        // Draw axes
        self.draw_axes(ctx, screen);
    }

    fn add_actor_label(&mut self, actor: &dyn Actor) {
        // For this actor, see if it has a descriptive name
        let actor_builder = self.get_actor_descriptive_name(actor);
        let mut parent_builder = None;
        let mut child_builder = None;

        // If there's no descriptive name for this actor, look for parent or children
        if actor_builder.is_none() {
            // Try to get a descriptive name from parent
            if let Some(parent) = actor.parent() {
                parent_builder = self.get_actor_descriptive_name(parent);
            }

            // If that failed, try to get a descriptive name from first few children
            if parent_builder.is_none() && actor.is_group() {
                child_builder = actor.children()
                    .take(Self::MAX_CHILD_SCAN)
                    .filter_map(|child| self.get_actor_descriptive_name(child))
                    .next();
            }
        }

        // Assemble name parts with fallback to plain class names for parent and actor
        if let Some(parent_builder) = parent_builder {
            parent_builder(&mut self.sb);
            self.sb.push('.');
        } else if let Some(parent) = actor.parent() {
            self.sb.push_str(&parent.type_name());
            self.sb.push('.');
        }

        if let Some(actor_builder) = actor_builder {
            actor_builder(&mut self.sb);
        } else {
            self.sb.push_str(&actor.type_name());
        }

        if let Some(child_builder) = child_builder {
            self.sb.push('(');
            child_builder(&mut self.sb);
            self.sb.push(')');
        }
    }

    fn get_actor_descriptive_name(&self, actor: &dyn Actor) -> Option<Box<dyn Fn(&mut String)>> {
        if let Some(name) = actor.name() {
            let class_name = actor.type_name();
            if name.starts_with(&class_name) {
                return Some(Box::new(move |sb| sb.push_str(name)));
            }
            return Some(Box::new(move |sb| {
                sb.push_str(&class_name);
                sb.push(':');
                sb.push_str(name);
            }));
        }

        if let Some(text) = actor.text() {
            if !text.is_empty() {
                return Some(Box::new(move |sb| {
                    sb.push_str("Label\"");
                    self.append_limited(sb, text);
                    sb.push('\"');
                }));
            }
        }

        if let Some(text) = actor.button_text() {
            if !text.is_empty() {
                return Some(Box::new(move |sb| {
                    sb.push_str("TextButton\"");
                    self.append_limited(sb, text);
                    sb.push('\"');
                }));
            }
        }

        None
    }

    fn append_limited(&self, sb: &mut String, text: &str) {
        let lf = text.find('\n').map_or(0, |i| i + 1);
        let len = (if lf == 0 { text.len() } else { lf }).min(Self::MAX_TEXT_LENGTH);

        if len == text.len() {
            sb.push_str(text);
            return;
        }

        sb.push_str(&text[..len]);
        sb.push('‥'); // '…' is taken
    }

    fn layout_label(&mut self, ctx: &mut EguiContexts, screen: &dyn BaseScreen) {
        if !self.label.needs_layout() {
            return;
        }

        let width = self.label.min_size().x + 2.0 * Self::PADDING;
        let height = self.label.min_size().y + 2.0 * Self::PADDING;

        self.label.set_size(Vec2::new(width, height));
        self.label.set_position(Vec2::new(screen.dimensions().x - width, 0.0));
    }

    fn draw_axes(&mut self, ctx: &mut EguiContexts, screen: &dyn BaseScreen) {
        let dims = screen.dimensions();

        // Initialize shape renderer if needed
        if self.shape_renderer.is_none() {
            self.shape_renderer = Some(egui::ShapeRenderer::new());
        }

        let sr = self.shape_renderer.as_mut().unwrap();

        // Enable blending
        ctx.ctx_mut().set_blend_mode(egui::BlendMode::ALPHA);

        // Set projection matrix
        sr.set_projection_matrix(screen.camera().combined);

        // Begin drawing
        sr.begin(egui::ShapeType::Filled);

        // Draw X-axis ticks
        for x in (0..dims.x as i32).step_by(Self::AXIS_INTERVAL) {
            let xf = x as f32;
            sr.rect_line(
                Vec2::new(xf, 0.0),
                Vec2::new(xf, Self::AXIS_TICK_LENGTH),
                Self::AXIS_TICK_WIDTH,
                self.axis_color,
                self.axis_color,
            );
        }

        // Draw Y-axis ticks
        let x2 = dims.x;
        let x1 = x2 - Self::AXIS_TICK_LENGTH;
        for y in (0..dims.y as i32).step_by(Self::AXIS_INTERVAL) {
            let yf = y as f32;
            sr.rect_line(
                Vec2::new(x1, yf),
                Vec2::new(x2, yf),
                Self::AXIS_TICK_WIDTH,
                self.axis_color,
                self.axis_color,
            );
        }

        // End drawing
        sr.end();

        // Disable blending
        ctx.ctx_mut().set_blend_mode(egui::BlendMode::NONE);
    }
}

/// Trait for actors in the UI
pub trait Actor: Send + Sync {
    fn name(&self) -> Option<&str>;
    fn type_name(&self) -> String;
    fn parent(&self) -> Option<&dyn Actor>;
    fn children(&self) -> Box<dyn Iterator<Item = &dyn Actor> + '_>;
    fn is_group(&self) -> bool;
    fn text(&self) -> Option<&str>;
    fn button_text(&self) -> Option<&str>;
}

/// Extension trait for BaseScreen to add debug functionality
pub trait BaseScreenDebug: BaseScreen {
    fn screen_to_stage_coordinates(&self, screen_coords: Vec2) -> Vec2;
    fn hit(&self, x: f32, y: f32, touchable: bool) -> Option<&dyn Actor>;
    fn camera(&self) -> &Camera;
}

/// Extension trait for egui::Label
pub trait LabelExt {
    fn set_style(&mut self, style: egui::LabelStyle);
    fn set_text(&mut self, text: &str);
    fn set_size(&mut self, size: Vec2);
    fn set_position(&mut self, position: Vec2);
    fn min_size(&self) -> Vec2;
    fn needs_layout(&self) -> bool;
}

impl LabelExt for egui::Label {
    fn set_style(&mut self, style: egui::LabelStyle) {
        // Implementation depends on bevy_egui's Label API
    }

    fn set_text(&mut self, text: &str) {
        // Implementation depends on bevy_egui's Label API
    }

    fn set_size(&mut self, size: Vec2) {
        // Implementation depends on bevy_egui's Label API
    }

    fn set_position(&mut self, position: Vec2) {
        // Implementation depends on bevy_egui's Label API
    }

    fn min_size(&self) -> Vec2 {
        // Implementation depends on bevy_egui's Label API
        Vec2::new(100.0, 20.0) // Placeholder
    }

    fn needs_layout(&self) -> bool {
        // Implementation depends on bevy_egui's Label API
        true // Placeholder
    }
}