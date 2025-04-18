use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{Datelike, Month};
use bevy::prelude::*;

use crate::logic::holiday_dates::{HolidayDates, Holidays};
use crate::logic::map::MapParameters;
use crate::models::ruleset::Ruleset;
use crate::models::ruleset::tile::{Terrain, TerrainType};
use crate::ui::screens::civilopediascreen::FormattedLine;

/// Module that provides special terrain features for holidays
pub struct EasterEggRulesets;

impl EasterEggRulesets {
    /// Modifies map parameters based on the current month for easter egg effects
    pub fn modify_for_easter_egg(map_params: &mut MapParameters) {
        let month = HolidayDates::get_month();

        // Adjust temperature based on the month
        map_params.temperature_shift = match month {
            Month::January => -1.4,
            Month::February => -1.3,
            Month::March => 0.4, // actually generates a lot of grassland
            Month::August => -0.4, // actually generates a lot more desert
            Month::November => -0.7,
            Month::December => -1.3,
            _ => 0.0,
        };

        // Increase rare features richness
        map_params.rare_features_richness = 0.15;
    }

    /// Gets the easter egg ruleset for today's holiday, if any
    pub fn get_today_easter_egg_ruleset() -> Option<Ruleset> {
        match HolidayDates::get_holiday_by_date() {
            Holidays::Easter => Some(Self::easter_ruleset()),
            Holidays::Samhain => Some(Self::samhain_ruleset()),
            Holidays::Xmas => Some(Self::xmas_ruleset()),
            _ => None,
        }
    }

    /// Creates the Easter ruleset with egg terrains
    fn easter_ruleset() -> Ruleset {
        let mut ruleset = Ruleset::new();
        ruleset.name = "Easter Eggs".to_string();

        // Add the wonder egg
        ruleset.terrains.insert("Giant Easter Egg".to_string(), Self::get_wonder_egg());

        // Add normal eggs
        for index in 1..=8 {
            let egg_name = format!("Easter Egg {}", index);
            ruleset.terrains.insert(egg_name.clone(), Self::get_normal_egg(index));
        }

        ruleset
    }

    /// Creates the Samhain (Halloween) ruleset with pumpkin and candy terrains
    fn samhain_ruleset() -> Ruleset {
        let mut ruleset = Ruleset::new();
        ruleset.name = "Samhain".to_string();

        // Add the wonder pumpkin
        ruleset.terrains.insert("Giant Pumpkin".to_string(), Self::get_wonder_pumpkin());

        // Add candies
        for index in 1..=5 {
            let candy_name = format!("Halloween candy {}", index);
            ruleset.terrains.insert(candy_name.clone(), Self::get_candy(index));
        }

        // Add special grassland for autumnal effect
        let mut grassland = Terrain::new();
        grassland.name = "Grassland".to_string();
        grassland.terrain_type = TerrainType::Land;
        grassland.uniques.push("Occurs at temperature between [1.0] and [1.0] and humidity between [0.0] and [0.0]".to_string());
        ruleset.terrains.insert("Grassland".to_string(), grassland);

        ruleset
    }

    /// Creates the Christmas ruleset with tree and decoration terrains
    fn xmas_ruleset() -> Ruleset {
        let mut ruleset = Ruleset::new();
        ruleset.name = "X-Mas".to_string();

        // Add the wonder tree
        ruleset.terrains.insert("Xmas Tree".to_string(), Self::get_wonder_tree());

        // Add decorations
        for index in 1..=7 {
            let decoration_name = format!("Xmas decoration {}", index);
            ruleset.terrains.insert(decoration_name.clone(), Self::get_xmas(index));
        }

        ruleset
    }

    /// Creates a base wonder terrain
    fn get_wonder() -> Terrain {
        let mut terrain = Terrain::new();
        terrain.terrain_type = TerrainType::NaturalWonder;
        terrain.happiness = 42.0;
        terrain.food = 9.0;
        terrain.faith = 9.0;
        terrain.occurs_on = vec!["Grassland".to_string(), "Plains".to_string(), "Desert".to_string()];
        terrain.uniques.push("Must be adjacent to [0] [Coast] tiles".to_string());
        terrain.turns_into = Some("Mountain".to_string());
        terrain.impassable = true;
        terrain.unbuildable = true;
        terrain.weight = 999999;
        terrain
    }

    /// Creates a base rare feature terrain
    fn get_rare_feature() -> Terrain {
        let mut terrain = Terrain::new();
        terrain.terrain_type = TerrainType::TerrainFeature;
        terrain.happiness = 2.0;
        terrain.food = 1.0;
        terrain.faith = 1.0;
        terrain.occurs_on = vec![
            "Grassland".to_string(),
            "Plains".to_string(),
            "Desert".to_string(),
            "Tundra".to_string(),
            "Snow".to_string()
        ];
        terrain.uniques.push("Rare feature".to_string());
        terrain
    }

    /// Creates the giant easter egg wonder
    fn get_wonder_egg() -> Terrain {
        let mut terrain = Self::get_wonder();
        terrain.name = "Giant Easter Egg".to_string();
        terrain.civilopedia_text = vec![
            FormattedLine::new("This monstrous Easter Egg could feed a whole country for a year!"),
            FormattedLine::new_with_style(
                "...Or certain first-world citizens for a week...",
                Some("#444".to_string()),
                Some(2),
                Some(15)
            ),
            FormattedLine::new_with_link(
                "[See also]: [Easter Egg]",
                "terrain/Easter Egg 1"
            ),
        ];
        terrain
    }

    /// Creates a normal easter egg
    fn get_normal_egg(index: i32) -> Terrain {
        let mut terrain = Self::get_rare_feature();
        terrain.name = format!("Easter Egg {}", index);
        terrain.civilopedia_text = vec![
            FormattedLine::new("This is an Easter Egg, just like those some families hide once a year to have other family members seek them, eat them and get caries."),
            FormattedLine::new_with_link(
                "[See also]: [Giant Easter Egg]",
                "terrain/Giant Easter Egg"
            ),
        ];
        terrain
    }

    /// Creates the giant pumpkin wonder
    fn get_wonder_pumpkin() -> Terrain {
        let mut terrain = Self::get_wonder();
        terrain.name = "Giant Pumpkin".to_string();
        terrain.civilopedia_text = vec![
            FormattedLine::new("Oh, a Halloween Pumpkin!"),
            FormattedLine::new_with_style(
                "Actually, Halloween comes from Samhain, a Gaelic festival marking the beginning of winter.",
                Some("#444".to_string()),
                Some(2),
                Some(15)
            ),
            FormattedLine::new_with_link(
                "{See also}: {Candies}",
                "terrain/Halloween candy 1"
            ),
        ];
        terrain
    }

    /// Creates a halloween candy
    fn get_candy(index: i32) -> Terrain {
        let mut terrain = Self::get_rare_feature();
        terrain.name = format!("Halloween candy {}", index);
        terrain.civilopedia_text = vec![
            FormattedLine::new("This is some candy, ritually extorted from innocent seniors by Trick-or-Treaters."),
            FormattedLine::new_with_link(
                "{See also}: {Giant Pumpkin}",
                "terrain/Giant Pumpkin"
            ),
        ];
        terrain
    }

    /// Creates the christmas tree wonder
    fn get_wonder_tree() -> Terrain {
        let mut terrain = Self::get_wonder();
        terrain.name = "Xmas Tree".to_string();
        terrain.occurs_on = vec!["Tundra".to_string(), "Snow".to_string()];
        terrain.uniques.push("Occurs on latitudes from [0] to [70] percent of distance equator to pole".to_string());
        terrain.uniques.push("Neighboring tiles will convert to [Snow]".to_string());
        terrain.civilopedia_text = vec![
            FormattedLine::new("The traditions demand cutting down trees like this to mount them in a home."),
            FormattedLine::new_with_style(
                "For for the whole family! And the cat! And the fire brigade!.",
                Some("#444".to_string()),
                Some(2),
                Some(15)
            ),
            FormattedLine::new_with_link(
                "{See also}: {Xmas decorations}",
                "terrain/Xmas decoration 1"
            ),
        ];
        terrain
    }

    /// Creates a christmas decoration
    fn get_xmas(index: i32) -> Terrain {
        let mut terrain = Self::get_rare_feature();
        terrain.name = format!("Xmas decoration {}", index);
        terrain.civilopedia_text = vec![
            FormattedLine::new("On a certain holiday of culturally varying names, capitalism runs rampant. Some of the more harmless symptoms look like this."),
            FormattedLine::new_with_link(
                "{See also}: {Xmas Tree}",
                "terrain/Xmas Tree"
            ),
        ];
        terrain
    }
}