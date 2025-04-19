// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/topbar/WorldScreenTopBarResources.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Response, Ui, Label, Image};
use std::collections::HashMap;

use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::images::ImageGetter;
use crate::game::civilization::CivilizationInfo;
use crate::game::civilization::managers::ResourceManager;

/// Resources section of the world screen top bar
pub struct WorldScreenTopBarResources {
    world_screen: Rc<RefCell<WorldScreen>>,
    resource_labels: HashMap<String, Label>,
    resource_images: HashMap<String, Image>,
    is_dirty: bool,
}

impl WorldScreenTopBarResources {
    /// Creates a new resources section
    pub fn new(world_screen: Rc<RefCell<WorldScreen>>) -> Self {
        Self {
            world_screen,
            resource_labels: HashMap::new(),
            resource_images: HashMap::new(),
            is_dirty: true,
        }
    }

    /// Updates the resources display
    pub fn update(&mut self) {
        if !self.is_dirty {
            return;
        }
        self.is_dirty = false;

        let world_screen = self.world_screen.borrow();
        let civ = world_screen.viewing_civ.borrow();

        // Clear existing resources
        self.resource_labels.clear();
        self.resource_images.clear();

        // Update resource displays
        self.update_resource_displays(&civ);
    }

    /// Updates the resource displays for a civilization
    fn update_resource_displays(&mut self, civ: &CivilizationInfo) {
        // Get resource amounts
        let resource_amounts = ResourceManager::get_resource_amounts(civ);

        // Create labels and images for each resource
        for (resource_name, amount) in resource_amounts {
            // Create resource label
            let label = Label::new(format!("{}", amount))
                .text_color(Color32::WHITE);
            self.resource_labels.insert(resource_name.clone(), label);

            // Create resource image
            if let Some(image) = ImageGetter::get_resource_image(&resource_name) {
                let sized_image = image.clone()
                    .max_size(Vec2::new(20.0, 20.0));
                self.resource_images.insert(resource_name, sized_image);
            }
        }
    }

    /// Draws the resources section
    pub fn draw(&self, ui: &mut Ui) -> Response {
        let mut response = ui.allocate_response(
            Vec2::new(0.0, 0.0),
            egui::Sense::hover(),
        );

        // Create horizontal layout
        ui.horizontal(|ui| {
            // Draw each resource
            for (resource_name, label) in &self.resource_labels {
                if let Some(image) = self.resource_images.get(resource_name) {
                    // Draw resource image
                    let image_response = ui.add(image.clone());
                    response = response.union(image_response);

                    // Add small spacing
                    ui.add_space(2.0);

                    // Draw resource amount
                    let label_response = ui.add(label.clone());
                    response = response.union(label_response);

                    // Add spacing between resources
                    ui.add_space(10.0);
                }
            }
        });

        response
    }

    /// Marks the resources as needing an update
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }
}