use gdx::scenes::scene2d::Actor;
use crate::ui::components::input::activation_types::ActivationTypes;
use crate::ui::components::input::key_shortcut_dispatcher::KeyShortcutDispatcher;
use crate::ui::components::input::keyboard_binding::KeyboardBinding;
use crate::ui::components::input::key_char_and_code::KeyCharAndCode;
use crate::ui::components::input::key_shortcut::KeyShortcut;
use crate::ui::components::input::activation_action_map::ActivationAction;

/// Simple subclass of `KeyShortcutDispatcher` for which all shortcut actions default to
/// activating the actor. However, other actions are possible too.
pub struct ActorKeyShortcutDispatcher {
    /// The actor this dispatcher is for
    actor: Actor,

    /// The base key shortcut dispatcher
    base: KeyShortcutDispatcher,

    /// The default action that activates the actor with a keystroke
    action: ActivationAction,
}

impl ActorKeyShortcutDispatcher {
    /// Creates a new ActorKeyShortcutDispatcher for the given actor
    pub fn new(actor: Actor) -> Self {
        let action = Box::new(move || {
            // This closure captures the actor and activates it with a keystroke
            // We need to clone the actor here to avoid ownership issues
            let actor_clone = actor.clone();
            actor_clone.activate(ActivationTypes::Keystroke)
        });

        Self {
            actor,
            base: KeyShortcutDispatcher::new(),
            action,
        }
    }

    /// Adds a shortcut with the default action
    pub fn add_shortcut(&mut self, shortcut: Option<KeyShortcut>) {
        self.base.add(shortcut, self.action.clone());
    }

    /// Adds a keyboard binding with the default action
    pub fn add_binding(&mut self, binding: KeyboardBinding, priority: i32) {
        self.base.add(binding, priority, self.action.clone());
    }

    /// Adds a keyboard binding with the default action and default priority
    pub fn add_binding_default_priority(&mut self, binding: KeyboardBinding) {
        self.add_binding(binding, 1);
    }

    /// Adds a key char and code with the default action
    pub fn add_key_char_and_code(&mut self, key: Option<KeyCharAndCode>) {
        self.base.add(key, self.action.clone());
    }

    /// Adds a character with the default action
    pub fn add_char(&mut self, char: Option<char>) {
        self.base.add(char, self.action.clone());
    }

    /// Adds a key code with the default action
    pub fn add_key_code(&mut self, key_code: Option<i32>) {
        self.base.add(key_code, self.action.clone());
    }

    /// Checks if the actor is active
    pub fn is_active(&self) -> bool {
        self.actor.is_active()
    }

    /// Gets the actor this dispatcher is for
    pub fn actor(&self) -> &Actor {
        &self.actor
    }

    /// Gets a mutable reference to the base key shortcut dispatcher
    pub fn base_mut(&mut self) -> &mut KeyShortcutDispatcher {
        &mut self.base
    }
}