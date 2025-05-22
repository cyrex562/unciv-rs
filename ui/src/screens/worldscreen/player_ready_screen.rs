// Port of orig_src/core/src/com/unciv/ui/screens/worldscreen/PlayerReadyScreen.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Color32, Ui, Vec2};

use crate::ui::screens::base_screen::BaseScreen;
use crate::ui::screens::world_screen::WorldScreen;
use crate::game::civilization::Civilization;
use crate::ui::components::keyboard::KeyboardBinding;

pub struct PlayerReadyScreen {
    world_screen: Rc<RefCell<WorldScreen>>,
    viewing_civ: Rc<RefCell<Civilization>>,
    background_color: Color32,
    text_color: Color32,
}

impl PlayerReadyScreen {
    pub fn new(world_screen: Rc<RefCell<WorldScreen>>) -> Self {
        let viewing_civ = Rc::clone(&world_screen.borrow().viewing_civ);
        let background_color = viewing_civ.borrow().nation.get_outer_color();
        let text_color = viewing_civ.borrow().nation.get_inner_color();

        Self {
            world_screen,
            viewing_civ,
            background_color,
            text_color,
        }
    }

    pub fn show(&self, ui: &mut Ui) {
        let civ_name = self.viewing_civ.borrow().get_name();

        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(self.background_color)
                .margin(Vec2::new(10.0, 10.0)))
            .show(ui.ctx(), |ui| {
                ui.centered_and_justified(|ui| {
                    ui.heading(format!("[{}] ready?", civ_name))
                        .text_color(self.text_color);
                });
            });

        // Handle keyboard input
        if ui.input().key_pressed(KeyboardBinding::NextTurnAlternate.into()) {
            self.return_to_world_screen();
        }

        // Handle click anywhere
        if ui.input().pointer.any_pressed() {
            self.return_to_world_screen();
        }
    }

    fn return_to_world_screen(&self) {
        // TODO: Implement screen transition back to world screen
        // This will need game.replace_current_screen(world_screen) equivalent
    }
}

impl BaseScreen for PlayerReadyScreen {
    fn update(&mut self) {
        // No continuous updates needed for this screen
    }
}