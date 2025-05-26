use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};


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
    Attack,
    Move,
    Heal,
    Build,
    Capture,
    SensorSweep,
    Deploy,
    UnDeploy,
    UseAbility,
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
            UnitActionType::Repair => "Repair a structure",
            UnitActionType::CreateImprovement => "Create",
            UnitActionType::HurryResearch => "Hurry Research",
            UnitActionType::HurryPolicy => "Hurry Policy",
            UnitActionType::HurryWonder => "Hurry Wonder",
            UnitActionType::HurryBuilding => "Hurry Construction",
            UnitActionType::ConductTradeMission => "Conduct Trade Mission",
            UnitActionType::FoundReligion => "Found a Religion",
            UnitActionType::SpreadReligion => "Spread Religion",
            UnitActionType::RemoveHeresy => "Remove Heresy",
            UnitActionType::EnhanceReligion => "Enhance a Religion",
            UnitActionType::DisbandUnit => "Disband unit",
            UnitActionType::GiftUnit => "Gift unit",
            UnitActionType::Skip => "Skip turn",
            UnitActionType::ShowAdditionalActions => "Show more",
            UnitActionType::HideAdditionalActions => "Back",
            UnitActionType::AddInCapital => "Add in capital",
            UnitActionType::Attack => "Attack selected target",
            UnitActionType::Move => "Move to another tile",
            UnitActionType::Heal => "Heal a unit",
            UnitActionType::Build => "Build a structure",
            UnitActionType::Capture => "Capture selected target",
            UnitActionType::SensorSweep => "Perform a sensor sweep",
            UnitActionType::Deploy => "Deploy unit",
            UnitActionType::UnDeploy => "Undeploy unit",
            UnitActionType::UseAbility => "Use a special ability",
        }
    }


}

/// Unit Actions - struct - carries dynamic data and actual execution.
/// Static properties are in [UnitActionType].
/// Note this is for the buttons offering actions, not the ongoing action stored with a MapUnit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitAction {
    /// The type of action
    pub action_type: UnitActionType,
    /// the sound to play
    pub sound: String,
    /// The title to display for this action
    pub title: String,
    /// unique identifier
    pub id: String,
    /// name of the action
    pub name: String,
    /// description of the action
    pub description: String,
    /// image of the action
    pub image: String,
    /// How often this action is used, a higher value means more often and that it should be on an earlier page.
    /// 100 is very frequent, 50 is somewhat frequent, less than 25 is press one time for multi-turn movement.
    /// A Rare case is > 100 if a button is something like add in capital, promote or something,
    /// we need to inform the player that taking the action is an option.
    pub display_priority: u32,
    pub keyboard_binding: KeyboardBinding,
    pub duration: u32, // 
    pub action_point_cost: u32, // cost in action points to perform this action
}

impl UnitAction {
    pub fn new (
        action_type: UnitActionType,
        sound: &str,
        tile: &str,
        id: &str,
        name: &str,
        description: &str,
        image: &str,
        display_priority: u32,
        duration: u32,
        action_point_cost: u32,
    ) -> Self {
        Self {
            action_type,
            sound: sound.to_string(),
            title: tile.to_string(),
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            image: image.to_string(),
            display_priority,
            keyboard_binding: KeyboardBinding::None,
            duration,
            action_point_cost,
        }
    }
}

