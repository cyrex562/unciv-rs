pub mod belief;
pub mod building;
pub mod construction_new;
pub mod event;
pub mod global_uniques;
pub mod last_seen_improvement;

pub mod nation;
pub mod policy;
pub mod policy_branch;
pub mod quest;
pub mod ruin_reward;
pub mod ruleset;
pub mod ruleset_cache;
pub mod ruleset_object;
pub mod specialist;
pub mod speed;
pub mod tech;
pub mod tile;
pub mod tutorial;
pub mod unit;
pub mod validation;
pub mod victory;
pub mod base_ruleset;
pub mod constants;
pub mod difficulty;
pub mod game;
pub mod game_info;
pub mod game_info_preview;
pub mod game_settings;
pub mod game_starter;
pub mod holiday_dates;
pub mod id_checker;
pub mod ranking_type;
pub mod unciv_game;
pub mod victory_data;
pub mod build_cost;

/// Remove technologies and policies that are no longer defined in the ruleset
// fn remove_tech_and_policies(game_info: &mut GameInfo) {
//     for civ in &mut game_info.civilizations {
//         // Remove technologies that are no longer defined in the ruleset
//         let techs_researched: Vec<String> = civ.tech.techs_researched.clone();
//         for tech in techs_researched {
//             if !game_info.ruleset.technologies.contains_key(&tech) {
//                 civ.tech.techs_researched.remove(&tech);
//             }
//         }
// 
//         // Remove policies that are no longer defined in the ruleset
//         let adopted_policies: Vec<String> = civ.policies.adopted_policies.clone();
//         for policy in adopted_policies {
//             if !game_info.ruleset.policies.contains_key(&policy) {
//                 civ.policies.adopted_policies.remove(&policy);
//             }
//         }
//     }
// }
