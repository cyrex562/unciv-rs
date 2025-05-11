use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use egui::{Ui, Color32, Align, ScrollArea, Button, Image, Response};
use crate::models::civilization::Civilization;
use crate::models::city::City;
use crate::models::spy::Spy;
use crate::models::spy_action::SpyAction;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::components::widgets::{TabbedPager, ExpanderTab};
use crate::ui::images::ImageGetter;
use crate::constants::Constants;
use crate::game::UncivGame;
use super::empire_overview_tab::EmpireOverviewTab;
use super::empire_overview_categories::EmpireOverviewCategories;

pub struct EspionageOverviewScreen {
    viewing_player: Rc<RefCell<Civilization>>,
    overview_screen: Rc<RefCell<dyn BaseScreen>>,
    persist_data: Rc<RefCell<EspionageOverviewTabPersistableData>>,
    spy_selection_table: Rc<RefCell<Ui>>,
    city_selection_table: Rc<RefCell<Ui>>,
    middle_panes: Rc<RefCell<Ui>>,
    selected_spy_button: Option<Rc<RefCell<Button>>>,
    selected_spy: Option<Rc<RefCell<Spy>>>,
    spy_action_buttons: HashMap<SpyCityActionButton, Option<Rc<RefCell<City>>>>,
    move_spy_buttons: HashMap<Rc<RefCell<Spy>>, Rc<RefCell<Button>>>,
}

impl EspionageOverviewScreen {
    pub fn new(
        viewing_player: Rc<RefCell<Civilization>>,
        overview_screen: Rc<RefCell<dyn BaseScreen>>,
        persist_data: Option<EspionageOverviewTabPersistableData>,
    ) -> Self {
        let mut screen = Self {
            viewing_player,
            overview_screen,
            persist_data: Rc::new(RefCell::new(persist_data.unwrap_or_default())),
            spy_selection_table: Rc::new(RefCell::new(Ui::default())),
            city_selection_table: Rc::new(RefCell::new(Ui::default())),
            middle_panes: Rc::new(RefCell::new(Ui::default())),
            selected_spy_button: None,
            selected_spy: None,
            spy_action_buttons: HashMap::new(),
            move_spy_buttons: HashMap::new(),
        };
        screen.update();
        screen
    }

    fn update(&mut self) {
        self.update_spy_list();
        self.update_city_list();
    }

    fn update_spy_list(&mut self) {
        let mut spy_selection_table = self.spy_selection_table.borrow_mut();
        spy_selection_table.clear();

        for spy in self.viewing_player.borrow().espionage_manager.get_spies() {
            let spy_ref = spy.borrow();

            // Add spy rank
            spy_selection_table.label(&spy_ref.rank.to_string());

            // Add location name
            spy_selection_table.label(&spy_ref.get_location_name());

            // Add action string
            let action_string = if spy_ref.action.show_turns {
                format!("[{}] {} turns", spy_ref.action.display_string, spy_ref.turns_remaining_for_action)
            } else {
                spy_ref.action.display_string.clone()
            };
            spy_selection_table.label(&action_string);

            // Create move spy button
            let move_spy_button = Button::new("Move");
            move_spy_button.on_click(|| {
                self.on_spy_clicked(move_spy_button.clone(), spy.clone());
            });
            move_spy_button.on_right_click(|| {
                self.on_spy_right_clicked(spy.clone());
            });

            // Disable button if spectator or spy is not alive
            if !self.overview_screen.borrow().can_change_state()
                || !spy_ref.is_alive()
                || self.viewing_player.borrow().is_defeated() {
                move_spy_button.disable();
            }

            self.move_spy_buttons.insert(spy.clone(), Rc::new(RefCell::new(move_spy_button)));
        }
    }

    fn update_city_list(&mut self) {
        let mut city_selection_table = self.city_selection_table.borrow_mut();
        city_selection_table.clear();

        // Add spy hideout
        city_selection_table.label("Spy Hideout");
        city_selection_table.add(self.get_spy_icons(
            self.viewing_player.borrow().espionage_manager.get_idle_spies()
        ));

        let move_spy_here_button = MoveToCityButton::new(None);
        city_selection_table.add(move_spy_here_button);
        city_selection_table.row();

        // Add cities
        let mut sorted_cities: Vec<Rc<RefCell<City>>> = self.viewing_player.borrow()
            .game_info.borrow()
            .get_cities()
            .iter()
            .filter(|city| self.viewing_player.borrow().has_explored(city.borrow().get_center_tile()))
            .cloned()
            .collect();

        // Sort cities
        sorted_cities.sort_by(|a, b| {
            let a_ref = a.borrow();
            let b_ref = b.borrow();

            // First by whether it's our city
            a_ref.civ.civ_name.cmp(&b_ref.civ.civ_name)
                .then(a_ref.civ.is_city_state.cmp(&b_ref.civ.is_city_state))
                .then(a_ref.civ.civ_name.cmp(&b_ref.civ.civ_name))
                .then(a_ref.name.cmp(&b_ref.name))
        });

        for city in sorted_cities {
            self.add_city_to_selection_table(city);
        }
    }

    fn add_city_to_selection_table(&mut self, city: Rc<RefCell<City>>) {
        let mut city_selection_table = self.city_selection_table.borrow_mut();
        let city_ref = city.borrow();

        // Add nation portrait
        city_selection_table.add(ImageGetter::get_nation_portrait(
            &city_ref.civ.borrow().nation,
            30.0
        ));

        // Add city name
        let label = city_selection_table.label(&city_ref.name);
        label.on_click(|| {
            let world_screen = UncivGame::current().reset_to_world_screen();
            world_screen.map_holder.set_center_position(city_ref.location);
        });

        // Add spy icons
        city_selection_table.add(self.get_spy_icons(
            self.viewing_player.borrow().espionage_manager.get_spies_in_city(&city)
        ));

        // Add coup button if applicable
        let spy = self.viewing_player.borrow().espionage_manager.get_spy_assigned_to_city(&city);
        if city_ref.civ.borrow().is_city_state && spy.is_some() && spy.as_ref().unwrap().borrow().can_do_coup() {
            let coup_button = CoupButton::new(city.clone(), spy.as_ref().unwrap().borrow().action == SpyAction::Coup);
            city_selection_table.add(coup_button);
        } else {
            let move_spy_here_button = MoveToCityButton::new(Some(city.clone()));
            city_selection_table.add(move_spy_here_button);
        }
    }

    fn get_spy_icon(&self, spy: &Rc<RefCell<Spy>>) -> Image {
        let spy_ref = spy.borrow();
        let mut icon = ImageGetter::get_image("OtherIcons/Spy_White");
        icon.color = Color32::WHITE;

        // Get color based on rank
        let get_color = |rank: i32| -> Color32 {
            match rank {
                1 => Color32::BROWN,
                2 => Color32::LIGHT_GRAY,
                _ => Color32::GOLD,
            }
        };

        // If rank >= 10, display with bigger star
        if spy_ref.rank >= 10 {
            let mut star = ImageGetter::get_image("OtherIcons/Star");
            star.color = get_color(spy_ref.rank / 10);
            icon.add(star).size(20.0).pad(3.0);
        }

        let color = get_color(spy_ref.rank);
        let mut star_table = Ui::default();

        // Create grid of up to 9 stars
        for i in 0..(spy_ref.rank % 10) {
            let mut star = ImageGetter::get_image("OtherIcons/Star");
            star.color = color;
            star_table.add(star).size(8.0).pad(1.0);
            if i % 3 == 2 {
                star_table.row();
            }
        }
        icon.add(star_table).center().pad_left(-4.0);

        // Add click handlers if spectator is allowed
        if self.overview_screen.borrow().can_change_state()
            && spy_ref.is_alive()
            && !self.viewing_player.borrow().is_defeated() {
            icon.on_click(|| {
                if let Some(button) = self.move_spy_buttons.get(spy) {
                    self.on_spy_clicked(button.clone(), spy.clone());
                }
            });
            icon.on_right_click(|| {
                self.on_spy_right_clicked(spy.clone());
            });
        }

        icon
    }

    fn get_spy_icons(&self, spies: Vec<Rc<RefCell<Spy>>>) -> Ui {
        let mut table = Ui::default();
        table.defaults().space(0.0, 2.0, 0.0, 2.0);

        for spy in spies {
            table.add(self.get_spy_icon(&spy));
        }

        table
    }

    fn on_spy_clicked(&mut self, move_spy_button: Rc<RefCell<Button>>, spy: Rc<RefCell<Spy>>) {
        if self.selected_spy_button.as_ref() == Some(&move_spy_button) {
            self.reset_selection();
            return;
        }

        self.reset_selection();
        self.selected_spy_button = Some(move_spy_button.clone());
        self.selected_spy = Some(spy.clone());

        if let Some(button) = &self.selected_spy_button {
            button.borrow_mut().set_text(Constants::cancel().tr());
        }

        for (button, city) in &self.spy_action_buttons {
            if let Some(city) = city {
                if city.borrow().id == spy.borrow().get_city_or_null().map(|c| c.borrow().id) {
                    button.set_visible(true);
                    button.set_direction(Align::RIGHT);
                } else {
                    button.set_visible(
                        city.borrow().espionage.has_spy_of(&self.viewing_player.borrow())
                    );
                    button.set_direction(Align::LEFT);
                }
            } else {
                button.set_visible(true);
                button.set_direction(Align::LEFT);
            }
        }
    }

    fn on_spy_right_clicked(&self, spy: Rc<RefCell<Spy>>) {
        let world_screen = self.overview_screen.borrow().as_any().downcast_ref::<WorldScreen>().unwrap();
        world_screen.bottom_unit_table.select_spy(spy);
        world_screen.game.pop_screen();
        world_screen.should_update = true;
    }

    fn reset_selection(&mut self) {
        self.selected_spy = None;
        if let Some(button) = &self.selected_spy_button {
            button.borrow_mut().set_text("Move".tr());
        }
        self.selected_spy_button = None;

        for (button, _) in &self.spy_action_buttons {
            button.set_visible(false);
        }
    }
}

impl EmpireOverviewTab for EspionageOverviewScreen {
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
pub struct EspionageOverviewTabPersistableData {
    selected_spy_id: Option<String>,
}

impl EmpireOverviewTabPersistableData for EspionageOverviewTabPersistableData {
    fn is_empty(&self) -> bool {
        self.selected_spy_id.is_none()
    }
}

// Helper structs for spy actions
pub struct SpyCityActionButton {
    direction: Align,
}

impl SpyCityActionButton {
    pub fn set_direction(&mut self, align: Align) {
        self.direction = align;
    }
}

pub struct MoveToCityButton {
    city: Option<Rc<RefCell<City>>>,
    arrow: Image,
}

impl MoveToCityButton {
    pub fn new(city: Option<Rc<RefCell<City>>>) -> Self {
        let mut button = Self {
            city,
            arrow: ImageGetter::get_arrow_image(Align::LEFT),
        };

        button.arrow.set_size(24.0);
        button.arrow.set_origin(Align::CENTER);
        button.arrow.color = Color32::WHITE;

        button
    }
}

pub struct CoupButton {
    city: Rc<RefCell<City>>,
    is_current_action: bool,
    fist: Image,
}

impl CoupButton {
    pub fn new(city: Rc<RefCell<City>>, is_current_action: bool) -> Self {
        let mut button = Self {
            city,
            is_current_action,
            fist: ImageGetter::get_stat_icon("Resistance"),
        };

        button.fist.set_size(24.0);
        button.fist.set_origin(Align::CENTER);
        button.fist.color = if is_current_action { Color32::WHITE } else { Color32::DARK_GRAY };

        button
    }
}