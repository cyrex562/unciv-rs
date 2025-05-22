use egui::{Color32, Response, Ui};
use egui_extras::Size;

use crate::ui::components::fonts::Fonts;
use crate::ui::components::widgets::ImageGetter;
use crate::utils::translations::tr;

/// A button with an icon and text, used in the mod management screen.
pub struct ModDecoratedButton {
    icon: String,
    text: String,
    tooltip: Option<String>,
    enabled: bool,
    on_click: Option<Box<dyn Fn()>>,
}

impl ModDecoratedButton {
    pub fn new(icon: String, text: String) -> Self {
        Self {
            icon,
            text,
            tooltip: None,
            enabled: true,
            on_click: None,
        }
    }

    pub fn with_tooltip(mut self, tooltip: String) -> Self {
        self.tooltip = Some(tooltip);
        this
    }

    pub fn with_enabled(mut this, enabled: bool) -> Self {
        self.enabled = enabled;
        this
    }

    pub fn on_click<F: Fn() + 'static>(mut this, f: F) -> Self {
        self.on_click = Some(Box::new(f));
        this
    }

    pub fn show(&self, ui: &mut Ui) -> Response {
        let button = egui::Button::new(format!("{} {}", self.icon, self.text))
            .enabled(self.enabled)
            .min_size(Size::new(100.0, 30.0));

        let response = ui.add(button);

        if response.hovered() {
            if let Some(tooltip) = &self.tooltip {
                egui::show_tooltip(ui.ctx(), egui::Id::new("mod_button_tooltip"), |ui| {
                    ui.label(tooltip);
                });
            }
        }

        if response.clicked() {
            if let Some(on_click) = &self.on_click {
                on_click();
            }
        }

        response
    }
}

/// Factory methods for creating common mod action buttons
impl ModDecoratedButton {
    pub fn install_button() -> Self {
        Self::new(Fonts::DOWNLOAD.to_string(), tr("Install"))
            .with_tooltip(tr("Install this mod"))
    }

    pub fn update_button() -> Self {
        Self::new(Fonts::UPDATE.to_string(), tr("Update"))
            .with_tooltip(tr("Update this mod"))
    }

    pub fn uninstall_button() -> Self {
        Self::new(Fonts::DELETE.to_string(), tr("Uninstall"))
            .with_tooltip(tr("Uninstall this mod"))
    }

    pub fn enable_button() -> Self {
        Self::new(Fonts::CHECK.to_string(), tr("Enable"))
            .with_tooltip(tr("Enable this mod"))
    }

    pub fn disable_button() -> Self {
        Self::new(Fonts::CROSS.to_string(), tr("Disable"))
            .with_tooltip(tr("Disable this mod"))
    }

    pub fn refresh_button() -> Self {
        Self::new(Fonts::REFRESH.to_string(), tr("Refresh"))
            .with_tooltip(tr("Refresh mod list"))
    }

    pub fn settings_button() -> Self {
        Self::new(Fonts::SETTINGS.to_string(), tr("Settings"))
            .with_tooltip(tr("Mod settings"))
    }

    pub fn info_button() -> Self {
        Self::new(Fonts::INFO.to_string(), tr("Info"))
            .with_tooltip(tr("Mod information"))
    }
}