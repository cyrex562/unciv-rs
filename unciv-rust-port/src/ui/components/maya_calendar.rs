use std::collections::HashMap;

/// A utility struct for handling Maya calendar conversions and calculations.
pub struct MayaCalendar {
    /// The number of days since the Maya Long Count start date (August 11, 3114 BCE)
    days_since_long_count_start: i64,
}

impl MayaCalendar {
    /// Creates a new MayaCalendar instance from a given number of days since the Long Count start date.
    pub fn new(days_since_long_count_start: i64) -> Self {
        Self {
            days_since_long_count_start,
        }
    }

    /// Converts a Gregorian date to Maya Long Count days.
    pub fn from_gregorian(year: i32, month: i32, day: i32) -> Self {
        // Algorithm for converting Gregorian to Julian Day Number
        let a = (14 - month) / 12;
        let y = year + 4800 - a;
        let m = month + 12 * a - 3;
        let jdn = day + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045;

        // Maya Long Count start date (August 11, 3114 BCE) in Julian Day Number
        let maya_start_jdn = 584283;

        Self {
            days_since_long_count_start: (jdn - maya_start_jdn) as i64,
        }
    }

    /// Returns the Maya Long Count date as a string in the format "b'ak'tun.k'atun.tun.winal.k'in"
    pub fn to_long_count_string(&self) -> String {
        let days = self.days_since_long_count_start;

        let baktun = days / (20 * 20 * 18 * 20);
        let remainder = days % (20 * 20 * 18 * 20);

        let katun = remainder / (20 * 18 * 20);
        let remainder = remainder % (20 * 18 * 20);

        let tun = remainder / (18 * 20);
        let remainder = remainder % (18 * 20);

        let winal = remainder / 20;
        let kin = remainder % 20;

        format!("{}.{}.{}.{}.{}", baktun, katun, tun, winal, kin)
    }

    /// Returns the Tzolkin date as a string in the format "number name"
    pub fn to_tzolkin_string(&self) -> String {
        let number = ((self.days_since_long_count_start % 13) + 1) as i32;
        let name_index = (self.days_since_long_count_start % 20) as usize;

        let names = [
            "Imix", "Ik'", "Ak'b'al", "K'an", "Chikchan",
            "Kimi", "Manik'", "Lamat", "Muluk", "Ok",
            "Chuwen", "Eb'", "B'en", "Ix", "Men",
            "K'ib'", "Kaban", "Etz'nab'", "Kawak", "Ajaw"
        ];

        format!("{} {}", number, names[name_index])
    }

    /// Returns the Haab date as a string in the format "day month"
    pub fn to_haab_string(&self) -> String {
        let day = ((self.days_since_long_count_start % 365) % 20) + 1;
        let month_index = ((self.days_since_long_count_start % 365) / 20) as usize;

        let months = [
            "Pop", "Wo'", "Sip", "Sotz'", "Sek",
            "Xul", "Yaxk'in", "Mol", "Ch'en", "Yax",
            "Sak'", "Keh", "Mak", "K'ank'in", "Muwan",
            "Pax", "K'ayab'", "Kumk'u", "Wayeb'"
        ];

        format!("{} {}", day, months[month_index])
    }

    /// Returns the Lord of the Night (G9) as a string
    pub fn to_lord_of_night_string(&self) -> String {
        let lords = ["G1", "G2", "G3", "G4", "G5", "G6", "G7", "G8", "G9"];
        let index = (self.days_since_long_count_start % 9) as usize;
        lords[index].to_string()
    }
}