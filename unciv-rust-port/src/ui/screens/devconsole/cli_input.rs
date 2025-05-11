use std::cmp::Ordering;
use std::fmt;
use bevy::prelude::*;
use regex::Regex;
use once_cell::sync::Lazy;

use crate::models::ruleset::IRulesetObject;
use crate::models::stats::{INamed, Stat};
use crate::ui::screens::devconsole::dev_console_popup::DevConsolePopup;
use crate::ui::screens::devconsole::console_error::ConsoleErrorException;

/// Represents the method used to convert/display ruleset object (or other) names in console input.
/// - Goal is to make them comparable, and to make parameter-delimiting spaces unambiguous, both in a user-friendly way.
/// - Method 1: Everything is lowercase and spaces replaced with '-': `mechanized-infantry`.
/// - Method 2: See [to_quoted_representation]: `"Mechanized Infantry"`, `"Ship of the Line"` (case from json except the first word which gets titlecased).
/// - Note: Method 2 supports "open" quoting, that is, the closing quote is missing from parsed input, for autocomplete purposes.
#[derive(Debug, Clone, PartialEq)]
pub struct CliInput {
    /// The method used to format and parse the input
    method: Method,
    /// The processed content
    content: String,
    /// The original input string
    original: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Dashed,
    Quoted,
}

impl Method {
    pub fn or(self, other: Method) -> Method {
        if self == Method::Dashed && other == Method::Dashed {
            Method::Dashed
        } else {
            Method::Quoted
        }
    }

    pub fn and(self, other: Method) -> Method {
        if self == Method::Quoted && other == Method::Quoted {
            Method::Quoted
        } else {
            Method::Dashed
        }
    }
}

static REPEATED_WHITESPACE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?<=\s)\s+").unwrap());
static SPLIT_STRING_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?x)
        "[^"]+(?:"|$)      # A quoted phrase, but the closing quote is optional at the end of the string
        |                  # OR
        \S+               # consecutive non-whitespace
        |                 # OR
        (?:(?<=\s)$)     # a terminal empty string if preceded by whitespace
    "#).unwrap()
});

impl CliInput {
    pub fn new(parameter: impl Into<String>) -> Self {
        let parameter = parameter.into();
        let method = if has_leading_quote(&parameter) {
            Method::Quoted
        } else {
            Method::Dashed
        };
        let content = match method {
            Method::Dashed => to_dashed_representation(&parameter),
            Method::Quoted => to_quoted_representation(&parameter),
        };
        Self {
            method,
            content,
            original: parameter,
        }
    }

    pub fn empty() -> Self {
        Self::new("")
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn original_length(&self) -> usize {
        self.original.len()
    }

    pub fn original_unquoted(&self) -> String {
        remove_outer_quotes(&self.original)
    }

    pub fn to_method(&self, method: Method) -> Self {
        if self.method == method {
            self.clone()
        } else {
            Self::new(self.original.clone()).with_method(method)
        }
    }

    pub fn to_int(&self) -> Result<i32, ConsoleErrorException> {
        self.content.parse().map_err(|_| {
            ConsoleErrorException::new(&format!("'{}' is not a valid number.", self))
        })
    }

    pub fn to_float(&self) -> Result<f32, ConsoleErrorException> {
        self.content.parse().map_err(|_| {
            ConsoleErrorException::new(&format!("'{}' is not a valid number.", self))
        })
    }

    pub fn to_boolean(&self) -> Result<bool, ConsoleErrorException> {
        match self.content.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(ConsoleErrorException::new(&format!("'{}' is not a valid boolean value.", self)))
        }
    }

    pub fn to_stat(&self) -> Result<Stat, ConsoleErrorException> {
        Stat::from_str(&self.content).ok_or_else(|| {
            ConsoleErrorException::new(&format!("'{}' is not an acceptable Stat.", self))
        })
    }

    pub fn find_or_null<T: INamed>(&self, options: &[T]) -> Option<&T> {
        options.iter().find(|item| self.equals(&item.name()))
    }

    pub fn find<T: INamed>(&self, options: &[T]) -> Result<&T, ConsoleErrorException> {
        self.find_or_null(options).ok_or_else(|| {
            let type_name = std::any::type_name::<T>();
            let options_str = options.iter()
                .map(|item| item.name())
                .collect::<Vec<_>>()
                .join(", ");
            ConsoleErrorException::new(&format!(
                "'{}' is not a valid {}. Options are: {}",
                self, type_name, options_str
            ))
        })
    }

    fn get_autocomplete_string(&self, param_method: Method, up_to: usize, to_append: &str) -> String {
        if param_method == Method::Dashed && self.method == Method::Dashed {
            format!("{}{}", &self.content[..up_to.min(self.content.len())], to_append)
        } else {
            let source = if self.method == Method::Quoted {
                &self.content
            } else {
                &to_quoted_representation(&self.original)
            };
            let suffix = if !to_append.is_empty() {
                format!("\"{}",to_append)
            } else {
                String::new()
            };
            format!("\"{}{}",
                &source[..up_to.min(source.len())],
                suffix
            )
        }
    }
}

impl PartialEq for CliInput {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for CliInput {}

impl PartialOrd for CliInput {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CliInput {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.method, other.method) {
            (Method::Dashed, Method::Dashed) => self.content.cmp(&other.content),
            (Method::Quoted, Method::Quoted) => {
                self.content.to_lowercase().cmp(&other.content.to_lowercase())
            }
            (Method::Dashed, _) => {
                self.content.cmp(&to_dashed_representation(&other.original))
            }
            (_, Method::Dashed) => {
                to_dashed_representation(&self.original).cmp(&other.content)
            }
        }
    }
}

impl fmt::Display for CliInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.method {
            Method::Dashed => write!(f, "{}", self.content),
            Method::Quoted => write!(f, "\"{}\"", self.content),
        }
    }
}

// Helper functions
fn has_leading_quote(s: &str) -> bool {
    s.starts_with('"')
}

fn to_dashed_representation(s: &str) -> String {
    remove_outer_quotes(s).to_lowercase().replace(' ', "-")
}

fn remove_outer_quotes(s: &str) -> String {
    s.trim_start_matches('"').trim_end_matches('"').to_string()
}

fn to_quoted_representation(s: &str) -> String {
    let s = remove_outer_quotes(s);
    let mut chars = s.chars();
    let mut result = String::with_capacity(s.len());

    if let Some(first) = chars.next() {
        result.push(first.to_uppercase().next().unwrap_or(first));
    }

    result.extend(chars);
    REPEATED_WHITESPACE_REGEX.replace_all(&result, " ").into_owned()
}

pub fn split_to_cli_input(s: &str) -> Vec<CliInput> {
    SPLIT_STRING_REGEX.find_iter(s)
        .map(|m| CliInput::new(m.as_str()))
        .collect()
}

// Extension trait for DevConsolePopup
pub trait DevConsolePopupExt {
    fn find_cli_input<T: IRulesetObject>(&self, param: &CliInput) -> Option<&T>;
    fn get_autocomplete_string(
        &self,
        last_word: &CliInput,
        all_options: &[CliInput],
    ) -> Option<String>;
}

impl DevConsolePopupExt for DevConsolePopup {
    fn find_cli_input<T: IRulesetObject>(&self, param: &CliInput) -> Option<&T> {
        self.game_info.ruleset.all_ruleset_objects()
            .filter_map(|obj| obj.as_any().downcast_ref::<T>())
            .find(|obj| param.equals(&obj.name()))
    }

    fn get_autocomplete_string(
        &self,
        last_word: &CliInput,
        all_options: &[CliInput],
    ) -> Option<String> {
        let matching_options: Vec<_> = all_options.iter()
            .filter(|opt| opt.starts_with(last_word))
            .collect();

        if matching_options.is_empty() {
            return None;
        }

        if matching_options.len() == 1 {
            return Some(matching_options[0].get_autocomplete_string(
                last_word.method,
                matching_options[0].content.len(),
                " "
            ));
        }

        let show_method = last_word.method.or(matching_options[0].method);
        let message = matching_options.iter()
            .map(|opt| opt.to_method(show_method))
            .map(|opt| opt.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        self.show_response(Some(&format!("Matching completions: {}", message)), Color::LIME);

        let first_option = &matching_options[0];
        let dashed = to_dashed_representation(&first_option.original);

        for (idx, ch) in dashed.chars().enumerate() {
            if matching_options.iter().any(|opt| {
                let opt_dashed = to_dashed_representation(&opt.original);
                opt_dashed.len() <= idx || opt_dashed.chars().nth(idx) != Some(ch)
            }) {
                return Some(first_option.get_autocomplete_string(last_word.method, idx, ""));
            }
        }

        Some(first_option.get_autocomplete_string(last_word.method, dashed.len(), ""))
    }
}