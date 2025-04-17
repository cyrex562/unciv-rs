use std::cmp::Ordering;
use crate::models::translations::tr;

/// Utility for converting years to human-readable text
///
/// This utility provides methods for converting years to human-readable text,
/// with support for both standard AD/BC notation and the Maya calendar.
pub struct YearTextUtil;

impl YearTextUtil {
    /// Converts a year to a human-readable year (e.g. "1800 AD" or "3000 BC") while respecting the Maya calendar.
    ///
    /// # Arguments
    ///
    /// * `year` - The year to convert
    /// * `uses_maya_calendar` - Whether to use the Maya calendar
    ///
    /// # Returns
    ///
    /// A translated string representing the year
    pub fn to_year_text(year: i32, uses_maya_calendar: bool) -> String {
        let year_text = if uses_maya_calendar {
            MayaCalendar::year_to_maya_date(year)
        } else {
            let abs_year = year.abs();
            let era = if year < 0 { "BC" } else { "AD" };
            format!("[{}] {}", abs_year, era)
        };

        tr(&year_text)
    }
}

/// Utility for converting years to Maya calendar dates
pub struct MayaCalendar;

impl MayaCalendar {
    /// Converts a year to a Maya calendar date
    ///
    /// # Arguments
    ///
    /// * `year` - The year to convert
    ///
    /// # Returns
    ///
    /// A string representing the year in the Maya calendar
    pub fn year_to_maya_date(year: i32) -> String {
        // This is a placeholder implementation
        // In a real implementation, this would convert the year to a Maya calendar date
        // For now, we'll just return a simple string
        format!("Maya Year {}", year)
    }
}