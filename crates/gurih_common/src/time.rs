use chrono::{Datelike, NaiveDate, Utc};

pub fn check_min_years(date_str: &str, min_years: u32) -> bool {
    if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        let now = Utc::now().date_naive();
        let years = now.year() - date.year();
        let mut diff = years;
        if now.month() < date.month() || (now.month() == date.month() && now.day() < date.day()) {
            diff -= 1;
        }
        diff >= min_years as i32
    } else {
        false
    }
}
