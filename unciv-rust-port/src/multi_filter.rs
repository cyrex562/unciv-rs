/// A module that implements `and` and `not` logic on top of a base filter function.
///
/// Syntax:
///     - `and`: `{filter1} {filter2}`... (can repeat as needed)
///     - `not`: `non-[filter]`
pub struct MultiFilter;

impl MultiFilter {
    const AND_PREFIX: &'static str = "{";
    const AND_SEPARATOR: &'static str = "} {";
    const AND_SUFFIX: &'static str = "}";
    const NOT_PREFIX: &'static str = "non-[";
    const NOT_SUFFIX: &'static str = "]";

    /// Implements `and` and `not` logic on top of a filter function.
    ///
    /// # Arguments
    ///
    /// * `input` - The complex filtering term
    /// * `filter_function` - The single filter implementation
    /// * `for_unique_validity_tests` - Inverts the `non-[filter]` test because Unique validity doesn't check for actual matching
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the input passes the filter criteria
    pub fn multi_filter<F>(input: &str, filter_function: F, for_unique_validity_tests: bool) -> bool
    where
        F: Fn(&str) -> bool,
    {
        if Self::has_surrounding(input, Self::AND_PREFIX, Self::AND_SUFFIX) && input.contains(Self::AND_SEPARATOR) {
            let inner = Self::remove_surrounding(input, Self::AND_PREFIX, Self::AND_SUFFIX);
            inner
                .split(Self::AND_SEPARATOR)
                .all(|part| Self::multi_filter(part, &filter_function, for_unique_validity_tests))
        } else if Self::has_surrounding(input, Self::NOT_PREFIX, Self::NOT_SUFFIX) {
            let inner = Self::remove_surrounding(input, Self::NOT_PREFIX, Self::NOT_SUFFIX);
            let internal_result = Self::multi_filter(inner, filter_function, for_unique_validity_tests);
            if for_unique_validity_tests {
                internal_result
            } else {
                !internal_result
            }
        } else {
            filter_function(input)
        }
    }

    /// Gets all single filters from a complex filter string.
    ///
    /// # Arguments
    ///
    /// * `input` - The complex filtering term
    ///
    /// # Returns
    ///
    /// An iterator over all single filters in the input
    pub fn get_all_single_filters(input: &str) -> impl Iterator<Item = String> {
        let mut result = Vec::new();
        Self::collect_single_filters(input, &mut result);
        result.into_iter()
    }

    /// Helper function to collect all single filters into a vector
    fn collect_single_filters(input: &str, result: &mut Vec<String>) {
        if Self::has_surrounding(input, Self::AND_PREFIX, Self::AND_SUFFIX) && input.contains(Self::AND_SEPARATOR) {
            // Resolve "AND" filters
            let inner = Self::remove_surrounding(input, Self::AND_PREFIX, Self::AND_SUFFIX);
            for part in inner.split(Self::AND_SEPARATOR) {
                Self::collect_single_filters(part, result);
            }
        } else if Self::has_surrounding(input, Self::NOT_PREFIX, Self::NOT_SUFFIX) {
            // Simply remove "non" syntax
            let inner = Self::remove_surrounding(input, Self::NOT_PREFIX, Self::NOT_SUFFIX);
            Self::collect_single_filters(inner, result);
        } else {
            result.push(input.to_string());
        }
    }

    /// Checks if a string has the given prefix and suffix
    fn has_surrounding(input: &str, prefix: &str, suffix: &str) -> bool {
        input.starts_with(prefix) && input.ends_with(suffix)
    }

    /// Removes the surrounding prefix and suffix from a string
    fn remove_surrounding(input: &str, prefix: &str, suffix: &str) -> &str {
        &input[prefix.len()..input.len() - suffix.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_filter() {
        let filter_fn = |s: &str| s == "test";
        assert!(MultiFilter::multi_filter("test", filter_fn, false));
        assert!(!MultiFilter::multi_filter("other", filter_fn, false));
    }

    #[test]
    fn test_and_filter() {
        let filter_fn = |s: &str| s == "test" || s == "other";
        assert!(MultiFilter::multi_filter("{test} {other}", filter_fn, false));
        assert!(!MultiFilter::multi_filter("{test} {invalid}", filter_fn, false));
    }

    #[test]
    fn test_not_filter() {
        let filter_fn = |s: &str| s == "test";
        assert!(!MultiFilter::multi_filter("non-[test]", filter_fn, false));
        assert!(MultiFilter::multi_filter("non-[other]", filter_fn, false));
    }

    #[test]
    fn test_not_filter_with_unique_validity() {
        let filter_fn = |s: &str| s == "test";
        assert!(MultiFilter::multi_filter("non-[test]", filter_fn, true));
        assert!(!MultiFilter::multi_filter("non-[other]", filter_fn, true));
    }

    #[test]
    fn test_complex_filter() {
        let filter_fn = |s: &str| s == "test" || s == "other";
        assert!(MultiFilter::multi_filter("{test} {non-[invalid]}", filter_fn, false));
        assert!(!MultiFilter::multi_filter("{test} {non-[test]}", filter_fn, false));
    }

    #[test]
    fn test_get_all_single_filters() {
        let input = "{test} {non-[other]}";
        let filters: Vec<String> = MultiFilter::get_all_single_filters(input).collect();
        assert_eq!(filters, vec!["test", "other"]);
    }
}