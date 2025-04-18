use std::fmt;
use serde::{Serialize, Deserialize};
use lazy_static::lazy_static;
use std::collections::HashMap;

/// Represents a key for use in keyboard input handling
///
/// Example: KeyCharAndCode::from_char('R'), KeyCharAndCode::from_code(59) // F1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyCharAndCode {
    /// The character representation of the key
    pub char: Option<char>,
    /// The key code
    pub code: i32,
}

impl KeyCharAndCode {
    /// Create a new KeyCharAndCode with the given character
    pub fn from_char(c: char) -> Self {
        Self {
            char: Some(c),
            code: c as i32,
        }
    }

    /// Create a new KeyCharAndCode with the given key code
    pub fn from_code(code: i32) -> Self {
        Self {
            char: None,
            code,
        }
    }

    /// Create a new KeyCharAndCode with the given character and code
    pub fn new(c: char, code: i32) -> Self {
        Self {
            char: Some(c),
            code,
        }
    }

    /// Create a new KeyCharAndCode with Ctrl+character
    pub fn ctrl(c: char) -> Self {
        Self {
            char: Some(c),
            code: c as i32,
        }
    }

    /// Unknown key
    pub const UNKNOWN: Self = Self {
        char: None,
        code: -1,
    };

    /// Backspace key
    pub const BACK: Self = Self {
        char: None,
        code: 8,
    };

    /// Escape key
    pub const ESC: Self = Self {
        char: None,
        code: 27,
    };

    /// Return key
    pub const RETURN: Self = Self {
        char: None,
        code: 13,
    };

    /// Numpad enter key
    pub const NUMPAD_ENTER: Self = Self {
        char: None,
        code: 156,
    };

    /// Space key
    pub const SPACE: Self = Self {
        char: Some(' '),
        code: 32,
    };

    /// Delete key
    pub const DEL: Self = Self {
        char: None,
        code: 127,
    };

    /// Tab key
    pub const TAB: Self = Self {
        char: None,
        code: 9,
    };

    /// Create a new KeyCharAndCode with the given character, comparing by character only
    pub fn ascii(c: char) -> Self {
        Self {
            char: Some(c.to_ascii_lowercase()),
            code: 0,
        }
    }

    /// Map a character to a key code if possible, otherwise return a character-based instance
    pub fn map_char(c: char) -> Self {
        let code = GdxKeyCodeFixes::value_of(&c.to_ascii_uppercase().to_string());
        if code == -1 {
            Self::from_char(c)
        } else {
            Self::from_code(code)
        }
    }

    /// Create a Ctrl+key from a key code
    pub fn ctrl_from_code(key_code: i32) -> Self {
        let name = GdxKeyCodeFixes::to_string(key_code);
        if name.len() == 1 && name.chars().next().unwrap().is_ascii_alphabetic() {
            Self::ctrl(name.chars().next().unwrap())
        } else {
            Self::from_code(key_code)
        }
    }

    /// Parse a human-readable representation into a KeyCharAndCode
    ///
    /// Understands:
    /// - Single characters or quoted single characters (double-quotes)
    /// - Names as produced by the non-conforming String.toString(Int) function
    /// - Ctrl-key combinations
    ///
    /// Not parseable input, including the empty string, results in KeyCharAndCode::UNKNOWN.
    pub fn parse(text: &str) -> Self {
        if text.is_empty() {
            return Self::UNKNOWN;
        }

        // Single character
        if text.len() == 1 {
            return Self::from_char(text.chars().next().unwrap());
        }

        // Quoted character
        if text.len() == 3 && text.starts_with('"') && text.ends_with('"') {
            return Self::from_char(text.chars().nth(1).unwrap());
        }

        // Ctrl-key combination
        if text.len() == 6 && text.starts_with("Ctrl-") {
            return Self::ctrl(text.chars().nth(5).unwrap());
        }

        // Special keys
        if text == "ESC" {
            return Self::ESC;
        }

        // Try to parse as a key code
        let code = GdxKeyCodeFixes::value_of(text);
        if code == -1 {
            Self::UNKNOWN
        } else {
            Self::from_code(code)
        }
    }
}

impl fmt::Display for KeyCharAndCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *this == Self::UNKNOWN {
            write!(f, "")
        } else if this.char.is_none() {
            write!(f, "{}", GdxKeyCodeFixes::to_string(this.code))
        } else if *this == Self::ESC {
            write!(f, "ESC")
        } else if let Some(c) = this.char {
            if (c as i32) < 32 {
                write!(f, "Ctrl-{}", ((c as i32) + 64) as u8 as char)
            } else {
                write!(f, "\"{}\"", c)
            }
        } else {
            write!(f, "{}", GdxKeyCodeFixes::to_string(this.code))
        }
    }
}

/// Helper functions for key code conversion
pub struct GdxKeyCodeFixes;

impl GdxKeyCodeFixes {
    /// Convert a key code to a string representation
    pub fn to_string(key_code: i32) -> String {
        lazy_static! {
            static ref KEY_CODE_MAP: HashMap<i32, &'static str> = {
                let mut map = HashMap::new();
                map.insert(8, "BACKSPACE");
                map.insert(9, "TAB");
                map.insert(13, "ENTER");
                map.insert(19, "UP");
                map.insert(20, "DOWN");
                map.insert(21, "LEFT");
                map.insert(22, "RIGHT");
                map.insert(27, "ESCAPE");
                map.insert(32, "SPACE");
                map.insert(33, "PAGE_UP");
                map.insert(34, "PAGE_DOWN");
                map.insert(35, "END");
                map.insert(36, "HOME");
                map.insert(37, "INSERT");
                map.insert(127, "DEL");
                map.insert(128, "NUMPAD_0");
                map.insert(129, "NUMPAD_1");
                map.insert(130, "NUMPAD_2");
                map.insert(131, "NUMPAD_3");
                map.insert(132, "NUMPAD_4");
                map.insert(133, "NUMPAD_5");
                map.insert(134, "NUMPAD_6");
                map.insert(135, "NUMPAD_7");
                map.insert(136, "NUMPAD_8");
                map.insert(137, "NUMPAD_9");
                map.insert(138, "NUMPAD_ADD");
                map.insert(139, "NUMPAD_SUBTRACT");
                map.insert(140, "NUMPAD_MULTIPLY");
                map.insert(141, "NUMPAD_DIVIDE");
                map.insert(142, "NUMPAD_ENTER");
                map.insert(143, "NUMPAD_DECIMAL");
                map.insert(144, "NUMPAD_LEFT");
                map.insert(145, "NUMPAD_RIGHT");
                map.insert(146, "NUMPAD_UP");
                map.insert(147, "NUMPAD_DOWN");
                map.insert(148, "NUMPAD_HOME");
                map.insert(149, "NUMPAD_END");
                map.insert(150, "NUMPAD_PAGE_UP");
                map.insert(151, "NUMPAD_PAGE_DOWN");
                map.insert(152, "NUMPAD_INSERT");
                map.insert(153, "NUMPAD_DELETE");
                map.insert(154, "NUMPAD_CLEAR");
                map.insert(155, "NUMPAD_ADD");
                map.insert(156, "NUMPAD_ENTER");
                map.insert(157, "NUMPAD_EQUALS");
                map.insert(59, "F1");
                map.insert(60, "F2");
                map.insert(61, "F3");
                map.insert(62, "F4");
                map.insert(63, "F5");
                map.insert(64, "F6");
                map.insert(65, "F7");
                map.insert(66, "F8");
                map.insert(67, "F9");
                map.insert(68, "F10");
                map.insert(87, "F11");
                map.insert(88, "F12");
                map
            };
        }

        KEY_CODE_MAP.get(&key_code).map(|&s| s.to_string()).unwrap_or_else(|| {
            if key_code >= 65 && key_code <= 90 {
                // A-Z
                ((key_code as u8) as char).to_string()
            } else if key_code >= 97 && key_code <= 122 {
                // a-z
                ((key_code as u8) as char).to_string()
            } else if key_code >= 48 && key_code <= 57 {
                // 0-9
                ((key_code as u8) as char).to_string()
            } else {
                format!("UNKNOWN({})", key_code)
            }
        })
    }

    /// Convert a string to a key code
    pub fn value_of(name: &str) -> i32 {
        lazy_static! {
            static ref NAME_TO_CODE: HashMap<&'static str, i32> = {
                let mut map = HashMap::new();
                map.insert("BACKSPACE", 8);
                map.insert("TAB", 9);
                map.insert("ENTER", 13);
                map.insert("UP", 19);
                map.insert("DOWN", 20);
                map.insert("LEFT", 21);
                map.insert("RIGHT", 22);
                map.insert("ESCAPE", 27);
                map.insert("ESC", 27);
                map.insert("SPACE", 32);
                map.insert("PAGE_UP", 33);
                map.insert("PAGE_DOWN", 34);
                map.insert("END", 35);
                map.insert("HOME", 36);
                map.insert("INSERT", 37);
                map.insert("DEL", 127);
                map.insert("DELETE", 127);
                map.insert("NUMPAD_0", 128);
                map.insert("NUMPAD_1", 129);
                map.insert("NUMPAD_2", 130);
                map.insert("NUMPAD_3", 131);
                map.insert("NUMPAD_4", 132);
                map.insert("NUMPAD_5", 133);
                map.insert("NUMPAD_6", 134);
                map.insert("NUMPAD_7", 135);
                map.insert("NUMPAD_8", 136);
                map.insert("NUMPAD_9", 137);
                map.insert("NUMPAD_ADD", 138);
                map.insert("NUMPAD_SUBTRACT", 139);
                map.insert("NUMPAD_MULTIPLY", 140);
                map.insert("NUMPAD_DIVIDE", 141);
                map.insert("NUMPAD_ENTER", 142);
                map.insert("NUMPAD_DECIMAL", 143);
                map.insert("NUMPAD_LEFT", 144);
                map.insert("NUMPAD_RIGHT", 145);
                map.insert("NUMPAD_UP", 146);
                map.insert("NUMPAD_DOWN", 147);
                map.insert("NUMPAD_HOME", 148);
                map.insert("NUMPAD_END", 149);
                map.insert("NUMPAD_PAGE_UP", 150);
                map.insert("NUMPAD_PAGE_DOWN", 151);
                map.insert("NUMPAD_INSERT", 152);
                map.insert("NUMPAD_DELETE", 153);
                map.insert("NUMPAD_CLEAR", 154);
                map.insert("NUMPAD_EQUALS", 157);
                map.insert("F1", 59);
                map.insert("F2", 60);
                map.insert("F3", 61);
                map.insert("F4", 62);
                map.insert("F5", 63);
                map.insert("F6", 64);
                map.insert("F7", 65);
                map.insert("F8", 66);
                map.insert("F9", 67);
                map.insert("F10", 68);
                map.insert("F11", 87);
                map.insert("F12", 88);
                map
            };
        }

        // Check if it's a single character
        if name.len() == 1 {
            let c = name.chars().next().unwrap();
            if c.is_ascii_alphabetic() {
                return c.to_ascii_uppercase() as i32;
            } else if c.is_ascii_digit() {
                return c as i32;
            }
        }

        // Check if it's in the map
        NAME_TO_CODE.get(name).copied().unwrap_or(-1)
    }
}

// Implement serialization/deserialization
impl serde::Serialize for KeyCharAndCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&this.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for KeyCharAndCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self::parse(&s))
    }
}