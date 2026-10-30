//! Turning numbers and times into display text.

use chrono::{Local, TimeZone};

use crate::i18n::Language;

/// A price with the symbol's number of decimals, e.g. `76913.70`.
pub fn price(value: f64, decimals: u32) -> String {
    format!("{value:.*}", decimals as usize)
}

/// A signed percentage, e.g. `+1.22%`.
pub fn percent(value: f64) -> String {
    format!("{value:+.2}%")
}

/// A whole number with thousands separators: `52.000` in Turkish, `52,000` in English.
pub fn count(value: u64, language: Language) -> String {
    let separator = match language {
        Language::Turkish => '.',
        Language::English => ',',
    };
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push(separator);
        }
        grouped.push(digit);
    }
    grouped
}

/// Milliseconds as signed seconds with one decimal, e.g. `+0,3` in Turkish.
pub fn seconds(ms: i64, language: Language) -> String {
    let text = format!("{:+.1}", ms as f64 / 1000.0);
    match language {
        Language::Turkish => text.replace('.', ","),
        Language::English => text,
    }
}

/// A timestamp in the computer's time zone, e.g. `2026-10-30 22:00`.
pub fn date_time(ms: i64) -> String {
    Local
        .timestamp_millis_opt(ms)
        .single()
        .map(|time| time.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default()
}

/// Time left as `05:09` or `03:12:05`, with days in front when needed (`2g 03:12:05`).
pub fn countdown(ms: i64, language: Language) -> String {
    let total = ms.max(0) / 1000;
    let (days, hours, minutes, seconds) = (
        total / 86_400,
        total % 86_400 / 3_600,
        total % 3_600 / 60,
        total % 60,
    );
    let day = match language {
        Language::Turkish => "g",
        Language::English => "d",
    };
    if days > 0 {
        format!("{days}{day} {hours:02}:{minutes:02}:{seconds:02}")
    } else if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_are_grouped_per_language() {
        assert_eq!(count(0, Language::Turkish), "0");
        assert_eq!(count(999, Language::Turkish), "999");
        assert_eq!(count(52_000, Language::Turkish), "52.000");
        assert_eq!(count(1_234_567, Language::English), "1,234,567");
    }

    #[test]
    fn prices_and_percentages() {
        assert_eq!(price(76913.7, 2), "76913.70");
        assert_eq!(price(0.0291234, 5), "0.02912");
        assert_eq!(price(5.0, 0), "5");
        assert_eq!(percent(1.2166), "+1.22%");
        assert_eq!(percent(-4.098), "-4.10%");
    }

    #[test]
    fn seconds_use_the_decimal_separator() {
        assert_eq!(seconds(312, Language::Turkish), "+0,3");
        assert_eq!(seconds(-1_250, Language::English), "-1.2");
    }

    #[test]
    fn countdowns() {
        assert_eq!(countdown(309_000, Language::Turkish), "05:09");
        assert_eq!(countdown(11_525_000, Language::English), "03:12:05");
        assert_eq!(countdown(184_325_000, Language::Turkish), "2g 03:12:05");
        assert_eq!(countdown(-5, Language::English), "00:00");
    }
}
