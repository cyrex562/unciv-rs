use bevy::prelude::*;
use std::collections::HashSet;

use crate::ui::components::*;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::mapeditorscreen::MapEditorScreen;
use crate::ui::screens::mapeditorscreen::tabs::map_editor_edit_sub_tabs::IMapEditorEditSubTabs;
use crate::ui::screens::mapeditorscreen::tabs::map_editor_options_tab::TileMatchFuzziness;
use crate::logic::map::tile::Tile;
use crate::logic::map::bfs::BFS;
use crate::logic::map::mapgenerator::{MapGenerationRandomness, MapGenerator, RiverGenerator};
use crate::logic::map::tile_normalizer::TileNormalizer;
use crate::models::ruleset::Ruleset;
use crate::utils::logging::Log;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BrushHandlerType {
    None,
    Direct,
    Tile,
    Road,
    River,
    RiverFromTo,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AllEditSubTabs {
    Terrain,
    TerrainFeatures,
    NaturalWonders,
    Resources,
    Improvements,
    Rivers,
    StartingLocations,
}

impl AllEditSubTabs {
    fn caption(&self) -> &'static str {
        match self {
            Self::Terrain => "Terrain",
            Self::TerrainFeatures => "Features",
            Self::NaturalWonders => "Wonders",
            Self::Resources => "Resources",
            Self::Improvements => "Improvements",
            Self::Rivers => "Rivers",
            Self::StartingLocations => "Starting locations",
        }
    }

    fn key(&self) -> char {
        match self {
            Self::Terrain => 't',
            Self::TerrainFeatures => 'f',
            Self::NaturalWonders => 'w',
            Self::Resources => 'r',
            Self::Improvements => 'i',
            Self::Rivers => 'v',
            Self::StartingLocations => 's',
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            Self::Terrain => "OtherIcons/Terrains",
            Self::TerrainFeatures | Self::NaturalWonders | Self::Rivers => "OtherIcons/Star",
            Self::Resources => "OtherIcons/Resources",
            Self::Improvements => "OtherIcons/Improvements",
            Self::StartingLocations => "OtherIcons/Nations",
        }
    }
}

pub struct MapEditorEditTab {
    editor_screen: Entity,
    header_height: f32,
    sub_tabs: TabbedPager,
    brush_table: Table,
    brush_slider: Slider,
    brush_label: Label,
    brush_cell: Cell<Entity>,
    ruleset: Ruleset,
    randomness: MapGenerationRandomness,
    brush_handler_type: BrushHandlerType,
    brush_action: Box<dyn Fn(&mut Tile) + Send + Sync>,
    brush_size: i32,
    tile_match_fuzziness: TileMatchFuzziness,
    river_start_tile: Option<Tile>,
    river_end_tile: Option<Tile>,
}

impl MapEditorEditTab {
    pub fn new(editor_screen: Entity, header_height: f32) -> Self {
        let mut tab = Self {
            editor_screen,
            header_height,
            sub_tabs: TabbedPager::new(),
            brush_table: Table::new(),
            brush_slider: Slider::new(1.0, 6.0, 1.0),
            brush_label: Label::new("Brush ([1]):"),
            brush_cell: Cell::new(),
            ruleset: Ruleset::default(),
            randomness: MapGenerationRandomness::new(),
            brush_handler_type: BrushHandlerType::None,
            brush_action: Box::new(|_| {}),
            brush_size: 1,
            tile_match_fuzziness: TileMatchFuzziness::CompleteMatch,
            river_start_tile: None,
            river_end_tile: None,
        };

        tab.init();
        tab
    }

    fn init(&mut self) {
        // Initialize brush table
        self.brush_table.pad(5.0);
        self.brush_table.defaults().pad(10.0).left();
        self.brush_table.add(self.brush_label.clone());
        self.brush_cell = self.brush_table.add(Entity::default()).pad_left(0.0);

        self.brush_slider.set_initial(1.0);
        self.brush_slider.set_tip_text(|value| Self::get_brush_tip(value, false));
        self.brush_slider.on_change(|value| {
            self.brush_size = if value > 5.0 { -1 } else { value as i32 };
            self.brush_label.set_text(&format!("Brush ([{}]):", Self::get_brush_tip(value, true)));
        });

        self.brush_table.add(self.brush_slider.clone()).pad_left(0.0);

        // Initialize sub tabs
        let sub_tabs_height = self.editor_screen.stage.height - 2.0 * self.header_height
            - self.brush_table.pref_height() - 2.0 + 10.0;
        let sub_tabs_width = self.editor_screen.get_tools_width();

        self.sub_tabs.set_dimensions(
            sub_tabs_height,
            sub_tabs_height,
            sub_tabs_width,
            sub_tabs_width,
        );
        self.sub_tabs.set_header_padding(5.0);

        // Add pages for each sub tab
        for page in AllEditSubTabs::iter() {
            self.sub_tabs.add_page(
                page.caption(),
                Entity::default(),
                page.icon(),
                20.0,
                Some(page.key()),
                true,
            );
        }

        self.sub_tabs.select_page(0);

        // Add keyboard shortcuts
        self.add_keyboard_shortcuts();
    }

    pub fn set_brush<F>(&mut self, name: &str, icon: &str, handler_type: BrushHandlerType,
        pedia_link: &str, is_remove: bool, apply_action: F)
    where
        F: Fn(&mut Tile) + Send + Sync + 'static,
    {
        self.brush_handler_type = handler_type;
        let brush_actor = FormattedLine::new(name, icon, is_remove).render(0.0);
        self.link_civilopedia(brush_actor, pedia_link);
        self.brush_cell.set_actor(brush_actor);
        self.brush_action = Box::new(apply_action);
    }

    fn link_civilopedia(&mut self, brush_actor: Entity, link: &str) {
        if link.is_empty() {
            return;
        }
        brush_actor.set_touchable(true);
        brush_actor.on_activation(move || {
            self.editor_screen.open_civilopedia(link);
        });
        brush_actor.add_keyboard_shortcut(KeyboardBinding::Civilopedia);
    }

    pub fn activated(&mut self, index: usize, caption: &str, pager: &mut TabbedPager) {
        if self.editor_screen.edit_tabs_need_refresh {
            self.ruleset = self.editor_screen.ruleset.clone();
            ImageGetter::set_new_ruleset(&self.ruleset);

            for page in AllEditSubTabs::iter() {
                let tab = page.instantiate(self, &self.ruleset);
                self.sub_tabs.replace_page(page.caption(), tab);
                self.sub_tabs.set_page_disabled(page.caption(),
                    (tab.as_any().downcast_ref::<dyn IMapEditorEditSubTabs>()
                        .map(|t| t.is_disabled())
                        .unwrap_or(true)));
            }

            self.brush_handler_type = BrushHandlerType::None;
            self.editor_screen.edit_tabs_need_refresh = false;
        }

        self.editor_screen.tile_click_handler = Some(Box::new(|tile| self.tile_click_handler(tile)));
        pager.set_scroll_disabled(true);
        self.tile_match_fuzziness = self.editor_screen.tile_match_fuzziness;
    }

    pub fn deactivated(&mut self, _index: usize, _caption: &str, pager: &mut TabbedPager) {
        pager.set_scroll_disabled(true);
        self.editor_screen.tile_click_handler = None;
    }

    fn tile_click_handler(&mut self, tile: &mut Tile) {
        if self.brush_size < -1 || self.brush_size > 5 || self.brush_handler_type == BrushHandlerType::None {
            return;
        }
        if self.editor_screen.map_holder.is_panning || self.editor_screen.map_holder.is_zooming() {
            return;
        }
        self.editor_screen.hide_selection();

        match self.brush_handler_type {
            BrushHandlerType::None => (),
            BrushHandlerType::RiverFromTo => self.select_river_from_or_to(tile),
            _ => self.paint_tiles_with_brush(tile),
        }
    }

    fn select_river_from_or_to(&mut self, tile: &mut Tile) {
        let mut tiles_to_highlight = HashSet::new();
        tiles_to_highlight.insert(tile.clone());

        if tile.is_land {
            self.river_start_tile = Some(tile.clone());
            if self.river_end_tile.is_some() {
                return self.paint_river_from_to();
            }
            let river_generator = RiverGenerator::new(&self.editor_screen.tile_map, &self.randomness, &self.ruleset);
            self.river_end_tile = river_generator.get_closest_water_tile(tile);
            if let Some(end_tile) = &self.river_end_tile {
                tiles_to_highlight.insert(end_tile.clone());
            }
        } else {
            self.river_end_tile = Some(tile.clone());
            if self.river_start_tile.is_some() {
                return self.paint_river_from_to();
            }
        }

        for tile_to_highlight in tiles_to_highlight {
            self.editor_screen.highlight_tile(&tile_to_highlight, Color::BLUE);
        }
    }

    fn paint_river_from_to(&mut self) {
        let mut resulting_tiles = HashSet::new();
        self.randomness.seed_rng(self.editor_screen.new_map_parameters.seed);

        if let (Some(start_tile), Some(end_tile)) = (&self.river_start_tile, &self.river_end_tile) {
            let river_generator = RiverGenerator::new(&self.editor_screen.tile_map, &self.randomness, &self.ruleset);
            match river_generator.spawn_river(start_tile, end_tile, &mut resulting_tiles) {
                Ok(_) => {
                    MapGenerator::new(&self.ruleset).convert_terrains(&resulting_tiles);
                }
                Err(e) => {
                    Log::error("Exception while generating rivers", &e);
                    ToastPopup::new("River generation failed!", &self.editor_screen);
                }
            }
        }

        self.river_start_tile = None;
        self.river_end_tile = None;
        self.editor_screen.is_dirty = true;

        for tile in resulting_tiles {
            self.editor_screen.update_and_highlight(&tile, Color::SKY_BLUE);
        }
    }

    pub fn paint_tiles_with_brush(&mut self, tile: &mut Tile) {
        let tiles = if self.brush_size == -1 {
            let bfs = BFS::new(tile, |t| t.is_similar_enough(tile));
            bfs.step_to_end();
            bfs.get_reached_tiles()
        } else {
            tile.get_tiles_in_distance(self.brush_size - 1)
        };

        for mut tile_to_paint in tiles {
            match self.brush_handler_type {
                BrushHandlerType::Direct | BrushHandlerType::River => {
                    self.direct_paint_tile(&mut tile_to_paint);
                }
                BrushHandlerType::Tile | BrushHandlerType::Road => {
                    self.paint_tile(&mut tile_to_paint);
                }
                _ => {}
            }
        }

        // Update adjacent tiles due to rivers/edge tiles/roads
        let tiles_to_update: HashSet<_> = tiles.iter()
            .flat_map(|t| t.neighbors.iter().chain(std::iter::once(t)))
            .collect();

        for tile_to_update in tiles_to_update {
            self.editor_screen.update_tile(tile_to_update);
        }
    }

    fn direct_paint_tile(&mut self, tile: &mut Tile) {
        (self.brush_action)(tile);
        self.editor_screen.is_dirty = true;
        self.editor_screen.highlight_tile(tile);
    }

    fn paint_tile(&mut self, tile: &mut Tile) -> bool {
        let saved_tile = tile.clone();
        let mut painted_tile = tile.clone();
        (self.brush_action)(&mut painted_tile);
        painted_tile.ruleset = self.ruleset.clone();

        match painted_tile.set_terrain_transients() {
            Ok(_) => (),
            Err(e) => {
                if !e.to_string().ends_with("not exist in this ruleset!") {
                    return Err(e);
                }
                ToastPopup::new(&e.to_string(), &self.editor_screen);
            }
        }

        (self.brush_action)(tile);
        tile.set_terrain_transients().unwrap();
        TileNormalizer::normalize_to_ruleset(tile, &self.ruleset);

        if !painted_tile.is_similar_enough(tile) {
            tile.apply_from(&saved_tile);
            return false;
        }

        if tile.natural_wonder != saved_tile.natural_wonder {
            self.editor_screen.natural_wonders_need_refresh = true;
        }
        self.editor_screen.is_dirty = true;
        self.editor_screen.highlight_tile(tile);
        true
    }

    fn get_brush_tip(value: f32, abbreviate: bool) -> String {
        if value <= 5.0 {
            value.to_string()
        } else if abbreviate {
            "Floodfill_Abbreviation".to_string()
        } else {
            "Floodfill".to_string()
        }
    }

    fn add_keyboard_shortcuts(&mut self) {
        self.add_keyboard_shortcut('t', || self.select_page(0));
        self.add_keyboard_shortcut('f', || self.select_page(1));
        self.add_keyboard_shortcut('w', || self.select_page(2));
        self.add_keyboard_shortcut('r', || self.select_page(3));
        self.add_keyboard_shortcut('i', || self.select_page(4));
        self.add_keyboard_shortcut('v', || self.select_page(5));
        self.add_keyboard_shortcut('s', || self.select_page(6));
        self.add_keyboard_shortcut('u', || self.select_page(7));
        self.add_keyboard_shortcut('1', || self.brush_size = 1);
        self.add_keyboard_shortcut('2', || self.brush_size = 2);
        self.add_keyboard_shortcut('3', || self.brush_size = 3);
        self.add_keyboard_shortcut('4', || self.brush_size = 4);
        self.add_keyboard_shortcut('5', || self.brush_size = 5);
        self.add_keyboard_shortcut_ctrl('f', || self.brush_size = -1);
    }

    fn select_page(&mut self, index: usize) {
        self.sub_tabs.select_page(index);
    }
}

// Extension trait for Tile to support the is_similar_enough and apply_from methods
pub trait TileExtensions {
    fn is_similar_enough(&self, other: &Tile) -> bool;
    fn apply_from(&mut self, other: &Tile);
}

impl TileExtensions for Tile {
    fn is_similar_enough(&self, other: &Tile) -> bool {
        match self.tile_match_fuzziness {
            TileMatchFuzziness::CompleteMatch if
                self.improvement != other.improvement ||
                self.road_status != other.road_status => false,
            TileMatchFuzziness::NoImprovement if
                self.resource != other.resource => false,
            TileMatchFuzziness::BaseAndFeatures if
                self.terrain_features != other.terrain_features => false,
            TileMatchFuzziness::BaseTerrain if
                self.base_terrain != other.base_terrain => false,
            TileMatchFuzziness::LandOrWater if
                self.is_land != other.is_land => false,
            _ => self.natural_wonder == other.natural_wonder,
        }
    }

    fn apply_from(&mut self, other: &Tile) {
        self.base_terrain = other.base_terrain;
        self.terrain_features = other.terrain_features.clone();
        self.resource = other.resource;
        self.improvement = other.improvement;
        self.natural_wonder = other.natural_wonder;
        self.road_status = other.road_status;
        self.has_bottom_left_river = other.has_bottom_left_river;
        self.has_bottom_right_river = other.has_bottom_right_river;
        self.has_bottom_river = other.has_bottom_river;
        self.set_terrain_transients().unwrap();
    }
}