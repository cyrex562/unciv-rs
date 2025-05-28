use crate::{
    constants::Constants,
    civilization::Civilization,
    models::ruleset::tile::ResourceType,
    ui::components::formatted_line::FormattedLine,
    utils::debug_utils::DebugUtils,
};

/// Provides functionality for generating descriptions of tiles
pub struct TileDescription;

impl TileDescription {
    /// Get info on a selected tile, used on WorldScreen (right side above minimap), CityScreen or MapEditorViewTab.
    pub fn to_markup(tile: &crate::map::tile::Tile, viewing_civ: Option<&Civilization>) -> Vec<FormattedLine> {
        let mut line_list = Vec::new();
        let is_viewable_to_player = viewing_civ.is_none() || DebugUtils::VISIBLE_MAP
            || viewing_civ.unwrap().viewable_tiles.contains(tile);

        if tile.is_city_center() {
            let city = tile.get_city().unwrap();
            let mut city_string = city.name.tr();
            if is_viewable_to_player {
                city_string.push_str(&format!(" ({})", city.health));
            }
            line_list.push(FormattedLine::new(city_string));

            if DebugUtils::VISIBLE_MAP || city.civ == viewing_civ.unwrap() {
                line_list.extend(city.city_constructions.get_production_markup(&tile.ruleset));
            }
        }

        line_list.push(FormattedLine::new_with_link(tile.base_terrain.clone(), format!("Terrain/{}", tile.base_terrain)));

        for terrain_feature in &tile.terrain_features {
            line_list.push(FormattedLine::new_with_link(terrain_feature.clone(), format!("Terrain/{}", terrain_feature)));
        }

        if let Some(resource) = &tile.resource {
            if viewing_civ.is_none() || tile.has_viewable_resource(viewing_civ.unwrap()) {
                if tile.tile_resource.resource_type == ResourceType::Strategic {
                    line_list.push(FormattedLine::new_with_link(
                        format!("{{{}}} ({})", resource, tile.resource_amount),
                        format!("Resource/{}", resource)
                    ));
                } else {
                    line_list.push(FormattedLine::new_with_link(
                        resource.clone(),
                        format!("Resource/{}", resource)
                    ));
                }
            }
        }

        if let Some(resource) = &tile.resource {
            if let Some(viewing_civ) = viewing_civ {
                if tile.has_viewable_resource(viewing_civ) {
                    if let Some(resource_improvement) = tile.tile_resource.get_improvements()
                        .iter()
                        .find(|&imp| {
                            if let Some(tile_improvement) = tile.ruleset.tile_improvements.get(imp) {
                                tile.improvement_functions.can_build_improvement(tile_improvement, viewing_civ)
                            } else {
                                false
                            }
                        })
                    {
                        if let Some(tile_improvement) = tile.ruleset.tile_improvements.get(resource_improvement) {
                            if let Some(tech_required) = &tile_improvement.tech_required {
                                if !viewing_civ.tech.is_researched(tech_required) {
                                    line_list.push(FormattedLine::new_with_link_and_color(
                                        format!("Requires [{}]", tech_required),
                                        format!("Technology/{}", tech_required),
                                        "#FAA".to_string()
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(natural_wonder) = &tile.natural_wonder {
            line_list.push(FormattedLine::new_with_link(
                natural_wonder.clone(),
                format!("Terrain/{}", natural_wonder)
            ));
        }

        if tile.road_status != crate::map::tile::RoadStatus::None && !tile.is_city_center() {
            let pillage_text = if tile.road_is_pillaged { " (Pillaged!)" } else { "" };
            line_list.push(FormattedLine::new_with_link(
                format!("[{}]{}", tile.road_status.name(), pillage_text),
                format!("Improvement/{}", tile.road_status.name())
            ));
        }

        if let Some(shown_improvement) = tile.get_shown_improvement(viewing_civ) {
            let pillage_text = if tile.improvement_is_pillaged { " (Pillaged!)" } else { "" };
            line_list.push(FormattedLine::new_with_link(
                format!("[{}]{}", shown_improvement, pillage_text),
                format!("Improvement/{}", shown_improvement)
            ));
        }

        if let Some(improvement_in_progress) = &tile.improvement_in_progress {
            if is_viewable_to_player {
                // Negative turnsToImprovement is used for UniqueType.CreatesOneImprovement
                let line = if tile.turns_to_improvement > 0 {
                    format!("{{{}}} - {}{}", improvement_in_progress, tile.turns_to_improvement, "turn")
                } else {
                    format!("{{{}}} (Under construction)", improvement_in_progress)
                };
                line_list.push(FormattedLine::new_with_link(
                    line,
                    format!("Improvement/{}", improvement_in_progress)
                ));
            }
        }

        if let Some(civilian_unit) = &tile.civilian_unit {
            if is_viewable_to_player {
                line_list.push(FormattedLine::new_with_link(
                    format!("{} - {}", civilian_unit.name.tr(), civilian_unit.civ.civ_name.tr()),
                    format!("Unit/{}", civilian_unit.name)
                ));
            }
        }

        if let Some(military_unit) = &tile.military_unit {
            if is_viewable_to_player && (viewing_civ.is_none() || !military_unit.is_invisible(viewing_civ.unwrap())) {
                let mil_unit_string = if military_unit.health < 100 {
                    format!("{}({}) - {}", military_unit.name.tr(), military_unit.health, military_unit.civ.civ_name.tr())
                } else {
                    format!("{} - {}", military_unit.name.tr(), military_unit.civ.civ_name.tr())
                };
                line_list.push(FormattedLine::new_with_link(
                    mil_unit_string,
                    format!("Unit/{}", military_unit.name)
                ));
            }
        }

        let defence_bonus = tile.get_defensive_bonus();
        if defence_bonus != 0.0 {
            let defence_percent_string = format!("{:+}%", (defence_bonus * 100.0) as i32);
            line_list.push(FormattedLine::new(format!("[{}] to unit defence", defence_percent_string)));
        }

        if tile.is_impassible() {
            line_list.push(FormattedLine::new(Constants::IMPASSABLE));
        }

        if tile.is_land && tile.is_adjacent_to(Constants::FRESH_WATER) {
            line_list.push(FormattedLine::new(Constants::FRESH_WATER));
        }

        line_list
    }
}