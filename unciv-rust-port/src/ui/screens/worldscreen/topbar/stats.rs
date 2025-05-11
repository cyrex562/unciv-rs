// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/topbar/WorldScreenTopBarStats.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Response, Ui, Label, Image, Color32, Vec2};
use std::collections::HashMap;

use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::images::ImageGetter;
use crate::game::civilization::CivilizationInfo;
use crate::game::civilization::managers::StatsManager;

/// Stats section of the world screen top bar
pub struct WorldScreenTopBarStats {
    world_screen: Rc<RefCell<WorldScreen>>,
    stat_labels: HashMap<String, Label>,
    stat_images: HashMap<String, Image>,
    is_dirty: bool,
}

impl WorldScreenTopBarStats {
    /// Creates a new stats section
    pub fn new(world_screen: Rc<RefCell<WorldScreen>>) -> Self {
        Self {
            world_screen,
            stat_labels: HashMap::new(),
            stat_images: HashMap::new(),
            is_dirty: true,
        }
    }

    /// Updates the stats display
    pub fn update(&mut self) {
        if !self.is_dirty {
            return;
        }
        self.is_dirty = false;

        let world_screen = self.world_screen.borrow();
        let civ = world_screen.viewing_civ.borrow();

        // Clear existing stats
        self.stat_labels.clear();
        self.stat_images.clear();

        // Update stat displays
        self.update_stat_displays(&civ);
    }

    /// Updates the stat displays for a civilization
    fn update_stat_displays(&mut self, civ: &CivilizationInfo) {
        // Get civilization stats
        let stats = StatsManager::get_current_stats(civ);

        // Create labels and images for each stat type
        let stat_types = ["Science", "Gold", "Culture", "Happiness"];
        for stat_type in stat_types.iter() {
            let amount = match *stat_type {
                "Science" => stats.science,
                "Gold" => stats.gold,
                "Culture" => stats.culture,
                "Happiness" => stats.happiness,
                _ => 0.0,
            };

            // Create stat label with appropriate color
            let color = match *stat_type {
                "Science" => Color32::from_rgb(0, 153, 255),  // Light blue
                "Gold" => Color32::from_rgb(255, 215, 0),     // Gold
                "Culture" => Color32::from_rgb(255, 0, 255),  // Purple
                "Happiness" => Color32::from_rgb(255, 192, 203), // Pink
                _ => Color32::WHITE,
            };

            let label = Label::new(format!("{:+.1}", amount))
                .text_color(color);
            self.stat_labels.insert(stat_type.to_string(), label);

            // Create stat image
            if let Some(image) = ImageGetter::get_stat_icon(stat_type) {
                let sized_image = image.clone()
                    .max_size(Vec2::new(20.0, 20.0))
                    .tint(color);
                self.stat_images.insert(stat_type.to_string(), sized_image);
            }
        }
    }

    /// Draws the stats section
    pub fn draw(&self, ui: &mut Ui) -> Response {
        let mut response = ui.allocate_response(
            Vec2::new(0.0, 0.0),
            egui::Sense::hover(),
        );

        // Create horizontal layout
        ui.horizontal(|ui| {
            // Draw each stat
            for stat_type in ["Science", "Gold", "Culture", "Happiness"].iter() {
                if let Some(image) = self.stat_images.get(*stat_type) {
                    // Draw stat image
                    let image_response = ui.add(image.clone());
                    response = response.union(image_response);

                    // Add small spacing
                    ui.add_space(2.0);

                    // Draw stat amount
                    if let Some(label) = self.stat_labels.get(*stat_type) {
                        let label_response = ui.add(label.clone());
                        response = response.union(label_response);
                    }

                    // Add spacing between stats
                    ui.add_space(10.0);
                }
            }
        });

        response
    }

    /// Marks the stats as needing an update
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }
}