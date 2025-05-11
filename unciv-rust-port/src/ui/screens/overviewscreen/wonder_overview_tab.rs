// Source: orig_src/core/src/com/unciv/ui/screens/overviewscreen/WonderOverviewTab.kt

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use egui::{Ui, Color32, Align, Response, Image};
use crate::models::civilization::Civilization;
use crate::models::city::City;
use crate::models::map::tile::Tile;
use crate::models::ruleset::building::Building;
use crate::models::ruleset::quest::QuestName;
use crate::models::ruleset::tech::era::Era;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::components::widgets::ExpanderTab;
use crate::ui::images::ImageGetter;
use crate::constants::Constants;
use crate::game::UncivGame;
use crate::utils::debug_utils::DebugUtils;
use super::empire_overview_tab::EmpireOverviewTab;
use super::empire_overview_categories::EmpireOverviewCategories;
use super::civilopedia_categories::CivilopediaCategories;

pub struct WonderOverviewTab {
    viewing_player: Rc<RefCell<Civilization>>,
    overview_screen: Rc<RefCell<dyn BaseScreen>>,
    persist_data: Rc<RefCell<WonderOverviewTabPersistableData>>,
    wonder_info: WonderInfo,
    wonders: Vec<WonderInfoData>,
    fixed_content: Ui,
}

impl WonderOverviewTab {
    pub fn new(
        viewing_player: Rc<RefCell<Civilization>>,
        overview_screen: Rc<RefCell<dyn BaseScreen>>,
        persist_data: Option<WonderOverviewTabPersistableData>,
    ) -> Self {
        let wonder_info = WonderInfo;
        let wonders = wonder_info.collect_info(&viewing_player);

        let mut tab = Self {
            viewing_player,
            overview_screen,
            persist_data: Rc::new(RefCell::new(persist_data.unwrap_or_default())),
            wonder_info,
            wonders,
            fixed_content: Ui::default(),
        };

        tab.init();
        tab
    }

    fn init(&mut self) {
        // Initialize fixed content table
        let mut fixed_content = Ui::default();
        fixed_content.defaults().pad(10.0).align(Align::Center);

        // Add column headers
        fixed_content.add();
        fixed_content.add_label("Name");
        fixed_content.add_label("Status");
        fixed_content.add_label("Location");
        fixed_content.add().min_width(30.0);
        fixed_content.row();

        // Add dummy rows for equalize columns
        self.top();
        self.defaults().pad(10.0).align(Align::Center);
        for _ in 0..5 {
            self.add(); // dummies so equalizeColumns can work because the first grid cell is colspan(5)
        }
        self.row();

        // Create the grid
        self.create_grid();

        // Equalize columns
        self.equalize_columns(&fixed_content, self);

        self.fixed_content = fixed_content;
    }

    fn create_grid(&mut self) {
        let mut last_group = String::new();

        for wonder in &self.wonders {
            if wonder.status == WonderStatus::Hidden {
                continue;
            }

            if wonder.group_name != last_group {
                last_group = wonder.group_name.clone();

                let mut group_row = Ui::default();
                group_row.add(ImageGetter::get_dot(wonder.group_color)).min_height(2.0).grow_x();

                let mut label = group_row.add_label(&last_group, wonder.group_color);
                label.set_alignment(Align::Right);
                label.pad_left(1.0).right();

                self.add(group_row).fill_x().colspan(5).pad_bottom(0.0).row();
            }

            let image = wonder.get_image();
            if let Some(img) = image {
                let mut img_clone = img.clone();
                img_clone.on_click(|| {
                    self.overview_screen.borrow().open_civilopedia(wonder.make_link());
                });
                self.add(img_clone).pad(0.0, 10.0, 0.0, 10.0);
            }

            let name_label = self.add_label(wonder.get_name_column(), true);
            name_label.pad(15.0, 10.0, 15.0, 10.0);

            self.add_label(wonder.get_status_column());

            let location_text = wonder.get_location_column();
            if !location_text.is_empty() {
                let mut location_label = self.add_label(&location_text, true);

                if let Some(location) = &wonder.location {
                    location_label.on_click(|| {
                        let world_screen = UncivGame::current().reset_to_world_screen();
                        world_screen.map_holder.set_center_position(location.position);
                    });
                }

                self.add(location_label).fill_y();
            }

            self.row();
        }
    }

    pub fn get_fixed_content(&self) -> &Ui {
        &self.fixed_content
    }
}

impl EmpireOverviewTab for WonderOverviewTab {
    fn viewing_player(&self) -> &Rc<RefCell<Civilization>> {
        &self.viewing_player
    }

    fn overview_screen(&self) -> &Rc<RefCell<dyn BaseScreen>> {
        &self.overview_screen
    }

    fn persist_data(&self) -> &Rc<RefCell<dyn EmpireOverviewTabPersistableData>> {
        &self.persist_data
    }
}

#[derive(Default)]
pub struct WonderOverviewTabPersistableData {
    // Add any persistent data fields here
}

impl EmpireOverviewTabPersistableData for WonderOverviewTabPersistableData {
    fn is_empty(&self) -> bool {
        true // Implement based on actual fields
    }
}

// WonderInfo implementation
pub struct WonderInfo;

#[derive(Debug, Clone, PartialEq)]
pub enum WonderStatus {
    Hidden,
    Unknown,
    Unbuilt,
    NotFound,
    Known,
    Owned,
}

impl WonderStatus {
    fn label(&self) -> &str {
        match self {
            WonderStatus::Hidden => "",
            WonderStatus::Unknown => "Unknown",
            WonderStatus::Unbuilt => "Not built",
            WonderStatus::NotFound => "Not found",
            WonderStatus::Known => "Known",
            WonderStatus::Owned => "Owned",
        }
    }
}

pub struct WonderInfoData {
    pub name: String,
    pub category: CivilopediaCategories,
    pub group_name: String,
    pub group_color: Color32,
    pub status: WonderStatus,
    pub civ: Option<Rc<RefCell<Civilization>>>,
    pub city: Option<Rc<RefCell<City>>>,
    pub location: Option<Rc<RefCell<Tile>>>,
}

impl WonderInfoData {
    fn get_image(&self) -> Option<Image> {
        let view_entire_map_for_debug = DebugUtils::VISIBLE_MAP;

        if self.status == WonderStatus::Unknown && !view_entire_map_for_debug {
            None
        } else {
            let size = if self.category == CivilopediaCategories::Terrain { 50.0 } else { 45.0 };
            Some(self.category.get_image(&self.name, size))
        }
    }

    fn get_name_column(&self) -> String {
        let view_entire_map_for_debug = DebugUtils::VISIBLE_MAP;

        if view_entire_map_for_debug {
            self.name.clone()
        } else if self.status == WonderStatus::Unknown {
            self.status.label().to_string()
        } else {
            self.name.clone()
        }
    }

    fn get_status_column(&self) -> String {
        if self.status != WonderStatus::Known {
            self.status.label().to_string()
        } else if let Some(civ) = &self.civ {
            civ.borrow().civ_name.clone()
        } else {
            self.status.label().to_string()
        }
    }

    fn get_location_column(&self) -> String {
        let view_entire_map_for_debug = DebugUtils::VISIBLE_MAP;

        if self.status <= WonderStatus::NotFound {
            String::new()
        } else if let Some(location) = &self.location {
            if location.borrow().is_city_center() {
                if let Some(city) = location.borrow().get_city() {
                    city.borrow().name.clone()
                } else {
                    String::new()
                }
            } else if let Some(city) = location.borrow().get_city() {
                format!("Near [{}]", city.borrow().name)
            } else if let Some(city) = &self.city {
                format!("Somewhere around [{}]", city.borrow().name)
            } else if view_entire_map_for_debug {
                location.borrow().position.to_string()
            } else {
                "Far away".to_string()
            }
        } else {
            String::new()
        }
    }

    fn make_link(&self) -> String {
        format!("{}/{}", self.category.name, self.name)
    }
}

impl WonderInfo {
    fn should_be_displayed(&self, viewing_player: &Rc<RefCell<Civilization>>, wonder: &Building, wonder_era: Option<i32>) -> bool {
        !wonder.is_hidden_from_civilopedia(&viewing_player.borrow().game_info) &&
        (wonder_era.is_none() || wonder_era.unwrap() <= viewing_player.borrow().get_era_number())
    }

    /// Do we know about a natural wonder despite not having found it yet?
    fn known_from_quest(&self, viewing_player: &Rc<RefCell<Civilization>>, name: &str) -> bool {
        // No, *your* civInfo's QuestManager has no idea about your quests
        for civ in &viewing_player.borrow().game_info.civilizations {
            for quest in civ.borrow().quest_manager.get_assigned_quests_for(&viewing_player.borrow().civ_name) {
                if quest.quest_name == QuestName::FindNaturalWonder.value && quest.data1 == name {
                    return true;
                }
            }
        }
        false
    }

    fn collect_info(&self, viewing_player: &Rc<RefCell<Civilization>>) -> Vec<WonderInfoData> {
        let collator = UncivGame::current().settings.get_collator_from_locale();
        let ruleset = &viewing_player.borrow().game_info.ruleset;

        // Maps all World Wonders by name to their era for grouping
        let wonder_era_map: HashMap<String, Option<Rc<RefCell<Era>>>> = ruleset.buildings.values()
            .filter(|it| it.is_wonder)
            .map(|it| (it.name.clone(), it.era(ruleset)))
            .collect();

        // Maps all World Wonders by their position in sort order to their name
        let mut all_wonder_map: HashMap<i32, String> = HashMap::new();
        let mut sorted_wonders: Vec<_> = ruleset.buildings.values()
            .filter(|it| it.is_wonder)
            .collect();

        // Sort wonders by era and name
        sorted_wonders.sort_by(|a, b| {
            let a_era = wonder_era_map.get(&a.name)
                .and_then(|e| e.as_ref().map(|era| era.borrow().era_number))
                .unwrap_or(100);

            let b_era = wonder_era_map.get(&b.name)
                .and_then(|e| e.as_ref().map(|era| era.borrow().era_number))
                .unwrap_or(100);

            a_era.cmp(&b_era).then_with(|| {
                collator.compare(&a.name.tr(true), &b.name.tr(true))
            })
        });

        for (index, wonder) in sorted_wonders.iter().enumerate() {
            all_wonder_map.insert(index as i32, wonder.name.clone());
        }

        let wonder_count = all_wonder_map.len();

        // Inverse of the above
        let wonder_index_map: HashMap<String, i32> = all_wonder_map.iter()
            .map(|(&k, v)| (v.clone(), k))
            .collect();

        // Maps all Natural Wonders on the map by name to their tile
        let all_naturals_map: HashMap<String, Rc<RefCell<Tile>>> = viewing_player.borrow().game_info.tile_map.values()
            .filter(|it| it.borrow().is_natural_wonder())
            .map(|it| (it.borrow().natural_wonder.unwrap().clone(), Rc::clone(it)))
            .collect();

        let naturals_count = all_naturals_map.len();

        // Natural Wonders sort order index to name
        let mut naturals_index_map: HashMap<i32, String> = HashMap::new();
        let mut sorted_naturals: Vec<_> = all_naturals_map.keys().cloned().collect();

        sorted_naturals.sort_by(|a, b| {
            collator.compare(&a.tr(), &b.tr())
        });

        for (index, name) in sorted_naturals.iter().enumerate() {
            naturals_index_map.insert(index as i32, name.clone());
        }

        // Pre-populate result with "Unknown" entries
        let mut wonders = Vec::with_capacity(wonder_count + naturals_count);

        for index in 0..(wonder_count + naturals_count) {
            if index < wonder_count {
                let wonder_name = all_wonder_map.get(&(index as i32)).unwrap();
                let wonder = ruleset.buildings.get(wonder_name).unwrap();
                let era = wonder_era_map.get(wonder_name);

                let status = if self.should_be_displayed(viewing_player, wonder, era.as_ref().and_then(|e| e.as_ref().map(|era| era.borrow().era_number))) {
                    WonderStatus::Unbuilt
                } else {
                    WonderStatus::Hidden
                };

                wonders.push(WonderInfoData {
                    name: wonder_name.clone(),
                    category: CivilopediaCategories::Wonder,
                    group_name: era.as_ref().and_then(|e| e.as_ref().map(|era| era.borrow().name.clone())).unwrap_or_else(|| "Other".to_string()),
                    group_color: era.as_ref().and_then(|e| e.as_ref().map(|era| era.borrow().get_color())).unwrap_or(Color32::WHITE),
                    status,
                    civ: None,
                    city: None,
                    location: None,
                });
            } else {
                let name = naturals_index_map.get(&((index - wonder_count) as i32)).unwrap();

                wonders.push(WonderInfoData {
                    name: name.clone(),
                    category: CivilopediaCategories::Terrain,
                    group_name: "Natural Wonders".to_string(),
                    group_color: Color32::FOREST,
                    status: WonderStatus::Unknown,
                    civ: None,
                    city: None,
                    location: None,
                });
            }
        }

        // Update wonders with built wonders
        for city in viewing_player.borrow().game_info.get_cities() {
            let built_buildings: Vec<_> = city.borrow().city_constructions.get_built_buildings()
                .iter()
                .map(|it| it.name.clone())
                .collect();

            for wonder_name in built_buildings.iter().filter(|name| wonder_index_map.contains_key(*name)) {
                let index = wonder_index_map.get(wonder_name).unwrap();

                let status = if Rc::ptr_eq(&city.borrow().civ, viewing_player) {
                    WonderStatus::Owned
                } else if viewing_player.borrow().has_explored(city.borrow().get_center_tile()) {
                    WonderStatus::Known
                } else {
                    WonderStatus::NotFound
                };

                wonders[*index as usize] = WonderInfoData {
                    name: wonder_name.clone(),
                    category: CivilopediaCategories::Wonder,
                    group_name: wonders[*index as usize].group_name.clone(),
                    group_color: wonders[*index as usize].group_color,
                    status,
                    civ: Some(Rc::clone(&city.borrow().civ)),
                    city: Some(Rc::clone(&city)),
                    location: Some(Rc::clone(city.borrow().get_center_tile())),
                };
            }
        }

        // Update natural wonders
        for (index, name) in naturals_index_map.iter() {
            let tile = all_naturals_map.get(name).unwrap();
            let civ = tile.borrow().get_owner();

            let status = if let Some(civ) = &civ {
                if Rc::ptr_eq(civ, viewing_player) {
                    WonderStatus::Owned
                } else if viewing_player.borrow().natural_wonders.contains(name) {
                    WonderStatus::Known
                } else {
                    WonderStatus::NotFound
                }
            } else {
                WonderStatus::NotFound
            };

            if status == WonderStatus::NotFound && !self.known_from_quest(viewing_player, name) {
                continue;
            }

            let city = if status == WonderStatus::NotFound {
                None
            } else {
                viewing_player.borrow().game_info.get_cities()
                    .iter()
                    .filter(|it| {
                        it.borrow().get_center_tile().borrow().aerial_distance_to(&tile) <= 5 &&
                        viewing_player.borrow().knows(&it.borrow().civ) &&
                        viewing_player.borrow().has_explored(it.borrow().get_center_tile())
                    })
                    .min_by_key(|it| it.borrow().get_center_tile().borrow().aerial_distance_to(&tile))
                    .map(|it| Rc::clone(it))
            };

            wonders[(*index + wonder_count) as usize] = WonderInfoData {
                name: name.clone(),
                category: CivilopediaCategories::Terrain,
                group_name: "Natural Wonders".to_string(),
                group_color: Color32::FOREST,
                status,
                civ,
                city,
                location: Some(Rc::clone(tile)),
            };
        }

        wonders
    }
}