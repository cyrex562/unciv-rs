use std::collections::HashSet;
use gdx::scenes::scene2d::{Actor, EventListener};
use gdx::scenes::scene2d::utils::Disableable;
use crate::models::UncivSound;
use crate::ui::components::input::activation_action_map::{ActivationAction, ActivationActionMap};
use crate::ui::components::input::activation_listener::ActivationListener;
use crate::ui::components::input::activation_types::ActivationTypes;
use crate::ui::components::input::actor_key_shortcut_dispatcher::ActorKeyShortcutDispatcher;

/// Attachments for an actor, including activation actions and keyboard shortcuts
///
/// This struct is attached to an actor and manages its activation actions and keyboard shortcuts.
/// It is created lazily when needed and stored in the actor's user object.
pub struct ActorAttachments {
    /// The actor this is attached to
    actor: Actor,

    /// The activation action map for this actor
    activation_actions: Option<ActivationActionMap>,

    /// The activation listener for this actor
    activation_listener: Option<ActivationListener>,

    /// The keyboard shortcut dispatcher for this actor
    key_shortcuts: ActorKeyShortcutDispatcher,
}

impl ActorAttachments {
    /// Creates a new ActorAttachments for the given actor
    fn new(actor: Actor) -> Self {
        Self {
            actor,
            activation_actions: None,
            activation_listener: None,
            key_shortcuts: ActorKeyShortcutDispatcher::new(actor),
        }
    }

    /// Gets the ActorAttachments for the given actor, or None if it doesn't exist
    pub fn get_or_null(actor: &Actor) -> Option<&ActorAttachments> {
        actor.user_object().and_then(|obj| obj.downcast_ref::<ActorAttachments>())
    }

    /// Gets the ActorAttachments for the given actor, creating it if it doesn't exist
    pub fn get(actor: &Actor) -> &ActorAttachments {
        if actor.user_object().is_none() {
            actor.set_user_object(Box::new(ActorAttachments::new(actor.clone())));
        }

        ActorAttachments::get_or_null(actor).unwrap()
    }

    /// Gets a mutable reference to the ActorAttachments for the given actor, creating it if it doesn't exist
    pub fn get_mut(actor: &mut Actor) -> &mut ActorAttachments {
        if actor.user_object().is_none() {
            actor.set_user_object(Box::new(ActorAttachments::new(actor.clone())));
        }

        actor.user_object_mut().unwrap().downcast_mut::<ActorAttachments>().unwrap()
    }

    /// Gets the actor this is attached to
    pub fn actor(&self) -> &Actor {
        &self.actor
    }

    /// Gets the keyboard shortcut dispatcher for this actor
    pub fn key_shortcuts(&self) -> &ActorKeyShortcutDispatcher {
        &self.key_shortcuts
    }

    /// Gets a mutable reference to the keyboard shortcut dispatcher for this actor
    pub fn key_shortcuts_mut(&mut self) -> &mut ActorKeyShortcutDispatcher {
        &mut self.key_shortcuts
    }

    /// Activates the actor with the given activation type
    ///
    /// # Arguments
    ///
    /// * `type` - The activation type
    ///
    /// # Returns
    ///
    /// `true` if the activation was successful, `false` otherwise
    pub fn activate(&self, activation_type: ActivationTypes) -> bool {
        // Check if activation actions are initialized
        let activation_actions = match &self.activation_actions {
            Some(actions) => actions,
            None => return false,
        };

        // Skip if disabled
        if let Some(disableable) = self.actor.as_any().downcast_ref::<Disableable>() {
            if disableable.is_disabled() {
                return false;
            }
        }

        // Activate the actions
        activation_actions.activate(activation_type)
    }

    /// Adds an activation action for the given activation type
    ///
    /// # Arguments
    ///
    /// * `type` - The activation type
    /// * `sound` - The sound to play when the action is activated
    /// * `no_equivalence` - If true, the action is only added for the given type, not for equivalent types
    /// * `action` - The action to execute
    pub fn add_activation_action(
        &mut self,
        activation_type: ActivationTypes,
        sound: UncivSound,
        no_equivalence: bool,
        action: ActivationAction,
    ) {
        // Initialize activation actions if needed
        if self.activation_actions.is_none() {
            self.activation_actions = Some(ActivationActionMap::new());
        }

        // Check if the activation listener is active
        if let Some(listener) = &self.activation_listener {
            if !self.actor.has_listener(listener) {
                // We think our listener should be active but it isn't - Actor.clear_listeners() was called.
                // Decision: To keep existing code (which could have to call clear_activation_actions otherwise),
                // we start over clearing any registered actions using that listener.
                self.actor.add_listener(listener.clone());
                if let Some(actions) = &mut self.activation_actions {
                    actions.clear_gestures();
                }
            }
        }

        // Add the action
        if let Some(actions) = &mut self.activation_actions {
            actions.add(activation_type, sound, no_equivalence, action);
        }

        // Add the activation listener if needed
        if activation_type.is_gesture() && self.activation_listener.is_none() {
            let listener = ActivationListener::new();
            self.actor.add_listener(listener.clone());
            self.activation_listener = Some(listener);
        }
    }

    /// Clears activation actions for the given activation type
    ///
    /// # Arguments
    ///
    /// * `type` - The activation type
    /// * `no_equivalence` - If true, only clears actions for the given type, not for equivalent types
    pub fn clear_activation_actions(&mut self, activation_type: ActivationTypes, no_equivalence: bool) {
        // Check if activation actions are initialized
        let activation_actions = match &mut self.activation_actions {
            Some(actions) => actions,
            None => return,
        };

        // Clear the actions
        activation_actions.clear_with_equivalence(activation_type, no_equivalence);

        // Remove the activation listener if there are no more actions
        if self.activation_listener.is_some() && !activation_actions.is_not_empty() {
            if let Some(listener) = self.activation_listener.take() {
                self.actor.remove_listener(&listener);
            }
        }
    }
}