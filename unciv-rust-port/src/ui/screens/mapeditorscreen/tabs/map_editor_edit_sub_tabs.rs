use std::sync::Arc;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use rand::Rng;

use crate::constants::SPECTATOR;
use crate::models::ruleset::{Ruleset, Terrain, TerrainType, TileResource, ResourceType, TileImprovement};
use crate::models::ruleset::nation::Nation;
use crate::models::ruleset::unique::{UniqueType, StateForConditionals};
use crate::models::map::{TileMap, Tile, RoadStatus, StartingLocation};
use crate::ui::components::widgets::{Button, CheckBox, Frame, Label, ScrollArea, Stack};
use crate::ui::components::extensions::{center, surround_with_circle};
use crate::ui::components::input::{KeyShortcutDispatcherVeto, KeyboardBinding};
use crate::ui::components::tilegroups::{TileGroup, TileSetStrings};
use crate::ui::images::ImageGetter;
use crate::ui::screens::basescreen::{BaseScreen, RecreateOnResize};
use crate::ui::screens::civilopediascreen::{FormattedLine, MarkupRenderer};
use crate::ui::screens::mapeditorscreen::tabs::map_editor_edit_tab::{MapEditorEditTab, BrushHandlerType};
use crate::ui::screens::tabbed_pager::TabbedPager;
use crate::utils::concurrency::Concurrency;

/// Interface for map editor edit sub-tabs
pub trait IMapEditorEditSubTabs {
    fn is_disabled(&self) -> bool;
}

/// Implements the Map editor Edit-Terrains UI Tab
pub struct MapEditorEditTerrainTab {
    edit_tab: Arc<MapEditorEditTab>,
    ruleset: Arc<Ruleset>,
}

impl MapEditorEditTerrainTab {
    pub fn new(edit_tab: Arc<MapEditorEditTab>, ruleset: Arc<Ruleset>) -> Self {
        Self {
            edit_tab,
            ruleset,
        }
    }

    fn all_terrains(&self) -> Vec<&Terrain> {
        self.ruleset.terrains.values()
            .filter(|t| t.terrain_type.is_base_terrain)
            .filter(|t| !t.has_unique(UniqueType::ExcludedFromMapEditor, StateForConditionals::IgnoreConditionals))
            .collect()
    }

    fn get_terrains(&self) -> Vec<FormattedLine> {
        self.all_terrains()
            .iter()
            .map(|t| FormattedLine::new(t.name.clone(), t.name.clone(), format!("Terrain/{}", t.name), 32))
            .collect()
    }
}

impl IMapEditorEditSubTabs for MapEditorEditTerrainTab {
    fn is_disabled(&self) -> bool {
        false // all_terrains().is_empty() // wanna see _that_ mod...
    }
}

impl BaseScreen for MapEditorEditTerrainTab {
    fn build(&mut self, ctx: &mut EguiContexts, game: &mut UncivGame) {
        let mut frame = Frame::new(egui::style::Frame::none())
            .inner_margin(10.0)
            .fill_x(true);

        let terrains = self.get_terrains();

        for terrain in terrains {
            let name = terrain.text.clone();
            let icon = terrain.icon.clone();

            let mut button = Button::new();
            button.set_text(&name);
            button.set_icon(&icon);
            button.set_icon_size(32.0);

            let edit_tab = self.edit_tab.clone();
            button.on_click(move || {
                edit_tab.set_brush(&name, &icon, |tile| {
                    tile.base_terrain = name.clone();
                    tile.natural_wonder = None;
                });
            });

            frame.add(button);
        }

        game.add_to_stage(&frame);
    }
}

/// Implements the Map editor Edit-Features UI Tab
pub struct MapEditorEditFeaturesTab {
    edit_tab: Arc<MapEditorEditTab>,
    ruleset: Arc<Ruleset>,
}

impl MapEditorEditFeaturesTab {
    pub fn new(edit_tab: Arc<MapEditorEditTab>, ruleset: Arc<Ruleset>) -> Self {
        Self {
            edit_tab,
            ruleset,
        }
    }

    fn allowed_features(&self) -> Vec<&Terrain> {
        self.ruleset.terrains.values()
            .filter(|t| t.terrain_type == TerrainType::TerrainFeature)
            .filter(|t| !t.has_unique(UniqueType::ExcludedFromMapEditor, StateForConditionals::IgnoreConditionals))
            .collect()
    }

    fn get_features(&self) -> Vec<FormattedLine> {
        self.allowed_features()
            .iter()
            .map(|t| FormattedLine::new(t.name.clone(), t.name.clone(), format!("Terrain/{}", t.name), 32))
            .collect()
    }
}

impl IMapEditorEditSubTabs for MapEditorEditFeaturesTab {
    fn is_disabled(&self) -> bool {
        self.allowed_features().is_empty()
    }
}

impl BaseScreen for MapEditorEditFeaturesTab {
    fn build(&mut self, ctx: &mut EguiContexts, game: &mut UncivGame) {
        let mut frame = Frame::new(egui::style::Frame::none())
            .inner_margin(10.0)
            .fill_x(true);

        let features = self.allowed_features();

        if let Some(first_feature) = features.first() {
            let eraser_icon = format!("Terrain/{}", first_feature.name);

            // Add eraser button
            let mut eraser_button = Button::new();
            eraser_button.set_text("Remove features");
            eraser_button.set_icon(&eraser_icon);
            eraser_button.set_icon_size(32.0);
            eraser_button.set_icon_crossed(true);

            let edit_tab = self.edit_tab.clone();
            eraser_button.on_click(move || {
                edit_tab.set_brush("Remove features", &eraser_icon, "", true, |tile| {
                    tile.remove_terrain_features();
                });
            });

            frame.add(eraser_button);

            // Add feature buttons
            for feature in features {
                let name = feature.name.clone();
                let icon = format!("Terrain/{}", name);

                let mut button = Button::new();
                button.set_text(&name);
                button.set_icon(&icon);
                button.set_icon_size(32.0);

                let edit_tab = self.edit_tab.clone();
                button.on_click(move || {
                    edit_tab.set_brush(&name, &icon, |tile| {
                        if !tile.terrain_features.contains(&name) {
                            tile.add_terrain_feature(&name);
                        }
                    });
                });

                frame.add(button);
            }
        }

        game.add_to_stage(&frame);
    }
}

/// Implements the Map editor Edit-NaturalWonders UI Tab
pub struct MapEditorEditWondersTab {
    edit_tab: Arc<MapEditorEditTab>,
    ruleset: Arc<Ruleset>,
}

impl MapEditorEditWondersTab {
    pub fn new(edit_tab: Arc<MapEditorEditTab>, ruleset: Arc<Ruleset>) -> Self {
        Self {
            edit_tab,
            ruleset,
        }
    }

    fn allowed_wonders(&self) -> Vec<&Terrain> {
        self.ruleset.terrains.values()
            .filter(|t| t.terrain_type == TerrainType::NaturalWonder)
            .filter(|t| !t.has_unique(UniqueType::ExcludedFromMapEditor, StateForConditionals::IgnoreConditionals))
            .collect()
    }

    fn get_wonders(&self) -> Vec<FormattedLine> {
        self.allowed_wonders()
            .iter()
            .map(|t| FormattedLine::new(t.name.clone(), t.name.clone(), format!("Terrain/{}", t.name), 32))
            .collect()
    }
}

impl IMapEditorEditSubTabs for MapEditorEditWondersTab {
    fn is_disabled(&self) -> bool {
        self.allowed_wonders().is_empty()
    }
}

impl BaseScreen for MapEditorEditWondersTab {
    fn build(&mut self, ctx: &mut EguiContexts, game: &mut UncivGame) {
        let mut frame = Frame::new(egui::style::Frame::none())
            .inner_margin(10.0)
            .fill_x(true);

        let wonders = self.get_wonders();

        for wonder in wonders {
            let name = wonder.text.clone();
            let icon = wonder.icon.clone();

            let mut button = Button::new();
            button.set_text(&name);
            button.set_icon(&icon);
            button.set_icon_size(32.0);

            let edit_tab = self.edit_tab.clone();
            let ruleset = self.ruleset.clone();
            button.on_click(move || {
                edit_tab.set_brush(&name, &icon, |tile| {
                    // Normally the caller would ensure compliance, but here we make an exception - place it no matter what
                    if let Some(turns_into) = ruleset.terrains.get(&name).and_then(|t| t.turns_into.clone()) {
                        tile.base_terrain = turns_into;
                    }
                    tile.remove_terrain_features();
                    tile.natural_wonder = Some(name.clone());
                });
            });

            frame.add(button);
        }

        game.add_to_stage(&frame);
    }
}

/// Implements the Map editor Edit-Resources UI Tab
pub struct MapEditorEditResourcesTab {
    edit_tab: Arc<MapEditorEditTab>,
    ruleset: Arc<Ruleset>,
}

impl MapEditorEditResourcesTab {
    pub fn new(edit_tab: Arc<MapEditorEditTab>, ruleset: Arc<Ruleset>) -> Self {
        Self {
            edit_tab,
            ruleset,
        }
    }

    fn allowed_resources(&self) -> Vec<&TileResource> {
        self.ruleset.tile_resources.values()
            .filter(|r| !r.has_unique(UniqueType::CityStateOnlyResource))
            .filter(|r| !r.has_unique(UniqueType::ExcludedFromMapEditor, StateForConditionals::IgnoreConditionals))
            .collect()
    }

    fn get_resources(&self) -> Vec<FormattedLine> {
        let mut result = Vec::new();
        let mut last_group = ResourceType::Bonus;

        for resource in self.allowed_resources() {
            let name = resource.name.clone();

            if resource.resource_type != last_group {
                last_group = resource.resource_type;
                result.push(FormattedLine::separator("#888"));
            }

            result.push(FormattedLine::new(name.clone(), name, format!("Resource/{}", name), 32));
        }

        result
    }
}

impl IMapEditorEditSubTabs for MapEditorEditResourcesTab {
    fn is_disabled(&self) -> bool {
        self.allowed_resources().is_empty()
    }
}

impl BaseScreen for MapEditorEditResourcesTab {
    fn build(&mut self, ctx: &mut EguiContexts, game: &mut UncivGame) {
        let mut frame = Frame::new(egui::style::Frame::none())
            .inner_margin(10.0)
            .fill_x(true);

        let resources = self.allowed_resources();

        if let Some(first_resource) = resources.first() {
            let eraser_icon = format!("Resource/{}", first_resource.name);

            // Add eraser button
            let mut eraser_button = Button::new();
            eraser_button.set_text("Remove resource");
            eraser_button.set_icon(&eraser_icon);
            eraser_button.set_icon_size(32.0);
            eraser_button.set_icon_crossed(true);

            let edit_tab = self.edit_tab.clone();
            eraser_button.on_click(move || {
                edit_tab.set_brush("Remove resource", &eraser_icon, "", true, |tile| {
                    tile.resource = None;
                    tile.resource_amount = 0;
                });
            });

            frame.add(eraser_button);

            // Add resource buttons
            let mut last_group = ResourceType::Bonus;

            for resource in resources {
                let name = resource.name.clone();
                let resource_type = resource.resource_type;

                if resource_type != last_group {
                    last_group = resource_type;
                    frame.add(Label::separator("#888"));
                }

                let mut button = Button::new();
                button.set_text(&name);
                button.set_icon(&format!("Resource/{}", name));
                button.set_icon_size(32.0);

                let edit_tab = self.edit_tab.clone();
                let ruleset = self.ruleset.clone();
                button.on_click(move || {
                    let resource = ruleset.tile_resources.get(&name).unwrap();
                    edit_tab.set_brush(&name, &resource.make_link(), |tile| {
                        if tile.resource.as_ref() == Some(&name) && resource.resource_type == ResourceType::Strategic {
                            tile.resource_amount = (tile.resource_amount + 1).min(42);
                        } else {
                            tile.set_tile_resource(resource, &mut edit_tab.randomness.rng);
                        }
                    });
                });

                frame.add(button);
            }
        }

        game.add_to_stage(&frame);
    }
}

/// Implements the Map editor Edit-Improvements UI Tab
pub struct MapEditorEditImprovementsTab {
    edit_tab: Arc<MapEditorEditTab>,
    ruleset: Arc<Ruleset>,
}

impl MapEditorEditImprovementsTab {
    pub fn new(edit_tab: Arc<MapEditorEditTab>, ruleset: Arc<Ruleset>) -> Self {
        Self {
            edit_tab,
            ruleset,
        }
    }

    fn allowed_improvements(&self) -> Vec<&TileImprovement> {
        self.ruleset.tile_improvements.values()
            .filter(|i| !i.has_unique(UniqueType::ExcludedFromMapEditor, StateForConditionals::IgnoreConditionals))
            .collect()
    }

    fn get_improvements(&self) -> Vec<FormattedLine> {
        let mut result = Vec::new();
        let mut last_group = 0;

        for improvement in self.allowed_improvements() {
            let name = improvement.name.clone();
            let group = improvement.group();

            if group != last_group {
                last_group = group;
                result.push(FormattedLine::separator("#888"));
            }

            result.push(FormattedLine::new(name.clone(), name, format!("Improvement/{}", name), 32));
        }

        result
    }

    fn improvement_group(improvement: &TileImprovement) -> i32 {
        if RoadStatus::entries().iter().any(|r| r.name == improvement.name) {
            2
        } else if improvement.uniques.contains(&"Great Improvement".to_string()) {
            3
        } else if improvement.unique_to.is_some() {
            4
        } else if improvement.uniques.contains(&"Unpillagable".to_string()) {
            5
        } else {
            0
        }
    }
}

impl IMapEditorEditSubTabs for MapEditorEditImprovementsTab {
    fn is_disabled(&self) -> bool {
        self.allowed_improvements().is_empty()
    }
}

impl BaseScreen for MapEditorEditImprovementsTab {
    fn build(&mut self, ctx: &mut EguiContexts, game: &mut UncivGame) {
        let mut frame = Frame::new(egui::style::Frame::none())
            .inner_margin(10.0)
            .fill_x(true);

        let improvements = self.allowed_improvements();

        if let Some(first_improvement) = improvements.first() {
            let eraser_icon = format!("Improvement/{}", first_improvement.name);

            // Add eraser button
            let mut eraser_button = Button::new();
            eraser_button.set_text("Remove improvement");
            eraser_button.set_icon(&eraser_icon);
            eraser_button.set_icon_size(32.0);
            eraser_button.set_icon_crossed(true);

            let edit_tab = self.edit_tab.clone();
            eraser_button.on_click(move || {
                edit_tab.set_brush("Remove improvement", &eraser_icon, "", true, |tile| {
                    tile.remove_improvement();
                    tile.remove_road();
                });
            });

            frame.add(eraser_button);

            // Add improvement buttons
            let mut last_group = 0;

            for improvement in improvements {
                let name = improvement.name.clone();
                let group = Self::improvement_group(improvement);

                if group != last_group {
                    last_group = group;
                    frame.add(Label::separator("#888"));
                }

                let mut button = Button::new();
                button.set_text(&name);
                button.set_icon(&format!("Improvement/{}", name));
                button.set_icon_size(32.0);

                let edit_tab = self.edit_tab.clone();
                button.on_click(move || {
                    if let Some(road) = RoadStatus::entries().iter().find(|r| r.name == name) {
                        edit_tab.set_brush(&name, &format!("Improvement/{}", name), BrushHandlerType::Road, |tile| {
                            tile.set_road_status(if tile.road_status == *road { RoadStatus::None } else { *road }, None);
                        });
                    } else {
                        edit_tab.set_brush(&name, &format!("Improvement/{}", name), |tile| {
                            tile.set_improvement(&name);
                        });
                    }
                });

                frame.add(button);
            }
        }

        game.add_to_stage(&frame);
    }
}

/// Implements the Map editor Edit-StartingLocations UI Tab
pub struct MapEditorEditStartsTab {
    edit_tab: Arc<MapEditorEditTab>,
    ruleset: Arc<Ruleset>,
    usage_option_group: Vec<CheckBox>,
}

impl MapEditorEditStartsTab {
    pub fn new(edit_tab: Arc<MapEditorEditTab>, ruleset: Arc<Ruleset>) -> Self {
        Self {
            edit_tab,
            ruleset,
            usage_option_group: Vec::new(),
        }
    }

    fn spectator_to_any_civ(name: &str) -> String {
        if name == SPECTATOR {
            "Any Civ".to_string()
        } else {
            name.to_string()
        }
    }

    fn allowed_nations(&self) -> Vec<&Nation> {
        self.ruleset.nations.values()
            .filter(|n| !n.has_unique(UniqueType::ExcludedFromMapEditor))
            .collect()
    }

    fn get_nations(&self) -> Vec<FormattedLine> {
        let mut nations = self.allowed_nations();

        // Sort nations: non-spectator first, then non-city-state, then alphabetically
        nations.sort_by(|a, b| {
            if a.is_spectator != b.is_spectator {
                return b.is_spectator.cmp(&a.is_spectator);
            }
            if a.is_city_state != b.is_city_state {
                return a.is_city_state.cmp(&b.is_city_state);
            }
            a.name.cmp(&b.name)
        });

        nations.iter()
            .map(|n| {
                let display_name = Self::spectator_to_any_civ(&n.name);
                FormattedLine::new(
                    format!("[{}] starting location", display_name),
                    n.name.clone(),
                    format!("Nation/{}", n.name),
                    24
                )
            })
            .collect()
    }
}

impl IMapEditorEditSubTabs for MapEditorEditStartsTab {
    fn is_disabled(&self) -> bool {
        self.allowed_nations().is_empty()
    }
}

impl BaseScreen for MapEditorEditStartsTab {
    fn build(&mut self, ctx: &mut EguiContexts, game: &mut UncivGame) {
        let mut frame = Frame::new(egui::style::Frame::none())
            .inner_margin(10.0)
            .fill_x(true);

        let nations = self.allowed_nations();

        if let Some(first_nation) = nations.first() {
            let eraser_icon = format!("Nation/{}", first_nation.name);

            // Add eraser button
            let mut eraser_button = Button::new();
            eraser_button.set_text("Remove starting locations");
            eraser_button.set_icon(&eraser_icon);
            eraser_button.set_icon_size(24.0);
            eraser_button.set_icon_crossed(true);

            let edit_tab = self.edit_tab.clone();
            eraser_button.on_click(move || {
                edit_tab.set_brush("Remove", &eraser_icon, BrushHandlerType::Direct, "", true, |tile| {
                    tile.tile_map.remove_starting_locations(tile.position);
                });
            });

            frame.add(eraser_button);

            // Add usage options
            let mut usage_frame = Frame::new(egui::style::Frame::none())
                .inner_margin(5.0);

            usage_frame.add(Label::new("Use for new game \"Select players\" button:"));

            let default_usage = StartingLocation::Usage::default();
            self.usage_option_group.clear();

            for usage in StartingLocation::Usage::entries() {
                let mut check_box = CheckBox::new(usage.label.tr());
                check_box.set_checked(usage == default_usage);
                self.usage_option_group.push(check_box.clone());
                usage_frame.add(check_box);
            }

            frame.add(usage_frame);

            // Add nation buttons
            let nations = self.get_nations();

            for nation in nations {
                let name = nation.text.clone();
                let link = nation.link.clone();
                let icon = format!("Nation/{}", link);

                let mut button = Button::new();
                button.set_text(&name);
                button.set_icon(&icon);
                button.set_icon_size(24.0);

                let edit_tab = self.edit_tab.clone();
                let ruleset = self.ruleset.clone();
                let usage_option_group = self.usage_option_group.clone();

                button.on_click(move || {
                    // Play nation theme music
                    game.music_controller.choose_track(&link, MusicMood::Theme, MusicTrackChooserFlags::set_specific);

                    let pedia_link = if link == SPECTATOR { "" } else { icon.clone() };
                    let is_major_civ = ruleset.nations.get(&link).map(|n| n.is_major_civ).unwrap_or(false);

                    let selected_usage = if is_major_civ {
                        let checked_index = usage_option_group.iter().position(|cb| cb.is_checked()).unwrap_or(0);
                        StartingLocation::Usage::entries()[checked_index]
                    } else {
                        StartingLocation::Usage::Normal
                    };

                    let display_name = Self::spectator_to_any_civ(&link);

                    edit_tab.set_brush(&display_name, &icon, BrushHandlerType::Direct, &pedia_link, |tile| {
                        // Toggle the starting location here, note this allows
                        // both multiple locations per nation and multiple nations per tile
                        if !tile.tile_map.add_starting_location(&link, tile, selected_usage) {
                            tile.tile_map.remove_starting_location(&link, tile);
                        }
                    });
                });

                frame.add(button);
            }
        }

        game.add_to_stage(&frame);
    }
}

/// Implements the Map editor Edit-Rivers UI Tab
pub struct MapEditorEditRiversTab {
    edit_tab: Arc<MapEditorEditTab>,
    ruleset: Arc<Ruleset>,
    icon_size: f32,
    show_on_terrain: Arc<Terrain>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RiverEdge {
    Left,
    Bottom,
    Right,
    All,
}

impl MapEditorEditRiversTab {
    pub fn new(edit_tab: Arc<MapEditorEditTab>, ruleset: Arc<Ruleset>) -> Self {
        let icon_size = 50.0;

        // Find the best terrain to show rivers on
        let show_on_terrain = ruleset.terrains.values()
            .filter(|t| t.terrain_type.is_base_terrain && !t.is_rough())
            .max_by_key(|t| t.production * 2 + t.food)
            .unwrap_or_else(|| {
                ruleset.terrains.get("Plains")
                    .unwrap_or_else(|| ruleset.terrains.values().next().unwrap())
            });

        Self {
            edit_tab,
            ruleset,
            icon_size,
            show_on_terrain: Arc::new(show_on_terrain.clone()),
        }
    }

    fn get_tile_group_with_rivers(&self, edge: RiverEdge) -> TileGroup {
        let mut tile = Tile::new();
        tile.base_terrain = self.show_on_terrain.name.clone();

        match edge {
            RiverEdge::Left => tile.has_bottom_left_river = true,
            RiverEdge::Bottom => tile.has_bottom_river = true,
            RiverEdge::Right => tile.has_bottom_right_river = true,
            RiverEdge::All => {
                tile.has_bottom_left_river = true;
                tile.has_bottom_right_river = true;
                tile.has_bottom_river = true;
            }
        }

        tile.make_tile_group(&self.ruleset, &game.settings, self.icon_size * 36.0 / 54.0)
    }

    fn get_remove_river_icon(&self) -> Image {
        ImageGetter::get_crossed_image(self.get_tile_group_with_rivers(RiverEdge::All), self.icon_size)
    }

    fn get_river_icon(&self, edge: RiverEdge) -> Image {
        let mut group = NonTransformGroup::new();
        group.set_size(self.icon_size, self.icon_size);

        let tile_group = self.get_tile_group_with_rivers(edge);
        tile_group.center(&group);
        group.add_actor(tile_group);

        group.into()
    }
}

impl IMapEditorEditSubTabs for MapEditorEditRiversTab {
    fn is_disabled(&self) -> bool {
        false
    }
}

impl TabbedPager::IPageExtensions for MapEditorEditRiversTab {
    fn activated(&self, index: i32, caption: &str, pager: &TabbedPager) {
        self.edit_tab.brush_size = 1;
    }
}

impl BaseScreen for MapEditorEditRiversTab {
    fn build(&mut self, ctx: &mut EguiContexts, game: &mut UncivGame) {
        let mut frame = Frame::new(egui::style::Frame::none())
            .inner_margin(10.0)
            .fill_x(true);

        let pedia_link = "Terrain/River";

        // Remove rivers button
        let mut remove_button = Button::new();
        remove_button.set_text("Remove rivers");
        remove_button.set_icon(self.get_remove_river_icon());

        let edit_tab = self.edit_tab.clone();
        remove_button.on_click(move || {
            edit_tab.set_brush(BrushHandlerType::River, "Remove rivers", self.get_remove_river_icon(), pedia_link, |tile| {
                tile.has_bottom_left_river = false;
                tile.has_bottom_right_river = false;
                tile.has_bottom_river = false;

                // User probably expects all six edges to be cleared
                let x = tile.position.x as i32;
                let y = tile.position.y as i32;

                if let Some(tile) = tile.tile_map.get_if_tile_exists_or_null(x, y + 1) {
                    tile.has_bottom_left_river = false;
                }

                if let Some(tile) = tile.tile_map.get_if_tile_exists_or_null(x + 1, y) {
                    tile.has_bottom_right_river = false;
                }

                if let Some(tile) = tile.tile_map.get_if_tile_exists_or_null(x + 1, y + 1) {
                    tile.has_bottom_river = false;
                }
            });
        });

        frame.add(remove_button);

        // Bottom left river button
        let mut left_river_button = Button::new();
        left_river_button.set_text("Bottom left river");
        left_river_button.set_icon(self.get_river_icon(RiverEdge::Left));

        let edit_tab = self.edit_tab.clone();
        left_river_button.on_click(move || {
            edit_tab.set_brush(BrushHandlerType::Direct, "Bottom left river", self.get_tile_group_with_rivers(RiverEdge::Left), pedia_link, |tile| {
                tile.has_bottom_left_river = !tile.has_bottom_left_river;
            });
        });

        frame.add(left_river_button);

        // Bottom river button
        let mut bottom_river_button = Button::new();
        bottom_river_button.set_text("Bottom river");
        bottom_river_button.set_icon(self.get_river_icon(RiverEdge::Bottom));

        let edit_tab = self.edit_tab.clone();
        bottom_river_button.on_click(move || {
            edit_tab.set_brush(BrushHandlerType::Direct, "Bottom river", self.get_tile_group_with_rivers(RiverEdge::Bottom), pedia_link, |tile| {
                tile.has_bottom_river = !tile.has_bottom_river;
            });
        });

        frame.add(bottom_river_button);

        // Bottom right river button
        let mut right_river_button = Button::new();
        right_river_button.set_text("Bottom right river");
        right_river_button.set_icon(self.get_river_icon(RiverEdge::Right));

        let edit_tab = self.edit_tab.clone();
        right_river_button.on_click(move || {
            edit_tab.set_brush(BrushHandlerType::Direct, "Bottom right river", self.get_tile_group_with_rivers(RiverEdge::Right), pedia_link, |tile| {
                tile.has_bottom_right_river = !tile.has_bottom_right_river;
            });
        });

        frame.add(right_river_button);

        // Spawn river from/to button
        let mut spawn_river_button = Button::new();
        spawn_river_button.set_text("Spawn river from/to");
        spawn_river_button.set_icon(self.get_river_icon(RiverEdge::All));

        let edit_tab = self.edit_tab.clone();
        spawn_river_button.on_click(move || {
            edit_tab.set_brush(
                BrushHandlerType::RiverFromTo,
                "Spawn river from/to",
                self.get_tile_group_with_rivers(RiverEdge::All),
                pedia_link,
                || {} // Actual effect done via BrushHandlerType
            );
        });

        frame.add(spawn_river_button);

        game.add_to_stage(&frame);
    }
}

/// Implements the Map editor Edit-Units UI Tab
pub struct MapEditorEditUnitsTab {
    edit_tab: Arc<MapEditorEditTab>,
    ruleset: Arc<Ruleset>,
}

impl MapEditorEditUnitsTab {
    pub fn new(edit_tab: Arc<MapEditorEditTab>, ruleset: Arc<Ruleset>) -> Self {
        Self {
            edit_tab,
            ruleset,
        }
    }
}

impl IMapEditorEditSubTabs for MapEditorEditUnitsTab {
    fn is_disabled(&self) -> bool {
        true
    }
}

impl BaseScreen for MapEditorEditUnitsTab {
    fn build(&mut self, ctx: &mut EguiContexts, game: &mut UncivGame) {
        let mut frame = Frame::new(egui::style::Frame::none())
            .inner_margin(10.0)
            .fill_x(true);

        let mut label = Label::new("Work in progress");
        label.set_color(egui::Color32::from_rgb(178, 34, 34)); // FIREBRICK
        label.set_font_size(24);

        frame.add(label);

        game.add_to_stage(&frame);
    }
}