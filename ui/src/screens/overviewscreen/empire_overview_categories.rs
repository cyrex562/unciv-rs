use std::rc::Rc;
use std::cell::RefCell;
use egui::{Ui, Color32, Align};
use crate::models::civilization::Civilization;
use crate::models::ruleset::tile::ResourceType;
use crate::ui::components::input::KeyCharAndCode;
use super::empire_overview_tab::EmpireOverviewTabPersistableData;

/// Controls which Tabs for the EmpireOverviewScreen exist and their order.
/// To add a Tab, build a new EmpireOverviewTab trait implementation and fill out a new entry here.
/// Note the enum value's name is used as Tab caption, so if you ever need a non-alphanumeric caption
/// please redesign to include a property for the caption
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmpireOverviewCategories {
    Cities,
    Stats,
    Trades,
    Units,
    Politics,
    Resources,
    Religion,
    Wonders,
    Notifications,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmpireOverviewTabState {
    Normal,
    Disabled,
    Hidden,
}

impl EmpireOverviewCategories {
    pub fn icon_name(&self) -> &'static str {
        match self {
            Self::Cities => "OtherIcons/Cities",
            Self::Stats => "StatIcons/Gold",
            Self::Trades => "StatIcons/Acquire",
            Self::Units => "OtherIcons/Shield",
            Self::Politics => "OtherIcons/Politics",
            Self::Resources => "StatIcons/Happiness",
            Self::Religion => "StatIcons/Faith",
            Self::Wonders => "OtherIcons/Wonders",
            Self::Notifications => "OtherIcons/Notifications",
        }
    }

    pub fn shortcut_key(&self) -> KeyCharAndCode {
        match self {
            Self::Cities => KeyCharAndCode::new('C'),
            Self::Stats => KeyCharAndCode::new('S'),
            Self::Trades => KeyCharAndCode::new('T'),
            Self::Units => KeyCharAndCode::new('U'),
            Self::Politics => KeyCharAndCode::new('P'),
            Self::Resources => KeyCharAndCode::new('R'),
            Self::Religion => KeyCharAndCode::new('F'),
            Self::Wonders => KeyCharAndCode::new('W'),
            Self::Notifications => KeyCharAndCode::new('N'),
        }
    }

    pub fn scroll_align(&self) -> Align {
        match self {
            Self::Cities | Self::Units | Self::Resources => Align::LEFT,
            _ => Align::TOP,
        }
    }

    pub fn create_tab(
        &self,
        viewing_player: Rc<RefCell<Civilization>>,
        overview_screen: Rc<RefCell<EmpireOverviewScreen>>,
        persisted_data: Option<Rc<RefCell<EmpireOverviewTabPersistableData>>>,
    ) -> Box<dyn EmpireOverviewTab> {
        match self {
            Self::Cities => Box::new(CityOverviewTab::new(viewing_player, overview_screen, persisted_data)),
            Self::Stats => Box::new(StatsOverviewTab::new(viewing_player, overview_screen)),
            Self::Trades => Box::new(TradesOverviewTab::new(viewing_player, overview_screen)),
            Self::Units => Box::new(UnitOverviewTab::new(viewing_player, overview_screen, persisted_data)),
            Self::Politics => Box::new(GlobalPoliticsOverviewTable::new(viewing_player, overview_screen, persisted_data)),
            Self::Resources => Box::new(ResourcesOverviewTab::new(viewing_player, overview_screen, persisted_data)),
            Self::Religion => Box::new(ReligionOverviewTab::new(viewing_player, overview_screen, persisted_data)),
            Self::Wonders => Box::new(WonderOverviewTab::new(viewing_player, overview_screen)),
            Self::Notifications => Box::new(NotificationsOverviewTable::new(viewing_player, overview_screen, persisted_data)),
        }
    }

    pub fn show_disabled(&self, viewing_player: &Rc<RefCell<Civilization>>) -> bool {
        match self {
            Self::Cities => viewing_player.borrow().cities.is_empty(),
            Self::Stats => viewing_player.borrow().is_spectator(),
            Self::Trades => {
                let player = viewing_player.borrow();
                player.diplomacy.values().all(|d| d.trades.is_empty()) &&
                !player.diplomacy.values().any(|d| {
                    d.other_civ().trade_requests.iter().any(|r| r.requesting_civ == player.civ_name)
                })
            },
            Self::Units => viewing_player.borrow().units.get_civ_units().is_empty(),
            Self::Politics => viewing_player.borrow().diplomacy.is_empty(),
            Self::Resources => !viewing_player.borrow().detailed_civ_resources.iter().any(|r| r.resource.resource_type != ResourceType::Bonus),
            Self::Religion => false, // Handled by test_state
            Self::Wonders => viewing_player.borrow().natural_wonders.is_empty() && viewing_player.borrow().cities.is_empty(),
            Self::Notifications => viewing_player.borrow().notifications.is_empty() && viewing_player.borrow().notifications_log.is_empty(),
        }
    }

    pub fn test_state(&self, viewing_player: &Rc<RefCell<Civilization>>) -> EmpireOverviewTabState {
        match self {
            Self::Religion => {
                let player = viewing_player.borrow();
                if !player.game_info.borrow().is_religion_enabled() {
                    EmpireOverviewTabState::Hidden
                } else if player.game_info.borrow().religions.is_empty() {
                    EmpireOverviewTabState::Disabled
                } else {
                    EmpireOverviewTabState::Normal
                }
            },
            _ => if self.show_disabled(viewing_player) {
                EmpireOverviewTabState::Disabled
            } else {
                EmpireOverviewTabState::Normal
            },
        }
    }

    pub fn get_persist_data_class(&self) -> Option<std::any::TypeId> {
        match self {
            Self::Cities => Some(std::any::TypeId::of::<CityOverviewTab::CityTabPersistableData>()),
            Self::Units => Some(std::any::TypeId::of::<UnitOverviewTab::UnitTabPersistableData>()),
            Self::Politics => Some(std::any::TypeId::of::<GlobalPoliticsOverviewTable::DiplomacyTabPersistableData>()),
            Self::Resources => Some(std::any::TypeId::of::<ResourcesOverviewTab::ResourcesTabPersistableData>()),
            _ => None,
        }
    }
}