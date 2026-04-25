use chrono::{DateTime, Datelike, Utc};

/// Format an ISO-8601 timestamp as a short human-readable date.
/// Returns the raw input on parse failure so we never swallow bad data silently.
#[must_use]
pub fn format_short_date(iso: &str) -> String {
    match iso.parse::<DateTime<Utc>>() {
        Ok(dt) => format!("{} {}, {}", month_short(dt.month()), dt.day(), dt.year()),
        Err(_) => iso.to_owned(),
    }
}

#[must_use]
pub fn format_iso_date(iso: &str) -> String {
    match iso.parse::<DateTime<Utc>>() {
        Ok(dt) => dt.format("%Y-%m-%d").to_string(),
        Err(_) => iso.to_owned(),
    }
}

#[must_use]
pub fn reading_time_label(minutes: u32) -> String {
    if minutes == 0 {
        "quick read".into()
    } else if minutes == 1 {
        "1 min read".into()
    } else {
        format!("{minutes} min read")
    }
}

fn month_short(m: u32) -> &'static str {
    match m {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_date_parses_iso() {
        assert_eq!(format_short_date("2026-04-23T12:00:00Z"), "Apr 23, 2026");
    }

    #[test]
    fn short_date_passthrough_on_bad_input() {
        assert_eq!(format_short_date("not-a-date"), "not-a-date");
    }

    #[test]
    fn reading_time_buckets() {
        assert_eq!(reading_time_label(0), "quick read");
        assert_eq!(reading_time_label(1), "1 min read");
        assert_eq!(reading_time_label(7), "7 min read");
    }
}
