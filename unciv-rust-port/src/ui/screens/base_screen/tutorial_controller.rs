use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::constants::Constants;
use crate::game::UncivGame;
use crate::models::TutorialTrigger;
use crate::models::ruleset::Tutorial;
use crate::ui::components::input::KeyCharAndCode;
use crate::ui::components::images::ImageGetter;
use crate::ui::popups::Popup;
use crate::ui::screens::base_screen::BaseScreen;
use crate::ui::screens::civilopedia_screen::ICivilopediaText;
use crate::utils::json::from_json_file;

/// Controller for managing and displaying tutorials
pub struct TutorialController<'a> {
    screen: &'a dyn BaseScreen,
    tutorial_queue: HashSet<TutorialTrigger>,
    is_tutorial_showing: bool,
    all_tutorials_showed_callback: Option<Box<dyn Fn() + Send + Sync>>,
    tutorial_render: TutorialRender<'a>,
    tutorials: HashMap<String, Tutorial>,
}

impl<'a> TutorialController<'a> {
    /// Create a new tutorial controller
    pub fn new(screen: &'a dyn BaseScreen) -> Self {
        Self {
            screen,
            tutorial_queue: HashSet::new(),
            is_tutorial_showing: false,
            all_tutorials_showed_callback: None,
            tutorial_render: TutorialRender::new(screen),
            tutorials: Self::load_tutorials_from_json(true),
        }
    }

    /// Load tutorials from JSON files
    pub fn load_tutorials_from_json(include_mods: bool) -> HashMap<String, Tutorial> {
        let mut result = HashMap::new();

        // Load base tutorials
        let base_tutorial_path = Path::new("assets/jsons/Tutorials.json");
        if base_tutorial_path.exists() {
            if let Ok(tutorials) = Self::load_tutorial_file(base_tutorial_path) {
                for tutorial in tutorials {
                    result.insert(tutorial.name.clone(), tutorial);
                }
            }
        }

        // Load mod tutorials if requested
        if include_mods {
            if let Some(game) = UncivGame::current() {
                if let Some(ruleset) = game.game_info.as_ref().and_then(|info| info.ruleset.as_ref()) {
                    if let Some(mods) = ruleset.mods.as_ref() {
                        for mod_name in mods {
                            let mod_tutorial_path = Path::new(&format!("assets/mods/{}/jsons/Tutorials.json", mod_name));
                            if mod_tutorial_path.exists() {
                                if let Ok(tutorials) = Self::load_tutorial_file(mod_tutorial_path) {
                                    for tutorial in tutorials {
                                        result.insert(tutorial.name.clone(), tutorial);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        result
    }

    /// Load a single tutorial file
    fn load_tutorial_file(path: &Path) -> Result<Vec<Tutorial>, Box<dyn std::error::Error>> {
        let file_content = fs::read_to_string(path)?;
        from_json_file(&file_content)
    }

    /// Show a tutorial
    pub fn show_tutorial(&mut self, tutorial: TutorialTrigger) {
        self.tutorial_queue.insert(tutorial);
        self.show_tutorial_if_needed();
    }

    /// Remove a tutorial from the queue
    fn remove_tutorial(&mut self, tutorial: TutorialTrigger) {
        self.is_tutorial_showing = false;
        self.tutorial_queue.remove(&tutorial);

        if let Some(game) = UncivGame::current() {
            if !game.settings.tutorials_shown.contains(&tutorial.name) {
                game.settings.tutorials_shown.insert(tutorial.name.clone());
                game.settings.save();
            }
        }

        self.show_tutorial_if_needed();
    }

    /// Show the next tutorial if needed
    fn show_tutorial_if_needed(&mut self) {
        if let Some(game) = UncivGame::current() {
            if !game.settings.show_tutorials {
                return;
            }

            if let Some(tutorial) = self.tutorial_queue.iter().next().cloned() {
                if !self.is_tutorial_showing {
                    self.is_tutorial_showing = true;
                    let texts = self.get_tutorial(&tutorial);

                    let tutorial_for_render = TutorialForRender {
                        tutorial: tutorial.clone(),
                        texts: texts.clone(),
                    };

                    let screen = self.screen;
                    let tutorial_clone = tutorial.clone();

                    self.tutorial_render.show_tutorial(tutorial_for_render, Box::new(move || {
                        // This closure will be called when the tutorial is closed
                        if let Some(controller) = screen.tutorial_controller() {
                            controller.remove_tutorial(tutorial_clone.clone());
                        }
                    }));
                }
            } else if let Some(callback) = &self.all_tutorials_showed_callback {
                callback();
            }
        }
    }

    /// Get the tutorial texts for a tutorial trigger
    fn get_tutorial(&self, tutorial: &TutorialTrigger) -> Vec<String> {
        let name = tutorial.value.replace('_', " ").trim_start().to_string();
        self.tutorials.get(&name)
            .map(|t| t.steps.clone())
            .unwrap_or_default()
    }

    /// Get all tutorials to be displayed in the Civilopedia
    pub fn get_civilopedia_tutorials(&self) -> Vec<Box<dyn ICivilopediaText>> {
        // TODO: This is essentially an 'un-private' kludge and the accessor
        // in CivilopediaCategories desperately needs independence from TutorialController:
        // Move storage to RuleSet someday?
        self.tutorials.values()
            .map(|t| Box::new(t.clone()) as Box<dyn ICivilopediaText>)
            .collect()
    }

    /// Set callback to be called when all tutorials are shown
    pub fn set_all_tutorials_showed_callback<F>(&mut self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.all_tutorials_showed_callback = Some(Box::new(callback));
    }
}

/// Data structure for rendering a tutorial
#[derive(Clone)]
pub struct TutorialForRender {
    pub tutorial: TutorialTrigger,
    pub texts: Vec<String>,
}

/// Renders tutorials as popups
pub struct TutorialRender<'a> {
    screen: &'a dyn BaseScreen,
}

impl<'a> TutorialRender<'a> {
    /// Create a new tutorial renderer
    pub fn new(screen: &'a dyn BaseScreen) -> Self {
        Self { screen }
    }

    /// Show a tutorial
    pub fn show_tutorial(&self, tutorial: TutorialForRender, close_action: Box<dyn Fn() + Send + Sync>) {
        self.show_dialog(&tutorial.tutorial.name, &tutorial.texts, close_action);
    }

    /// Show a dialog for a tutorial
    fn show_dialog(&self, tutorial_name: &str, texts: &[String], close_action: Box<dyn Fn() + Send + Sync>) {
        if texts.is_empty() {
            close_action();
            return;
        }

        let mut popup = Popup::new(self.screen);
        popup.set_name(format!("{}{}", Constants::TUTORIAL_POPUP_NAME_PREFIX, tutorial_name));

        // Add external image if available
        if let Some(external_image) = ImageGetter::find_external_image(tutorial_name) {
            popup.add(ImageGetter::get_external_image(external_image)).row();
        }

        // Add the first text
        popup.add_good_sized_label(&texts[0]).row();

        // Add close button
        let remaining_texts = texts[1..].to_vec();
        let screen = self.screen;

        popup.add_close_button(Some(KeyCharAndCode::SPACE), Box::new(move || {
            popup.remove();

            // Show the next dialog with remaining texts
            if let Some(controller) = screen.tutorial_controller() {
                let tutorial_render = TutorialRender::new(screen);
                tutorial_render.show_dialog(
                    tutorial_name,
                    &remaining_texts,
                    close_action.clone(),
                );
            }
        }));

        popup.open();
    }
}