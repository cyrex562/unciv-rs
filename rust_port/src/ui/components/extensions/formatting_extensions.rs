// Source: orig_src/core/src/com/unciv/ui/components/extensions/FormattingExtensions.kt
// Ported to Rust

use std::time::Duration;
use std::collections::BTreeMap;
use std::fmt;
use chrono::{DateTime, Local, TimeZone, Utc};
use chrono::format::ParseError;
use chrono::format::strptime;
use cgmath::Vector2;

use crate::models::translations::tr;
use crate::ui::components::fonts::Fonts;

/// Trait for percentage conversion
pub trait ToPercent {
    /// Convert a percentage number to a multiplication value
    fn to_percent(&self) -> f32;
}

impl ToPercent for f32 {
    /// Convert a percentage number (e.g. 25) to the multiplication value (e.g. 1.25)
    fn to_percent(&self) -> f32 {
        1.0 + self / 100.0
    }
}

impl ToPercent for i32 {
    /// Convert a percentage number (e.g. 25) to the multiplication value (e.g. 1.25)
    fn to_percent(&self) -> f32 {
        (*self as f32).to_percent()
    }
}

impl ToPercent for String {
    /// Convert a percentage number (e.g. 25) to the multiplication value (e.g. 1.25)
    fn to_percent(&self) -> f32 {
        self.parse::<f32>().unwrap_or(0.0).to_percent()
    }
}

/// Trait for resource consumption string formatting
pub trait ResourceConsumption {
    /// Convert a resource name into "Consumes [amount] $resource" string (untranslated)
    fn get_consumes_amount_string(&self, amount: i32, is_stockpiled: bool) -> String;

    /// Convert a resource name into "Need [amount] more $resource" string (untranslated)
    fn get_need_more_amount_string(&self, amount: i32) -> String;
}

impl ResourceConsumption for String {
    /// Convert a resource name into "Consumes [amount] $resource" string (untranslated)
    fn get_consumes_amount_string(&self, amount: i32, is_stockpiled: bool) -> String {
        let unique_string = format!("{{Consumes [{}] [{}]}}", amount, self);
        if is_stockpiled {
            format!("{} /{}", unique_string, Fonts::turn())
        } else {
            unique_string
        }
    }

    /// Convert a resource name into "Need [amount] more $resource" string (untranslated)
    fn get_need_more_amount_string(&self, amount: i32) -> String {
        format!("Need [{}] more [{}]", amount, self)
    }
}

/// Trait for signed number formatting
pub trait ToStringSigned {
    /// Convert a number to a string with a sign prefix
    fn to_string_signed(&self) -> String;
}

impl ToStringSigned for i32 {
    /// Convert a number to a string with a sign prefix
    fn to_string_signed(&self) -> String {
        if *self > 0 {
            format!("+{}", self.tr())
        } else {
            self.tr()
        }
    }
}

/// Trait for duration formatting
pub trait DurationFormat {
    /// Format a duration into a translated string
    fn format(&self) -> String;

    /// Format a duration into a translated string, but only showing the most significant time unit
    fn format_short(&self) -> String;
}

impl DurationFormat for Duration {
    /// Format a duration into a translated string
    fn format(&self) -> String {
        let parts = self.to_parts();
        let mut result = String::new();
        let mut first_part_already_added = false;

        for (unit, part) in parts {
            if part == 0 {
                continue;
            }

            if first_part_already_added {
                result.push_str(", ");
            }

            result.push_str(&format!("[{}] {}", part.tr(), unit));
            first_part_already_added = true;
        }

        result
    }

    /// Format a duration into a translated string, but only showing the most significant time unit
    fn format_short(&self) -> String {
        let parts = self.to_parts();

        for (unit, part) in parts {
            if part > 0 {
                return format!("[{}] {}", part.tr(), unit);
            }
        }

        // If all parts are zero, return seconds
        format!("[{}] {}", parts.get(&ChronoUnit::Seconds).unwrap_or(&0).tr(), ChronoUnit::Seconds)
    }
}

impl Duration {
    /// Convert a duration to its parts
    fn to_parts(&self) -> BTreeMap<ChronoUnit, i64> {
        let mut parts = BTreeMap::new();

        let seconds_part = self.as_secs() % 60;
        let minute_part = self.as_secs() / 60 % 60;
        let hour_part = self.as_secs() / 3600 % 24;
        let day_part = self.as_secs() / 86400;

        parts.insert(ChronoUnit::Seconds, seconds_part as i64);
        parts.insert(ChronoUnit::Minutes, minute_part as i64);
        parts.insert(ChronoUnit::Hours, hour_part as i64);
        parts.insert(ChronoUnit::Days, day_part as i64);

        parts
    }
}

/// Enum for time units
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChronoUnit {
    Seconds,
    Minutes,
    Hours,
    Days,
}

impl fmt::Display for ChronoUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChronoUnit::Seconds => write!(f, "seconds"),
            ChronoUnit::Minutes => write!(f, "minutes"),
            ChronoUnit::Hours => write!(f, "hours"),
            ChronoUnit::Days => write!(f, "days"),
        }
    }
}

/// Standardize date formatting
pub struct UncivDateFormat;

impl UncivDateFormat {
    /// Format a date to ISO format with minutes
    pub fn format_date(date: DateTime<Local>) -> String {
        date.format("%Y-%m-%d %H:%M").to_string()
    }

    /// Parse an UTC date as passed by online API's
    /// example: `"2021-04-11T14:43:33Z".parse_date()`
    pub fn parse_date(date_str: &str) -> Result<DateTime<Utc>, ParseError> {
        Utc.datetime_from_str(date_str, "%Y-%m-%dT%H:%M:%SZ")
    }
}

/// Trait for Vector2 formatting
pub trait Vector2Format {
    /// Format a Vector2 like (0,0) instead of (0.0,0.0)
    fn to_pretty_string(&self) -> String;
}

impl Vector2Format for Vector2<f32> {
    /// Format a Vector2 like (0,0) instead of (0.0,0.0)
    fn to_pretty_string(&self) -> String {
        format!("({},{})", self.x as i32, self.y as i32)
    }
}