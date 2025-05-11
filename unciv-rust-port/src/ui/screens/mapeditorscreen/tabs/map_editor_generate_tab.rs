use bevy::prelude::*;
use std::sync::Arc;

use crate::ui::components::*;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::mapeditorscreen::MapEditorScreen;
use crate::ui::screens::mapeditorscreen::MapGeneratorSteps;
use crate::ui::screens::newgamescreen::MapParametersTable;
use crate::logic::map::{MapGeneratedMainType, MapParameters, MapType, TileMap};
use crate::logic::map::mapgenerator::MapGenerator;
use crate::models::ruleset::{Ruleset, RulesetCache};
use crate::utils::concurrency::Concurrency;
use crate::utils::logging::Log;
use crate::utils::translations::tr;

pub struct MapEditorGenerateTab {
    editor_screen: Entity,
    header_height: f32,
    new_tab: MapEditorNewMapTab,
    partial_tab: MapEditorGenerateStepsTab,
    name: String,
}

impl MapEditorGenerateTab {
    pub fn new(editor_screen: Entity, header_height: f32) -> Self {
        let mut tab = Self {
            editor_screen,
            header_height,
            new_tab: MapEditorNewMapTab::new(Arc::new(editor_screen)),
            partial_tab: MapEditorGenerateStepsTab::new(Arc::new(editor_screen)),
            name: "Generate".to_string(),
        };

        tab.init();
        tab
    }

    fn init(&mut self) {
        self.top();
        self.add_page(
            "New map",
            self.new_tab.clone(),
            ImageGetter::get_image("OtherIcons/New"),
            20.0,
            Some(KeyCharAndCode::ctrl('n')),
        );
        self.add_page(
            "Partial",
            self.partial_tab.clone(),
            ImageGetter::get_image("OtherIcons/Settings"),
            20.0,
            Some(KeyCharAndCode::ctrl('g')),
        );
        self.select_page(0);
        self.set_buttons_enabled(true);
        self.partial_tab.generate_button.disable(); // Starts with choice "None"
    }

    fn set_buttons_enabled(&mut self, enable: bool) {
        self.new_tab.generate_button.set_enabled(enable);
        self.new_tab.generate_button.set_text(if enable { "Create".tr() } else { Constants::WORKING.tr() });
        self.partial_tab.generate_button.set_enabled(enable);
        self.partial_tab.generate_button.set_text(if enable { "Generate".tr() } else { Constants::WORKING.tr() });
    }

    fn generate(&mut self, step: MapGeneratorSteps) {
        if self.new_tab.map_parameters_table.randomize_seed {
            // reseed visibly if the "Randomize seed" checkbox is checked
            self.new_tab.map_parameters_table.reseed();
        }

        let mut map_parameters = self.editor_screen.new_map_parameters.clone(); // this clone is very important here
        if let Some(message) = map_parameters.map_size.fix_undesired_sizes(map_parameters.world_wrap) {
            Concurrency::run_on_gl_thread(|| {
                ToastPopup::new(message, &self.editor_screen, 4000);
                self.new_tab.map_parameters_table.run(|table| {
                    map_parameters.map_size.also(|size| {
                        table.custom_map_size_radius.set_text(size.radius.tr());
                        table.custom_map_width.set_text(size.width.tr());
                        table.custom_map_height.set_text(size.height.tr());
                    });
                });
            });
            return;
        }

        if step == MapGeneratorSteps::Landmass && map_parameters.map_type == MapType::Empty {
            ToastPopup::new(
                "Please don't use step 'Landmass' with map type 'Empty', create a new empty map instead.".tr(),
                &self.editor_screen,
            );
            return;
        }

        // Remove input processing - nothing will be clicked!
        self.editor_screen.set_input_processor(None);
        self.set_buttons_enabled(false);

        let fresh_map_completed = |generated_map: TileMap, map_parameters: MapParameters, new_ruleset: Ruleset, select_page: usize| {
            MapEditorScreen::save_default_parameters(&map_parameters);
            self.editor_screen.load_map(generated_map, new_ruleset, select_page); // also reactivates inputProcessor
            self.editor_screen.is_dirty = true;
            self.set_buttons_enabled(true);
        };

        let step_completed = |step: MapGeneratorSteps| {
            if step == MapGeneratorSteps::NaturalWonders {
                self.editor_screen.natural_wonders_need_refresh = true;
            }
            self.editor_screen.map_holder.update_tile_groups();
            self.editor_screen.is_dirty = true;
            self.set_buttons_enabled(true);
            self.editor_screen.set_input_processor(Some(self.editor_screen.stage.clone()));
        };

        // Map generation can take a while and we don't want ANRs
        self.editor_screen.start_background_job("MapEditor.MapGenerator", move || {
            let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                let (new_ruleset, generator) = if step > MapGeneratorSteps::Landmass {
                    (None, None)
                } else {
                    let new_ruleset = RulesetCache::get_complex_ruleset(&map_parameters);
                    (Some(new_ruleset.clone()), Some(MapGenerator::new(&new_ruleset)))
                };

                match step {
                    MapGeneratorSteps::All => {
                        let generated_map = generator.unwrap().generate_map(&map_parameters);
                        let saved_scale = self.editor_screen.map_holder.scale_x;
                        Concurrency::run_on_gl_thread(|| {
                            fresh_map_completed(generated_map, map_parameters, new_ruleset.unwrap(), 0);
                            self.editor_screen.map_holder.zoom(saved_scale);
                        });
                    }
                    MapGeneratorSteps::Landmass => {
                        // This step _could_ run on an existing tileMap, but that opens a loophole where you get hills on water - fixing that is more expensive than always recreating
                        map_parameters.map_type = MapType::Empty;
                        let generated_map = generator.unwrap().generate_map(&map_parameters);
                        map_parameters.map_type = self.editor_screen.new_map_parameters.map_type;
                        generator.unwrap().generate_single_step(&mut generated_map, step);
                        let saved_scale = self.editor_screen.map_holder.scale_x;
                        Concurrency::run_on_gl_thread(|| {
                            fresh_map_completed(generated_map, map_parameters, new_ruleset.unwrap(), 1);
                            self.editor_screen.map_holder.zoom(saved_scale);
                        });
                    }
                    _ => {
                        self.editor_screen.tile_map.map_parameters.seed = map_parameters.seed;
                        MapGenerator::new(&self.editor_screen.ruleset).generate_single_step(&mut self.editor_screen.tile_map, step);
                        Concurrency::run_on_gl_thread(|| {
                            step_completed(step);
                        });
                    }
                }
                Ok(())
            })();

            if let Err(e) = result {
                Log::error("Exception while generating map", &e);
                Concurrency::run_on_gl_thread(|| {
                    self.set_buttons_enabled(true);
                    self.editor_screen.set_input_processor(Some(self.editor_screen.stage.clone()));
                    Popup::new(&self.editor_screen)
                        .add_good_sized_label("It looks like we can't make a map with the parameters you requested!".tr())
                        .row()
                        .add_close_button()
                        .open();
                });
            }
        });
    }
}

pub struct MapEditorNewMapTab {
    parent: Arc<Entity>,
    generate_button: TextButton,
    map_parameters_table: MapParametersTable,
}

impl MapEditorNewMapTab {
    pub fn new(parent: Arc<Entity>) -> Self {
        let mut tab = Self {
            parent: parent.clone(),
            generate_button: TextButton::new(""),
            map_parameters_table: MapParametersTable::new(
                None,
                parent.new_map_parameters.clone(),
                MapGeneratedMainType::Generated,
                true,
            ),
        };

        tab.init();
        tab
    }

    fn init(&mut self) {
        self.top();
        self.pad(10.0);
        self.add(Label::new("Map Options".tr()).with_font_size(24.0)).row();
        self.add(self.map_parameters_table.clone()).row();
        self.add(self.generate_button.clone()).pad_top(15.0).row();

        self.generate_button.on_click(|| {
            self.parent.generate(MapGeneratorSteps::All);
        });

        self.map_parameters_table.resource_select_box.on_change(|| {
            self.parent.run(|| {
                // normally the 'new map' parameters are independent, this needs to be an exception so strategic resource painting will use it
                self.parent.tile_map.map_parameters.map_resources = self.parent.new_map_parameters.map_resources;
            });
        });
    }
}

pub struct MapEditorGenerateStepsTab {
    parent: Arc<Entity>,
    option_group: ButtonGroup<CheckBox>,
    generate_button: TextButton,
    choice: MapGeneratorSteps,
    new_map_parameters: MapParameters,
    tile_map: TileMap,
    actual_map_parameters: MapParameters,
}

impl MapEditorGenerateStepsTab {
    pub fn new(parent: Arc<Entity>) -> Self {
        let mut tab = Self {
            parent: parent.clone(),
            option_group: ButtonGroup::new(),
            generate_button: TextButton::new(""),
            choice: MapGeneratorSteps::None,
            new_map_parameters: parent.new_map_parameters.clone(),
            tile_map: parent.tile_map.clone(),
            actual_map_parameters: parent.tile_map.map_parameters.clone(),
        };

        tab.init();
        tab
    }

    fn init(&mut self) {
        self.top();
        self.pad(10.0);
        self.defaults().pad(2.5);
        self.add(Label::new("Generator steps".tr()).with_font_size(24.0)).row();

        self.option_group.set_min_check_count(0);

        for option in MapGeneratorSteps::iter() {
            if option <= MapGeneratorSteps::All {
                continue;
            }

            let check_box = CheckBox::new(option.label().tr())
                .on_change(|| {
                    self.choice = option;
                    self.generate_button.enable();
                });

            self.add(check_box.clone()).row();
            self.option_group.add(check_box);
        }

        self.add(self.generate_button.clone()).pad_top(15.0).row();

        self.generate_button.on_click(|| {
            self.parent.generate(self.choice);
            if let Some(copy_parameters) = self.choice.copy_parameters() {
                copy_parameters(&self.new_map_parameters, &mut self.actual_map_parameters);
            }
        });
    }
}

// Extension trait for MapGeneratorSteps to support the copy_parameters method
pub trait MapGeneratorStepsExtensions {
    fn copy_parameters(&self) -> Option<Box<dyn Fn(&MapParameters, &mut MapParameters) + Send + Sync>>;
}

impl MapGeneratorStepsExtensions for MapGeneratorSteps {
    fn copy_parameters(&self) -> Option<Box<dyn Fn(&MapParameters, &mut MapParameters) + Send + Sync>> {
        match self {
            MapGeneratorSteps::Resources => Some(Box::new(|src, dst| {
                dst.map_resources = src.map_resources.clone();
            })),
            MapGeneratorSteps::Improvements => Some(Box::new(|src, dst| {
                dst.map_improvements = src.map_improvements.clone();
            })),
            _ => None,
        }
    }
}