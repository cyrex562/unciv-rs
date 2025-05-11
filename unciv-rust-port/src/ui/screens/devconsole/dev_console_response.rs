use egui::Color32;

/// Response from the developer console
#[derive(Debug, Clone)]
pub struct DevConsoleResponse {
    /// Color of the response text
    pub color: Color32,
    /// Optional message to display
    pub message: Option<String>,
    /// Whether the command executed successfully
    pub is_ok: bool,
}

impl DevConsoleResponse {
    /// Creates a new successful response with no message
    pub const fn ok() -> Self {
        Self {
            color: Color32::GREEN,
            message: None,
            is_ok: true,
        }
    }

    /// Creates a new successful response with a message
    pub fn ok_with_message(message: impl Into<String>) -> Self {
        Self {
            color: Color32::GREEN,
            message: Some(message.into()),
            is_ok: true,
        }
    }

    /// Creates a new error response
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            color: Color32::RED,
            message: Some(message.into()),
            is_ok: false,
        }
    }

    /// Creates a new hint response
    pub fn hint(message: impl Into<String>) -> Self {
        Self {
            color: Color32::GOLD,
            message: Some(message.into()),
            is_ok: false,
        }
    }
}