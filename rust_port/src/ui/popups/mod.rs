// UI popups module

mod popup;
mod loading_popup;
mod confirm_popup;
mod city_screen_construction_menu;
mod auth_popup;
mod ask_text_popup;
mod ask_number_popup;
mod animated_menu_popup;
mod scrollable_animated_menu_popup;
mod toast_popup;

pub use popup::{Popup, PopupExt, Scrollability};
pub use loading_popup::LoadingPopup;
pub use confirm_popup::ConfirmPopup;
pub use city_screen_construction_menu::CityScreenConstructionMenu;
pub use auth_popup::AuthPopup;
pub use ask_text_popup::AskTextPopup;
pub use ask_number_popup::AskNumberPopup;
pub use animated_menu_popup::AnimatedMenuPopup;
pub use scrollable_animated_menu_popup::ScrollableAnimatedMenuPopup;
pub use toast_popup::ToastPopup;

use eframe::egui::Ui;

/// Trait for popup dialogs in the game
pub trait Popup {
    /// Show the popup in the given UI
    /// Returns true if the popup should be closed
    fn show(&mut self, ui: &mut Ui) -> bool;

    /// Get the title of the popup
    fn title(&self) -> String;
}