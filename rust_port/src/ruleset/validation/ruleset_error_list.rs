use std::collections::HashSet;
use std::cmp::Ordering;
use crate::models::{
    ruleset::Ruleset,
    ruleset::unique::{IHasUniques, Unique, UniqueType, StateForConditionals},
};

/// Represents an error in a ruleset
#[derive(Debug, Clone, PartialEq)]
pub struct RulesetError {
    /// The error message
    pub text: String,
    /// The severity of the error
    pub error_severity_to_report: RulesetErrorSeverity,
}

/// Represents the severity of a ruleset error
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RulesetErrorSeverity {
    /// No errors
    OK,
    /// Warning that only affects options
    WarningOptionsOnly,
    /// General warning
    Warning,
    /// Error that prevents the ruleset from being used
    Error,
}

impl RulesetErrorSeverity {
    /// Gets the color associated with this severity
    pub fn color(&self) -> [f32; 4] {
        match self {
            RulesetErrorSeverity::OK => [0.0, 1.0, 0.0, 1.0], // Green
            RulesetErrorSeverity::WarningOptionsOnly => [1.0, 1.0, 0.0, 1.0], // Yellow
            RulesetErrorSeverity::Warning => [1.0, 1.0, 0.0, 1.0], // Yellow
            RulesetErrorSeverity::Error => [1.0, 0.0, 0.0, 1.0], // Red
        }
    }
}

/// A container collecting errors in a Ruleset
///
/// While this is based on a standard collection, please do not use the standard add or extend methods.
/// Mod-controlled warning suppression is handled in add methods that provide a source object, which can host suppression uniques.
/// Bypassing these add methods means suppression is ignored. Thus using extend is fine when the elements to add are all already checked.
///
/// # Arguments
///
/// * `ruleset` - The ruleset being validated (needed to check modOptions for suppression uniques). Leave `None` only for validation results that need no suppression checks.
pub struct RulesetErrorList {
    /// The list of errors
    errors: Vec<RulesetError>,
    /// Global suppression filters
    global_suppression_filters: HashSet<String>,
}

impl RulesetErrorList {
    /// Creates a new empty RulesetErrorList
    pub fn new(ruleset: Option<&Ruleset>) -> Self {
        let global_suppression_filters = ruleset
            .and_then(|r| r.mod_options.get_matching_uniques(UniqueType::SuppressWarnings, &StateForConditionals::ignore_conditionals()))
            .map(|uniques| uniques.iter().map(|u| u.params[0].clone()).collect())
            .unwrap_or_default();

        Self {
            errors: Vec::new(),
            global_suppression_filters,
        }
    }

    /// Adds an error to the list, preventing duplicates (in which case the highest severity wins).
    ///
    /// # Arguments
    ///
    /// * `element` - The error to add
    /// * `source_object` - The originating object, which can host suppressions. When it is not known or not a IHasUniques, pass `None`.
    /// * `source_unique` - The originating unique, so look for suppression modifiers. Leave `None` if unavailable.
    ///
    /// # Returns
    ///
    /// `true` if the error was added, `false` otherwise
    pub fn add(&mut self, element: RulesetError, source_object: Option<&dyn IHasUniques>, source_unique: Option<&Unique>) -> bool {
        // The dupe check may be faster than the Suppression check, so do it first
        if !self.remove_lower_severity_duplicate(&element) {
            return false;
        }

        if Suppression::is_error_suppressed(&self.global_suppression_filters, source_object, source_unique, &element) {
            return false;
        }

        self.errors.push(element);
        true
    }

    /// Shortcut: Add a new RulesetError built from text and error_severity_to_report.
    ///
    /// # Arguments
    ///
    /// * `text` - The error message
    /// * `error_severity_to_report` - The severity of the error
    /// * `source_object` - The originating object, which can host suppressions. When it is not known or not a IHasUniques, pass `None`.
    /// * `source_unique` - The originating unique, so look for suppression modifiers. Leave `None` if unavailable.
    ///
    /// # Returns
    ///
    /// `true` if the error was added, `false` otherwise
    pub fn add_text(&mut self, text: String, error_severity_to_report: RulesetErrorSeverity, source_object: Option<&dyn IHasUniques>, source_unique: Option<&Unique>) -> bool {
        self.add(RulesetError {
            text,
            error_severity_to_report,
        }, source_object, source_unique)
    }

    /// Adds all errors with duplicate check, but without suppression check
    pub fn extend(&mut self, elements: &[RulesetError]) -> bool {
        let mut result = false;
        for element in elements {
            if self.add_with_duplicate_check(element.clone()) {
                result = true;
            }
        }
        result
    }

    /// Adds an error with duplicate check, but without suppression check
    fn add_with_duplicate_check(&mut self, element: RulesetError) -> bool {
        if self.remove_lower_severity_duplicate(&element) {
            self.errors.push(element);
            true
        } else {
            false
        }
    }

    /// Returns `true` if the element is not present, or it was removed due to having a lower severity
    fn remove_lower_severity_duplicate(&mut self, element: &RulesetError) -> bool {
        // Suppress duplicates due to the double run of some checks for invariant/specific,
        // Without changing collection type or making RulesetError obey the equality contract
        if let Some(pos) = self.errors.iter().position(|e| e.text == element.text) {
            let existing = &self.errors[pos];
            if existing.error_severity_to_report >= element.error_severity_to_report {
                return false;
            }
            self.errors.remove(pos);
        }
        true
    }

    /// Gets the final severity of the errors
    pub fn get_final_severity(&self) -> RulesetErrorSeverity {
        if self.errors.is_empty() {
            return RulesetErrorSeverity::OK;
        }
        self.errors.iter().map(|e| e.error_severity_to_report).max().unwrap()
    }

    /// Returns `true` if there are severe errors that make the mod unplayable
    pub fn is_error(&self) -> bool {
        self.get_final_severity() == RulesetErrorSeverity::Error
    }

    /// Returns `true` if there are problems, Options screen mod checker or unit tests for vanilla ruleset should complain
    pub fn is_not_ok(&self) -> bool {
        self.get_final_severity() != RulesetErrorSeverity::OK
    }

    /// Returns `true` if there are at least errors impacting gameplay, new game screen should warn or block
    pub fn is_warn_user(&self) -> bool {
        self.get_final_severity() >= RulesetErrorSeverity::Warning
    }

    /// Gets the error text
    ///
    /// # Arguments
    ///
    /// * `unfiltered` - If `true`, include all errors, otherwise filter out WarningOptionsOnly
    pub fn get_error_text(&self, unfiltered: bool) -> String {
        self.get_error_text_filtered(|e| unfiltered || e.error_severity_to_report > RulesetErrorSeverity::WarningOptionsOnly)
    }

    /// Gets the error text filtered by a predicate
    ///
    /// # Arguments
    ///
    /// * `filter` - A predicate that determines which errors to include
    pub fn get_error_text_filtered<F>(&self, filter: F) -> String
    where
        F: Fn(&RulesetError) -> bool,
    {
        let mut filtered: Vec<&RulesetError> = self.errors.iter().filter(|e| filter(e)).collect();
        filtered.sort_by(|a, b| b.error_severity_to_report.cmp(&a.error_severity_to_report));

        filtered.iter().map(|e| {
            format!("{}: {}",
                match e.error_severity_to_report {
                    RulesetErrorSeverity::OK => "OK",
                    RulesetErrorSeverity::WarningOptionsOnly => "WarningOptionsOnly",
                    RulesetErrorSeverity::Warning => "Warning",
                    RulesetErrorSeverity::Error => "Error",
                },
                // This will go through tr(), unavoidably, which will move the conditionals
                // out of place. Prevent via kludge:
                e.text.replace('<', "〈").replace('>', "〉")
            )
        }).collect::<Vec<String>>().join("\n")
    }

    /// Helper factory for a single entry list (which can result in an empty list due to suppression)
    ///
    /// Note: Valid source for extend since suppression is already taken care of.
    ///
    /// # Arguments
    ///
    /// * `text` - The error message
    /// * `severity` - The severity of the error
    /// * `ruleset` - The ruleset being validated
    /// * `source_object` - The originating object, which can host suppressions
    /// * `source_unique` - The originating unique, so look for suppression modifiers
    pub fn of(
        text: String,
        severity: RulesetErrorSeverity,
        ruleset: Option<&Ruleset>,
        source_object: Option<&dyn IHasUniques>,
        source_unique: Option<&Unique>
    ) -> Self {
        let mut result = Self::new(ruleset);
        result.add_text(text, severity, source_object, source_unique);
        result
    }
}

impl Default for RulesetErrorList {
    fn default() -> Self {
        Self::new(None)
    }
}

/// Helper for checking if an error is suppressed
struct Suppression;

impl Suppression {
    /// Checks if an error is suppressed
    fn is_error_suppressed(
        global_suppression_filters: &HashSet<String>,
        source_object: Option<&dyn IHasUniques>,
        source_unique: Option<&Unique>,
        error: &RulesetError
    ) -> bool {
        // Check global suppression filters
        if global_suppression_filters.iter().any(|filter| error.text.contains(filter)) {
            return true;
        }

        // Check source object suppression
        if let Some(source_object) = source_object {
            if let Some(uniques) = source_object.get_matching_uniques(UniqueType::SuppressWarnings, &StateForConditionals::ignore_conditionals()) {
                if uniques.iter().any(|unique| error.text.contains(&unique.params[0])) {
                    return true;
                }
            }
        }

        // Check source unique suppression
        if let Some(source_unique) = source_unique {
            if source_unique.name == UniqueType::SuppressWarnings.to_string() && error.text.contains(&source_unique.params[0]) {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ruleset_error_list() {
        let mut list = RulesetErrorList::new(None);

        // Test adding errors
        assert!(list.add_text("Error 1".to_string(), RulesetErrorSeverity::Error, None, None));
        assert!(list.add_text("Warning 1".to_string(), RulesetErrorSeverity::Warning, None, None));

        // Test duplicate handling
        assert!(!list.add_text("Error 1".to_string(), RulesetErrorSeverity::Warning, None, None));
        assert!(list.add_text("Error 1".to_string(), RulesetErrorSeverity::Error, None, None));

        // Test severity
        assert_eq!(list.get_final_severity(), RulesetErrorSeverity::Error);
        assert!(list.is_error());
        assert!(list.is_not_ok());
        assert!(list.is_warn_user());

        // Test error text
        let error_text = list.get_error_text(false);
        assert!(error_text.contains("Error 1"));
        assert!(error_text.contains("Warning 1"));
    }
}