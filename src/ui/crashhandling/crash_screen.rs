use ggez::graphics::{self, Color, DrawParam, Drawable, Mesh, Rect, Text};
use ggez::mint::Vector2;
use ggez::{Context, GameResult};
use std::any::type_name;
use std::backtrace::{Backtrace, BacktraceStatus};
use std::panic::PanicInfo;
use std::sync::Arc;

use crate::constants::Constants;
use crate::game::UncivGame;
use crate::models::game_info::GameInfo;
use crate::ui::components::widgets::AutoScrollPane;
use crate::ui::components::widgets::IconTextButton;
use crate::ui::screens::base_screen::BaseScreen;
use crate::ui::screens::toast_popup::ToastPopup;
use crate::utils::log::Log;
use crate::utils::files::UncivFiles;
use crate::models::ruleset::RulesetCache;

/// Screen to crash to when an otherwise unhandled exception or error is thrown.
pub struct CrashScreen {
    /// The exception that caused the crash
    exception: Box<dyn std::any::Any + Send + 'static>,

    /// The formatted error report text
    text: String,

    /// Whether the error report has been copied to the clipboard
    copied: bool,

    /// The type name of the last active screen
    last_screen_type: String,

    /// The base screen that this crash screen extends
    base_screen: BaseScreen,
}

impl CrashScreen {
    /// Creates a new crash screen with the given exception
    ///
    /// # Arguments
    ///
    /// * `exception` - The exception that caused the crash
    pub fn new(exception: Box<dyn std::any::Any + Send + 'static>) -> Self {
        // Get the last screen type
        let last_screen_type = match UncivGame::current().screen() {
            Some(screen) => type_name::<dyn Drawable>(screen.as_ref()),
            None => "Could not get screen type",
        }.to_string();

        // Format the error report
        let text = Self::format_report(&Self::stringify_exception(&exception));

        Self {
            exception,
            text,
            copied: false,
            last_screen_type,
            base_screen: BaseScreen::new(),
        }
    }

    /// Converts an exception to a string representation
    ///
    /// # Arguments
    ///
    /// * `exception` - The exception to convert
    ///
    /// # Returns
    ///
    /// A string representation of the exception
    fn stringify_exception(exception: &Box<dyn std::any::Any + Send + 'static>) -> String {
        // In Rust, we can't directly get a stack trace from a Box<dyn Any>
        // We'll use a backtrace as a fallback
        let backtrace = Backtrace::capture();

        if backtrace.status() == BacktraceStatus::Captured {
            format!("{:?}", backtrace)
        } else {
            "No backtrace available".to_string()
        }
    }

    /// Attempts to get the save game as a string
    ///
    /// # Returns
    ///
    /// The save game as a string, or an empty string if it couldn't be retrieved
    fn try_get_save_game() -> String {
        match UncivGame::get_game_info_or_null() {
            Some(game_info) => {
                match UncivFiles::game_info_to_string(&game_info, true) {
                    Ok(save_data) => {
                        format!("\n**Save Data:**\n<details><summary>Show Saved Game</summary>\n\n```\n{}\n```\n</details>\n", save_data)
                    },
                    Err(e) => {
                        format!("\n**Save Data:**\n<details><summary>Show Saved Game</summary>\n\n```\nNo save data: {}\n```\n</details>\n", e)
                    }
                }
            },
            None => String::new(),
        }
    }

    /// Attempts to get the save mods as a string
    ///
    /// # Returns
    ///
    /// The save mods as a string, or an empty string if it couldn't be retrieved
    fn try_get_save_mods() -> String {
        let mut result = String::new();

        // Get mods from the last active save game
        if let Some(game_info) = UncivGame::get_game_info_or_null() {
            result.push_str("\n**Save Mods:**\n```\n");
            match game_info.game_parameters.get_mods_and_base_ruleset() {
                Ok(mods) => {
                    result.push_str(&mods.to_string());
                },
                Err(e) => {
                    result.push_str(&format!("No mod data: {}", e));
                }
            }
            result.push_str("\n```\n");
        }

        // Get visual mods
        let visual_mods = UncivGame::current().settings().visual_mods();
        if !visual_mods.is_empty() {
            result.push_str("**Permanent audiovisual Mods**:\n```\n");
            result.push_str(&visual_mods.to_string());
            result.push_str("\n```\n");
        }

        result
    }

    /// Formats the error report
    ///
    /// # Arguments
    ///
    /// * `message` - The error message
    ///
    /// # Returns
    ///
    /// A formatted error report
    fn format_report(message: &str) -> String {
        let indent = " ".repeat(4);
        let base_indent = indent.repeat(3); // To be even with the template string
        let sub_indent = base_indent.clone() + &indent; // To be one level more than the template string

        // Helper function to prepend indent to only new lines
        fn prepend_indent_to_only_new_lines(text: &str, indent: &str) -> String {
            let lines: Vec<&str> = text.lines().collect();
            if lines.is_empty() {
                return String::new();
            }

            let first_line = lines[0].to_string();
            let rest_lines: String = lines[1..].iter()
                .map(|line| format!("\n{}{}", indent, line))
                .collect();

            first_line + &rest_lines
        }

        // Get platform info
        let platform = "Desktop"; // In a real implementation, this would be more specific

        // Get version info
        let version = UncivGame::VERSION.to_string();

        // Get ruleset info
        let rulesets = RulesetCache::keys().join(", ");

        // Get system info
        let system_info = Log::get_system_info();

        // Format the report
        format!(
            "**Platform:** {}\n**Version:** {}\n**Rulesets:** {}\n**Last Screen:** `{}`\n\n\
            --------------------------------\n\n\
            {}\n\n\
            --------------------------------\n\n\n\
            **Message:**\n```\n{}\n```\n{}",
            prepend_indent_to_only_new_lines(platform, &sub_indent),
            prepend_indent_to_only_new_lines(&version, &sub_indent),
            prepend_indent_to_only_new_lines(&rulesets, &sub_indent),
            self.last_screen_type,
            prepend_indent_to_only_new_lines(&system_info, &base_indent),
            prepend_indent_to_only_new_lines(message, &base_indent),
            Self::try_get_save_mods() + &Self::try_get_save_game()
        )
    }

    /// Creates the layout table for the crash screen
    fn make_layout_table(&self) -> Box<dyn Drawable> {
        // In a real implementation, this would create a table with the layout
        // For now, we'll just return a simple container
        Box::new(CrashScreenLayout {
            title: self.make_title_label(),
            error_scroll: self.make_error_scroll(),
            instruction: self.make_instruction_label(),
            action_buttons: self.make_action_buttons_table(),
        })
    }

    /// Creates the title label for the crash screen
    fn make_title_label(&self) -> Text {
        let mut text = Text::new("An unrecoverable error has occurred in Unciv:");
        text.set_font_size(Constants::HEADING_FONT_SIZE);
        text
    }

    /// Creates the error scroll for the crash screen
    fn make_error_scroll(&self) -> Box<dyn Drawable> {
        // In a real implementation, this would create a scrollable text area
        // For now, we'll just return a simple text
        let mut text = Text::new(&self.text);
        text.set_font_size(15);
        Box::new(AutoScrollPane::new(text))
    }

    /// Creates the instruction label for the crash screen
    fn make_instruction_label(&self) -> Text {
        let mut text = Text::new("If this keeps happening, you can try disabling mods.\nYou can also report this on the issue tracker.");
        text
    }

    /// Creates the action buttons table for the crash screen
    fn make_action_buttons_table(&self) -> Box<dyn Drawable> {
        // In a real implementation, this would create a table with buttons
        // For now, we'll just return a simple container
        Box::new(ActionButtonsContainer {
            copy_button: IconTextButton::new("Copy", None, Constants::HEADING_FONT_SIZE),
            report_button: IconTextButton::new("Open Issue Tracker", Some("OtherIcons/Link"), Constants::HEADING_FONT_SIZE),
            close_button: IconTextButton::new("Close Unciv", None, Constants::HEADING_FONT_SIZE),
            copied: self.copied,
        })
    }

    /// Copies the error report to the clipboard
    pub fn copy_to_clipboard(&mut self) -> Result<(), String> {
        // In a real implementation, this would copy the text to the clipboard
        // For now, we'll just set the copied flag
        self.copied = true;
        Ok(())
    }

    /// Opens the issue tracker
    pub fn open_issue_tracker(&self) -> Result<(), String> {
        if self.copied {
            // In a real implementation, this would open the issue tracker
            // For now, we'll just return Ok
            Ok(())
        } else {
            Err("Please copy the error report first.".to_string())
        }
    }

    /// Closes the application
    pub fn close_application(&self) {
        // In a real implementation, this would close the application
        // For now, we'll just do nothing
    }
}

impl Drawable for CrashScreen {
    fn bounds(&self) -> Rect {
        self.base_screen.bounds()
    }

    fn draw(&self, ctx: &mut Context) -> GameResult {
        // Draw the base screen
        self.base_screen.draw(ctx)?;

        // Draw the layout table
        self.make_layout_table().draw(ctx)?;

        Ok(())
    }
}

/// A container for the crash screen layout
struct CrashScreenLayout {
    title: Text,
    error_scroll: Box<dyn Drawable>,
    instruction: Text,
    action_buttons: Box<dyn Drawable>,
}

impl Drawable for CrashScreenLayout {
    fn bounds(&self) -> Rect {
        Rect::new(0.0, 0.0, 800.0, 600.0) // Default size
    }

    fn draw(&self, ctx: &mut Context) -> GameResult {
        // Draw the title
        graphics::draw(ctx, &self.title, DrawParam::default())?;

        // Draw the error scroll
        graphics::draw(ctx, &*self.error_scroll, DrawParam::default())?;

        // Draw the instruction
        graphics::draw(ctx, &self.instruction, DrawParam::default())?;

        // Draw the action buttons
        graphics::draw(ctx, &*self.action_buttons, DrawParam::default())?;

        Ok(())
    }
}

/// A container for the action buttons
struct ActionButtonsContainer {
    copy_button: IconTextButton,
    report_button: IconTextButton,
    close_button: IconTextButton,
    copied: bool,
}

impl Drawable for ActionButtonsContainer {
    fn bounds(&self) -> Rect {
        Rect::new(0.0, 0.0, 800.0, 100.0) // Default size
    }

    fn draw(&self, ctx: &mut Context) -> GameResult {
        // Draw the copy button
        graphics::draw(ctx, &self.copy_button, DrawParam::default())?;

        // Draw the report button
        graphics::draw(ctx, &self.report_button, DrawParam::default())?;

        // Draw the close button
        graphics::draw(ctx, &self.close_button, DrawParam::default())?;

        Ok(())
    }
}