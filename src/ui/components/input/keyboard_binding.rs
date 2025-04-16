use std::collections::HashMap;
use regex::Regex;
use lazy_static::lazy_static;

/// This is the database of supported "bindable" keyboard shortcuts.
///
/// Note a label is automatically generated from the name by inserting spaces before each uppercase letter (except the initial one),
/// and translation keys are automatically generated for all labels. This also works for [Category].
///
/// Label entries containing a placeholder need special treatment - see [get_translation_entries] and update it when adding more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyboardBinding {
    // None
    None,

    // MainMenu
    QuitMainMenu,
    Resume,
    Quickstart,
    StartNewGame,
    MainMenuLoad,
    Multiplayer,
    MapEditor,
    ModManager,
    Scenarios,
    MainMenuOptions,

    // Worldscreen
    DeselectOrQuit,
    Menu,
    NextTurn,
    NextTurnAlternate,
    AutoPlayMenu,
    AutoPlay,
    EmpireOverview,
    MusicPlayer,
    DeveloperConsole,
    PrevIdleButton,
    NextIdleButton,
    Cycle,

    // Civ5-inspired bindings
    Civilopedia,
    EmpireOverviewTrades,
    EmpireOverviewUnits,
    EmpireOverviewPolitics,
    SocialPolicies,
    TechnologyTree,
    EmpireOverviewNotifications,
    VictoryScreen,
    EmpireOverviewStats,
    EmpireOverviewResources,
    QuickSave,
    QuickLoad,
    ViewCapitalCity,
    Options,
    SaveGame,
    LoadGame,
    ToggleResourceDisplay,
    ToggleYieldDisplay,
    QuitGame,
    NewGame,
    Diplomacy,
    Espionage,
    Undo,
    ToggleUI,
    ToggleWorkedTilesDisplay,
    ToggleMovementDisplay,
    ZoomIn,
    ZoomOut,

    // Map Panning
    PanUp,
    PanLeft,
    PanDown,
    PanRight,
    PanUpAlternate,
    PanLeftAlternate,
    PanDownAlternate,
    PanRightAlternate,

    // Unit actions
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
    Create,
    HurryResearch,
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

    // AutoPlayMenu
    AutoPlayMenuEndTurn,
    AutoPlayMenuMilitary,
    AutoPlayMenuCivilians,
    AutoPlayMenuEconomy,

    // NextTurnMenu
    NextTurnMenuNextTurn,
    NextTurnMenuMoveAutomatedUnits,

    // City Screen
    AddConstruction,
    RaisePriority,
    LowerPriority,
    BuyConstruction,
    BuyTile,
    BuildUnits,
    BuildBuildings,
    BuildWonders,
    BuildNationalWonders,
    BuildOther,
    BuildDisabled,
    NextCity,
    PreviousCity,
    ShowStats,
    ShowStatDetails,
    CitizenManagement,
    GreatPeopleDetail,
    SpecialistDetail,
    ReligionDetail,
    BuildingsDetail,
    ResetCitizens,
    AvoidGrowth,
    NoFocus,
    FoodFocus,
    ProductionFocus,
    GoldFocus,
    ScienceFocus,
    CultureFocus,
    FaithFocus,

    // CityScreenConstructionMenu
    AddConstructionTop,
    AddConstructionAll,
    AddConstructionAllTop,
    RemoveConstructionAll,

    // Civilopedia
    PediaBuildings,
    PediaWonders,
    PediaResources,
    PediaTerrains,
    PediaImprovements,
    PediaUnits,
    PediaUnitTypes,
    PediaNations,
    PediaTechnologies,
    PediaPromotions,
    PediaPolicies,
    PediaBeliefs,
    PediaTutorials,
    PediaDifficulties,
    PediaEras,
    PediaSpeeds,
    PediaSearch,

    // Popups
    Confirm,
    Cancel,
    UpgradeAll,
}

/// Categories for keyboard bindings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    None,
    MainMenu,
    WorldScreen,
    AutoPlayMenu,
    NextTurnMenu,
    MapPanning,
    UnitActions,
    CityScreen,
    CityScreenConstructionMenu,
    Civilopedia,
    Popups,
}

impl Category {
    /// Get the label for this category
    pub fn label(&self) -> &'static str {
        match self {
            Category::None => "None",
            Category::MainMenu => "Main Menu",
            Category::WorldScreen => "World Screen",
            Category::AutoPlayMenu => "AutoPlay menu",
            Category::NextTurnMenu => "NextTurn menu",
            Category::MapPanning => "Map Panning",
            Category::UnitActions => "Unit Actions",
            Category::CityScreen => "City Screen",
            Category::CityScreenConstructionMenu => "City Screen Construction Menu",
            Category::Civilopedia => "Civilopedia",
            Category::Popups => "Popups",
        }
    }

    /// Get the categories to check for conflicts
    pub fn check_conflicts_in(&self) -> Vec<Category> {
        match self {
            Category::WorldScreen => vec![Category::WorldScreen, Category::MapPanning, Category::UnitActions],
            Category::MapPanning => vec![Category::MapPanning, Category::WorldScreen],
            Category::UnitActions => vec![Category::WorldScreen],
            _ => vec![*self],
        }
    }
}

/// Represents a key character and code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCharAndCode {
    /// The character representation of the key
    pub char: Option<char>,
    /// The key code
    pub code: i32,
}

impl KeyCharAndCode {
    /// Create a new KeyCharAndCode with the given character
    pub fn from_char(c: char) -> Self {
        Self {
            char: Some(c),
            code: c as i32,
        }
    }

    /// Create a new KeyCharAndCode with the given key code
    pub fn from_code(code: i32) -> Self {
        Self {
            char: None,
            code,
        }
    }

    /// Create a new KeyCharAndCode with the given character and code
    pub fn new(c: char, code: i32) -> Self {
        Self {
            char: Some(c),
            code,
        }
    }

    /// Create a new KeyCharAndCode with Ctrl+character
    pub fn ctrl(c: char) -> Self {
        Self {
            char: Some(c),
            code: c as i32,
        }
    }

    /// Unknown key
    pub const UNKNOWN: Self = Self {
        char: None,
        code: -1,
    };

    /// Backspace key
    pub const BACK: Self = Self {
        char: None,
        code: 8,
    };

    /// Tab key
    pub const TAB: Self = Self {
        char: None,
        code: 9,
    };

    /// Return key
    pub const RETURN: Self = Self {
        char: None,
        code: 13,
    };

    /// Delete key
    pub const DEL: Self = Self {
        char: None,
        code: 127,
    };

    /// Space key
    pub const SPACE: Self = Self {
        char: Some(' '),
        code: 32,
    };
}

impl KeyboardBinding {
    /// Get the category for this binding
    pub fn category(&self) -> Category {
        match self {
            // None
            KeyboardBinding::None => Category::None,

            // MainMenu
            KeyboardBinding::QuitMainMenu | KeyboardBinding::Resume | KeyboardBinding::Quickstart |
            KeyboardBinding::StartNewGame | KeyboardBinding::MainMenuLoad | KeyboardBinding::Multiplayer |
            KeyboardBinding::MapEditor | KeyboardBinding::ModManager | KeyboardBinding::Scenarios |
            KeyboardBinding::MainMenuOptions => Category::MainMenu,

            // Worldscreen
            KeyboardBinding::DeselectOrQuit | KeyboardBinding::Menu | KeyboardBinding::NextTurn |
            KeyboardBinding::NextTurnAlternate | KeyboardBinding::AutoPlayMenu | KeyboardBinding::AutoPlay |
            KeyboardBinding::EmpireOverview | KeyboardBinding::MusicPlayer | KeyboardBinding::DeveloperConsole |
            KeyboardBinding::PrevIdleButton | KeyboardBinding::NextIdleButton | KeyboardBinding::Cycle |
            KeyboardBinding::Civilopedia | KeyboardBinding::EmpireOverviewTrades | KeyboardBinding::EmpireOverviewUnits |
            KeyboardBinding::EmpireOverviewPolitics | KeyboardBinding::SocialPolicies | KeyboardBinding::TechnologyTree |
            KeyboardBinding::EmpireOverviewNotifications | KeyboardBinding::VictoryScreen | KeyboardBinding::EmpireOverviewStats |
            KeyboardBinding::EmpireOverviewResources | KeyboardBinding::QuickSave | KeyboardBinding::QuickLoad |
            KeyboardBinding::ViewCapitalCity | KeyboardBinding::Options | KeyboardBinding::SaveGame |
            KeyboardBinding::LoadGame | KeyboardBinding::ToggleResourceDisplay | KeyboardBinding::ToggleYieldDisplay |
            KeyboardBinding::QuitGame | KeyboardBinding::NewGame | KeyboardBinding::Diplomacy |
            KeyboardBinding::Espionage | KeyboardBinding::Undo | KeyboardBinding::ToggleUI |
            KeyboardBinding::ToggleWorkedTilesDisplay | KeyboardBinding::ToggleMovementDisplay |
            KeyboardBinding::ZoomIn | KeyboardBinding::ZoomOut => Category::WorldScreen,

            // Map Panning
            KeyboardBinding::PanUp | KeyboardBinding::PanLeft | KeyboardBinding::PanDown |
            KeyboardBinding::PanRight | KeyboardBinding::PanUpAlternate | KeyboardBinding::PanLeftAlternate |
            KeyboardBinding::PanDownAlternate | KeyboardBinding::PanRightAlternate => Category::MapPanning,

            // Unit actions
            KeyboardBinding::SwapUnits | KeyboardBinding::Automate | KeyboardBinding::ConnectRoad |
            KeyboardBinding::StopAutomation | KeyboardBinding::StopMovement | KeyboardBinding::ShowUnitDestination |
            KeyboardBinding::Sleep | KeyboardBinding::SleepUntilHealed | KeyboardBinding::Fortify |
            KeyboardBinding::FortifyUntilHealed | KeyboardBinding::Explore | KeyboardBinding::StopExploration |
            KeyboardBinding::Promote | KeyboardBinding::Upgrade | KeyboardBinding::Transform |
            KeyboardBinding::Pillage | KeyboardBinding::Paradrop | KeyboardBinding::AirSweep |
            KeyboardBinding::SetUp | KeyboardBinding::FoundCity | KeyboardBinding::ConstructImprovement |
            KeyboardBinding::Repair | KeyboardBinding::Create | KeyboardBinding::HurryResearch |
            KeyboardBinding::HurryWonder | KeyboardBinding::HurryBuilding | KeyboardBinding::ConductTradeMission |
            KeyboardBinding::FoundReligion | KeyboardBinding::TriggerUnique | KeyboardBinding::SpreadReligion |
            KeyboardBinding::RemoveHeresy | KeyboardBinding::EnhanceReligion | KeyboardBinding::DisbandUnit |
            KeyboardBinding::GiftUnit | KeyboardBinding::Skip | KeyboardBinding::ShowAdditionalActions |
            KeyboardBinding::HideAdditionalActions | KeyboardBinding::AddInCapital => Category::UnitActions,

            // AutoPlayMenu
            KeyboardBinding::AutoPlayMenuEndTurn | KeyboardBinding::AutoPlayMenuMilitary |
            KeyboardBinding::AutoPlayMenuCivilians | KeyboardBinding::AutoPlayMenuEconomy => Category::AutoPlayMenu,

            // NextTurnMenu
            KeyboardBinding::NextTurnMenuNextTurn | KeyboardBinding::NextTurnMenuMoveAutomatedUnits => Category::NextTurnMenu,

            // City Screen
            KeyboardBinding::AddConstruction | KeyboardBinding::RaisePriority | KeyboardBinding::LowerPriority |
            KeyboardBinding::BuyConstruction | KeyboardBinding::BuyTile | KeyboardBinding::BuildUnits |
            KeyboardBinding::BuildBuildings | KeyboardBinding::BuildWonders | KeyboardBinding::BuildNationalWonders |
            KeyboardBinding::BuildOther | KeyboardBinding::BuildDisabled | KeyboardBinding::NextCity |
            KeyboardBinding::PreviousCity | KeyboardBinding::ShowStats | KeyboardBinding::ShowStatDetails |
            KeyboardBinding::CitizenManagement | KeyboardBinding::GreatPeopleDetail | KeyboardBinding::SpecialistDetail |
            KeyboardBinding::ReligionDetail | KeyboardBinding::BuildingsDetail | KeyboardBinding::ResetCitizens |
            KeyboardBinding::AvoidGrowth | KeyboardBinding::NoFocus | KeyboardBinding::FoodFocus |
            KeyboardBinding::ProductionFocus | KeyboardBinding::GoldFocus | KeyboardBinding::ScienceFocus |
            KeyboardBinding::CultureFocus | KeyboardBinding::FaithFocus => Category::CityScreen,

            // CityScreenConstructionMenu
            KeyboardBinding::AddConstructionTop | KeyboardBinding::AddConstructionAll |
            KeyboardBinding::AddConstructionAllTop | KeyboardBinding::RemoveConstructionAll => Category::CityScreenConstructionMenu,

            // Civilopedia
            KeyboardBinding::PediaBuildings | KeyboardBinding::PediaWonders | KeyboardBinding::PediaResources |
            KeyboardBinding::PediaTerrains | KeyboardBinding::PediaImprovements | KeyboardBinding::PediaUnits |
            KeyboardBinding::PediaUnitTypes | KeyboardBinding::PediaNations | KeyboardBinding::PediaTechnologies |
            KeyboardBinding::PediaPromotions | KeyboardBinding::PediaPolicies | KeyboardBinding::PediaBeliefs |
            KeyboardBinding::PediaTutorials | KeyboardBinding::PediaDifficulties | KeyboardBinding::PediaEras |
            KeyboardBinding::PediaSpeeds | KeyboardBinding::PediaSearch => Category::Civilopedia,

            // Popups
            KeyboardBinding::Confirm | KeyboardBinding::Cancel | KeyboardBinding::UpgradeAll => Category::Popups,
        }
    }

    /// Get the label for this binding
    pub fn label(&self) -> &'static str {
        match self {
            // None
            KeyboardBinding::None => "None",

            // MainMenu
            KeyboardBinding::QuitMainMenu => "Quit",
            KeyboardBinding::Resume => "Resume",
            KeyboardBinding::Quickstart => "Quickstart",
            KeyboardBinding::StartNewGame => "Start new game",
            KeyboardBinding::MainMenuLoad => "Load game",
            KeyboardBinding::Multiplayer => "Multiplayer",
            KeyboardBinding::MapEditor => "Map editor",
            KeyboardBinding::ModManager => "Mods",
            KeyboardBinding::Scenarios => "Scenarios",
            KeyboardBinding::MainMenuOptions => "Options",

            // Worldscreen
            KeyboardBinding::DeselectOrQuit => "Deselect then Quit",
            KeyboardBinding::Menu => "Menu",
            KeyboardBinding::NextTurn => "Next Turn",
            KeyboardBinding::NextTurnAlternate => "Next Turn",
            KeyboardBinding::AutoPlayMenu => "Open AutoPlay menu",
            KeyboardBinding::AutoPlay => "Start AutoPlay",
            KeyboardBinding::EmpireOverview => "Empire Overview",
            KeyboardBinding::MusicPlayer => "Music Player",
            KeyboardBinding::DeveloperConsole => "Developer Console",
            KeyboardBinding::PrevIdleButton => "Idle Prev",
            KeyboardBinding::NextIdleButton => "Idle Next",
            KeyboardBinding::Cycle => "Cycle",
            KeyboardBinding::Civilopedia => "Civilopedia",
            KeyboardBinding::EmpireOverviewTrades => "Economic info",
            KeyboardBinding::EmpireOverviewUnits => "Military info",
            KeyboardBinding::EmpireOverviewPolitics => "Diplomacy info",
            KeyboardBinding::SocialPolicies => "Social Policies Screen",
            KeyboardBinding::TechnologyTree => "Tech Screen",
            KeyboardBinding::EmpireOverviewNotifications => "Notification Log",
            KeyboardBinding::VictoryScreen => "Victory status",
            KeyboardBinding::EmpireOverviewStats => "Demographics",
            KeyboardBinding::EmpireOverviewResources => "Strategic View",
            KeyboardBinding::QuickSave => "Quick Save",
            KeyboardBinding::QuickLoad => "Quick Load",
            KeyboardBinding::ViewCapitalCity => "Capital City View",
            KeyboardBinding::Options => "Game Options",
            KeyboardBinding::SaveGame => "Save",
            KeyboardBinding::LoadGame => "Load",
            KeyboardBinding::ToggleResourceDisplay => "Show Resources Icons",
            KeyboardBinding::ToggleYieldDisplay => "Yield Icons",
            KeyboardBinding::QuitGame => "Quit Game",
            KeyboardBinding::NewGame => "New Game",
            KeyboardBinding::Diplomacy => "Diplomacy",
            KeyboardBinding::Espionage => "Espionage",
            KeyboardBinding::Undo => "Undo",
            KeyboardBinding::ToggleUI => "Toggle UI",
            KeyboardBinding::ToggleWorkedTilesDisplay => "Toggle Worked Tiles Display",
            KeyboardBinding::ToggleMovementDisplay => "Toggle Movement Display",
            KeyboardBinding::ZoomIn => "Zoom In",
            KeyboardBinding::ZoomOut => "Zoom Out",

            // Map Panning
            KeyboardBinding::PanUp => "Pan Up",
            KeyboardBinding::PanLeft => "Pan Left",
            KeyboardBinding::PanDown => "Pan Down",
            KeyboardBinding::PanRight => "Pan Right",
            KeyboardBinding::PanUpAlternate => "Pan Up",
            KeyboardBinding::PanLeftAlternate => "Pan Left",
            KeyboardBinding::PanDownAlternate => "Pan Down",
            KeyboardBinding::PanRightAlternate => "Pan Right",

            // Unit actions
            KeyboardBinding::SwapUnits => "Swap units",
            KeyboardBinding::Automate => "Automate",
            KeyboardBinding::ConnectRoad => "Connect road",
            KeyboardBinding::StopAutomation => "Stop automation",
            KeyboardBinding::StopMovement => "Stop movement",
            KeyboardBinding::ShowUnitDestination => "Show unit destination",
            KeyboardBinding::Sleep => "Sleep",
            KeyboardBinding::SleepUntilHealed => "Sleep until healed",
            KeyboardBinding::Fortify => "Fortify",
            KeyboardBinding::FortifyUntilHealed => "Fortify until healed",
            KeyboardBinding::Explore => "Explore",
            KeyboardBinding::StopExploration => "Stop exploration",
            KeyboardBinding::Promote => "Promote",
            KeyboardBinding::Upgrade => "Upgrade",
            KeyboardBinding::Transform => "Transform",
            KeyboardBinding::Pillage => "Pillage",
            KeyboardBinding::Paradrop => "Paradrop",
            KeyboardBinding::AirSweep => "Air Sweep",
            KeyboardBinding::SetUp => "Set up",
            KeyboardBinding::FoundCity => "Found city",
            KeyboardBinding::ConstructImprovement => "Construct improvement",
            KeyboardBinding::Repair => "Repair",
            KeyboardBinding::Create => "Create",
            KeyboardBinding::HurryResearch => "Hurry Research",
            KeyboardBinding::HurryWonder => "Hurry Wonder",
            KeyboardBinding::HurryBuilding => "Hurry Construction",
            KeyboardBinding::ConductTradeMission => "Conduct Trade Mission",
            KeyboardBinding::FoundReligion => "Found a Religion",
            KeyboardBinding::TriggerUnique => "Trigger unique",
            KeyboardBinding::SpreadReligion => "Spread Religion",
            KeyboardBinding::RemoveHeresy => "Remove Heresy",
            KeyboardBinding::EnhanceReligion => "Enhance a Religion",
            KeyboardBinding::DisbandUnit => "Disband unit",
            KeyboardBinding::GiftUnit => "Gift unit",
            KeyboardBinding::Skip => "Skip",
            KeyboardBinding::ShowAdditionalActions => "Show more",
            KeyboardBinding::HideAdditionalActions => "Back",
            KeyboardBinding::AddInCapital => "Add in capital",

            // AutoPlayMenu
            KeyboardBinding::AutoPlayMenuEndTurn => "AutoPlay End Turn",
            KeyboardBinding::AutoPlayMenuMilitary => "AutoPlay Military Once",
            KeyboardBinding::AutoPlayMenuCivilians => "AutoPlay Civilians Once",
            KeyboardBinding::AutoPlayMenuEconomy => "AutoPlay Economy Once",

            // NextTurnMenu
            KeyboardBinding::NextTurnMenuNextTurn => "Next Turn",
            KeyboardBinding::NextTurnMenuMoveAutomatedUnits => "Move Automated Units",

            // City Screen
            KeyboardBinding::AddConstruction => "Add to or remove from queue",
            KeyboardBinding::RaisePriority => "Raise queue priority",
            KeyboardBinding::LowerPriority => "Lower queue priority",
            KeyboardBinding::BuyConstruction => "Buy Construction",
            KeyboardBinding::BuyTile => "Buy Tile",
            KeyboardBinding::BuildUnits => "Buildable Units",
            KeyboardBinding::BuildBuildings => "Buildable Buildings",
            KeyboardBinding::BuildWonders => "Buildable Wonders",
            KeyboardBinding::BuildNationalWonders => "Buildable National Wonders",
            KeyboardBinding::BuildOther => "Other Constructions",
            KeyboardBinding::BuildDisabled => "Disabled Constructions",
            KeyboardBinding::NextCity => "Next City",
            KeyboardBinding::PreviousCity => "Previous City",
            KeyboardBinding::ShowStats => "Show Stats",
            KeyboardBinding::ShowStatDetails => "Toggle Stat Details",
            KeyboardBinding::CitizenManagement => "Citizen Management",
            KeyboardBinding::GreatPeopleDetail => "Great People Detail",
            KeyboardBinding::SpecialistDetail => "Specialist Detail",
            KeyboardBinding::ReligionDetail => "Religion Detail",
            KeyboardBinding::BuildingsDetail => "Buildings Detail",
            KeyboardBinding::ResetCitizens => "Reset Citizens",
            KeyboardBinding::AvoidGrowth => "Avoid Growth",
            KeyboardBinding::NoFocus => "Default Focus",
            KeyboardBinding::FoodFocus => "[Food] Focus",
            KeyboardBinding::ProductionFocus => "[Production] Focus",
            KeyboardBinding::GoldFocus => "[Gold] Focus",
            KeyboardBinding::ScienceFocus => "[Science] Focus",
            KeyboardBinding::CultureFocus => "[Culture] Focus",
            KeyboardBinding::FaithFocus => "[Faith] Focus",

            // CityScreenConstructionMenu
            KeyboardBinding::AddConstructionTop => "Add to the top of the queue",
            KeyboardBinding::AddConstructionAll => "Add to the queue in all cities",
            KeyboardBinding::AddConstructionAllTop => "Add or move to the top in all cities",
            KeyboardBinding::RemoveConstructionAll => "Remove from the queue in all cities",

            // Civilopedia
            KeyboardBinding::PediaBuildings => "Buildings",
            KeyboardBinding::PediaWonders => "Wonders",
            KeyboardBinding::PediaResources => "Resources",
            KeyboardBinding::PediaTerrains => "Terrains",
            KeyboardBinding::PediaImprovements => "Tile Improvements",
            KeyboardBinding::PediaUnits => "Units",
            KeyboardBinding::PediaUnitTypes => "Unit types",
            KeyboardBinding::PediaNations => "Nations",
            KeyboardBinding::PediaTechnologies => "Technologies",
            KeyboardBinding::PediaPromotions => "Promotions",
            KeyboardBinding::PediaPolicies => "Policies",
            KeyboardBinding::PediaBeliefs => "Religions and Beliefs",
            KeyboardBinding::PediaTutorials => "Tutorials",
            KeyboardBinding::PediaDifficulties => "Difficulty levels",
            KeyboardBinding::PediaEras => "Eras",
            KeyboardBinding::PediaSpeeds => "Speeds",
            KeyboardBinding::PediaSearch => "Open the Search Dialog",

            // Popups
            KeyboardBinding::Confirm => "Confirm Dialog",
            KeyboardBinding::Cancel => "Cancel Dialog",
            KeyboardBinding::UpgradeAll => "Upgrade All",
        }
    }

    /// Get the default key for this binding
    pub fn default_key(&self) -> KeyCharAndCode {
        match self {
            // None
            KeyboardBinding::None => KeyCharAndCode::UNKNOWN,

            // MainMenu
            KeyboardBinding::QuitMainMenu => KeyCharAndCode::BACK,
            KeyboardBinding::Resume => KeyCharAndCode::from_char('R'),
            KeyboardBinding::Quickstart => KeyCharAndCode::from_char('Q'),
            KeyboardBinding::StartNewGame => KeyCharAndCode::from_char('N'),
            KeyboardBinding::MainMenuLoad => KeyCharAndCode::from_char('L'),
            KeyboardBinding::Multiplayer => KeyCharAndCode::from_char('M'),
            KeyboardBinding::MapEditor => KeyCharAndCode::from_char('E'),
            KeyboardBinding::ModManager => KeyCharAndCode::from_char('D'),
            KeyboardBinding::Scenarios => KeyCharAndCode::from_char('S'),
            KeyboardBinding::MainMenuOptions => KeyCharAndCode::from_char('O'),

            // Worldscreen
            KeyboardBinding::DeselectOrQuit => KeyCharAndCode::BACK,
            KeyboardBinding::Menu => KeyCharAndCode::TAB,
            KeyboardBinding::NextTurn => KeyCharAndCode::from_char('N'),
            KeyboardBinding::NextTurnAlternate => KeyCharAndCode::SPACE,
            KeyboardBinding::AutoPlayMenu => KeyCharAndCode::UNKNOWN,
            KeyboardBinding::AutoPlay => KeyCharAndCode::ctrl('a'),
            KeyboardBinding::EmpireOverview => KeyCharAndCode::from_char('E'),
            KeyboardBinding::MusicPlayer => KeyCharAndCode::ctrl('m'),
            KeyboardBinding::DeveloperConsole => KeyCharAndCode::from_char('`'),
            KeyboardBinding::PrevIdleButton => KeyCharAndCode::from_char(','),
            KeyboardBinding::NextIdleButton => KeyCharAndCode::from_char('.'),
            KeyboardBinding::Cycle => KeyCharAndCode::from_char(';'),
            KeyboardBinding::Civilopedia => KeyCharAndCode::from_code(59), // F1
            KeyboardBinding::EmpireOverviewTrades => KeyCharAndCode::from_code(60), // F2
            KeyboardBinding::EmpireOverviewUnits => KeyCharAndCode::from_code(61), // F3
            KeyboardBinding::EmpireOverviewPolitics => KeyCharAndCode::from_code(62), // F4
            KeyboardBinding::SocialPolicies => KeyCharAndCode::from_code(63), // F5
            KeyboardBinding::TechnologyTree => KeyCharAndCode::from_code(64), // F6
            KeyboardBinding::EmpireOverviewNotifications => KeyCharAndCode::from_code(65), // F7
            KeyboardBinding::VictoryScreen => KeyCharAndCode::from_code(66), // F8
            KeyboardBinding::EmpireOverviewStats => KeyCharAndCode::from_code(67), // F9
            KeyboardBinding::EmpireOverviewResources => KeyCharAndCode::from_code(68), // F10
            KeyboardBinding::QuickSave => KeyCharAndCode::from_code(87), // F11
            KeyboardBinding::QuickLoad => KeyCharAndCode::from_code(88), // F12
            KeyboardBinding::ViewCapitalCity => KeyCharAndCode::from_code(36), // HOME
            KeyboardBinding::Options => KeyCharAndCode::ctrl('o'),
            KeyboardBinding::SaveGame => KeyCharAndCode::ctrl('s'),
            KeyboardBinding::LoadGame => KeyCharAndCode::ctrl('l'),
            KeyboardBinding::ToggleResourceDisplay => KeyCharAndCode::ctrl('r'),
            KeyboardBinding::ToggleYieldDisplay => KeyCharAndCode::ctrl('y'),
            KeyboardBinding::QuitGame => KeyCharAndCode::ctrl('q'),
            KeyboardBinding::NewGame => KeyCharAndCode::ctrl('n'),
            KeyboardBinding::Diplomacy => KeyCharAndCode::UNKNOWN,
            KeyboardBinding::Espionage => KeyCharAndCode::UNKNOWN,
            KeyboardBinding::Undo => KeyCharAndCode::ctrl('z'),
            KeyboardBinding::ToggleUI => KeyCharAndCode::ctrl('u'),
            KeyboardBinding::ToggleWorkedTilesDisplay => KeyCharAndCode::UNKNOWN,
            KeyboardBinding::ToggleMovementDisplay => KeyCharAndCode::UNKNOWN,
            KeyboardBinding::ZoomIn => KeyCharAndCode::from_code(78), // NUMPAD_ADD
            KeyboardBinding::ZoomOut => KeyCharAndCode::from_code(74), // NUMPAD_SUBTRACT

            // Map Panning
            KeyboardBinding::PanUp => KeyCharAndCode::from_code(19), // UP
            KeyboardBinding::PanLeft => KeyCharAndCode::from_code(21), // LEFT
            KeyboardBinding::PanDown => KeyCharAndCode::from_code(20), // DOWN
            KeyboardBinding::PanRight => KeyCharAndCode::from_code(22), // RIGHT
            KeyboardBinding::PanUpAlternate => KeyCharAndCode::from_char('W'),
            KeyboardBinding::PanLeftAlternate => KeyCharAndCode::from_char('A'),
            KeyboardBinding::PanDownAlternate => KeyCharAndCode::from_char('S'),
            KeyboardBinding::PanRightAlternate => KeyCharAndCode::from_char('D'),

            // Unit actions
            KeyboardBinding::SwapUnits => KeyCharAndCode::from_char('y'),
            KeyboardBinding::Automate => KeyCharAndCode::from_char('m'),
            KeyboardBinding::ConnectRoad => KeyCharAndCode::from_char('c'),
            KeyboardBinding::StopAutomation => KeyCharAndCode::from_char('m'),
            KeyboardBinding::StopMovement => KeyCharAndCode::from_char('m'),
            KeyboardBinding::ShowUnitDestination => KeyCharAndCode::from_char('j'),
            KeyboardBinding::Sleep => KeyCharAndCode::from_char('f'),
            KeyboardBinding::SleepUntilHealed => KeyCharAndCode::from_char('h'),
            KeyboardBinding::Fortify => KeyCharAndCode::from_char('f'),
            KeyboardBinding::FortifyUntilHealed => KeyCharAndCode::from_char('h'),
            KeyboardBinding::Explore => KeyCharAndCode::from_char('x'),
            KeyboardBinding::StopExploration => KeyCharAndCode::from_char('x'),
            KeyboardBinding::Promote => KeyCharAndCode::from_char('o'),
            KeyboardBinding::Upgrade => KeyCharAndCode::from_char('u'),
            KeyboardBinding::Transform => KeyCharAndCode::from_char('k'),
            KeyboardBinding::Pillage => KeyCharAndCode::from_char('p'),
            KeyboardBinding::Paradrop => KeyCharAndCode::from_char('p'),
            KeyboardBinding::AirSweep => KeyCharAndCode::from_char('a'),
            KeyboardBinding::SetUp => KeyCharAndCode::from_char('t'),
            KeyboardBinding::FoundCity => KeyCharAndCode::from_char('c'),
            KeyboardBinding::ConstructImprovement => KeyCharAndCode::from_char('i'),
            KeyboardBinding::Repair => KeyCharAndCode::from_char('r'),
            KeyboardBinding::Create => KeyCharAndCode::from_char('i'),
            KeyboardBinding::HurryResearch => KeyCharAndCode::from_char('g'),
            KeyboardBinding::HurryWonder => KeyCharAndCode::from_char('g'),
            KeyboardBinding::HurryBuilding => KeyCharAndCode::from_char('g'),
            KeyboardBinding::ConductTradeMission => KeyCharAndCode::from_char('g'),
            KeyboardBinding::FoundReligion => KeyCharAndCode::from_char('g'),
            KeyboardBinding::TriggerUnique => KeyCharAndCode::from_char('g'),
            KeyboardBinding::SpreadReligion => KeyCharAndCode::from_char('g'),
            KeyboardBinding::RemoveHeresy => KeyCharAndCode::from_char('h'),
            KeyboardBinding::EnhanceReligion => KeyCharAndCode::from_char('g'),
            KeyboardBinding::DisbandUnit => KeyCharAndCode::DEL,
            KeyboardBinding::GiftUnit => KeyCharAndCode::UNKNOWN,
            KeyboardBinding::Skip => KeyCharAndCode::from_char('z'),
            KeyboardBinding::ShowAdditionalActions => KeyCharAndCode::from_code(77), // PAGE_DOWN
            KeyboardBinding::HideAdditionalActions => KeyCharAndCode::from_code(76), // PAGE_UP
            KeyboardBinding::AddInCapital => KeyCharAndCode::from_char('g'),

            // AutoPlayMenu
            KeyboardBinding::AutoPlayMenuEndTurn => KeyCharAndCode::from_char('t'),
            KeyboardBinding::AutoPlayMenuMilitary => KeyCharAndCode::from_char('m'),
            KeyboardBinding::AutoPlayMenuCivilians => KeyCharAndCode::from_char('c'),
            KeyboardBinding::AutoPlayMenuEconomy => KeyCharAndCode::from_char('e'),

            // NextTurnMenu
            KeyboardBinding::NextTurnMenuNextTurn => KeyCharAndCode::from_char('n'),
            KeyboardBinding::NextTurnMenuMoveAutomatedUnits => KeyCharAndCode::from_char('m'),

            // City Screen
            KeyboardBinding::AddConstruction => KeyCharAndCode::RETURN,
            KeyboardBinding::RaisePriority => KeyCharAndCode::from_code(19), // UP
            KeyboardBinding::LowerPriority => KeyCharAndCode::from_code(20), // DOWN
            KeyboardBinding::BuyConstruction => KeyCharAndCode::from_char('b'),
            KeyboardBinding::BuyTile => KeyCharAndCode::from_char('t'),
            KeyboardBinding::BuildUnits => KeyCharAndCode::from_char('u'),
            KeyboardBinding::BuildBuildings => KeyCharAndCode::from_char('l'),
            KeyboardBinding::BuildWonders => KeyCharAndCode::from_char('w'),
            KeyboardBinding::BuildNationalWonders => KeyCharAndCode::from_char('n'),
            KeyboardBinding::BuildOther => KeyCharAndCode::from_char('o'),
            KeyboardBinding::BuildDisabled => KeyCharAndCode::ctrl('h'),
            KeyboardBinding::NextCity => KeyCharAndCode::from_code(22), // RIGHT
            KeyboardBinding::PreviousCity => KeyCharAndCode::from_code(21), // LEFT
            KeyboardBinding::ShowStats => KeyCharAndCode::from_char('s'),
            KeyboardBinding::ShowStatDetails => KeyCharAndCode::from_code(78), // NUMPAD_ADD
            KeyboardBinding::CitizenManagement => KeyCharAndCode::from_char('c'),
            KeyboardBinding::GreatPeopleDetail => KeyCharAndCode::from_char('g'),
            KeyboardBinding::SpecialistDetail => KeyCharAndCode::from_char('p'),
            KeyboardBinding::ReligionDetail => KeyCharAndCode::from_char('r'),
            KeyboardBinding::BuildingsDetail => KeyCharAndCode::from_char('d'),
            KeyboardBinding::ResetCitizens => KeyCharAndCode::ctrl('r'),
            KeyboardBinding::AvoidGrowth => KeyCharAndCode::ctrl('a'),
            KeyboardBinding::NoFocus => KeyCharAndCode::ctrl('d'),
            KeyboardBinding::FoodFocus => KeyCharAndCode::ctrl('f'),
            KeyboardBinding::ProductionFocus => KeyCharAndCode::ctrl('p'),
            KeyboardBinding::GoldFocus => KeyCharAndCode::ctrl('g'),
            KeyboardBinding::ScienceFocus => KeyCharAndCode::ctrl('s'),
            KeyboardBinding::CultureFocus => KeyCharAndCode::ctrl('c'),
            KeyboardBinding::FaithFocus => KeyCharAndCode::UNKNOWN,

            // CityScreenConstructionMenu
            KeyboardBinding::AddConstructionTop => KeyCharAndCode::from_char('t'),
            KeyboardBinding::AddConstructionAll => KeyCharAndCode::ctrl('a'),
            KeyboardBinding::AddConstructionAllTop => KeyCharAndCode::ctrl('t'),
            KeyboardBinding::RemoveConstructionAll => KeyCharAndCode::ctrl('r'),

            // Civilopedia
            KeyboardBinding::PediaBuildings => KeyCharAndCode::from_char('b'),
            KeyboardBinding::PediaWonders => KeyCharAndCode::from_char('w'),
            KeyboardBinding::PediaResources => KeyCharAndCode::from_char('r'),
            KeyboardBinding::PediaTerrains => KeyCharAndCode::from_char('t'),
            KeyboardBinding::PediaImprovements => KeyCharAndCode::from_char('i'),
            KeyboardBinding::PediaUnits => KeyCharAndCode::from_char('u'),
            KeyboardBinding::PediaUnitTypes => KeyCharAndCode::from_char('y'),
            KeyboardBinding::PediaNations => KeyCharAndCode::from_char('n'),
            KeyboardBinding::PediaTechnologies => KeyCharAndCode::ctrl('t'),
            KeyboardBinding::PediaPromotions => KeyCharAndCode::from_char('p'),
            KeyboardBinding::PediaPolicies => KeyCharAndCode::from_char('o'),
            KeyboardBinding::PediaBeliefs => KeyCharAndCode::from_char('f'),
            KeyboardBinding::PediaTutorials => KeyCharAndCode::from_code(59), // F1
            KeyboardBinding::PediaDifficulties => KeyCharAndCode::from_char('d'),
            KeyboardBinding::PediaEras => KeyCharAndCode::from_char('e'),
            KeyboardBinding::PediaSpeeds => KeyCharAndCode::from_char('s'),
            KeyboardBinding::PediaSearch => KeyCharAndCode::ctrl('f'),

            // Popups
            KeyboardBinding::Confirm => KeyCharAndCode::from_char('y'),
            KeyboardBinding::Cancel => KeyCharAndCode::from_char('n'),
            KeyboardBinding::UpgradeAll => KeyCharAndCode::ctrl('a'),
        }
    }

    /// Check if this binding is hidden
    pub fn is_hidden(&self) -> bool {
        self.category() == Category::None
    }

    /// Get all translation entries
    pub fn get_translation_entries() -> Vec<String> {
        let mut entries = Vec::new();

        // Add category labels
        entries.push(Category::None.label().to_string());
        entries.push(Category::MainMenu.label().to_string());
        entries.push(Category::WorldScreen.label().to_string());
        entries.push(Category::AutoPlayMenu.label().to_string());
        entries.push(Category::NextTurnMenu.label().to_string());
        entries.push(Category::MapPanning.label().to_string());
        entries.push(Category::UnitActions.label().to_string());
        entries.push(Category::CityScreen.label().to_string());
        entries.push(Category::CityScreenConstructionMenu.label().to_string());
        entries.push(Category::Civilopedia.label().to_string());
        entries.push(Category::Popups.label().to_string());

        // Add binding labels that don't contain placeholders
        for binding in Self::values() {
            let label = binding.label();
            if !label.contains('[') {
                entries.push(label.to_string());
            }
        }

        // Add placeholder for stat focus
        entries.push("[stat] Focus".to_string());

        entries
    }

    /// Get all values of this enum
    pub fn values() -> Vec<KeyboardBinding> {
        vec![
            // None
            KeyboardBinding::None,

            // MainMenu
            KeyboardBinding::QuitMainMenu,
            KeyboardBinding::Resume,
            KeyboardBinding::Quickstart,
            KeyboardBinding::StartNewGame,
            KeyboardBinding::MainMenuLoad,
            KeyboardBinding::Multiplayer,
            KeyboardBinding::MapEditor,
            KeyboardBinding::ModManager,
            KeyboardBinding::Scenarios,
            KeyboardBinding::MainMenuOptions,

            // Worldscreen
            KeyboardBinding::DeselectOrQuit,
            KeyboardBinding::Menu,
            KeyboardBinding::NextTurn,
            KeyboardBinding::NextTurnAlternate,
            KeyboardBinding::AutoPlayMenu,
            KeyboardBinding::AutoPlay,
            KeyboardBinding::EmpireOverview,
            KeyboardBinding::MusicPlayer,
            KeyboardBinding::DeveloperConsole,
            KeyboardBinding::PrevIdleButton,
            KeyboardBinding::NextIdleButton,
            KeyboardBinding::Cycle,
            KeyboardBinding::Civilopedia,
            KeyboardBinding::EmpireOverviewTrades,
            KeyboardBinding::EmpireOverviewUnits,
            KeyboardBinding::EmpireOverviewPolitics,
            KeyboardBinding::SocialPolicies,
            KeyboardBinding::TechnologyTree,
            KeyboardBinding::EmpireOverviewNotifications,
            KeyboardBinding::VictoryScreen,
            KeyboardBinding::EmpireOverviewStats,
            KeyboardBinding::EmpireOverviewResources,
            KeyboardBinding::QuickSave,
            KeyboardBinding::QuickLoad,
            KeyboardBinding::ViewCapitalCity,
            KeyboardBinding::Options,
            KeyboardBinding::SaveGame,
            KeyboardBinding::LoadGame,
            KeyboardBinding::ToggleResourceDisplay,
            KeyboardBinding::ToggleYieldDisplay,
            KeyboardBinding::QuitGame,
            KeyboardBinding::NewGame,
            KeyboardBinding::Diplomacy,
            KeyboardBinding::Espionage,
            KeyboardBinding::Undo,
            KeyboardBinding::ToggleUI,
            KeyboardBinding::ToggleWorkedTilesDisplay,
            KeyboardBinding::ToggleMovementDisplay,
            KeyboardBinding::ZoomIn,
            KeyboardBinding::ZoomOut,

            // Map Panning
            KeyboardBinding::PanUp,
            KeyboardBinding::PanLeft,
            KeyboardBinding::PanDown,
            KeyboardBinding::PanRight,
            KeyboardBinding::PanUpAlternate,
            KeyboardBinding::PanLeftAlternate,
            KeyboardBinding::PanDownAlternate,
            KeyboardBinding::PanRightAlternate,

            // Unit actions
            KeyboardBinding::SwapUnits,
            KeyboardBinding::Automate,
            KeyboardBinding::ConnectRoad,
            KeyboardBinding::StopAutomation,
            KeyboardBinding::StopMovement,
            KeyboardBinding::ShowUnitDestination,
            KeyboardBinding::Sleep,
            KeyboardBinding::SleepUntilHealed,
            KeyboardBinding::Fortify,
            KeyboardBinding::FortifyUntilHealed,
            KeyboardBinding::Explore,
            KeyboardBinding::StopExploration,
            KeyboardBinding::Promote,
            KeyboardBinding::Upgrade,
            KeyboardBinding::Transform,
            KeyboardBinding::Pillage,
            KeyboardBinding::Paradrop,
            KeyboardBinding::AirSweep,
            KeyboardBinding::SetUp,
            KeyboardBinding::FoundCity,
            KeyboardBinding::ConstructImprovement,
            KeyboardBinding::Repair,
            KeyboardBinding::Create,
            KeyboardBinding::HurryResearch,
            KeyboardBinding::HurryWonder,
            KeyboardBinding::HurryBuilding,
            KeyboardBinding::ConductTradeMission,
            KeyboardBinding::FoundReligion,
            KeyboardBinding::TriggerUnique,
            KeyboardBinding::SpreadReligion,
            KeyboardBinding::RemoveHeresy,
            KeyboardBinding::EnhanceReligion,
            KeyboardBinding::DisbandUnit,
            KeyboardBinding::GiftUnit,
            KeyboardBinding::Skip,
            KeyboardBinding::ShowAdditionalActions,
            KeyboardBinding::HideAdditionalActions,
            KeyboardBinding::AddInCapital,

            // AutoPlayMenu
            KeyboardBinding::AutoPlayMenuEndTurn,
            KeyboardBinding::AutoPlayMenuMilitary,
            KeyboardBinding::AutoPlayMenuCivilians,
            KeyboardBinding::AutoPlayMenuEconomy,

            // NextTurnMenu
            KeyboardBinding::NextTurnMenuNextTurn,
            KeyboardBinding::NextTurnMenuMoveAutomatedUnits,

            // City Screen
            KeyboardBinding::AddConstruction,
            KeyboardBinding::RaisePriority,
            KeyboardBinding::LowerPriority,
            KeyboardBinding::BuyConstruction,
            KeyboardBinding::BuyTile,
            KeyboardBinding::BuildUnits,
            KeyboardBinding::BuildBuildings,
            KeyboardBinding::BuildWonders,
            KeyboardBinding::BuildNationalWonders,
            KeyboardBinding::BuildOther,
            KeyboardBinding::BuildDisabled,
            KeyboardBinding::NextCity,
            KeyboardBinding::PreviousCity,
            KeyboardBinding::ShowStats,
            KeyboardBinding::ShowStatDetails,
            KeyboardBinding::CitizenManagement,
            KeyboardBinding::GreatPeopleDetail,
            KeyboardBinding::SpecialistDetail,
            KeyboardBinding::ReligionDetail,
            KeyboardBinding::BuildingsDetail,
            KeyboardBinding::ResetCitizens,
            KeyboardBinding::AvoidGrowth,
            KeyboardBinding::NoFocus,
            KeyboardBinding::FoodFocus,
            KeyboardBinding::ProductionFocus,
            KeyboardBinding::GoldFocus,
            KeyboardBinding::ScienceFocus,
            KeyboardBinding::CultureFocus,
            KeyboardBinding::FaithFocus,

            // CityScreenConstructionMenu
            KeyboardBinding::AddConstructionTop,
            KeyboardBinding::AddConstructionAll,
            KeyboardBinding::AddConstructionAllTop,
            KeyboardBinding::RemoveConstructionAll,

            // Civilopedia
            KeyboardBinding::PediaBuildings,
            KeyboardBinding::PediaWonders,
            KeyboardBinding::PediaResources,
            KeyboardBinding::PediaTerrains,
            KeyboardBinding::PediaImprovements,
            KeyboardBinding::PediaUnits,
            KeyboardBinding::PediaUnitTypes,
            KeyboardBinding::PediaNations,
            KeyboardBinding::PediaTechnologies,
            KeyboardBinding::PediaPromotions,
            KeyboardBinding::PediaPolicies,
            KeyboardBinding::PediaBeliefs,
            KeyboardBinding::PediaTutorials,
            KeyboardBinding::PediaDifficulties,
            KeyboardBinding::PediaEras,
            KeyboardBinding::PediaSpeeds,
            KeyboardBinding::PediaSearch,

            // Popups
            KeyboardBinding::Confirm,
            KeyboardBinding::Cancel,
            KeyboardBinding::UpgradeAll,
        ]
    }
}

impl std::fmt::Display for KeyboardBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{:?}({:?})", self.category().label(), self, self.default_key())
    }
}

/// Convert a camelCase string to a space-separated string
fn un_camel_case(name: &str) -> String {
    lazy_static! {
        static ref UN_CAMEL_CASE_REGEX: Regex = Regex::new("([A-Z])([A-Z])([a-z])|([a-z])([A-Z])").unwrap();
    }

    UN_CAMEL_CASE_REGEX.replace_all(name, "$1$4 $2$3$5").to_string()
}