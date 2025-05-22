// Source: orig_src/core/src/com/unciv/ui/screens/pickerscreens/CityRenamePopup.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Ui, Window, TextEdit, Response};
use crate::models::city::City;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::popups::ask_text_popup::AskTextPopup;
use crate::utils::translation::tr;

/// Popup to allow renaming a city.
///
/// Note - The translated name will be offered, and translation markers are removed.
/// The saved name will not treat translation in any way, so possibly the user will see his text unexpectedly translated if there is a translation entry for it.
pub struct CityRenamePopup {
    screen: Rc<RefCell<dyn BaseScreen>>,
    city: Rc<RefCell<City>>,
    action_on_close: Box<dyn Fn()>,
}

impl CityRenamePopup {
    pub fn new(
        screen: Rc<RefCell<dyn BaseScreen>>,
        city: Rc<RefCell<City>>,
        action_on_close: Box<dyn Fn()>,
    ) -> Self {
        let popup = Self {
            screen,
            city,
            action_on_close,
        };

        popup.open();
        popup
    }

    fn open(&self) {
        let city_name = self.city.borrow().name.clone();
        let translated_name = tr(&city_name, true);

        AskTextPopup::new(
            Rc::clone(&self.screen),
            "Please enter a new name for your city".to_string(),
            translated_name,
            Box::new(|text| !text.is_empty()),
            Box::new(move |text| {
                self.city.borrow_mut().name = text;
                (self.action_on_close)();
            }),
        ).open();
    }
}