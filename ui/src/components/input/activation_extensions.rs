use gdx::scenes::scene2d::{Actor, Stage};
use gdx::scenes::scene2d::utils::{ChangeListener, Disableable};
use crate::models::UncivSound;
use crate::ui::components::UncivTooltip;
use crate::ui::components::input::activation_action_map::{ActivationAction, ActivationActionMap};
use crate::ui::components::input::actor_attachments::ActorAttachments;
use crate::ui::components::input::keyboard_binding::KeyboardBinding;
use crate::ui::components::input::key_shortcut_dispatcher::{KeyShortcutDispatcher, KeyShortcutListener};
use crate::ui::components::input::dispatcher_vetoer::DispatcherVetoer;

/// Extension trait for Actor to add activation-related functionality
pub trait ActorActivationExt {
    /// Used to stop activation events if this returns `true`.
    fn is_active(&self) -> bool;

    /// Routes events from the listener to ActorAttachments
    fn activate(&self, activation_type: ActivationTypes) -> bool;

    /// Accesses the shortcut dispatcher for your actor
    /// (creates one if the actor has none).
    fn key_shortcuts(&self) -> &KeyShortcutDispatcher;

    /// Routes input events of type to your handler action.
    /// Will also be activated for events equivalent to type unless no_equivalence is true.
    /// A sound will be played (concurrently) on activation unless you specify UncivSound::Silent.
    fn on_activation(
        &mut self,
        activation_type: ActivationTypes,
        sound: UncivSound,
        no_equivalence: bool,
        action: ActivationAction,
    ) -> &mut Actor;

    /// Assigns an activation handler to your Widget, which reacts to clicks and a key stroke.
    /// A tooltip is attached automatically, if there is a keyboard and the binding has a mapping.
    /// A sound will be played (concurrently) on activation unless you specify UncivSound::Silent.
    fn on_activation_with_binding(
        &mut self,
        sound: UncivSound,
        binding: KeyboardBinding,
        action: ActivationAction,
    ) -> &mut Actor;

    /// Routes clicks and keyboard shortcuts to your handler action.
    /// A sound will be played (concurrently) on activation unless you specify UncivSound::Silent.
    fn on_activation_with_sound(
        &mut self,
        sound: UncivSound,
        action: ActivationAction,
    ) -> &mut Actor;

    /// Routes clicks and keyboard shortcuts to your handler action.
    /// A Click sound will be played (concurrently).
    fn on_activation_default(
        &mut self,
        action: ActivationAction,
    ) -> &mut Actor;

    /// Routes clicks to your handler action, ignoring keyboard shortcuts.
    /// A sound will be played (concurrently) on activation unless you specify UncivSound::Silent.
    fn on_click_with_sound(
        &mut self,
        sound: UncivSound,
        action: ActivationAction,
    ) -> &mut Actor;

    /// Routes clicks to your handler action, ignoring keyboard shortcuts.
    /// A Click sound will be played (concurrently).
    fn on_click(
        &mut self,
        action: ActivationAction,
    ) -> &mut Actor;

    /// Routes double-clicks to your handler action.
    /// A sound will be played (concurrently) on activation unless you specify UncivSound::Silent.
    fn on_double_click(
        &mut self,
        sound: UncivSound,
        action: ActivationAction,
    ) -> &mut Actor;

    /// Routes right-clicks and long-presses to your handler action.
    /// These are treated as equivalent so both desktop and mobile can access the same functionality with methods common to the platform.
    /// A sound will be played (concurrently) on activation unless you specify UncivSound::Silent.
    fn on_right_click(
        &mut self,
        sound: UncivSound,
        action: ActivationAction,
    ) -> &mut Actor;

    /// Routes long-presses (but not right-clicks) to your handler action.
    /// A sound will be played (concurrently) on activation unless you specify UncivSound::Silent.
    fn on_long_press(
        &mut self,
        sound: UncivSound,
        action: ActivationAction,
    ) -> &mut Actor;

    /// Clears activation actions for a specific type, and, if no_equivalence is true,
    /// its equivalent types.
    fn clear_activation_actions(&mut self, activation_type: ActivationTypes, no_equivalence: bool);

    /// Attach a ChangeListener to this and route its changed event to action
    fn on_change<F>(&mut self, action: F) -> &mut Actor
    where
        F: Fn(Option<&ChangeListener::ChangeEvent>) + 'static;
}

impl ActorActivationExt for Actor {
    fn is_active(&self) -> bool {
        self.is_visible() && !self.is_disabled()
    }

    fn activate(&self, activation_type: ActivationTypes) -> bool {
        if !self.is_active() {
            return false;
        }

        let attachment = match ActorAttachments::get_or_null(self) {
            Some(attachment) => attachment,
            None => return false,
        };

        attachment.activate(activation_type)
    }

    fn key_shortcuts(&self) -> &KeyShortcutDispatcher {
        ActorAttachments::get(self).key_shortcuts()
    }

    fn on_activation(
        &mut self,
        activation_type: ActivationTypes,
        sound: UncivSound,
        no_equivalence: bool,
        action: ActivationAction,
    ) -> &mut Actor {
        ActorAttachments::get_mut(self).add_activation_action(activation_type, sound, no_equivalence, action);
        self
    }

    fn on_activation_with_binding(
        &mut self,
        sound: UncivSound,
        binding: KeyboardBinding,
        action: ActivationAction,
    ) -> &mut Actor {
        self.on_activation(ActivationTypes::Tap, sound, false, action);
        self.key_shortcuts().add(binding);
        UncivTooltip::add_tooltip(self, binding);
        self
    }

    fn on_activation_with_sound(
        &mut self,
        sound: UncivSound,
        action: ActivationAction,
    ) -> &mut Actor {
        self.on_activation(ActivationTypes::Tap, sound, false, action)
    }

    fn on_activation_default(
        &mut self,
        action: ActivationAction,
    ) -> &mut Actor {
        self.on_activation(ActivationTypes::Tap, UncivSound::Click, false, action)
    }

    fn on_click_with_sound(
        &mut self,
        sound: UncivSound,
        action: ActivationAction,
    ) -> &mut Actor {
        self.on_activation(ActivationTypes::Tap, sound, true, action)
    }

    fn on_click(
        &mut self,
        action: ActivationAction,
    ) -> &mut Actor {
        self.on_activation(ActivationTypes::Tap, UncivSound::Click, true, action)
    }

    fn on_double_click(
        &mut self,
        sound: UncivSound,
        action: ActivationAction,
    ) -> &mut Actor {
        self.on_activation(ActivationTypes::Doubletap, sound, false, action)
    }

    fn on_right_click(
        &mut self,
        sound: UncivSound,
        action: ActivationAction,
    ) -> &mut Actor {
        self.on_activation(ActivationTypes::RightClick, sound, false, action)
    }

    fn on_long_press(
        &mut self,
        sound: UncivSound,
        action: ActivationAction,
    ) -> &mut Actor {
        self.on_activation(ActivationTypes::Longpress, sound, true, action)
    }

    fn clear_activation_actions(&mut self, activation_type: ActivationTypes, no_equivalence: bool) {
        ActorAttachments::get_mut(self).clear_activation_actions(activation_type, no_equivalence);
    }

    fn on_change<F>(&mut self, action: F) -> &mut Actor
    where
        F: Fn(Option<&ChangeListener::ChangeEvent>) + 'static,
    {
        let listener = OnChangeListener::new(action);
        self.add_listener(listener);
        self
    }
}

/// Extension trait for Stage to add shortcut dispatcher functionality
pub trait StageShortcutExt {
    /// Install shortcut dispatcher for this stage. It activates all actions associated with the
    /// pressed key in additional_shortcuts (if specified) and all actors in the stage - recursively.
    ///
    /// It is possible to temporarily disable or veto some shortcut dispatchers by passing an appropriate
    /// dispatcher_vetoer_creator function. This function may return a DispatcherVetoer, which
    /// will then be used to evaluate all shortcut sources in the stage.
    fn install_shortcut_dispatcher<F>(
        &mut self,
        additional_shortcuts: Option<&KeyShortcutDispatcher>,
        dispatcher_vetoer_creator: F,
    ) where
        F: Fn() -> Option<Box<dyn DispatcherVetoer>> + 'static;
}

impl StageShortcutExt for Stage {
    fn install_shortcut_dispatcher<F>(
        &mut self,
        additional_shortcuts: Option<&KeyShortcutDispatcher>,
        dispatcher_vetoer_creator: F,
    ) where
        F: Fn() -> Option<Box<dyn DispatcherVetoer>> + 'static,
    {
        let actors = self.actors().collect::<Vec<_>>();
        let listener = KeyShortcutListener::new(
            actors.into_iter(),
            additional_shortcuts,
            dispatcher_vetoer_creator,
        );
        self.add_listener(listener);
    }
}

/// A ChangeListener that routes its changed event to a function
struct OnChangeListener {
    function: Box<dyn Fn(Option<&ChangeListener::ChangeEvent>)>,
}

impl OnChangeListener {
    /// Creates a new OnChangeListener with the given function
    fn new<F>(function: F) -> Self
    where
        F: Fn(Option<&ChangeListener::ChangeEvent>) + 'static,
    {
        Self {
            function: Box::new(function),
        }
    }
}

impl ChangeListener for OnChangeListener {
    fn changed(&self, event: Option<&ChangeListener::ChangeEvent>, _actor: Option<&Actor>) {
        (self.function)(event);
    }
}