use std::collections::HashMap;
use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};
use rand::Rng;

/// Known holidays (for easter egg use)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Holiday {
    Easter,
    Samhain,
    Xmas,
    DiaDeLosMuertos,
    YuleGoat,
    Qingming,
    Diwali,
    LunarNewYear,
    AprilFoolsDay,
    PrideDay,
    TowelDay,
    UncivBirthday,
    Friday13th,
    StarWarsDay,
    Passover,
}

impl Holiday {
    /// Get the chance of this holiday occurring
    pub fn chance(&self) -> f32 {
        match self {
            Holiday::DiaDeLosMuertos => 0.5,
            Holiday::Diwali => 0.2,
            Holiday::PrideDay => 0.333,
            Holiday::Passover => 0.2,
            _ => 1.0,
        }
    }

    /// Get all holidays
    pub fn all() -> Vec<Holiday> {
        vec![
            Holiday::Easter,
            Holiday::Samhain,
            Holiday::Xmas,
            Holiday::DiaDeLosMuertos,
            Holiday::YuleGoat,
            Holiday::Qingming,
            Holiday::Diwali,
            Holiday::LunarNewYear,
            Holiday::AprilFoolsDay,
            Holiday::PrideDay,
            Holiday::TowelDay,
            Holiday::UncivBirthday,
            Holiday::Friday13th,
            Holiday::StarWarsDay,
            Holiday::Passover,
        ]
    }

    /// Try to parse a holiday from a string
    pub fn from_str(s: &str) -> Option<Holiday> {
        match s {
            "Easter" => Some(Holiday::Easter),
            "Samhain" => Some(Holiday::Samhain),
            "Xmas" => Some(Holiday::Xmas),
            "DiaDeLosMuertos" => Some(Holiday::DiaDeLosMuertos),
            "YuleGoat" => Some(Holiday::YuleGoat),
            "Qingming" => Some(Holiday::Qingming),
            "Diwali" => Some(Holiday::Diwali),
            "LunarNewYear" => Some(Holiday::LunarNewYear),
            "AprilFoolsDay" => Some(Holiday::AprilFoolsDay),
            "PrideDay" => Some(Holiday::PrideDay),
            "TowelDay" => Some(Holiday::TowelDay),
            "UncivBirthday" => Some(Holiday::UncivBirthday),
            "Friday13th" => Some(Holiday::Friday13th),
            "StarWarsDay" => Some(Holiday::StarWarsDay),
            "Passover" => Some(Holiday::Passover),
            _ => None,
        }
    }
}

/// A range of dates
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl DateRange {
    /// Create a new date range from a single date
    pub fn of_date(date: NaiveDate) -> Self {
        Self {
            start: date,
            end: date,
        }
    }

    /// Create a new date range from year, month, day
    pub fn of_ymd(year: i32, month: u32, day: u32) -> Self {
        let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
        Self::of_date(date)
    }

    /// Create a new date range from a date and duration
    pub fn of_date_and_duration(date: NaiveDate, duration: i64) -> Self {
        Self {
            start: date,
            end: date + Duration::days(duration - 1),
        }
    }

    /// Create a new date range from year, month, day and duration
    pub fn of_ymd_and_duration(year: i32, month: u32, day: u32, duration: i64) -> Self {
        let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
        Self::of_date_and_duration(date, duration)
    }

    /// Get a never-occurring date range
    pub fn never() -> Self {
        let now = Local::now().date_naive();
        Self {
            start: now,
            end: now - Duration::days(1),
        }
    }

    /// Check if a date is in this range
    pub fn contains(&self, date: NaiveDate) -> bool {
        date >= self.start && date <= self.end
    }

    /// Get the length of this range in days
    pub fn length(&self) -> i64 {
        (self.end - self.start).num_days().max(0) + 1
    }
}

/// Holiday dates manager
pub struct HolidayDates {
    equinoxes: HashMap<i32, NaiveDate>,
    passover: HashMap<i32, NaiveDate>,
    diwali: HashMap<i32, NaiveDate>,
    lunar_new_year: HashMap<i32, NaiveDate>,
}

impl HolidayDates {
    /// Create a new HolidayDates instance
    pub fn new() -> Self {
        Self {
            equinoxes: Self::init_equinoxes(),
            passover: Self::init_passover(),
            diwali: Self::init_diwali(),
            lunar_new_year: Self::init_lunar_new_year(),
        }
    }

    /// Get the date range for a holiday in a specific year
    pub fn get_holiday_by_year(&self, holiday: &Holiday, year: i32) -> DateRange {
        match holiday {
            Holiday::Easter => self.get_easter(year),
            Holiday::Samhain => DateRange::of_ymd(year, 10, 31),
            Holiday::Xmas => DateRange::of_ymd_and_duration(year, 12, 24, 4),
            Holiday::DiaDeLosMuertos => DateRange::of_ymd_and_duration(year, 11, 1, 2),
            Holiday::YuleGoat => self.get_yule_goat(year),
            Holiday::Qingming => self.get_qingming(year),
            Holiday::Diwali => self.get_diwali(year),
            Holiday::LunarNewYear => self.get_lunar_new_year(year),
            Holiday::AprilFoolsDay => DateRange::of_ymd(year, 4, 1),
            Holiday::PrideDay => DateRange::of_ymd_and_duration(year, 6, 28, 3),
            Holiday::TowelDay => DateRange::of_ymd(year, 5, 25),
            Holiday::UncivBirthday => DateRange::of_ymd(year, 11, 21),
            Holiday::Friday13th => self.get_friday_13th(year),
            Holiday::StarWarsDay => DateRange::of_ymd(year, 5, 4),
            Holiday::Passover => self.get_passover(year),
        }
    }

    /// Get the holiday for a specific date
    pub fn get_holiday_by_date(&self, date: Option<NaiveDate>) -> Option<Holiday> {
        let date = date.unwrap_or_else(|| Local::now().date_naive());

        // Check for easter egg override from system property
        if let Ok(easter_egg) = std::env::var("easter_egg") {
            return Holiday::from_str(&easter_egg);
        }

        let mut rng = rand::thread_rng();
        Holiday::all().into_iter().find(|holiday| {
            let range = self.get_holiday_by_year(holiday, date.year());
            range.contains(date) && rng.gen::<f32>() <= holiday.chance()
        })
    }

    /// Get Easter date for a specific year
    fn get_easter(&self, year: i32) -> DateRange {
        // Algorithm from https://en.wikipedia.org/wiki/Date_of_Easter
        let a = year % 19;
        let b = year / 100;
        let c = year % 100;
        let d = b / 4;
        let e = b % 4;
        let g = (8 * b + 13) / 25;
        let h = (19 * a + b - d - g + 15) % 30;
        let i = c / 4;
        let k = c % 4;
        let l = (32 + 2 * e + 2 * i - h - k) % 7;
        let m = (a + 11 * h + 19 * l) / 433;
        let n = (h + l - 7 * m + 90) / 25;
        let p = (h + l - 7 * m + 33 * n + 19) % 32;

        let sunday = NaiveDate::from_ymd_opt(year, n as u32, p as u32).unwrap();
        DateRange::of_date_and_duration(sunday - Duration::days(2), 4)
    }

    /// Get Yule Goat date for a specific year
    fn get_yule_goat(&self, year: i32) -> DateRange {
        let nov_30 = NaiveDate::from_ymd_opt(year, 11, 30).unwrap();
        DateRange::of_date(self.closest_weekday(nov_30, Weekday::Sun))
    }

    /// Get Qingming date for a specific year
    fn get_qingming(&self, year: i32) -> DateRange {
        self.equinoxes.get(&year)
            .map(|date| DateRange::of_date(*date + Duration::days(15)))
            .unwrap_or_else(DateRange::never)
    }

    /// Get Diwali date for a specific year
    fn get_diwali(&self, year: i32) -> DateRange {
        self.diwali.get(&year)
            .map(|date| DateRange::of_date_and_duration(*date - Duration::days(2), 5))
            .unwrap_or_else(DateRange::never)
    }

    /// Get Lunar New Year date for a specific year
    fn get_lunar_new_year(&self, year: i32) -> DateRange {
        self.lunar_new_year.get(&year)
            .map(DateRange::of_date)
            .unwrap_or_else(DateRange::never)
    }

    /// Get Friday 13th date for a specific year
    fn get_friday_13th(&self, year: i32) -> DateRange {
        let mut rng = rand::thread_rng();
        let friday_13ths: Vec<_> = (1..=12)
            .filter_map(|month| {
                NaiveDate::from_ymd_opt(year, month, 13)
                    .filter(|date| date.weekday() == Weekday::Fri)
            })
            .collect();

        friday_13ths.choose(&mut rng)
            .map(|&date| DateRange::of_date(date))
            .unwrap_or_else(DateRange::never)
    }

    /// Get Passover date for a specific year
    fn get_passover(&self, year: i32) -> DateRange {
        self.passover.get(&year)
            .map(|date| DateRange::of_date_and_duration(*date - Duration::days(2), 5))
            .unwrap_or_else(DateRange::never)
    }

    /// Get the closest weekday to a date
    fn closest_weekday(&self, date: NaiveDate, target: Weekday) -> NaiveDate {
        let current = date.weekday();
        let days_between = (7 + current.num_days_from_monday() - target.num_days_from_monday()) % 7;
        if days_between < 4 {
            date + Duration::days(days_between as i64)
        } else {
            date - Duration::days((7 - days_between) as i64)
        }
    }

    // Initialize lookup tables
    fn init_equinoxes() -> HashMap<i32, NaiveDate> {
        let mut map = HashMap::new();
        for (year, date_str) in [
            (2024, "2024-03-20"), (2025, "2025-03-20"), (2026, "2026-03-20"),
            (2027, "2027-03-20"), (2028, "2028-03-20"), (2029, "2029-03-20"),
            (2030, "2030-03-20"), (2031, "2031-03-20"), (2032, "2032-03-20"),
            (2033, "2033-03-20"), (2034, "2034-03-20"), (2035, "2035-03-20"),
            (2036, "2036-03-20"), (2037, "2037-03-20"), (2038, "2038-03-20"),
            (2039, "2039-03-20"), (2040, "2040-03-20"), (2041, "2041-03-20"),
            (2042, "2042-03-20"), (2043, "2043-03-20"), (2044, "2044-03-19"),
            (2045, "2045-03-20"), (2046, "2046-03-20"), (2047, "2047-03-20"),
            (2048, "2048-03-19"), (2049, "2049-03-20"), (2050, "2050-03-20"),
            // Add more years as needed
        ] {
            map.insert(year, NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap());
        }
        map
    }

    fn init_passover() -> HashMap<i32, NaiveDate> {
        let mut map = HashMap::new();
        for (year, day) in [
            (2023, 5), (2024, 22), (2025, 12), (2026, 1),
            (2027, 21), (2028, 10), (2029, -2), (2030, 17),
            (2031, 7), (2032, -6), (2033, 13), (2034, 3),
            (2035, 23), (2036, 11), (2037, -2), (2038, 19),
            (2039, 8), (2040, -4), (2041, 15), (2042, 4),
            (2043, 24), (2044, 11), (2045, 1), (2046, 20),
            (2047, 10), (2048, -4), (2049, 16), (2050, 6),
            // Add more years as needed
        ] {
            map.insert(year, NaiveDate::from_ymd_opt(year, 4, 1).unwrap() + Duration::days(day));
        }
        map
    }

    fn init_diwali() -> HashMap<i32, NaiveDate> {
        let mut map = HashMap::new();
        for (year, date_str) in [
            (2024, "2024-11-01"), (2025, "2025-10-21"), (2026, "2026-11-08"),
            (2027, "2027-10-29"), (2028, "2028-10-17"), (2029, "2029-11-05"),
            (2030, "2030-10-26"), (2031, "2031-11-14"), (2032, "2032-11-02"),
            (2033, "2033-10-22"), (2034, "2034-11-10"), (2035, "2035-10-20"),
            (2036, "2036-10-19"), (2037, "2037-11-07"), (2038, "2038-10-27"),
            (2039, "2039-10-17"), (2040, "2040-11-04"),
            // Add more years as needed
        ] {
            map.insert(year, NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap());
        }
        map
    }

    fn init_lunar_new_year() -> HashMap<i32, NaiveDate> {
        let mut map = HashMap::new();
        for (year, day) in [
            (2024, 31+10), (2025, 29), (2026, 31+17), (2027, 31+6),
            (2028, 26), (2029, 31+13), (2030, 31+3), (2031, 23),
            (2032, 31+11), (2033, 31), (2034, 31+19), (2035, 31+8),
            (2036, 28), (2037, 31+15), (2038, 31+4), (2039, 24),
            (2040, 31+12), (2041, 31+1), (2042, 22), (2043, 31+10),
            (2044, 30), (2045, 31+17), (2046, 31+6), (2047, 26),
            (2048, 31+14), (2049, 31+2), (2050, 23), (2051, 31+11),
            (2052, 31+1), (2053, 31+19), (2054, 31+8), (2055, 28),
            (2056, 31+15), (2057, 31+4), (2058, 24), (2059, 31+12),
            (2060, 31+2), (2061, 21), (2062, 31+9), (2063, 29),
            (2064, 31+17), (2065, 31+5), (2066, 26), (2067, 31+14),
            (2068, 31+3), (2069, 23), (2070, 31+11), (2071, 31),
            (2072, 31+19), (2073, 31+7), (2074, 27), (2075, 31+15),
            (2076, 31+5), (2077, 24), (2078, 31+12), (2079, 31+2),
            (2080, 22), (2081, 31+9), (2082, 29), (2083, 31+17),
            (2084, 31+6), (2085, 26), (2086, 31+14), (2087, 31+3),
            (2088, 24), (2089, 31+10), (2090, 30), (2091, 31+18),
            (2092, 31+7), (2093, 27), (2094, 31+15), (2095, 31+5),
            (2096, 25), (2097, 31+12), (2098, 31+1), (2099, 21),
            (2100, 31+9),
        ] {
            let base_date = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
            map.insert(year, base_date + Duration::days((day - 1) as i64));
        }
        map
    }
}

impl Default for HolidayDates {
    fn default() -> Self {
        Self::new()
    }
}