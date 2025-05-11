use std::collections::HashSet;
use crate::models::ruleset::{
    ModOptions,
    Ruleset,
    unique::{IHasUniques, StateForConditionals, Unique, UniqueType},
    validation::{RulesetError, RulesetErrorSeverity},
};
use crate::models::files::FileHandle;
use crate::models::json::JsonSerializer;

/// Module for handling how mod authors can suppress RulesetValidator output.
///
/// This allows the outside code to be agnostic about how each entry operates, it's all here, and can easily be expanded.
/// `unique_doc_description`, `parameter_doc_description`, `parameter_doc_example`, `is_error_suppressed` and `is_valid_filter` need to agree on the rules!
/// Note there is minor influence on the rules where `RulesetErrorList.add` is called, as supplying null for its source parameters limits the scope where suppressions are found.
///
/// Current decisions:
/// * You cannot suppress `RulesetErrorSeverity.Error` level messages.
/// * Each suppression entry is either compared verbatim or as primitive wildcard pattern with '*' on both ends, case-insensitive.
/// * Minimum selectivity of `minimum_selectivity` characters matching.
/// * Validation of the suppression entries themselves is rudimentary.
pub struct Suppression;

impl Suppression {
    /// Minimum match length for a valid suppression filter
    const MINIMUM_SELECTIVITY: usize = 12; // arbitrary

    /// Documentation for the `UniqueType::SuppressWarnings` unique type
    pub const UNIQUE_DOC_DESCRIPTION: &'static str =
        "Allows suppressing specific validation warnings. \
        Errors, deprecation warnings, or warnings about untyped and non-filtering uniques should be heeded, not suppressed, and are therefore not accepted. \
        Note that this can be used in ModOptions, in the uniques a warning is about, or as modifier on the unique triggering a warning - \
        but you still need to be specific. Even in the modifier case you will need to specify a sufficiently selective portion of the warning text as parameter.";

    /// Documentation for the `UniqueParameterType::ValidationWarning` parameter type
    pub const PARAMETER_DOC_DESCRIPTION: &'static str =
        "Suppresses one specific Ruleset validation warning. \
        This can specify the full text verbatim including correct upper/lower case, \
        or it can be a wildcard case-insensitive simple pattern starting and ending in an asterisk ('*'). \
        If the suppression unique is used within an object or as modifier (not ModOptions), \
        the wildcard symbols can be omitted, as selectivity is better due to the limited scope.";

    /// Example for the `UniqueParameterType::ValidationWarning` parameter type
    pub const PARAMETER_DOC_EXAMPLE: &'static str =
        "Tinman is supposed to automatically upgrade at tech Clockwork, and therefore Servos for its upgrade Mecha may not yet be researched! \
        -or- *is supposed to automatically upgrade*";

    const DEPRECATION_WARNING_PATTERN: &'static str = r#"unique "~" is deprecated as of ~, replace with"#;
    const UNTYPED_WARNING_PATTERN: &'static str = r#"unique "~" not found in Unciv's unique types, and is not used as a filtering unique"#;

    /// Determine whether `parameter_text` is a valid Suppression filter as implemented by `is_error_suppressed`
    pub fn is_valid_filter(parameter_text: &str) -> bool {
        // Cannot contain {} or <>
        if parameter_text.contains('{') || parameter_text.contains('<') {
            return false;
        }

        // Must not be a deprecation - these should be implemented by their replacement not suppressed
        if Self::has_common_substring_length(parameter_text, Self::DEPRECATION_WARNING_PATTERN, Self::MINIMUM_SELECTIVITY) {
            return false;
        }

        // Must not be a untyped/nonfiltering warning (a case for the Comment UniqueType instead)
        if Self::has_common_substring_length(parameter_text, Self::UNTYPED_WARNING_PATTERN, Self::MINIMUM_SELECTIVITY) {
            return false;
        }

        // Check wildcard suppression - '*' on both ends, rest of pattern selective enough
        if parameter_text.starts_with('*') != parameter_text.ends_with('*') {
            return false;
        }

        if parameter_text.len() < Self::MINIMUM_SELECTIVITY + 2 {
            return false;
        }

        // More rules here???
        true
    }

    /// Check if an error matches a filter
    fn matches_filter(error: &RulesetError, filter: &str) -> bool {
        if error.text == filter {
            return true;
        }

        if !filter.ends_with('*') || !filter.starts_with('*') {
            return false;
        }

        let pattern = filter.trim_start_matches('*').trim_end_matches('*');
        error.text.to_lowercase().contains(&pattern.to_lowercase())
    }

    /// Determine if `error` matches any suppression Unique in `ModOptions` or the `source_object`, or any suppression modifier in `source_unique`
    pub fn is_error_suppressed(
        global_suppression_filters: &[String],
        source_object: Option<&dyn IHasUniques>,
        source_unique: Option<&Unique>,
        error: &RulesetError
    ) -> bool {
        if error.error_severity_to_report >= RulesetErrorSeverity::Error {
            return false;
        }

        if source_object.is_none() && global_suppression_filters.is_empty() {
            return false;
        }

        let get_wildcard_filter = |unique: &Unique| {
            let param = &unique.params[0];
            if param.starts_with('*') {
                param.clone()
            } else {
                format!("*{}*", param)
            }
        };

        // Allow suppressing from ModOptions
        let mut suppressions: Vec<String> = global_suppression_filters.to_vec();

        // Allow suppressing from suppression uniques in the same Unique collection
        if let Some(source_obj) = source_object {
            let matching_uniques = source_obj.get_matching_uniques(
                UniqueType::SuppressWarnings,
                &StateForConditionals::ignore_conditionals()
            );

            for unique in matching_uniques {
                suppressions.push(get_wildcard_filter(unique));
            }
        }

        // Allow suppressing from modifiers in the same Unique
        if let Some(unique) = source_unique {
            let modifiers = unique.get_modifiers(UniqueType::SuppressWarnings);

            for modifier in modifiers {
                suppressions.push(get_wildcard_filter(modifier));
            }
        }

        for filter in suppressions {
            if Self::matches_filter(error, &filter) {
                return true;
            }
        }

        false
    }

    /// Automatically suppress all warnings in a ruleset
    ///
    /// # Safety
    ///
    /// This method should not be made available to mod authors as it could be used to suppress important warnings.
    #[allow(dead_code)]
    pub fn auto_suppress_all_warnings(ruleset: &Ruleset, to_mod_options: &mut ModOptions) -> Result<(), String> {
        if ruleset.folder_location.is_none() {
            return Err("auto_suppress_all_warnings needs Ruleset.folder_location".to_string());
        }

        let folder_location = ruleset.folder_location.as_ref().unwrap();

        for error in RulesetValidator::new(ruleset).get_error_list(false) {
            if error.error_severity_to_report >= RulesetErrorSeverity::Error {
                continue;
            }

            to_mod_options.uniques.push(UniqueType::SuppressWarnings.text_with_params(&[&error.text]));
        }

        let json_serializer = JsonSerializer::new();
        let json = json_serializer.to_json(to_mod_options)?;

        let mod_options_path = folder_location.child("jsons/ModOptions.json");
        std::fs::write(mod_options_path.path(), json)?;

        Ok(())
    }

    /// Check if two strings have a common substring of at least the specified length
    fn has_common_substring_length(x: &str, y: &str, min_common_length: usize) -> bool {
        // This is brute-force, but adapting a public "longest-common-substring" algorithm was complex and still slooow.
        // Using the knowledge that we're only interested in the common length exceeding a threshold saves time.
        // This uses the fact that Int.until will _not_ throw on upper < lower.
        for x_index in 0..x.len().saturating_sub(min_common_length) {
            let x_sub = &x[x_index..];
            for y_index in 0..y.len().saturating_sub(min_common_length) {
                let y_sub = &y[y_index..];
                let common_prefix_len = x_sub.chars()
                    .zip(y_sub.chars())
                    .take_while(|(a, b)| a.to_lowercase().eq(b.to_lowercase()))
                    .count();

                if common_prefix_len >= min_common_length {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_filter() {
        // Valid filters
        assert!(Suppression::is_valid_filter("*This is a valid filter*"));
        assert!(Suppression::is_valid_filter("This is a valid filter with more than 12 chars"));

        // Invalid filters
        assert!(!Suppression::is_valid_filter("Too short")); // Too short
        assert!(!Suppression::is_valid_filter("*Missing end")); // Missing end asterisk
        assert!(!Suppression::is_valid_filter("Missing start*")); // Missing start asterisk
        assert!(!Suppression::is_valid_filter("*Contains {invalid} chars*")); // Contains invalid chars
        assert!(!Suppression::is_valid_filter("*Contains <invalid> chars*")); // Contains invalid chars
    }

    #[test]
    fn test_matches_filter() {
        let error = RulesetError {
            text: "Building 'Barracks' requires nonexistent tech 'Military Science'".to_string(),
            error_severity_to_report: RulesetErrorSeverity::Warning,
            source_object: None,
            source_unique: None,
        };

        // Exact match
        assert!(Suppression::matches_filter(&error, &error.text));

        // Wildcard match
        assert!(Suppression::matches_filter(&error, "*requires nonexistent tech*"));
        assert!(Suppression::matches_filter(&error, "*Barracks*"));

        // Case insensitive
        assert!(Suppression::matches_filter(&error, "*REQUIRES*"));

        // No match
        assert!(!Suppression::matches_filter(&error, "*nonexistent building*"));
        assert!(!Suppression::matches_filter(&error, "Missing asterisks"));
    }

    #[test]
    fn test_has_common_substring_length() {
        // Has common substring of sufficient length
        assert!(Suppression::has_common_substring_length(
            "unique \"~\" is deprecated as of ~, replace with",
            "This is a deprecation warning",
            12
        ));

        // No common substring of sufficient length
        assert!(!Suppression::has_common_substring_length(
            "This is a completely different string",
            "This is another completely different string",
            20
        ));
    }
}