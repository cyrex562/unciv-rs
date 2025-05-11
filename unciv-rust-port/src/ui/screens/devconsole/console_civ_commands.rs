use std::collections::HashMap;

use crate::logic::civilization::PlayerType;
use crate::models::ruleset::{Policy, Technology};
use crate::models::stats::Stat;
use crate::ui::screens::devconsole::cli_input::DevConsolePopupExt;
use crate::ui::screens::devconsole::console_command::ConsoleAction;
use crate::ui::screens::devconsole::console_command_node::ConsoleCommandNode;
use crate::ui::screens::devconsole::console_error::ConsoleErrorException;
use crate::ui::screens::devconsole::console_trigger_action::ConsoleTriggerAction;
use crate::ui::screens::devconsole::dev_console_popup::DevConsolePopup;
use crate::ui::screens::devconsole::dev_console_response::DevConsoleResponse;

/// Commands for managing civilizations in the dev console
pub struct ConsoleCivCommands;

impl ConsoleCommandNode for ConsoleCivCommands {
    fn subcommands(&self) -> HashMap<String, Box<dyn ConsoleAction>> {
        let mut commands = HashMap::new();

        // addstat command
        commands.insert(
            "addstat".to_string(),
            Box::new(ConsoleAction::new("civ addstat <stat> <amount> [civ]", |console, params| {
                let stat = params[0].to_stat()?;
                if !Stat::stats_with_civ_wide_field().contains(&stat) {
                    return Err(ConsoleErrorException::new(&format!("{} is not civ-wide", stat)));
                }
                let amount = params[1].to_int()?;
                let civ = console.get_civ_by_name_or_selected(params.get(2))?;

                civ.add_stat(stat, amount);
                Ok(DevConsoleResponse::OK)
            })),
        );

        // setplayertype command
        commands.insert(
            "setplayertype".to_string(),
            Box::new(ConsoleAction::new("civ setplayertype <civName> <ai/human>", |console, params| {
                let civ = console.get_civ_by_name(params[0].original_unquoted())?;
                if !civ.is_major_civ() {
                    return Err(ConsoleErrorException::new("Can only change player type for major civs"));
                }
                civ.player_type = params[1].enum_value::<PlayerType>()?;
                Ok(DevConsoleResponse::OK)
            })),
        );

        // revealmap command
        commands.insert(
            "revealmap".to_string(),
            Box::new(ConsoleAction::new("civ revealmap [civName]", |console, params| {
                let civ = console.get_civ_by_name_or_selected(params.get(0))?;
                for tile in console.game_info.tile_map.values() {
                    tile.set_explored(&civ, true);
                }
                Ok(DevConsoleResponse::OK)
            })),
        );

        // activatetrigger command
        commands.insert(
            "activatetrigger".to_string(),
            Box::new(ConsoleTriggerAction::new("civ")),
        );

        // addpolicy command
        commands.insert(
            "addpolicy".to_string(),
            Box::new(ConsoleAction::new("civ addpolicy <civName> <policyName>", |console, params| {
                let civ = console.get_civ_by_name(params[0].original_unquoted())?;
                let policy = console.find_cli_input::<Policy>(&params[1])
                    .ok_or_else(|| ConsoleErrorException::new("Unrecognized policy"))?;

                if civ.policies.is_adopted(&policy.name) {
                    DevConsoleResponse::hint(&format!("{} already has adopted {}", civ.civ_name, policy.name));
                } else {
                    civ.policies.free_policies += 1;
                    civ.policies.adopt(policy);
                    return Ok(DevConsoleResponse::OK);
                }
                Ok(DevConsoleResponse::OK)
            })),
        );

        // removepolicy command
        commands.insert(
            "removepolicy".to_string(),
            Box::new(ConsoleAction::new("civ removepolicy <civName> <policyName>", |console, params| {
                let civ = console.get_civ_by_name(params[0].original_unquoted())?;
                let policy = console.find_cli_input::<Policy>(&params[1])
                    .ok_or_else(|| ConsoleErrorException::new("Unrecognized policy"))?;

                if !civ.policies.is_adopted(&policy.name) {
                    DevConsoleResponse::hint(&format!("{} does not have {}", civ.civ_name, policy.name));
                } else {
                    civ.policies.remove_policy(policy, true); // assumeWasFree = true, See UniqueType.OneTimeRemovePolicy
                    return Ok(DevConsoleResponse::OK);
                }
                Ok(DevConsoleResponse::OK)
            })),
        );

        // addtech command
        commands.insert(
            "addtech".to_string(),
            Box::new(ConsoleAction::new("civ addtechnology <civName> <techName>", |console, params| {
                let civ = console.get_civ_by_name(params[0].original_unquoted())?;
                let tech = console.find_cli_input::<Technology>(&params[1])
                    .ok_or_else(|| ConsoleErrorException::new("Unrecognized technology"))?;

                if civ.tech.is_researched(&tech.name) {
                    DevConsoleResponse::hint(&format!("{} already has researched {}", civ.civ_name, tech.name));
                } else {
                    civ.tech.add_technology(&tech.name, false);
                    return Ok(DevConsoleResponse::OK);
                }
                Ok(DevConsoleResponse::OK)
            })),
        );

        // removetech command
        commands.insert(
            "removetech".to_string(),
            Box::new(ConsoleAction::new("civ removetechnology <civName> <techName>", |console, params| {
                let civ = console.get_civ_by_name(params[0].original_unquoted())?;
                let tech = console.find_cli_input::<Technology>(&params[1])
                    .ok_or_else(|| ConsoleErrorException::new("Unrecognized technology"))?;

                if !civ.tech.is_researched(&tech.name) {
                    DevConsoleResponse::hint(&format!("{} does not have {}", civ.civ_name, tech.name));
                } else {
                    // Can have multiple for researchable techs
                    civ.tech.techs_researched.retain(|t| t != &tech.name);
                    return Ok(DevConsoleResponse::OK);
                }
                Ok(DevConsoleResponse::OK)
            })),
        );

        commands
    }
}