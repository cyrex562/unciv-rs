use std::collections::HashMap;
use std::fmt;
use crate::ui::components::input::key_char_and_code::KeyCharAndCode;
use crate::ui::components::input::keyboard_binding::{KeyboardBinding, GlobalKeyboardBindings};
use crate::ui::components::activation::ActivationAction;

/// Dispatches keyboard shortcuts to actions
pub struct KeyShortcutDispatcher {
    /// The list of shortcuts and their associated actions
    shortcuts: Vec<ShortcutAction>,
    /// Whether the dispatcher is active
    active: bool,
}

/// Represents a keyboard shortcut with a binding, key, and priority
///
/// # Arguments
///
/// * `binding` - The abstract KeyboardBinding that will be bound to an action
/// * `key` - The hardcoded key that will be bound to an action
/// * `priority` - Used by the Resolver - only the actions bound to the incoming key with the _highest priority_ will run
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyShortcut {
    /// The binding associated with this shortcut
    binding: KeyboardBinding,
    /// The key associated with this shortcut
    key: KeyCharAndCode,
    /// The priority of this shortcut
    priority: i32,
}

impl KeyShortcut {
    /// Create a new KeyShortcut
    pub fn new(binding: KeyboardBinding, key: KeyCharAndCode, priority: i32) -> Self {
        Self {
            binding,
            key,
            priority,
        }
    }

    /// Get the real key that this shortcut represents
    pub fn get_real_key(&self) -> KeyCharAndCode {
        if self.binding == KeyboardBinding::None {
            self.key
        } else {
            GlobalKeyboardBindings::get(self.binding)
        }
    }

    /// Get the real priority of this shortcut
    pub fn get_real_priority(&self) -> i32 {
        // Bindings with the default key (user-untouched) are less prioritized than unique, user-set bindings
        if self.binding != KeyboardBinding::None && GlobalKeyboardBindings::get(self.binding) == self.binding.default_key() {
            self.priority - 1
        } else {
            self.priority
        }
    }
}

impl fmt::Display for KeyShortcut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.binding.is_hidden() {
            write!(f, "{}@{}", self.key, self.priority)
        } else {
            write!(f, "{}@{}", self.binding, self.priority)
        }
    }
}

/// Represents a shortcut and its associated action
#[derive(Debug)]
struct ShortcutAction {
    /// The shortcut
    shortcut: KeyShortcut,
    /// The action to perform when the shortcut is triggered
    action: ActivationAction,
}

impl KeyShortcutDispatcher {
    /// Create a new KeyShortcutDispatcher
    pub fn new() -> Self {
        Self {
            shortcuts: Vec::new(),
            active: true,
        }
    }

    /// Clear all shortcuts
    pub fn clear(&mut self) {
        self.shortcuts.clear();
    }

    /// Add a shortcut and action
    pub fn add(&mut self, shortcut: Option<KeyShortcut>, action: Option<ActivationAction>) {
        if action.is_none() || shortcut.is_none() {
            return;
        }

        let shortcut = shortcut.unwrap();
        let action = action.unwrap();

        // Remove any existing shortcuts with the same key
        self.shortcuts.retain(|sa| sa.shortcut != shortcut);

        // Add the new shortcut and action
        self.shortcuts.push(ShortcutAction { shortcut, action });
    }

    /// Add a binding with a priority and action
    pub fn add_binding(&mut self, binding: KeyboardBinding, priority: i32, action: Option<ActivationAction>) {
        self.add(Some(KeyShortcut::new(binding, KeyCharAndCode::UNKNOWN, priority)), action);
    }

    /// Add a key and action
    pub fn add_key(&mut self, key: Option<KeyCharAndCode>, action: Option<ActivationAction>) {
        if let Some(key) = key {
            self.add(Some(KeyShortcut::new(KeyboardBinding::None, key, 0)), action);
        }
    }

    /// Add a character and action
    pub fn add_char(&mut self, c: Option<char>, action: Option<ActivationAction>) {
        if let Some(c) = c {
            self.add_key(Some(KeyCharAndCode::from_char(c)), action);
        }
    }

    /// Add a key code and action
    pub fn add_key_code(&mut self, key_code: Option<i32>, action: Option<ActivationAction>) {
        if let Some(key_code) = key_code {
            self.add_key(Some(KeyCharAndCode::from_code(key_code)), action);
        }
    }

    /// Check if the dispatcher contains a key
    pub fn contains(&self, key: KeyCharAndCode) -> bool {
        self.shortcuts.iter().any(|sa| key == sa.shortcut.get_real_key())
    }

    /// Check if the dispatcher is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Set whether the dispatcher is active
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Get a resolver for a key
    pub fn resolver(&self, key: KeyCharAndCode) -> Resolver {
        Resolver::new(key, self)
    }
}

/// Resolves which actions to trigger for a key
///
/// Given that several different shortcuts could be mapped to the same key,
/// this class decides what will actually happen when the key is pressed
pub struct Resolver {
    /// The key to resolve
    key: KeyCharAndCode,
    /// The current priority
    priority: i32,
    /// The actions that will be triggered
    triggered_actions: Vec<ActivationAction>,
}

impl Resolver {
    /// Create a new Resolver
    fn new(key: KeyCharAndCode, dispatcher: &KeyShortcutDispatcher) -> Self {
        let mut resolver = Self {
            key,
            priority: i32::MIN,
            triggered_actions: Vec::new(),
        };
        resolver.update_for(dispatcher);
        resolver
    }

    /// Update the resolver for a dispatcher
    fn update_for(&mut self, dispatcher: &KeyShortcutDispatcher) {
        if !dispatcher.is_active() {
            return;
        }

        for sa in &dispatcher.shortcuts {
            if sa.shortcut.get_real_key() != self.key {
                continue;
            }

            let shortcut_priority = sa.shortcut.get_real_priority();

            // We always want to take the highest priority action, but if there are several of the same priority we do them all
            if shortcut_priority < self.priority {
                continue;
            }

            if shortcut_priority > self.priority {
                self.priority = shortcut_priority;
                self.triggered_actions.clear();
            }

            self.triggered_actions.push(sa.action.clone());
        }
    }

    /// Get the actions that will be triggered
    pub fn triggered_actions(&self) -> &[ActivationAction] {
        &this.triggered_actions
    }
}