use std::option::Option;
use crate::ui::components::actor::Actor;
use crate::ui::components::tilegroups::TileGroupMap;

/// Result of a dispatcher veto check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatcherVetoResult {
    /// Accept the shortcut
    Accept,
    /// Skip this actor
    Skip,
    /// Skip this actor and its children
    SkipWithChildren,
}

/// A function that tests whether shortcuts should be processed for a given actor
///
/// # Arguments
///
/// * `associated_actor` - The actor to test. If None, it means the keyDispatcher is BaseScreen.globalShortcuts
pub type DispatcherVetoer = Box<dyn Fn(Option<&Actor>) -> DispatcherVetoResult + Send + Sync>;

/// Provides veto functions for keyboard shortcut dispatchers
pub struct KeyShortcutDispatcherVeto;

impl KeyShortcutDispatcherVeto {
    /// The default dispatcher vetoer that accepts all shortcuts
    pub fn default_dispatcher_vetoer() -> DispatcherVetoer {
        Box::new(|_| DispatcherVetoResult::Accept)
    }

    /// When a Popup is active, this creates a DispatcherVetoer that disables all
    /// shortcuts on actors outside the popup and also the global shortcuts on the screen itself.
    pub fn create_popup_based_dispatcher_vetoer(active_popup: &Actor) -> DispatcherVetoer {
        Box::new(move |associated_actor: Option<&Actor>| {
            match associated_actor {
                None => DispatcherVetoResult::Skip,
                Some(actor) => {
                    if actor.is_descendant_of(active_popup) {
                        DispatcherVetoResult::Accept
                    } else {
                        DispatcherVetoResult::SkipWithChildren
                    }
                }
            }
        })
    }

    /// Return this from BaseScreen.getShortcutDispatcherVetoer for Screens containing a TileGroupMap
    pub fn create_tile_group_map_dispatcher_vetoer() -> DispatcherVetoer {
        Box::new(|associated_actor: Option<&Actor>| {
            match associated_actor {
                Some(actor) => {
                    if actor.is::<TileGroupMap>() {
                        DispatcherVetoResult::SkipWithChildren
                    } else {
                        DispatcherVetoResult::Accept
                    }
                }
                None => DispatcherVetoResult::Accept,
            }
        })
    }
}