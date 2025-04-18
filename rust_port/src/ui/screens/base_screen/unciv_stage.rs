use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use std::time::Duration;

use crate::game::UncivGame;
use crate::logic::event::{Event, EventBus};
use crate::ui::crashhandling::wrap_crash_handling;
use crate::ui::crashhandling::wrap_crash_handling_unit;
use crate::ui::screens::base_screen::stage_mouse_over_debug::StageMouseOverDebug;
use crate::utils::log::Log;

/// Main stage for the game. Catches all exceptions or errors thrown by event handlers,
/// calling [UncivGame::handle_uncaught_throwable] with the thrown exception or error.
pub struct UncivStage {
    /// The viewport for this stage
    viewport: egui::Viewport,

    /// The batch for rendering
    batch: egui::SpriteBatch,

    /// Whether to perform pointer enter/exit events
    ///
    /// Checking for the enter/exit bounds is a relatively expensive operation
    /// and may thus be disabled temporarily.
    perform_pointer_enter_exit_events: bool,

    /// The last known visible area of the stage
    last_known_visible_area: egui::Rect,

    /// The mouse over debug implementation
    mouse_over_debug_impl: Option<StageMouseOverDebug>,

    /// The event receiver
    events: EventBus::EventReceiver,

    /// The root actor
    root: egui::Group,

    /// The mouse over actor
    mouse_over_actor: Option<Box<dyn egui::Actor>>,

    /// The pointer over actors
    pointer_over_actors: Vec<Box<dyn egui::Actor>>,
}

impl UncivStage {
    /// Create a new UncivStage
    pub fn new(viewport: egui::Viewport) -> Self {
        let batch = Self::get_batch(1000);

        let mut stage = Self {
            viewport,
            batch,
            perform_pointer_enter_exit_events: true,
            last_known_visible_area: egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(viewport.width(), viewport.height()),
            ),
            mouse_over_debug_impl: None,
            events: EventBus::EventReceiver::new(),
            root: egui::Group::new(),
            mouse_over_actor: None,
            pointer_over_actors: Vec::new(),
        };

        // Initialize event handling
        stage.events.receive::<VisibleAreaChanged>(|event| {
            Log::debug("Visible stage area changed: {:?}", event.visible_area);
            stage.last_known_visible_area = event.visible_area;
        });

        stage
    }

    /// Get a new sprite batch
    pub fn get_batch(size: i32) -> egui::SpriteBatch {
        egui::SpriteBatch::new(size)
    }

    /// Get whether mouse over debug is enabled
    pub fn mouse_over_debug(&self) -> bool {
        self.mouse_over_debug_impl.is_some()
    }

    /// Set whether mouse over debug is enabled
    pub fn set_mouse_over_debug(&mut self, value: bool) {
        self.mouse_over_debug_impl = if value {
            Some(StageMouseOverDebug::new())
        } else {
            None
        };
    }

    /// Get the last known visible area
    pub fn last_known_visible_area(&self) -> egui::Rect {
        self.last_known_visible_area
    }

    /// Dispose of the stage
    pub fn dispose(&mut self) {
        self.events.stop_receiving();

        // Clear all references
        self.root.clear();

        // Clear mouse over properties
        self.mouse_over_actor = None;
        self.pointer_over_actors.clear();

        // Act once to update properties
        self.act();
    }

    /// Draw the stage
    pub fn draw(&mut self, ctx: &mut EguiContexts) {
        wrap_crash_handling_unit(|| {
            // Draw the root
            self.root.draw(ctx);

            // Draw mouse over debug if enabled
            if let Some(debug) = &mut self.mouse_over_debug_impl {
                debug.draw(ctx, self);
            }
        })();
    }

    /// Act the stage
    pub fn act(&mut self) {
        wrap_crash_handling_unit(|| {
            // We're replicating Stage.act, so this value is simply taken from there
            let delta = (1.0 / 30.0).min(ctx.delta_seconds());

            if self.perform_pointer_enter_exit_events {
                self.act_with_delta(delta);
            } else {
                self.root.act(delta);
            }
        })();
    }

    /// Act the stage with a delta time
    pub fn act_with_delta(&mut self, delta: f32) {
        wrap_crash_handling_unit(|| {
            self.root.act(delta);
        })();
    }

    /// Handle touch down
    pub fn touch_down(&mut self, screen_x: i32, screen_y: i32, pointer: i32, button: i32) -> bool {
        wrap_crash_handling(|| {
            self.root.touch_down(screen_x, screen_y, pointer, button)
        })().unwrap_or(true)
    }

    /// Handle touch dragged
    pub fn touch_dragged(&mut self, screen_x: i32, screen_y: i32, pointer: i32) -> bool {
        wrap_crash_handling(|| {
            self.root.touch_dragged(screen_x, screen_y, pointer)
        })().unwrap_or(true)
    }

    /// Handle touch up
    pub fn touch_up(&mut self, screen_x: i32, screen_y: i32, pointer: i32, button: i32) -> bool {
        wrap_crash_handling(|| {
            self.root.touch_up(screen_x, screen_y, pointer, button)
        })().unwrap_or(true)
    }

    /// Handle mouse moved
    pub fn mouse_moved(&mut self, screen_x: i32, screen_y: i32) -> bool {
        wrap_crash_handling(|| {
            self.root.mouse_moved(screen_x, screen_y)
        })().unwrap_or(true)
    }

    /// Handle scrolled
    pub fn scrolled(&mut self, amount_x: f32, amount_y: f32) -> bool {
        wrap_crash_handling(|| {
            self.root.scrolled(amount_x, amount_y)
        })().unwrap_or(true)
    }

    /// Handle key down
    pub fn key_down(&mut self, key_code: i32) -> bool {
        wrap_crash_handling(|| {
            self.root.key_down(key_code)
        })().unwrap_or(true)
    }

    /// Handle key up
    pub fn key_up(&mut self, key_code: i32) -> bool {
        wrap_crash_handling(|| {
            self.root.key_up(key_code)
        })().unwrap_or(true)
    }

    /// Handle key typed
    pub fn key_typed(&mut self, character: char) -> bool {
        wrap_crash_handling(|| {
            self.root.key_typed(character)
        })().unwrap_or(true)
    }

    /// Get the root actor
    pub fn root(&self) -> &egui::Group {
        &self.root
    }

    /// Get the root actor mutably
    pub fn root_mut(&mut self) -> &mut egui::Group {
        &mut self.root
    }

    /// Get the mouse over actor
    pub fn mouse_over_actor(&self) -> Option<&dyn egui::Actor> {
        self.mouse_over_actor.as_deref()
    }

    /// Get the pointer over actors
    pub fn pointer_over_actors(&self) -> &[Box<dyn egui::Actor>] {
        &self.pointer_over_actors
    }
}

/// Event for when the visible area changes
#[derive(Clone, Debug)]
pub struct VisibleAreaChanged {
    /// The new visible area
    pub visible_area: egui::Rect,
}

impl Event for VisibleAreaChanged {}