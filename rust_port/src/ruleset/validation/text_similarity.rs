use std::cmp::min;

/// Module for text similarity comparison algorithms.
pub struct TextSimilarity;

impl TextSimilarity {
    /// Calculates the approximate distance between two strings.
    ///
    /// Algorithm:
    ///  - Keep an index for each string.
    ///  - Iteratively advance by one character in each string.
    ///      - If the character at the index of each string is not the same, then pause.
    ///          - Try to find the minimum number of characters to skip in the first string to find the current character of the second string.
    ///          - Try to find the minimum number of characters to skip in the second string to find the current character of the first string.
    ///          - If the above condition cannot be satisfied for either string, then skip both by one character and continue advancing them together.
    ///          - Otherwise, skip ahead in either the first string or the second string, depending on which requires the lowest offset, and continue advancing both strings together.
    ///      - Stop when either one of the above steps cannot be completed or the end of either string has been reached.
    ///  - The distance returned is the approximately total number of characters skipped, plus the total number of characters unaccounted for at the end.
    ///
    /// Meant to run in linear-ish time.
    /// Order of comparands shouldn't matter too much, but does a little.
    /// This seemed simpler than a thorough implementation of other string comparison algorithms, and maybe more performant than a naïve implementation of other string comparisons, as well as sufficient for the fairly simple use case.
    ///
    /// # Arguments
    ///
    /// * `text1` - First string to compare
    /// * `text2` - Second string to compare
    ///
    /// # Returns
    ///
    /// Approximate distance between the strings
    pub fn get_text_distance(text1: &str, text2: &str) -> i32 {
        let mut dist = 0;
        let mut i1 = 0;
        let mut i2 = 0;

        // Convert strings to character vectors for easier indexing
        let chars1: Vec<char> = text1.chars().collect();
        let chars2: Vec<char> = text2.chars().collect();

        // Debug function (commented out as in the original)
        // fn debug_traversal(chars: &[char], index: usize) {
        //     let mut result = String::new();
        //     for (i, &c) in chars.iter().enumerate() {
        //         if i == index {
        //             result.push('[');
        //             result.push(c);
        //             result.push(']');
        //         } else {
        //             result.push(c);
        //         }
        //     }
        //     println!("{}", result);
        // }

        // fn debug_traversal_both(chars1: &[char], i1: usize, chars2: &[char], i2: usize) {
        //     println!();
        //     debug_traversal(chars1, i1);
        //     debug_traversal(chars2, i2);
        // }

        while i1 < chars1.len() && i2 < chars2.len() {
            // debug_traversal_both(&chars1, i1, &chars2, i2);

            let char1 = chars1[i1];
            let char2 = chars2[i2];

            if char1 == char2 {
                i1 += 1;
                i2 += 1;
            } else if char1.to_lowercase().eq(char2.to_lowercase()) {
                dist += 1;
                i1 += 1;
                i2 += 1;
            } else {
                // Find the first match for char2 in the rest of text1
                let first_match_index1 = (i1..chars1.len()).find(|&i| chars1[i] == char2);

                // Find the first match for char1 in the rest of text2
                let first_match_index2 = (i2..chars2.len()).find(|&i| chars2[i] == char1);

                if first_match_index1.is_none() && first_match_index2.is_none() {
                    dist += 1;
                    i1 += 1;
                    i2 += 1;
                    continue;
                }

                let first_match_offset1 = first_match_index1.map(|i| i - i1);
                let first_match_offset2 = first_match_index2.map(|i| i - i2);

                match (first_match_offset1, first_match_offset2) {
                    (Some(offset1), Some(offset2)) if offset1 < offset2 => {
                        // Preferential behaviour when the offsets are equal does make the operation slightly non-commutative
                        dist += offset1 as i32;
                        i1 = first_match_index1.unwrap() + 1;
                        i2 += 1;
                    },
                    (Some(_), Some(_)) | (None, Some(offset2)) => {
                        dist += offset2 as i32;
                        i1 += 1;
                        i2 = first_match_index2.unwrap() + 1;
                    },
                    (Some(offset1), None) => {
                        dist += offset1 as i32;
                        i1 = first_match_index1.unwrap() + 1;
                        i2 += 1;
                    },
                    (None, None) => {
                        // This should never happen due to the check above
                        unreachable!("Can't compare Strings:\n\t{}\n\t{}", text1, text2);
                    }
                }
            }
        }

        // Add remaining characters
        dist += ((chars1.len() - i1) + (chars2.len() - i2)) as i32 / 2;
        dist
    }

    /// Returns the relative text distance between two strings.
    ///
    /// The original algorithm is very weak to short strings with errors at the start
    /// (can't figure out that "on [] tiles" and "in [] tiles" are the same).
    /// So we run it twice, once with the string reversed.
    ///
    /// # Arguments
    ///
    /// * `text1` - First string to compare
    /// * `text2` - Second string to compare
    ///
    /// # Returns
    ///
    /// The relative distance between the strings (0.0 to 1.0, where 0.0 means identical)
    pub fn get_relative_text_distance(text1: &str, text2: &str) -> f64 {
        let text_distance = |a: &str, b: &str| -> f64 {
            Self::get_text_distance(a, b) as f64 / (text1.len() + text2.len()) as f64 * 2.0
        };

        // Calculate distance both forward and backward
        let forward_distance = text_distance(text1, text2);
        let backward_distance = text_distance(&text1.chars().rev().collect::<String>(), &text2.chars().rev().collect::<String>());

        // Return the minimum of the two distances
        min(forward_distance, backward_distance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_text_distance() {
        // Identical strings
        assert_eq!(TextSimilarity::get_text_distance("hello", "hello"), 0);

        // Case difference
        assert_eq!(TextSimilarity::get_text_distance("Hello", "hello"), 1);

        // Different strings
        assert_eq!(TextSimilarity::get_text_distance("hello", "world"), 5);

        // Strings with common parts
        assert_eq!(TextSimilarity::get_text_distance("hello", "help"), 2);

        // Strings with different lengths
        assert_eq!(TextSimilarity::get_text_distance("hello", "hello world"), 6);

        // Empty strings
        assert_eq!(TextSimilarity::get_text_distance("", ""), 0);
        assert_eq!(TextSimilarity::get_text_distance("hello", ""), 5);
        assert_eq!(TextSimilarity::get_text_distance("", "hello"), 5);
    }

    #[test]
    fn test_get_relative_text_distance() {
        // Identical strings
        assert_eq!(TextSimilarity::get_relative_text_distance("hello", "hello"), 0.0);

        // Case difference
        assert!(TextSimilarity::get_relative_text_distance("Hello", "hello") < 0.2);

        // Different strings
        assert!(TextSimilarity::get_relative_text_distance("hello", "world") > 0.8);

        // Strings with common parts
        assert!(TextSimilarity::get_relative_text_distance("hello", "help") < 0.5);

        // Strings with different lengths
        assert!(TextSimilarity::get_relative_text_distance("hello", "hello world") < 0.6);

        // Empty strings
        assert_eq!(TextSimilarity::get_relative_text_distance("", ""), 0.0);
        assert_eq!(TextSimilarity::get_relative_text_distance("hello", ""), 1.0);
        assert_eq!(TextSimilarity::get_relative_text_distance("", "hello"), 1.0);

        // Test the specific case mentioned in the comments
        assert!(TextSimilarity::get_relative_text_distance("on [] tiles", "in [] tiles") < 0.3);
    }
}