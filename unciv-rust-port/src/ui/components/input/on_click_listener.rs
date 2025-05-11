use std::option::Option;
use ggez::event::{EventHandler, MouseButton, MouseInput};
use ggez::mint::Point2;
use crate::ui::components::actor::Actor;
use crate::ui::components::actor_attachments::ActorAttachments;
use crate::ui::components::input::click_listener::ClickListener;
use crate::ui::components::input::click_listener::ClickListenerType;

/// Listens for click events and dispatches them to the appropriate handlers
pub struct OnClickListener {
    /// The actors to check for click events
    actors: Vec<Actor>,
    /// The current click listener
    current_click_listener: Option<ClickListener>,
    /// Whether to handle double clicks
    handle_double_click: bool,
    /// Whether to handle triple clicks
    handle_triple_click: bool,
    /// Whether to handle quadruple clicks
    handle_quadruple_click: bool,
    /// Whether to handle quintuple clicks
    handle_quintuple_click: bool,
    /// Whether to handle sextuple clicks
    handle_sextuple_click: bool,
    /// Whether to handle septuple clicks
    handle_septuple_click: bool,
    /// Whether to handle octuple clicks
    handle_octuple_click: bool,
    /// Whether to handle nonuple clicks
    handle_nonuple_click: bool,
    /// Whether to handle decuple clicks
    handle_decuple_click: bool,
}

impl OnClickListener {
    /// Create a new OnClickListener
    pub fn new(actors: Vec<Actor>) -> Self {
        Self {
            actors,
            current_click_listener: None,
            handle_double_click: true,
            handle_triple_click: true,
            handle_quadruple_click: true,
            handle_quintuple_click: true,
            handle_sextuple_click: true,
            handle_septuple_click: true,
            handle_octuple_click: true,
            handle_nonuple_click: true,
            handle_decuple_click: true,
        }
    }

    /// Handle a mouse button down event
    fn mouse_button_down_event(&mut self, _ctx: &mut ggez::Context, button: MouseButton, x: f32, y: f32) -> bool {
        let click_listener = self.get_click_listener();
        if let Some(click_listener) = click_listener {
            click_listener.clicked(button, Point2 { x, y });
            true
        } else {
            false
        }
    }

    /// Handle a mouse button up event
    fn mouse_button_up_event(&mut self, _ctx: &mut ggez::Context, button: MouseButton, x: f32, y: f32) -> bool {
        let click_listener = self.get_click_listener();
        if let Some(click_listener) = click_listener {
            click_listener.unclicked(button, Point2 { x, y });
            true
        } else {
            false
        }
    }

    /// Get the click listener for the actor at the given position
    fn get_click_listener(&mut self) -> Option<&mut ClickListener> {
        if let Some(current_click_listener) = &mut self.current_click_listener {
            return Some(current_click_listener);
        }

        for actor in &self.actors {
            if let Some(click_listener) = ActorAttachments::get_or_null(actor).and_then(|attachments| attachments.click_listener.clone()) {
                self.current_click_listener = Some(click_listener);
                return self.current_click_listener.as_mut();
            }
        }

        None
    }

    /// Set whether to handle double clicks
    pub fn set_handle_double_click(&mut self, handle_double_click: bool) {
        self.handle_double_click = handle_double_click;
    }

    /// Set whether to handle triple clicks
    pub fn set_handle_triple_click(&mut self, handle_triple_click: bool) {
        self.handle_triple_click = handle_triple_click;
    }

    /// Set whether to handle quadruple clicks
    pub fn set_handle_quadruple_click(&mut self, handle_quadruple_click: bool) {
        self.handle_quadruple_click = handle_quadruple_click;
    }

    /// Set whether to handle quintuple clicks
    pub fn set_handle_quintuple_click(&mut self, handle_quintuple_click: bool) {
        self.handle_quintuple_click = handle_quintuple_click;
    }

    /// Set whether to handle sextuple clicks
    pub fn set_handle_sextuple_click(&mut self, handle_sextuple_click: bool) {
        self.handle_sextuple_click = handle_sextuple_click;
    }

    /// Set whether to handle septuple clicks
    pub fn set_handle_septuple_click(&mut self, handle_septuple_click: bool) {
        self.handle_septuple_click = handle_septuple_click;
    }

    /// Set whether to handle octuple clicks
    pub fn set_handle_octuple_click(&mut self, handle_octuple_click: bool) {
        self.handle_octuple_click = handle_octuple_click;
    }

    /// Set whether to handle nonuple clicks
    pub fn set_handle_nonuple_click(&mut self, handle_nonuple_click: bool) {
        self.handle_nonuple_click = handle_nonuple_click;
    }

    /// Set whether to handle decuple clicks
    pub fn set_handle_decuple_click(&mut self, handle_decuple_click: bool) {
        self.handle_decuple_click = handle_decuple_click;
    }
}

impl EventHandler for OnClickListener {
    fn mouse_button_down_event(&mut self, ctx: &mut ggez::Context, button: MouseButton, x: f32, y: f32) -> bool {
        self.mouse_button_down_event(ctx, button, x, y)
    }

    fn mouse_button_up_event(&mut self, ctx: &mut ggez::Context, button: MouseButton, x: f32, y: f32) -> bool {
        self.mouse_button_up_event(ctx, button, x, y)
    }
}