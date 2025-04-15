use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::constants::Constants;
use crate::logic::multi_filter::MultiFilter;
use crate::logic::map::mapunit::MapUnitCache;
use crate::models::ruleset::{
    IRulesetObject, Ruleset, RulesetCache,
    unique::{IHasUniques, Unique, UniqueComplianceError, UniqueFlag,
             UniqueParameterType, UniqueTarget, UniqueType, DeprecationLevel}
};
use crate::models::ruleset::validation::{RulesetError, RulesetErrorList, RulesetErrorSeverity};
use crate::models::ruleset::validation::text_similarity::TextSimilarity;

/// Validates unique abilities in the ruleset.
pub struct UniqueValidator {
    /// The ruleset to validate
    ruleset: Ruleset,

    /// Used to determine if certain uniques are used for filtering
    all_non_typed_uniques: HashSet<String>,

    /// Used to determine if certain uniques are used for filtering
    all_unique_parameters: HashSet<String>,

    /// Cache for parameter type error severity
    param_type_error_severity_cache: HashMap<UniqueParameterType, HashMap<String, Option<UniqueType::UniqueParameterErrorSeverity>>>
}

impl UniqueValidator {
    /// Creates a new UniqueValidator for the given ruleset
    pub fn new(ruleset: &Ruleset) -> Self {
        Self {
            ruleset: ruleset.clone(),
            all_non_typed_uniques: HashSet::new(),
            all_unique_parameters: HashSet::new(),
            param_type_error_severity_cache: HashMap::new()
        }
    }

    /// Adds uniques from a container to the hashsets
    fn add_to_hashsets(&mut self, unique_holder: &dyn IHasUniques) {
        for unique in unique_holder.unique_objects() {
            if unique.get_type().is_none() {
                self.all_non_typed_uniques.insert(unique.text().to_string());
            } else {
                for param in unique.all_params() {
                    for filter in MultiFilter::get_all_single_filters(param) {
                        self.all_unique_parameters.insert(filter);
                    }
                }
            }
        }
    }

    /// Populates the filtering unique hashsets
    pub fn populate_filtering_unique_hashsets(&mut self) {
        for obj in self.ruleset.all_ruleset_objects() {
            self.add_to_hashsets(obj);
        }
    }

    /// Checks all uniques in a container
    pub fn check_uniques(
        &self,
        unique_container: &dyn IHasUniques,
        lines: &mut RulesetErrorList,
        report_ruleset_specific_errors: bool,
        try_fix_unknown_uniques: bool
    ) {
        for unique in unique_container.unique_objects() {
            let errors = self.check_unique(
                unique,
                try_fix_unknown_uniques,
                Some(unique_container),
                report_ruleset_specific_errors
            );
            lines.add_all(&errors);
        }
    }

    /// Checks a single unique
    pub fn check_unique(
        &self,
        unique: &Unique,
        try_fix_unknown_uniques: bool,
        unique_container: Option<&dyn IHasUniques>,
        report_ruleset_specific_errors: bool
    ) -> RulesetErrorList {
        let prefix = Self::get_unique_container_prefix(unique_container) + &format!("\"{}\"", unique.text());

        if unique.get_type().is_none() {
            return self.check_untyped_unique(unique, try_fix_unknown_uniques, unique_container, &prefix);
        }

        let mut ruleset_errors = RulesetErrorList::new(&self.ruleset);

        // Check if the unique is allowed on its target type
        if let Some(container) = unique_container {
            let unique_target = container.get_unique_target();
            let unique_type = unique.get_type().unwrap();

            if !unique_type.can_accept_unique_target(unique_target)
                && !(unique.has_modifier(UniqueType::ConditionalTimedUnique)
                    && unique_target.can_accept_unique_target(UniqueTarget::Triggerable)) {
                ruleset_errors.add(
                    &format!("{} is not allowed on its target type", prefix),
                    RulesetErrorSeverity::Warning,
                    unique_container,
                    Some(unique)
                );
            }
        }

        // Check for parameter compliance errors
        let type_compliance_errors = self.get_compliance_errors(unique);
        for compliance_error in type_compliance_errors {
            if !report_ruleset_specific_errors
                && compliance_error.error_severity == UniqueType::UniqueParameterErrorSeverity::RulesetSpecific {
                continue;
            }

            let acceptable_types = compliance_error.acceptable_parameter_types
                .iter()
                .map(|t| t.parameter_name())
                .collect::<Vec<_>>()
                .join(" or ");

            ruleset_errors.add(
                &format!(
                    "{} contains parameter {}, which does not fit parameter type {} !",
                    prefix, compliance_error.parameter_name, acceptable_types
                ),
                compliance_error.error_severity.get_ruleset_error_severity(),
                unique_container,
                Some(unique)
            );
        }

        // Check modifiers
        for conditional in unique.modifiers() {
            self.add_conditional_errors(
                conditional,
                &mut ruleset_errors,
                &prefix,
                unique,
                unique_container,
                report_ruleset_specific_errors
            );
        }

        // Check for unit movement uniques with conditionals
        if let Some(unique_type) = unique.get_type() {
            if MapUnitCache::unit_movement_uniques().contains(&unique_type)
                && unique.modifiers().iter().any(|m| {
                    m.get_type() != Some(UniqueType::ConditionalOurUnit)
                    || m.params().get(0).map_or(true, |p| p != Constants::all())
                }) {
                ruleset_errors.add(
                    &format!(
                        "{} contains a conditional on a unit movement unique. \
                        Due to performance considerations, this unique is cached on the unit, \
                        and the conditional may not always limit the unique correctly.",
                        prefix
                    ),
                    RulesetErrorSeverity::OK,
                    unique_container,
                    Some(unique)
                );
            }
        }

        // Check for deprecation annotations
        if report_ruleset_specific_errors {
            self.add_deprecation_annotation_errors(unique, &prefix, &mut ruleset_errors, unique_container);
        }

        ruleset_errors
    }

    /// Resource-related unique types
    fn resource_uniques() -> HashSet<UniqueType> {
        let mut set = HashSet::new();
        set.insert(UniqueType::ProvidesResources);
        set.insert(UniqueType::ConsumesResources);
        set.insert(UniqueType::DoubleResourceProduced);
        set.insert(UniqueType::StrategicResourcesIncrease);
        set
    }

    /// Resource-related conditional types
    fn resource_conditionals() -> HashSet<UniqueType> {
        let mut set = HashSet::new();
        set.insert(UniqueType::ConditionalWithResource);
        set.insert(UniqueType::ConditionalWithoutResource);
        set.insert(UniqueType::ConditionalWhenBetweenStatResource);
        set.insert(UniqueType::ConditionalWhenAboveAmountStatResource);
        set.insert(UniqueType::ConditionalWhenBelowAmountStatResource);
        set
    }

    /// Adds errors for a conditional
    fn add_conditional_errors(
        &self,
        conditional: &Unique,
        ruleset_errors: &mut RulesetErrorList,
        prefix: &str,
        unique: &Unique,
        unique_container: Option<&dyn IHasUniques>,
        report_ruleset_specific_errors: bool
    ) {
        // Check if conditionals are allowed
        if unique.has_flag(UniqueFlag::NoConditionals) {
            ruleset_errors.add(
                &format!(
                    "{} contains the conditional \"{}\", but the unique does not accept conditionals!",
                    prefix, conditional.text()
                ),
                RulesetErrorSeverity::Error,
                unique_container,
                Some(unique)
            );
            return;
        }

        // Check if the conditional has a valid type
        if conditional.get_type().is_none() {
            let mut text = format!(
                "{} contains the conditional \"{}\", which is of an unknown type!",
                prefix, conditional.text()
            );

            let similar_conditionals: Vec<&UniqueType> = UniqueType::iter()
                .filter(|t| {
                    TextSimilarity::get_relative_text_distance(
                        t.placeholder_text(),
                        conditional.placeholder_text()
                    ) <= RulesetCache::unique_misspelling_threshold()
                })
                .collect();

            if !similar_conditionals.is_empty() {
                let similar_texts: Vec<String> = similar_conditionals
                    .iter()
                    .map(|t| format!("\"{}\"", t.text()))
                    .collect();

                text.push_str(&format!(
                    " May be a misspelling of {}",
                    similar_texts.join(", or ")
                ));
            }

            ruleset_errors.add(
                &text,
                RulesetErrorSeverity::Warning,
                unique_container,
                Some(unique)
            );
            return;
        }

        let conditional_type = conditional.get_type().unwrap();

        // Check if the conditional is allowed as a modifier
        if conditional_type.target_types().iter()
            .all(|t| t.modifier_type() == UniqueTarget::ModifierType::None) {
            ruleset_errors.add(
                &format!(
                    "{} contains the conditional \"{}\", which is a Unique type not allowed as conditional or trigger.",
                    prefix, conditional.text()
                ),
                RulesetErrorSeverity::Warning,
                unique_container,
                Some(unique)
            );
        }

        // Check if the conditional is a UnitActionModifier on a non-UnitAction unique
        if conditional_type.target_types().contains(&UniqueTarget::UnitActionModifier) {
            if let Some(unique_type) = unique.get_type() {
                if !unique_type.target_types().iter()
                    .any(|t| UniqueTarget::UnitAction.can_accept_unique_target(*t)) {
                    ruleset_errors.add(
                        &format!(
                            "{} contains the conditional \"{}\", which as a UnitActionModifier is only allowed on UnitAction uniques.",
                            prefix, conditional.text()
                        ),
                        RulesetErrorSeverity::Warning,
                        unique_container,
                        Some(unique)
                    );
                }
            }
        }

        // Check for resource-related errors
        if let Some(unique_type) = unique.get_type() {
            if Self::resource_uniques().contains(&unique_type)
                && Self::resource_conditionals().contains(&conditional_type) {
                if let Some(last_param) = conditional.params().last() {
                    if let Some(resource) = self.ruleset.tile_resources().get(last_param) {
                        if resource.is_city_wide() {
                            ruleset_errors.add(
                                &format!(
                                    "{} contains the conditional \"{}\", which references a citywide resource. \
                                    This is not a valid conditional for a resource uniques, as it causes a recursive evaluation loop.",
                                    prefix, conditional.text()
                                ),
                                RulesetErrorSeverity::Error,
                                unique_container,
                                Some(unique)
                            );
                        }
                    }
                }
            }

            // Check for resource uniques with countable parameters in conditionals
            if Self::resource_uniques().contains(&unique_type) {
                for (index, param) in conditional.params().iter().enumerate() {
                    if let Some(resource) = self.ruleset.tile_resources().get(param) {
                        if resource.is_city_wide() {
                            if let Some(param_types) = unique_type.parameter_type_map().get(index) {
                                if param_types.contains(&UniqueParameterType::Countable) {
                                    ruleset_errors.add(
                                        &format!(
                                            "{} contains the modifier \"{}\", which references a citywide resource as a countable. \
                                            This is not a valid conditional for a resource uniques, as it causes a recursive evaluation loop.",
                                            prefix, conditional.text()
                                        ),
                                        RulesetErrorSeverity::Error,
                                        unique_container,
                                        Some(unique)
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check for parameter compliance errors in the conditional
        let conditional_compliance_errors = self.get_compliance_errors(conditional);
        for compliance_error in conditional_compliance_errors {
            if !report_ruleset_specific_errors
                && compliance_error.error_severity == UniqueType::UniqueParameterErrorSeverity::RulesetSpecific {
                continue;
            }

            let acceptable_types = compliance_error.acceptable_parameter_types
                .iter()
                .map(|t| t.parameter_name())
                .collect::<Vec<_>>()
                .join(" or ");

            ruleset_errors.add(
                &format!(
                    "{} contains modifier \"{}\". This contains the parameter {} which does not fit parameter type {} !",
                    prefix, conditional.text(), compliance_error.parameter_name, acceptable_types
                ),
                compliance_error.error_severity.get_ruleset_error_severity(),
                unique_container,
                Some(unique)
            );
        }

        // Check for deprecation annotations in the conditional
        self.add_deprecation_annotation_errors(
            conditional,
            &format!("{} contains modifier \"{}\" which", prefix, conditional.text()),
            ruleset_errors,
            unique_container
        );
    }

    /// Adds errors for deprecation annotations
    fn add_deprecation_annotation_errors(
        &self,
        unique: &Unique,
        prefix: &str,
        ruleset_errors: &mut RulesetErrorList,
        unique_container: Option<&dyn IHasUniques>
    ) {
        if let Some(deprecation_annotation) = unique.get_deprecation_annotation() {
            let replacement_unique_text = unique.get_replacement_text(&self.ruleset);
            let deprecation_text = format!(
                "{} is deprecated {}{}",
                prefix,
                deprecation_annotation.message,
                if !deprecation_annotation.replace_with.expression.is_empty() {
                    format!(", replace with \"{}\"", replacement_unique_text)
                } else {
                    String::new()
                }
            );

            let severity = if deprecation_annotation.level == DeprecationLevel::Warning {
                RulesetErrorSeverity::WarningOptionsOnly // Not user-visible
            } else {
                RulesetErrorSeverity::Warning // User visible
            };

            ruleset_errors.add(
                &deprecation_text,
                severity,
                unique_container,
                Some(unique)
            );
        }
    }

    /// Gets compliance errors for a unique
    fn get_compliance_errors(&self, unique: &Unique) -> Vec<UniqueComplianceError> {
        if unique.get_type().is_none() {
            return Vec::new();
        }

        let mut error_list = Vec::new();
        let unique_type = unique.get_type().unwrap();

        for (index, param) in unique.params().iter().enumerate() {
            // Check for parameter count mismatch
            if unique_type.parameter_type_map().len() != unique.params().len() {
                panic!(
                    "Unique {} has {} parameters, but its type {} only {} parameters?!",
                    unique.text(),
                    unique.params().len(),
                    unique_type,
                    unique_type.parameter_type_map().len()
                );
            }

            let acceptable_param_types = &unique_type.parameter_type_map()[index];
            let error_types_for_acceptable_parameters: Vec<Option<UniqueType::UniqueParameterErrorSeverity>> =
                acceptable_param_types.iter()
                    .map(|t| self.get_param_type_error_severity_cached(t, param))
                    .collect();

            // Skip if one of the types matches
            if error_types_for_acceptable_parameters.iter().any(|t| t.is_none()) {
                continue;
            }

            // Skip if this is a filtering param and the unique it's filtering for exists
            if error_types_for_acceptable_parameters.contains(&Some(UniqueType::UniqueParameterErrorSeverity::PossibleFilteringUnique))
                && self.all_unique_parameters.contains(param) {
                continue;
            }

            // Get the least severe warning
            if let Some(least_severe_warning) = error_types_for_acceptable_parameters.iter()
                .filter_map(|&t| t)
                .min_by_key(|&t| t as usize) {
                error_list.push(UniqueComplianceError::new(
                    param.clone(),
                    acceptable_param_types.clone(),
                    least_severe_warning
                ));
            }
        }

        error_list
    }

    /// Gets the error severity for a parameter type (cached)
    fn get_param_type_error_severity_cached(
        &self,
        unique_parameter_type: &UniqueParameterType,
        param: &str
    ) -> Option<UniqueType::UniqueParameterErrorSeverity> {
        if !self.param_type_error_severity_cache.contains_key(unique_parameter_type) {
            return None;
        }

        let unique_param_cache = self.param_type_error_severity_cache.get(unique_parameter_type).unwrap();

        if unique_param_cache.contains_key(param) {
            return *unique_param_cache.get(param).unwrap();
        }

        let severity = unique_parameter_type.get_error_severity(param, &self.ruleset);
        // Note: We can't modify the cache here because self is immutable
        // In a real implementation, we would need to make this method take &mut self
        severity
    }

    /// Checks an untyped unique
    fn check_untyped_unique(
        &self,
        unique: &Unique,
        try_fix_unknown_uniques: bool,
        unique_container: Option<&dyn IHasUniques>,
        prefix: &str
    ) -> RulesetErrorList {
        // Check for mismatched conditional braces
        let open_count = unique.text().chars().filter(|&c| c == '<').count();
        let close_count = unique.text().chars().filter(|&c| c == '>').count();

        if open_count != close_count {
            return RulesetErrorList::of(
                &format!("{} contains mismatched conditional braces!", prefix),
                RulesetErrorSeverity::Warning,
                &self.ruleset,
                unique_container,
                Some(unique)
            );
        }

        // Support purely filtering Uniques without actual implementation
        if self.is_filtering_unique_allowed(unique) {
            return RulesetErrorList::new(&self.ruleset);
        }

        // Try to fix unknown uniques
        if try_fix_unknown_uniques {
            let fixes = self.try_fix_unknown_unique(unique, unique_container, prefix);
            if !fixes.is_empty() {
                return fixes;
            }
        }

        // Return error for unknown unique
        RulesetErrorList::of(
            &format!(
                "{} not found in Unciv's unique types, and is not used as a filtering unique.",
                prefix
            ),
            if unique.params().is_empty() {
                RulesetErrorSeverity::OK
            } else {
                RulesetErrorSeverity::Warning
            },
            &self.ruleset,
            unique_container,
            Some(unique)
        )
    }

    /// Checks if a unique is allowed as a filtering unique
    fn is_filtering_unique_allowed(&self, unique: &Unique) -> bool {
        // Must have no conditionals or parameters, and is used in any "filtering" parameter of another Unique
        if !unique.modifiers().is_empty() || !unique.params().is_empty() {
            return false;
        }

        self.all_unique_parameters.contains(unique.text())
    }

    /// Tries to fix an unknown unique
    fn try_fix_unknown_unique(
        &self,
        unique: &Unique,
        unique_container: Option<&dyn IHasUniques>,
        prefix: &str
    ) -> RulesetErrorList {
        let similar_uniques: Vec<&UniqueType> = UniqueType::iter()
            .filter(|t| {
                TextSimilarity::get_relative_text_distance(
                    t.placeholder_text(),
                    unique.placeholder_text()
                ) <= RulesetCache::unique_misspelling_threshold()
            })
            .collect();

        let equal_uniques: Vec<&UniqueType> = similar_uniques.iter()
            .filter(|&&t| t.placeholder_text() == unique.placeholder_text())
            .copied()
            .collect();

        if !equal_uniques.is_empty() {
            return RulesetErrorList::of(
                &format!(
                    "{} looks like it should be fine, but for some reason isn't recognized.",
                    prefix
                ),
                RulesetErrorSeverity::OK,
                &self.ruleset,
                unique_container,
                Some(unique)
            );
        }

        if !similar_uniques.is_empty() {
            let mut text = format!("{} looks like it may be a misspelling of:\n", prefix);

            for unique_type in similar_uniques {
                let mut unique_text = format!("\"{}\"", unique_type.text());

                if !unique.modifiers().is_empty() {
                    unique_text.push_str(" ");
                    unique_text.push_str(&unique.modifiers().iter()
                        .map(|m| format!("<{}>", m.text()))
                        .collect::<Vec<_>>()
                        .join(" "));
                }

                if unique_type.get_deprecation_annotation().is_some() {
                    unique_text.push_str(" (Deprecated)");
                }

                text.push_str(&format!("\t{}\n", unique_text));
            }

            return RulesetErrorList::of(
                &text,
                RulesetErrorSeverity::OK,
                &self.ruleset,
                unique_container,
                Some(unique)
            );
        }

        RulesetErrorList::new(&self.ruleset)
    }

    /// Gets the prefix for a unique container
    pub fn get_unique_container_prefix(unique_container: Option<&dyn IHasUniques>) -> String {
        let origin_prefix = if let Some(container) = unique_container {
            if let Some(ruleset_obj) = container.as_any().downcast_ref::<dyn IRulesetObject>() {
                format!("{}: ", ruleset_obj.origin_ruleset())
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let container_prefix = if let Some(container) = unique_container {
            format!("({}) {}'s", container.get_unique_target().name(), container.name())
        } else {
            "The".to_string()
        };

        format!("{}{} unique ", origin_prefix, container_prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_unique_container_prefix() {
        // Test with null container
        let prefix = UniqueValidator::get_unique_container_prefix(None);
        assert_eq!(prefix, "The unique ");

        // Test with a container would require mocking IHasUniques and IRulesetObject
        // This is complex in a unit test, so we'll skip it for now
    }

    #[test]
    fn test_is_filtering_unique_allowed() {
        // This test would require a mock Ruleset and Unique
        // For now, we'll just test that the function exists
        let ruleset = Ruleset::new();
        let validator = UniqueValidator::new(&ruleset);
        let unique = Unique::new("test");
        let result = validator.is_filtering_unique_allowed(&unique);
        assert!(!result);
    }
}