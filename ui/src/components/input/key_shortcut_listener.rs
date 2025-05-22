use std::collections::VecDeque;
use std::option::Option;
use ggez::event::{EventHandler, KeyCode, KeyInput, KeyMods};
use crate::ui::components::actor::Actor;
use crate::ui::components::actor_attachments::ActorAttachments;
use crate::ui::components::input::key_char_and_code::KeyCharAndCode;
use crate::ui::components::input::key_shortcut_dispatcher::KeyShortcutDispatcher;
use crate::ui::components::input::key_shortcut_dispatcher_veto::{DispatcherVetoer, DispatcherVetoResult, KeyShortcutDispatcherVeto};

/// Listens for keyboard shortcuts and dispatches them to the appropriate handlers
pub struct KeyShortcutListener {
    /// The actors to check for shortcuts
    actors: Vec<Actor>,
    /// Additional shortcuts to check
    additional_shortcuts: Option<KeyShortcutDispatcher>,
    /// Function that creates a dispatcher vetoer
    dispatcher_vetoer_creator: Box<dyn Fn() -> Option<DispatcherVetoer> + Send + Sync>,
}

impl KeyShortcutListener {
    /// Create a new KeyShortcutListener
    pub fn new(
        actors: Vec<Actor>,
        additional_shortcuts: Option<KeyShortcutDispatcher>,
        dispatcher_vetoer_creator: Box<dyn Fn() -> Option<DispatcherVetoer> + Send + Sync>,
    ) -> Self {
        Self {
            actors,
            additional_shortcuts,
            dispatcher_vetoer_creator,
        }
    }

    /// Handle a key down event
    fn key_down(&self, keycode: KeyCode, mods: KeyMods) -> bool {
        let key = if mods.contains(KeyMods::CTRL) {
            KeyCharAndCode::ctrl_from_code(keycode as i32)
        } else {
            KeyCharAndCode::from_code(keycode as i32)
        };

        if key == KeyCharAndCode::UNKNOWN {
            return false;
        }

        let dispatcher_vetoer = self.dispatcher_vetoer_creator()
            .unwrap_or_else(KeyShortcutDispatcherVeto::default_dispatcher_vetoer);

        if self.activate(key, &dispatcher_vetoer) {
            return true;
        }

        // Make both Enter keys equivalent
        if (key == KeyCharAndCode::NUMPAD_ENTER && self.activate(KeyCharAndCode::RETURN, &dispatcher_vetoer))
            || (key == KeyCharAndCode::RETURN && self.activate(KeyCharAndCode::NUMPAD_ENTER, &dispatcher_vetoer)) {
            return true;
        }

        // Likewise always match Back to ESC
        if (key == KeyCharAndCode::ESC && self.activate(KeyCharAndCode::BACK, &dispatcher_vetoer))
            || (key == KeyCharAndCode::BACK && self.activate(KeyCharAndCode::ESC, &dispatcher_vetoer)) {
            return true;
        }

        false
    }

    /// Activate a key with the given dispatcher vetoer
    fn activate(&self, key: KeyCharAndCode, dispatcher_vetoer: &DispatcherVetoer) -> bool {
        let mut shortcut_resolver = KeyShortcutDispatcher::resolver(key);
        let mut pending_actors = VecDeque::from(self.actors.clone());

        if let Some(additional_shortcuts) = &self.additional_shortcuts {
            if dispatcher_vetoer(None) == DispatcherVetoResult::Accept {
                shortcut_resolver.update_for(additional_shortcuts);
            }
        }

        while let Some(actor) = pending_actors.pop_front() {
            let shortcuts = ActorAttachments::get_or_null(&actor).and_then(|attachments| attachments.key_shortcuts.clone());
            let veto_result = dispatcher_vetoer(Some(&actor));

            if let Some(shortcuts) = shortcuts {
                if veto_result == DispatcherVetoResult::Accept {
                    shortcut_resolver.update_for(&shortcuts);
                }
            }

            if actor.is_group() && veto_result != DispatcherVetoResult::SkipWithChildren {
                pending_actors.extend(actor.children().iter().cloned());
            }
        }

        for action in shortcut_resolver.triggered_actions() {
            action();
        }

        !shortcut_resolver.triggered_actions().is_empty()
    }
}

impl EventHandler for KeyShortcutListener {
    fn key_down_event(&mut self, _ctx: &mut ggez::Context, keycode: KeyCode, _keymods: KeyMods, _repeat: bool) -> bool {
        self.key_down(keycode, _keymods)
    }
}