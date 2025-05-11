use std::collections::{HashMap, VecDeque};
use std::collections::hash_map::Entry;
use crate::models::UncivSound;
use crate::ui::audio::SoundPlayer;
use crate::utils::Concurrency;

/// Type alias for activation actions (functions that take no parameters and return nothing)
pub type ActivationAction = Box<dyn Fn()>;

/// A list of activation actions with an associated sound
pub struct ActivationActionList {
    /// The sound to play when the action is activated
    pub sound: UncivSound,
    /// The list of actions to execute
    actions: VecDeque<ActivationAction>,
}

impl ActivationActionList {
    /// Creates a new ActivationActionList with the given sound
    pub fn new(sound: UncivSound) -> Self {
        Self {
            sound,
            actions: VecDeque::new(),
        }
    }

    /// Adds an action to the list
    pub fn add(&mut self, action: ActivationAction) {
        self.actions.push_back(action);
    }

    /// Returns true if the list is empty
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Returns a copy of the actions as a Vec
    pub fn to_vec(&self) -> Vec<ActivationAction> {
        self.actions.iter().cloned().collect()
    }
}

/// A map of activation types to activation action lists
///
/// The map is used to store and execute actions for different input types.
/// It handles equivalence between different activation types and plays sounds when actions are activated.
pub struct ActivationActionMap {
    /// The map of activation types to action lists
    actions: HashMap<ActivationTypes, ActivationActionList>,
}

impl ActivationActionMap {
    /// Creates a new ActivationActionMap
    pub fn new() -> Self {
        Self {
            actions: HashMap::new(),
        }
    }

    /// Adds an action for the given activation type
    ///
    /// # Arguments
    ///
    /// * `type` - The activation type
    /// * `sound` - The sound to play when the action is activated
    /// * `no_equivalence` - If true, the action is only added for the given type, not for equivalent types
    /// * `action` - The action to execute
    pub fn add(
        &mut self,
        activation_type: ActivationTypes,
        sound: UncivSound,
        no_equivalence: bool,
        action: ActivationAction,
    ) {
        // Add the action for the given type
        self.get_or_create_list(activation_type, sound).add(action);

        // If no_equivalence is false, add the action for equivalent types
        if !no_equivalence {
            for other in ActivationTypes::equivalent_values(activation_type) {
                self.get_or_create_list(other, sound).add(action);
            }
        }
    }

    /// Gets or creates an action list for the given activation type
    fn get_or_create_list(&mut self, activation_type: ActivationTypes, sound: UncivSound) -> &mut ActivationActionList {
        match self.actions.entry(activation_type) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(ActivationActionList::new(sound)),
        }
    }

    /// Clears all actions for the given activation type
    pub fn clear(&mut self, activation_type: ActivationTypes) {
        self.actions.remove(&activation_type);
    }

    /// Clears all actions for the given activation type and its equivalent types
    pub fn clear_with_equivalence(&mut self, activation_type: ActivationTypes, no_equivalence: bool) {
        self.clear(activation_type);
        if !no_equivalence {
            for other in ActivationTypes::equivalent_values(activation_type) {
                self.clear(other);
            }
        }
    }

    /// Clears all gesture actions
    pub fn clear_gestures(&mut self) {
        for activation_type in ActivationTypes::gestures() {
            self.clear(activation_type);
        }
    }

    /// Returns true if there are any actions in the map
    pub fn is_not_empty(&self) -> bool {
        self.actions.values().any(|list| !list.is_empty())
    }

    /// Activates the actions for the given activation type
    ///
    /// # Arguments
    ///
    /// * `activation_type` - The activation type to activate
    ///
    /// # Returns
    ///
    /// True if any actions were activated, false otherwise
    pub fn activate(&self, activation_type: ActivationTypes) -> bool {
        // Get the action list for the given type
        let actions = match self.actions.get(&activation_type) {
            Some(list) => list,
            None => return false,
        };

        // If the list is empty, return false
        if actions.is_empty() {
            return false;
        }

        // Play the sound if it's not silent
        if actions.sound != UncivSound::Silent {
            Concurrency::run_on_gl_thread("Sound", || {
                SoundPlayer::play(actions.sound);
            });
        }

        // Execute all actions
        // We can't know an activation handler won't redefine activations, so better iterate over a copy
        for action in actions.to_vec() {
            action();
        }

        true
    }
}

impl Default for ActivationActionMap {
    fn default() -> Self {
        Self::new()
    }
}