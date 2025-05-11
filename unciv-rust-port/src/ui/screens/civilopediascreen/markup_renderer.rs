use bevy::prelude::*;
use bevy_egui::egui::{self, Align, Color32, Frame, Layout, Rect, ScrollArea, Ui, Vec2};
use std::iter::IntoIterator;

use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::civilopediascreen::formatted_line::{FormattedLine, IconDisplay, LinkType};
use crate::ui::components::widgets::Button;
use crate::utils::clipboard::Clipboard;
use crate::utils::url::open_url;

/// Makes renderer available outside [ICivilopediaText]
pub struct MarkupRenderer;

impl MarkupRenderer {
    /// Height of empty line (`FormattedLine()`) - about half a normal text line, independent of font size
    const EMPTY_LINE_HEIGHT: f32 = 10.0;
    /// Default cell padding of non-empty lines
    const DEFAULT_PADDING: f32 = 2.5;
    /// Padding above a [separator][FormattedLine::separator] line
    const SEPARATOR_TOP_PADDING: f32 = 10.0;
    /// Padding below a [separator][FormattedLine::separator] line
    const SEPARATOR_BOTTOM_PADDING: f32 = 10.0;

    /// Build a UI element showing formatted content.
    ///
    /// # Arguments
    /// * `lines` - The formatted lines to render
    /// * `label_width` - Available width needed for wrapping labels and [centered][FormattedLine::centered] attribute.
    /// * `padding` - Default cell padding (default 2.5) to control line spacing
    /// * `icon_display` - Flag to omit link or all images (but not linking itself if link_action is supplied)
    /// * `link_action` - Delegate to call for internal links. Leave None to suppress linking.
    ///
    /// Returns a UI element containing the rendered content
    pub fn render(
        lines: &[FormattedLine],
        label_width: f32,
        padding: Option<f32>,
        icon_display: Option<IconDisplay>,
        link_action: Option<Box<dyn Fn(&str)>>,
    ) -> Frame {
        let padding = padding.unwrap_or(Self::DEFAULT_PADDING);
        let icon_display = icon_display.unwrap_or(IconDisplay::All);

        let mut frame = egui::Frame::new();
        let mut layout = Layout::left_to_right(egui::Align::TOP);

        for line in lines {
            if line.is_empty() {
                // Add empty line with padding
                frame.add(egui::Frame::new().min_size(Vec2::new(0.0, Self::EMPTY_LINE_HEIGHT)));
                continue;
            }

            if line.separator {
                // Add separator line
                let separator_height = if line.size == i32::MIN_VALUE { 2.0 } else { line.size as f32 };
                let separator_color = line.display_color();

                frame.add(egui::Frame::new()
                    .fill(separator_color)
                    .min_size(Vec2::new(label_width, separator_height))
                    .inner_margin(egui::style::Margin::same(Self::SEPARATOR_TOP_PADDING)));
                continue;
            }

            // Render the line
            let mut line_frame = line.render(label_width, icon_display);

            // Add click handlers for links
            if line.link_type() == LinkType::Internal {
                if let Some(action) = &link_action {
                    let link = line.link.clone();
                    let action_clone = action.clone();
                    line_frame = line_frame.on_click(move |_| {
                        action_clone(&link);
                    });
                }
            } else if line.link_type() == LinkType::External {
                let link = line.link.clone();
                line_frame = line_frame.on_click(move |_| {
                    open_url(&link);
                }).on_right_click(move |_| {
                    Clipboard::set_text(&link);
                });
            }

            // Add the line to the frame
            if label_width == 0.0 {
                frame.add(line_frame).align(line.align());
            } else {
                frame.add(line_frame).width(label_width).align(line.align());
            }
        }

        frame
    }

    /// Convenience method to render with default parameters
    pub fn render_default(
        lines: &[FormattedLine],
        label_width: f32,
        link_action: Option<Box<dyn Fn(&str)>>,
    ) -> Frame {
        Self::render(lines, label_width, None, None, link_action)
    }
}