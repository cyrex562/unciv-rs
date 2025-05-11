// UI popups module
pub mod animated_menu_popup;
pub mod ask_number_popup;
pub mod ask_text_popup;
pub mod auth_popup;
pub mod city_screen_construction_menu;
pub mod confirm_popup;
pub mod loading_popup;
pub mod popup;
pub mod scrollable_animated_menu_popup;
pub mod toast_popup;
pub mod unit_upgrade_menu;
pub mod options;

/// Trait for popup dialogs in the game
// pub trait Popup {
//     /// Show the popup in the given UI
//     /// Returns true if the popup should be closed
//     fn show(&mut self, ui: &mut Ui) -> bool;
// 
//     /// Get the title of the popup
//     fn title(&self) -> String;
// }