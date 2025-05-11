use std::collections::VecDeque;
use crate::models::ruleset::unique::{IHasUniques, Unique, UniqueType};
use crate::ui::screens::civilopedia_screen::FormattedLine;

/// Extension trait for objects that have uniques to add description-related functionality
pub trait DescriptionHelpersExt: IHasUniques {
    /// Appends user-visible Uniques as translated text to a line collection.
    /// Follows json order.
    fn uniques_to_description(
        &self,
        line_list: &mut Vec<String>,
        exclude: impl Fn(&Unique) -> bool,
    ) {
        for unique in self.unique_objects() {
            if unique.is_hidden_to_users() {
                continue;
            }
            if exclude(unique) {
                continue;
            }
            line_list.push(unique.get_display_text().tr());
        }
    }

    /// Returns a sequence of user-visible Uniques as FormattedLines.
    ///
    /// # Arguments
    /// * `leading_separator` - If there are lines to display and this parameter is not None,
    ///   a leading line is output, as separator or empty line.
    /// * `sorted` - If set, sorts alphabetically (not using a locale-specific Collator).
    ///   Otherwise lists in json order.
    /// * `color_consumes_resources` - If set, ConsumesResources Uniques get a reddish color.
    /// * `exclude` - Predicate that can exclude Uniques by returning true.
    fn uniques_to_civilopedia_text_lines(
        &self,
        leading_separator: Option<bool>,
        sorted: bool,
        color_consumes_resources: bool,
        exclude: impl Fn(&Unique) -> bool,
    ) -> Vec<FormattedLine> {
        let mut lines = Vec::new();
        let mut ordered_uniques: Vec<_> = self.unique_objects()
            .filter(|unique| !unique.is_hidden_to_users() && !exclude(unique))
            .collect();

        if sorted {
            ordered_uniques.sort_by(|a, b| a.text.cmp(&b.text));
        }

        for (index, unique) in ordered_uniques.iter().enumerate() {
            if leading_separator.is_some() && index == 0 {
                lines.push(FormattedLine::new_separator(leading_separator.unwrap()));
            }

            // Special case for ConsumesResources to give it a reddish color
            // Also ensures link always points to the resource
            if color_consumes_resources && unique.unique_type == UniqueType::ConsumesResources {
                lines.push(FormattedLine::new(
                    unique.get_display_text(),
                    Some(format!("Resources/{}", unique.params[1])),
                    Some("#F42".to_string()),
                    None,
                    None,
                    None,
                ));
            } else {
                lines.push(FormattedLine::from_unique(unique));
            }
        }

        lines
    }

    /// Appends user-visible Uniques as FormattedLines to a line collection.
    ///
    /// # Arguments
    /// * `line_list` - The collection to append to
    /// * `leading_separator` - If there are lines to display and this parameter is not None,
    ///   a leading line is output, as separator or empty line.
    /// * `sorted` - If set, sorts alphabetically (not using a locale-specific Collator).
    ///   Otherwise lists in json order.
    /// * `color_consumes_resources` - If set, ConsumesResources Uniques get a reddish color.
    /// * `exclude` - Predicate that can exclude Uniques by returning true.
    fn uniques_to_civilopedia_text_lines_mut(
        &self,
        line_list: &mut Vec<FormattedLine>,
        leading_separator: Option<bool>,
        sorted: bool,
        color_consumes_resources: bool,
        exclude: impl Fn(&Unique) -> bool,
    ) {
        line_list.extend(self.uniques_to_civilopedia_text_lines(
            leading_separator,
            sorted,
            color_consumes_resources,
            exclude,
        ));
    }
}

// Implement the trait for any type that implements IHasUniques
impl<T: IHasUniques> DescriptionHelpersExt for T {}