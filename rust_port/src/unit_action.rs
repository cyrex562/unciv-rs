use std::fmt;
use std::hash::{Hash, Hasher};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use crate::unciv_sound::{UncivSound, UncivSounds};
use crate::unique::Unique;
use crate::ruleset::unit::BaseUnit;
use crate::counter::Counter;
use crate::constants::REPAIR;
use crate::fonts::Fonts;

/// Represents a keyboard binding for unit actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyboardBinding {
    None,
    // Add other keyboard bindings as needed
}

/// Unit Actions - enum with static properties
///
/// # Parameters
/// * `value` - Default label to display, can be overridden in UnitAction instantiation
/// * `image_getter` - Optional function to get an Icon - None if icon is dependent on outside factors
/// * `binding` - Keyboard binding - omitting it will look up the KeyboardBinding of the same name
/// * `is_skipping_to_next_unit` - If "Auto Unit Cycle" setting and this bit are on, this action will skip to the next unit
/// * `unciv_sound` - Default sound, can be overridden in UnitAction instantiation
/// * `default_page` - UI "page" preference, 0-based - Dynamic overrides to this are in `UnitActions.action_type_to_page_getter`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitActionType {
    StopEscortFormation,
    EscortFormation,
    SwapUnits,
    Automate,
    ConnectRoad,
    StopAutomation,
    StopMovement,
    ShowUnitDestination,
    Sleep,
    SleepUntilHealed,
    Fortify,
    FortifyUntilHealed,
    Guard,
    Explore,
    StopExploration,
    Promote,
    Upgrade,
    Transform,
    Pillage,
    Paradrop,
    AirSweep,
    SetUp,
    FoundCity,
    ConstructImprovement,
    Repair,
    CreateImprovement,
    HurryResearch,
    HurryPolicy,
    HurryWonder,
    HurryBuilding,
    ConductTradeMission,
    FoundReligion,
    TriggerUnique,
    SpreadReligion,
    RemoveHeresy,
    EnhanceReligion,
    DisbandUnit,
    GiftUnit,
    Skip,
    ShowAdditionalActions,
    HideAdditionalActions,
    AddInCapital,
}

impl UnitActionType {
    /// Get the display value for this action type
    pub fn value(&self) -> &'static str {
        match self {
            UnitActionType::StopEscortFormation => "Stop Escort formation",
            UnitActionType::EscortFormation => "Escort formation",
            UnitActionType::SwapUnits => "Swap units",
            UnitActionType::Automate => "Automate",
            UnitActionType::ConnectRoad => "Connect road",
            UnitActionType::StopAutomation => "Stop automation",
            UnitActionType::StopMovement => "Stop movement",
            UnitActionType::ShowUnitDestination => "Show unit destination",
            UnitActionType::Sleep => "Sleep",
            UnitActionType::SleepUntilHealed => "Sleep until healed",
            UnitActionType::Fortify => "Fortify",
            UnitActionType::FortifyUntilHealed => "Fortify until healed",
            UnitActionType::Guard => "Guard",
            UnitActionType::Explore => "Explore",
            UnitActionType::StopExploration => "Stop exploration",
            UnitActionType::Promote => "Promote",
            UnitActionType::Upgrade => "Upgrade",
            UnitActionType::Transform => "Transform",
            UnitActionType::Pillage => "Pillage",
            UnitActionType::Paradrop => "Paradrop",
            UnitActionType::AirSweep => "Air Sweep",
            UnitActionType::SetUp => "Set up",
            UnitActionType::FoundCity => "Found city",
            UnitActionType::ConstructImprovement => "Construct improvement",
            UnitActionType::Repair => REPAIR,
            UnitActionType::CreateImprovement => "Create",
            UnitActionType::HurryResearch => format!("{{Hurry Research}} ({})", Fonts::DEATH).as_str(),
            UnitActionType::HurryPolicy => format!("{{Hurry Policy}} ({})", Fonts::DEATH).as_str(),
            UnitActionType::HurryWonder => format!("{{Hurry Wonder}} ({})", Fonts::DEATH).as_str(),
            UnitActionType::HurryBuilding => format!("{{Hurry Construction}} ({})", Fonts::DEATH).as_str(),
            UnitActionType::ConductTradeMission => format!("{{Conduct Trade Mission}} ({})", Fonts::DEATH).as_str(),
            UnitActionType::FoundReligion => "Found a Religion",
            UnitActionType::TriggerUnique => "Trigger unique",
            UnitActionType::SpreadReligion => "Spread Religion",
            UnitActionType::RemoveHeresy => "Remove Heresy",
            UnitActionType::EnhanceReligion => "Enhance a Religion",
            UnitActionType::DisbandUnit => "Disband unit",
            UnitActionType::GiftUnit => "Gift unit",
            UnitActionType::Skip => "Skip turn",
            UnitActionType::ShowAdditionalActions => "Show more",
            UnitActionType::HideAdditionalActions => "Back",
            UnitActionType::AddInCapital => "Add in capital",
        }
    }

    /// Get the default sound for this action type
    pub fn unciv_sound(&self) -> UncivSound {
        match self {
            UnitActionType::Fortify | UnitActionType::FortifyUntilHealed => UncivSounds::FORTIFY,
            UnitActionType::Promote => UncivSounds::PROMOTE,
            UnitActionType::Upgrade | UnitActionType::Transform => UncivSounds::UPGRADE,
            UnitActionType::SetUp => UncivSounds::SETUP,
            UnitActionType::FoundCity | UnitActionType::GiftUnit | UnitActionType::Skip => UncivSounds::SILENT,
            UnitActionType::Repair => UncivSounds::CONSTRUCTION,
            UnitActionType::CreateImprovement | UnitActionType::HurryResearch | UnitActionType::HurryPolicy |
            UnitActionType::HurryWonder | UnitActionType::HurryBuilding | UnitActionType::ConductTradeMission |
            UnitActionType::TriggerUnique | UnitActionType::AddInCapital => UncivSounds::CHIMES,
            UnitActionType::FoundReligion | UnitActionType::SpreadReligion | UnitActionType::EnhanceReligion => UncivSounds::CHOIR,
            UnitActionType::RemoveHeresy => UncivSounds::FIRE,
            _ => UncivSounds::CLICK,
        }
    }

    /// Get whether this action should skip to the next unit
    pub fn is_skipping_to_next_unit(&self) -> bool {
        match self {
            UnitActionType::StopEscortFormation | UnitActionType::EscortFormation |
            UnitActionType::ConnectRoad | UnitActionType::StopAutomation |
            UnitActionType::StopMovement | UnitActionType::StopExploration |
            UnitActionType::Promote | UnitActionType::Pillage | UnitActionType::Paradrop |
            UnitActionType::AirSweep | UnitActionType::ConstructImprovement |
            UnitActionType::CreateImprovement | UnitActionType::TriggerUnique |
            UnitActionType::ShowAdditionalActions | UnitActionType::HideAdditionalActions => false,
            _ => true,
        }
    }

    /// Get the default page for this action type
    pub fn default_page(&self) -> i32 {
        match self {
            UnitActionType::StopEscortFormation | UnitActionType::EscortFormation |
            UnitActionType::ShowUnitDestination | UnitActionType::Guard |
            UnitActionType::DisbandUnit | UnitActionType::GiftUnit |
            UnitActionType::HideAdditionalActions => 1,
            _ => 0,
        }
    }

    /// Get the keyboard binding for this action type
    pub fn binding(&self) -> KeyboardBinding {
        // In a real implementation, this would look up the binding by name
        // For now, we'll just return None
        KeyboardBinding::None
    }
}

/// Unit Actions - struct - carries dynamic data and actual execution.
/// Static properties are in [UnitActionType].
/// Note this is for the buttons offering actions, not the ongoing action stored with a MapUnit
#[derive(Clone, Serialize, Deserialize)]
pub struct UnitAction {
    /// The type of action
    pub action_type: UnitActionType,

    /// How often this action is used, a higher value means more often and that it should be on an earlier page.
    /// 100 is very frequent, 50 is somewhat frequent, less than 25 is press one time for multi-turn movement.
    /// A Rare case is > 100 if a button is something like add in capital, promote or something,
    /// we need to inform the player that taking the action is an option.
    pub use_frequency: f32,

    /// The title to display for this action
    pub title: String,

    /// Whether this is the current action
    pub is_current_action: bool,

    /// The sound to play when this action is performed
    pub unciv_sound: UncivSound,

    /// The unique associated with this action, if any
    #[serde(skip)]
    pub associated_unique: Option<Unique>,

    /// The action to perform when this action is selected
    /// Action is None if this unit *can* execute the action but *not right now* - it's embarked, out of moves, etc
    #[serde(skip)]
    pub action: Option<Box<dyn Fn() -> ()>>,
}

impl UnitAction {
    /// Creates a new UnitAction
    pub fn new(
        action_type: UnitActionType,
        use_frequency: f32,
        title: Option<String>,
        is_current_action: bool,
        unciv_sound: Option<UncivSound>,
        associated_unique: Option<Unique>,
        action: Option<Box<dyn Fn() -> ()>>,
    ) -> Self {
        UnitAction {
            action_type,
            use_frequency,
            title: title.unwrap_or_else(|| action_type.value().to_string()),
            is_current_action,
            unciv_sound: unciv_sound.unwrap_or_else(|| action_type.unciv_sound()),
            associated_unique,
            action,
        }
    }

    /// Gets the icon for this action
    pub fn get_icon(&self) -> String {
        // In a real implementation, this would return an Actor
        // For now, we'll just return a string representation
        match self.action_type {
            UnitActionType::CreateImprovement => {
                // In a real implementation, this would get the improvement portrait
                format!("Improvement: {}", self.title.split_whitespace().next().unwrap_or(""))
            },
            UnitActionType::SpreadReligion => {
                // In a real implementation, this would get the religion portrait
                let religion_name = self.title.split_whitespace().nth(2).unwrap_or("Pantheon");
                format!("Religion: {}", religion_name)
            },
            _ => "Star".to_string(),
        }
    }
}

impl PartialEq for UnitAction {
    fn eq(&self, other: &Self) -> bool {
        self.action_type == other.action_type &&
        self.is_current_action == other.is_current_action &&
        // We can't compare the action directly, so we'll just compare the pointers
        std::ptr::eq(self.action.as_ref().map(|a| a.as_ref() as *const _).unwrap_or(std::ptr::null()),
                     other.action.as_ref().map(|a| a.as_ref() as *const _).unwrap_or(std::ptr::null()))
    }
}

impl Eq for UnitAction {}

impl Hash for UnitAction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.action_type.hash(state);
        self.is_current_action.hash(state);
        // We can't hash the action directly, so we'll just hash the pointer
        self.action.as_ref().map(|a| a.as_ref() as *const _).unwrap_or(std::ptr::null()).hash(state);
    }
}

impl fmt::Debug for UnitAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UnitAction(action_type={:?}, title='{}', is_current_action={})",
               self.action_type, self.title, self.is_current_action)
    }
}

impl fmt::Display for UnitAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.title)
    }
}

/// Specialized UnitAction for upgrades
///
/// Transports unit_to_upgrade_to from creation to UI
#[derive(Clone, Serialize, Deserialize)]
pub struct UpgradeUnitAction {
    /// The base UnitAction
    pub base_action: UnitAction,

    /// The unit to upgrade to
    pub unit_to_upgrade_to: BaseUnit,

    /// The gold cost of the upgrade
    pub gold_cost_of_upgrade: i32,

    /// The new resource requirements
    pub new_resource_requirements: Counter<String>,
}

impl UpgradeUnitAction {
    /// Creates a new UpgradeUnitAction
    pub fn new(
        title: String,
        unit_to_upgrade_to: BaseUnit,
        gold_cost_of_upgrade: i32,
        new_resource_requirements: Counter<String>,
        action: Option<Box<dyn Fn() -> ()>>,
    ) -> Self {
        UpgradeUnitAction {
            base_action: UnitAction::new(
                UnitActionType::Upgrade,
                120.0,
                Some(title),
                false,
                None,
                None,
                action,
            ),
            unit_to_upgrade_to,
            gold_cost_of_upgrade,
            new_resource_requirements,
        }
    }
}

impl std::ops::Deref for UpgradeUnitAction {
    type Target = UnitAction;

    fn deref(&self) -> &Self::Target {
        &self.base_action
    }
}