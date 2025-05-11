use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};
use crate::models::ruleset::{Ruleset, RulesetObject, UniqueTarget};
use crate::models::stats::INamed;
use crate::models::counter::Counter;
use crate::models::civilization::Civilization;
use crate::models::game_info::GameInfo;
use crate::models::translations::{get_placeholder_parameters, get_placeholder_text, tr};
use crate::ui::components::extensions::to_text_button;
use crate::ui::screens::civilopediascreen::FormattedLine;
use crate::constants::Constants;

/// Type of milestone in a victory condition
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MilestoneType {
    /// Build a specific building
    BuiltBuilding,
    /// Anyone should build a specific building
    BuildingBuiltGlobally,
    /// Add all spaceship parts in capital
    AddedSSPartsInCapital,
    /// Destroy all players
    DestroyAllPlayers,
    /// Capture all capitals
    CaptureAllCapitals,
    /// Complete policy branches
    CompletePolicyBranches,
    /// Become the world religion
    WorldReligion,
    /// Win diplomatic vote
    WinDiplomaticVote,
    /// Have highest score after max turns
    ScoreAfterTimeOut,
}

impl MilestoneType {
    /// Get the text description for this milestone type
    pub fn text(&self) -> &'static str {
        match self {
            MilestoneType::BuiltBuilding => "Build [building]",
            MilestoneType::BuildingBuiltGlobally => "Anyone should build [building]",
            MilestoneType::AddedSSPartsInCapital => "Add all [comment] in capital",
            MilestoneType::DestroyAllPlayers => "Destroy all players",
            MilestoneType::CaptureAllCapitals => "Capture all capitals",
            MilestoneType::CompletePolicyBranches => "Complete [amount] Policy branches",
            MilestoneType::WorldReligion => "Become the world religion",
            MilestoneType::WinDiplomaticVote => "Win diplomatic vote",
            MilestoneType::ScoreAfterTimeOut => "Have highest score after max turns",
        }
    }
}

/// Status of a victory condition
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompletionStatus {
    /// Fully completed
    Completed,
    /// Partially completed
    Partially,
    /// Not completed
    Incomplete,
}

/// Focus area for a victory condition
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Focus {
    /// Production focus
    Production,
    /// Gold focus
    Gold,
    /// Culture focus
    Culture,
    /// Science focus
    Science,
    /// Faith focus
    Faith,
    /// Military focus
    Military,
    /// City-states focus
    CityStates,
    /// Score focus
    Score,
}

/// Represents a victory condition in the game
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Victory {
    /// The name of the victory condition
    pub name: String,
    /// The header text for the victory screen
    pub victory_screen_header: String,
    /// Whether this victory is hidden in the victory screen
    pub hidden_in_victory_screen: bool,
    /// The milestones required to achieve this victory
    pub milestones: Vec<String>,
    /// The required spaceship parts for this victory
    pub required_spaceship_parts: Vec<String>,
    /// The victory message when achieved
    pub victory_string: String,
    /// The defeat message when not achieved
    pub defeat_string: String,
}

impl Victory {
    /// Create a new Victory instance
    pub fn new() -> Self {
        Self {
            name: String::new(),
            victory_screen_header: "Do things to win!".to_string(),
            hidden_in_victory_screen: false,
            milestones: Vec::new(),
            required_spaceship_parts: Vec::new(),
            victory_string: "Your civilization stands above all others! The exploits of your people shall be remembered until the end of civilization itself!".to_string(),
            defeat_string: "You have been defeated. Your civilization has been overwhelmed by its many foes. But your people do not despair, for they know that one day you shall return - and lead them forward to victory!".to_string(),
        }
    }

    /// Get the milestone objects for this victory
    pub fn milestone_objects(&self) -> Vec<Milestone> {
        self.milestones.iter()
            .map(|m| Milestone::new(m.clone(), self))
            .collect()
    }

    /// Get the required spaceship parts as a counter
    pub fn required_spaceship_parts_as_counter(&self) -> Counter<String> {
        let mut parts = Counter::new();
        for spaceship_part in &self.required_spaceship_parts {
            parts.add(spaceship_part.clone(), 1);
        }
        parts
    }

    /// Check if this victory enables max turns
    pub fn enables_max_turns(&self) -> bool {
        self.milestone_objects().iter().any(|m| m.milestone_type == Some(MilestoneType::ScoreAfterTimeOut))
    }

    /// Get the things to focus on for a civilization
    pub fn get_things_to_focus(&self, civ_info: &Civilization) -> HashSet<Focus> {
        self.milestone_objects().iter()
            .filter(|m| !m.has_been_completed_by(civ_info))
            .map(|m| m.get_focus(civ_info))
            .collect()
    }
}

impl INamed for Victory {
    fn name(&self) -> &str {
        &self.name
    }
}

/// Represents a milestone in a victory condition
pub struct Milestone {
    /// The unique description of this milestone
    pub unique_description: String,
    /// The parent victory this milestone belongs to
    parent_victory: Victory,
    /// The type of this milestone
    pub milestone_type: Option<MilestoneType>,
    /// The parameters for this milestone
    pub params: Vec<String>,
}

impl Milestone {
    /// Create a new Milestone instance
    pub fn new(unique_description: String, parent_victory: &Victory) -> Self {
        let params = get_placeholder_parameters(&unique_description);
        let placeholder_text = get_placeholder_text(&unique_description);

        let milestone_type = MilestoneType::values().iter()
            .find(|t| get_placeholder_text(t.text()) == placeholder_text)
            .cloned();

        Self {
            unique_description,
            parent_victory: parent_victory.clone(),
            milestone_type,
            params,
        }
    }

    /// Get the incomplete spaceship parts for a civilization
    fn get_incomplete_spaceship_parts(&self, civ_info: &Civilization) -> Counter<String> {
        let mut incomplete_spaceship_parts = self.parent_victory.required_spaceship_parts_as_counter();
        incomplete_spaceship_parts.remove(&civ_info.victory_manager.currents_spaceship_parts);
        incomplete_spaceship_parts
    }

    /// Get the number of original major capitals owned by a civilization
    fn original_major_capitals_owned(&self, civ_info: &Civilization) -> i32 {
        civ_info.cities.iter()
            .filter(|c| c.is_original_capital && !c.founding_civ.is_empty() &&
                   civ_info.game_info.get_civilization(&c.founding_civ).is_major_civ())
            .count() as i32
    }

    /// Get civilizations with potential capitals to own
    fn civs_with_potential_capitals_to_own(&self, game_info: &GameInfo) -> HashSet<Civilization> {
        // Capitals that still exist, even if the civ is dead
        let civs_with_capitals: HashSet<Civilization> = game_info.get_cities().iter()
            .filter(|c| c.is_original_capital)
            .map(|c| game_info.get_civilization(&c.founding_civ))
            .filter(|c| c.is_major_civ())
            .cloned()
            .collect();

        // If the civ is alive, they can still create a capital, so we need them as well
        let living_civs: HashSet<Civilization> = game_info.civilizations.iter()
            .filter(|c| c.is_major_civ() && !c.is_defeated())
            .cloned()
            .collect();

        civs_with_capitals.union(&living_civs).cloned().collect()
    }

    /// Check if this milestone has been completed by a civilization
    pub fn has_been_completed_by(&self, civ_info: &Civilization) -> bool {
        if let Some(milestone_type) = &self.milestone_type {
            match milestone_type {
                MilestoneType::BuiltBuilding => {
                    civ_info.cities.iter().any(|c| c.city_constructions.is_built(&self.params[0]))
                },
                MilestoneType::AddedSSPartsInCapital => {
                    self.get_incomplete_spaceship_parts(civ_info).is_empty()
                },
                MilestoneType::DestroyAllPlayers => {
                    civ_info.game_info.get_alive_major_civs() == vec![civ_info.clone()]
                },
                MilestoneType::CaptureAllCapitals => {
                    self.original_major_capitals_owned(civ_info) ==
                        self.civs_with_potential_capitals_to_own(&civ_info.game_info).len() as i32
                },
                MilestoneType::CompletePolicyBranches => {
                    civ_info.policies.completed_branches.len() >= self.params[0].parse::<usize>().unwrap_or(0)
                },
                MilestoneType::BuildingBuiltGlobally => {
                    civ_info.game_info.get_cities().iter().any(|c| c.city_constructions.is_built(&self.params[0]))
                },
                MilestoneType::WinDiplomaticVote => {
                    civ_info.victory_manager.has_ever_won_diplomatic_vote
                },
                MilestoneType::ScoreAfterTimeOut => {
                    civ_info.game_info.turns >= civ_info.game_info.game_parameters.max_turns &&
                    civ_info == civ_info.game_info.civilizations.iter()
                        .max_by_key(|c| c.calculate_total_score())
                        .unwrap_or(civ_info)
                },
                MilestoneType::WorldReligion => {
                    civ_info.game_info.is_religion_enabled() &&
                    civ_info.religion_manager.religion.is_some() &&
                    civ_info.game_info.civilizations.iter()
                        .filter(|c| c.is_major_civ() && c.is_alive())
                        .all(|c| {
                            if let Some(religion) = &civ_info.religion_manager.religion {
                                c.religion_manager.is_majority_religion_for_civ(religion)
                            } else {
                                false
                            }
                        })
                },
            }
        } else {
            false
        }
    }

    /// Get the victory screen button header text
    pub fn get_victory_screen_button_header_text(&self, completed: bool, civ_info: &Civilization) -> String {
        if let Some(milestone_type) = &self.milestone_type {
            match milestone_type {
                MilestoneType::BuildingBuiltGlobally | MilestoneType::WinDiplomaticVote |
                MilestoneType::ScoreAfterTimeOut | MilestoneType::BuiltBuilding => {
                    self.unique_description.clone()
                },
                MilestoneType::CompletePolicyBranches => {
                    let amount_to_do = tr(&self.params[0]);
                    let amount_done = if completed {
                        amount_to_do.clone()
                    } else {
                        tr(&civ_info.get_completed_policy_branches_count().to_string())
                    };
                    format!("{} ({}/{})", self.unique_description, amount_done, amount_to_do)
                },
                MilestoneType::CaptureAllCapitals => {
                    let amount_to_do = self.civs_with_potential_capitals_to_own(&civ_info.game_info).len();
                    let amount_done = if completed {
                        amount_to_do
                    } else {
                        self.original_major_capitals_owned(civ_info) as usize
                    };
                    if civ_info.hide_civ_count() {
                        format!("{} ({}/?)", self.unique_description, tr(&amount_done.to_string()))
                    } else {
                        format!("{} ({}/{})", self.unique_description,
                                tr(&amount_done.to_string()), tr(&amount_to_do.to_string()))
                    }
                },
                MilestoneType::DestroyAllPlayers => {
                    let amount_to_do = civ_info.game_info.civilizations.iter()
                        .filter(|c| c.is_major_civ()).count() - 1; // Don't count yourself
                    let amount_done = if completed {
                        amount_to_do
                    } else {
                        amount_to_do - civ_info.game_info.get_alive_major_civs()
                            .iter()
                            .filter(|c| c != civ_info)
                            .count()
                    };
                    if civ_info.hide_civ_count() {
                        format!("{} ({}/?)", self.unique_description, tr(&amount_done.to_string()))
                    } else {
                        format!("{} ({}/{})", self.unique_description,
                                tr(&amount_done.to_string()), tr(&amount_to_do.to_string()))
                    }
                },
                MilestoneType::AddedSSPartsInCapital => {
                    let complete_spaceship_parts = &civ_info.victory_manager.currents_spaceship_parts;
                    let mut incomplete_spaceship_parts = self.parent_victory.required_spaceship_parts_as_counter();
                    let amount_to_do = incomplete_spaceship_parts.sum_values();
                    incomplete_spaceship_parts.remove(complete_spaceship_parts);
                    let amount_done = amount_to_do - incomplete_spaceship_parts.sum_values();
                    format!("{} ({}/{})", self.unique_description,
                            tr(&amount_done.to_string()), tr(&amount_to_do.to_string()))
                },
                MilestoneType::WorldReligion => {
                    let amount_to_do = civ_info.game_info.civilizations.iter()
                        .filter(|c| c.is_major_civ() && c.is_alive()).count() - 1; // Don't count yourself
                    let amount_done = if completed {
                        amount_to_do
                    } else if civ_info.religion_manager.religion.is_none() {
                        0
                    } else if let Some(religion) = &civ_info.religion_manager.religion {
                        if religion.is_pantheon() {
                            1
                        } else {
                            civ_info.game_info.civilizations.iter()
                                .filter(|c| c.is_major_civ() && c.is_alive() &&
                                       c.religion_manager.is_majority_religion_for_civ(religion))
                                .count()
                        }
                    } else {
                        0
                    };
                    format!("{} ({}/{})", self.unique_description,
                            tr(&amount_done.to_string()), tr(&amount_to_do.to_string()))
                },
            }
        } else {
            self.unique_description.clone()
        }
    }

    /// Get the focus for this milestone
    pub fn get_focus(&self, civ_info: &Civilization) -> Focus {
        let ruleset = &civ_info.game_info.ruleset;

        if let Some(milestone_type) = &self.milestone_type {
            match milestone_type {
                MilestoneType::BuiltBuilding => {
                    let building = &ruleset.buildings[&self.params[0]];
                    if !civ_info.tech.is_researched(building) {
                        Focus::Science
                    } else {
                        Focus::Production
                    }
                },
                MilestoneType::BuildingBuiltGlobally => {
                    let building = &ruleset.buildings[&self.params[0]];
                    if !civ_info.tech.is_researched(building) {
                        Focus::Science
                    } else {
                        Focus::Production
                    }
                },
                MilestoneType::AddedSSPartsInCapital => {
                    let constructions = self.get_incomplete_spaceship_parts(civ_info).keys()
                        .map(|k| {
                            if ruleset.buildings.contains_key(k) {
                                &ruleset.buildings[k]
                            } else {
                                &ruleset.units[k]
                            }
                        })
                        .collect::<Vec<_>>();

                    if constructions.iter().any(|c| !civ_info.tech.is_researched(c)) {
                        Focus::Science
                    } else {
                        Focus::Production
                    }
                },
                MilestoneType::DestroyAllPlayers | MilestoneType::CaptureAllCapitals => {
                    Focus::Military
                },
                MilestoneType::CompletePolicyBranches => {
                    Focus::Culture
                },
                MilestoneType::WinDiplomaticVote => {
                    Focus::CityStates
                },
                MilestoneType::ScoreAfterTimeOut => {
                    Focus::Score
                },
                MilestoneType::WorldReligion => {
                    Focus::Faith
                },
            }
        } else {
            Focus::Score
        }
    }
}

impl RulesetObject for Victory {
    fn get_unique_target(&self) -> UniqueTarget {
        UniqueTarget::Victory
    }

    fn make_link(&self) -> String {
        format!("Victory/{}", self.name)
    }

    fn get_civilopedia_text_lines(&self, _ruleset: &Ruleset) -> Vec<FormattedLine> {
        vec![
            FormattedLine::new(self.victory_string.clone()),
            FormattedLine::new(self.defeat_string.clone()),
        ]
    }
}