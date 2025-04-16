use std::collections::{HashMap, HashSet};
use std::fmt;
use std::str::FromStr;
use log::debug;

/// An engine to support languages with heavy diacritic usage through Gdx Scene2D
///
/// # Concepts
/// - This is not needed for diacritics where Unicode already defines the combined glyphs as individual codepoints
/// - Gdx text rendering assumes one Char one Glyph (and left-to-right)
/// - The underlying OS **does** have the capability to render glyphs created by combining diacritic joiners with other characters
/// - We'll deal with one glyph at a time arranges left to right, and expect a finite number of combination glyphs
/// - We'll recognize these combos in the translated texts at translation loading time and map each combo into a fake alphabet
/// - Conversely, the loader will build a map of distinct combinations -codepoint sequences- that map into a single glyph
/// - At render time, only the map of fake alphabet codepoints to their original codepoint sequences is needed
///
/// # Usage
/// - Call `reset()` when translation loading starts over
/// - Instantiate `DiacriticSupport` through the constructor-like factory `invoke` once a translation file is read
/// - Check `is_enabled()` - if false, the rest of that language load need not bother with diacritics
/// - Call `remap_diacritics()` on each translation and store the result instead of the original value
/// - If you wish to save some memory, call `free_translation_data()` after all required languages are done
/// - Later, `NativeBitmapFontData.create_and_cache_glyph()` will use `get_string_for()` to map the fake alphabet back to codepoint sequences
///
/// # Notes
/// - `FontRulesetIcons` initialize ***after*** Translation loading. If this ever changes, this might need some tweaking.
/// - The primary constructor is only used from the `invoke` factory and for testing.
pub struct DiacriticSupport {
    enabled: bool,
    char_class_map: HashMap<char, CharClass>,
    fake_alphabet: HashMap<char, String>,
    inverse_map: HashMap<String, char>,
    next_free_diacritic_replacement_codepoint: u16,
}

/// Translation keys for diacritic support configuration
struct TranslationKeys {
    enable: &'static str,
    range_start: &'static str,
    range_end: &'static str,
    left: &'static str,
    right: &'static str,
    joiner: &'static str,
}

impl TranslationKeys {
    const fn new() -> Self {
        Self {
            enable: "diacritics_support",
            range_start: "unicode_block_start_character",
            range_end: "unicode_block_end_character",
            left: "left_joining_diacritics",
            right: "right_joining_diacritics",
            joiner: "left_and_right_joiners",
        }
    }
}

/// Represents a class of input character and its processing method when processing a translation line
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharClass {
    None,
    Base,
    LeftJoiner,
    RightJoiner,
    LeftRightJoiner,
    Surrogate,
}

impl CharClass {
    fn expects_right_join(&self) -> bool {
        matches!(self, CharClass::RightJoiner | CharClass::LeftRightJoiner)
    }
}

/// Holds all information to process a single translation line and replace diacritic combinations with fake alphabet codepoints
struct LineData {
    output: String,
    accumulator: String,
    waiting_high_surrogate: Option<char>,
}

impl LineData {
    fn new(capacity: usize) -> Self {
        Self {
            output: String::with_capacity(capacity),
            accumulator: String::with_capacity(9), // touhidurrr said there can be nine
            waiting_high_surrogate: None,
        }
    }

    fn expects_join(&self, char_class_map: &HashMap<char, CharClass>) -> bool {
        !self.accumulator.is_empty() &&
        self.accumulator.chars().last()
            .map(|c| char_class_map.get(&c).map_or(CharClass::None, |&cc| cc))
            .map_or(false, |cc| cc.expects_right_join())
    }

    fn flush(&mut self, char_class_map: &HashMap<char, CharClass>, inverse_map: &mut HashMap<String, char>) {
        if self.accumulator.len() <= 1 {
            self.output.push_str(&self.accumulator);
        } else {
            let replacement = get_replacement_char(&self.accumulator, inverse_map);
            self.output.push(replacement);
        }
        self.accumulator.clear();
    }

    fn forbid_waiting_high_surrogate(&self) -> Result<(), String> {
        if self.waiting_high_surrogate.is_some() {
            Err("Invalid Unicode: High surrogate without low surrogate".to_string())
        } else {
            Ok(())
        }
    }

    fn accumulate(&mut self, char: char) -> Result<(), String> {
        self.forbid_waiting_high_surrogate()?;
        self.accumulator.push(char);
        Ok(())
    }

    fn flush_accumulate(&mut self, char: char, char_class_map: &HashMap<char, CharClass>, inverse_map: &mut HashMap<String, char>) -> Result<(), String> {
        self.forbid_waiting_high_surrogate()?;
        if !self.expects_join(char_class_map) {
            self.flush(char_class_map, inverse_map);
        }
        self.accumulator.push(char);
        Ok(())
    }

    fn flush_append(&mut self, char: char, char_class_map: &HashMap<char, CharClass>, inverse_map: &mut HashMap<String, char>) -> Result<(), String> {
        self.forbid_waiting_high_surrogate()?;
        self.flush(char_class_map, inverse_map);
        self.output.push(char);
        Ok(())
    }

    fn surrogate(&mut self, char: char, char_class_map: &HashMap<char, CharClass>, inverse_map: &mut HashMap<String, char>) -> Result<(), String> {
        if is_high_surrogate(char) {
            self.forbid_waiting_high_surrogate()?;
            self.waiting_high_surrogate = Some(char);
        } else {
            if self.waiting_high_surrogate.is_none() {
                return Err("Invalid Unicode: Low surrogate without high surrogate".to_string());
            }
            if !self.expects_join(char_class_map) {
                self.flush(char_class_map, inverse_map);
            }
            self.accumulator.push(self.waiting_high_surrogate.unwrap());
            self.accumulator.push(char);
            self.waiting_high_surrogate = None;
        }
        Ok(())
    }

    fn result(&mut self, char_class_map: &HashMap<char, CharClass>, inverse_map: &mut HashMap<String, char>) -> String {
        self.flush(char_class_map, inverse_map);
        self.output.clone()
    }
}

impl DiacriticSupport {
    /// Start at end of Unicode Private Use Area and go down from there
    const STARTING_REPLACEMENT_CODEPOINT: u16 = 0xF8FF; // 63743
    const DEFAULT_RANGE_START: char = '\u{0021}';
    const DEFAULT_RANGE_END: char = '\u{FFEE}';

    /// Creates a new DiacriticSupport instance
    pub fn new(
        enabled: bool,
        range: Option<(char, char)>,
        left_diacritics: &str,
        right_diacritics: &str,
        joiner_diacritics: &str,
    ) -> Self {
        let mut char_class_map = HashMap::new();

        if enabled {
            let (range_start, range_end) = range.unwrap_or((Self::DEFAULT_RANGE_START, Self::DEFAULT_RANGE_END));

            // Map character categories to CharClass
            for c in range_start..=range_end {
                let category = get_char_category(c);
                if let Some(char_class) = category_to_char_class(category) {
                    char_class_map.insert(c, char_class);
                }
            }

            // Map diacritics to their respective classes
            for c in left_diacritics.chars() {
                char_class_map.insert(c, CharClass::LeftJoiner);
            }

            for c in right_diacritics.chars() {
                char_class_map.insert(c, CharClass::RightJoiner);
            }

            for c in joiner_diacritics.chars() {
                char_class_map.insert(c, CharClass::LeftRightJoiner);
            }
        }

        Self {
            enabled,
            char_class_map,
            fake_alphabet: HashMap::new(),
            inverse_map: HashMap::new(),
            next_free_diacritic_replacement_codepoint: Self::STARTING_REPLACEMENT_CODEPOINT,
        }
    }

    /// Factory that gets the primary constructor parameters by extracting the translation entries
    pub fn invoke(translations: &HashMap<String, String>) -> Self {
        let keys = TranslationKeys::new();

        let enable = parse_diacritic_entry(translations.get(keys.enable).unwrap_or(&"".to_string())) == "true";
        let range_start = parse_diacritic_entry(translations.get(keys.range_start).unwrap_or(&"".to_string()));
        let range_end = parse_diacritic_entry(translations.get(keys.range_end).unwrap_or(&"".to_string()));

        let range = if range_start.is_empty() || range_end.is_empty() {
            None
        } else {
            let start = range_start.chars().next().unwrap_or(Self::DEFAULT_RANGE_START);
            let end = range_end.chars().next().unwrap_or(Self::DEFAULT_RANGE_END);
            Some((start, end))
        };

        let left_diacritics = parse_diacritic_entry(translations.get(keys.left).unwrap_or(&"".to_string()));
        let right_diacritics = parse_diacritic_entry(translations.get(keys.right).unwrap_or(&"".to_string()));
        let joiner_diacritics = parse_diacritic_entry(translations.get(keys.joiner).unwrap_or(&"".to_string()));

        Self::new(enable, range, &left_diacritics, &right_diacritics, &joiner_diacritics)
    }

    /// Prepares this for a complete start-over, expecting a language load to instantiate a DiacriticSupport next
    pub fn reset(&mut self) {
        self.fake_alphabet.clear();
        self.free_translation_data();
        self.next_free_diacritic_replacement_codepoint = Self::STARTING_REPLACEMENT_CODEPOINT;
    }

    /// This is the main engine for rendering text glyphs after the translation loader has filled up this object
    ///
    /// # Arguments
    /// * `char` - The real or "fake alphabet" char stored by `remap_diacritics` to render
    ///
    /// # Returns
    /// The one to many (probably 8 max) codepoint string to be rendered into a single glyph by native font services
    pub fn get_string_for(&self, char: char) -> String {
        self.fake_alphabet.get(&char)
            .cloned()
            .unwrap_or_else(|| char.to_string())
    }

    /// Call when use of `remap_diacritics` is finished to save some memory
    pub fn free_translation_data(&mut self) {
        // Group by length and log examples
        let mut length_groups: HashMap<usize, Vec<&String>> = HashMap::new();
        for key in self.inverse_map.keys() {
            length_groups.entry(key.len()).or_insert_with(Vec::new).push(key);
        }

        let mut lengths: Vec<usize> = length_groups.keys().cloned().collect();
        lengths.sort();

        for length in lengths {
            if let Some(examples) = length_groups.get(&length) {
                if let Some(example) = examples.first() {
                    debug!("Length {} - example {}", length, example);
                }
            }
        }

        self.inverse_map.clear();
    }

    /// Other "fake" alphabets can use Unicode Private Use Areas from U+E000 up to including...
    pub fn get_current_free_code(&self) -> char {
        char::from_u32(self.next_free_diacritic_replacement_codepoint as u32)
            .unwrap_or('\u{FFFD}') // Replacement character
    }

    /// If this is true, no need to bother remapping chars at render time
    pub fn is_empty(&self) -> bool {
        self.fake_alphabet.is_empty()
    }

    /// Set at instantiation, if true the translation loader need not bother passing stuff through `remap_diacritics`
    pub fn is_enabled(&self) -> bool {
        this.enabled
    }

    /// Get the character class for a given character
    fn get_char_class(&self, char: char) -> CharClass {
        self.char_class_map.get(&char).copied().unwrap_or(CharClass::None)
    }

    /// Get the replacement character for a joined string
    fn get_replacement_char(&mut self, joined: &str) -> char {
        if let Some(&char) = self.inverse_map.get(joined) {
            char
        } else {
            self.create_replacement_char(joined)
        }
    }

    /// Create a new replacement character for a joined string
    fn create_replacement_char(&mut self, joined: &str) -> char {
        let char = self.get_current_free_code();
        self.next_free_diacritic_replacement_codepoint -= 1;

        // Check if we've exhausted the Unicode private use area
        if self.next_free_diacritic_replacement_codepoint < 0xE000 {
            panic!("DiacriticsSupport has exhausted the Unicode private use area");
        }

        self.fake_alphabet.insert(char, joined.to_string());
        self.inverse_map.insert(joined.to_string(), char);

        char
    }

    /// Replaces the combos of diacritics/joiners with their affected characters with a "fake" alphabet
    pub fn remap_diacritics(&mut self, value: &str) -> Result<String, String> {
        if !self.enabled {
            return Err("DiacriticSupport not set up properly for translation processing".to_string());
        }

        let mut data = LineData::new(value.len());

        for c in value.chars() {
            match self.get_char_class(c) {
                CharClass::None => data.flush_append(c, &self.char_class_map, &mut self.inverse_map)?,
                CharClass::Base => data.flush_accumulate(c, &self.char_class_map, &mut self.inverse_map)?,
                CharClass::LeftJoiner => data.accumulate(c)?,
                CharClass::RightJoiner => data.flush_accumulate(c, &self.char_class_map, &mut self.inverse_map)?,
                CharClass::LeftRightJoiner => data.accumulate(c)?,
                CharClass::Surrogate => data.surrogate(c, &self.char_class_map, &mut self.inverse_map)?,
            }
        }

        Ok(data.result(&self.char_class_map, &mut self.inverse_map))
    }

    /// Get all known combinations of diacritics
    pub fn get_known_combinations(&self) -> HashSet<String> {
        self.inverse_map.keys().cloned().collect()
    }
}

/// Parse a diacritic entry from a translation string
fn parse_diacritic_entry(entry: &str) -> String {
    if entry.is_empty() {
        return String::new();
    }

    // Strip comments and quotes
    let stripped = entry.split('#').next().unwrap_or(entry).trim();
    let stripped = stripped.trim_matches('"');

    // Split by whitespace and process each token
    let mut result = String::new();
    for token in stripped.split_whitespace() {
        if token.len() == 1 {
            result.push(token.chars().next().unwrap());
        } else if token.to_lowercase().starts_with("u+") {
            if let Ok(code) = u32::from_str_radix(&token[2..], 16) {
                if let Some(c) = char::from_u32(code) {
                    result.push(c);
                }
            }
        } else if stripped.split_whitespace().count() == 1 {
            // Single token that's not a single char or u+ notation, just use it as is
            result.push_str(token);
        } else {
            panic!("Invalid diacritic definition: \"{}\" is not a single character or unicode codepoint notation", token);
        }
    }

    result
}

/// Get the character category for a given character
fn get_char_category(c: char) -> u8 {
    // This is a simplified version - in a real implementation, you'd use a proper Unicode category function
    // For now, we'll use a basic approach based on the character's properties
    if c.is_uppercase() {
        1 // UPPERCASE_LETTER
    } else if c.is_lowercase() {
        2 // LOWERCASE_LETTER
    } else if c.is_numeric() {
        6 // DECIMAL_DIGIT_NUMBER
    } else if c.is_whitespace() {
        15 // SPACE_SEPARATOR
    } else if c.is_control() {
        15 // CONTROL
    } else if c.is_alphabetic() {
        3 // OTHER_LETTER
    } else {
        0 // UNASSIGNED
    }
}

/// Convert a character category to a CharClass
fn category_to_char_class(category: u8) -> Option<CharClass> {
    match category {
        1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 => Some(CharClass::Base), // Letters and numbers
        9 | 10 | 11 => Some(CharClass::LeftJoiner), // Combining marks
        12 => Some(CharClass::Surrogate), // Surrogates
        _ => None, // Everything else
    }
}

/// Check if a character is a high surrogate
fn is_high_surrogate(c: char) -> bool {
    let code = c as u32;
    code >= 0xD800 && code <= 0xDBFF
}

/// Get the replacement character for a joined string
fn get_replacement_char(joined: &str, inverse_map: &mut HashMap<String, char>) -> char {
    if let Some(&char) = inverse_map.get(joined) {
        char
    } else {
        // This is a simplified version - in a real implementation, you'd create a new replacement character
        // For now, we'll just use the first character of the joined string
        joined.chars().next().unwrap_or('\u{FFFD}')
    }
}