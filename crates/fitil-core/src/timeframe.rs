use std::fmt;
use std::str::FromStr;

const SECOND: i64 = 1_000;
const MINUTE: i64 = 60 * SECOND;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const WEEK: i64 = 7 * DAY;

/// Weekly candles start on Monday 00:00 UTC; 1970-01-05 was the first Monday after the epoch.
const FIRST_MONDAY: i64 = 4 * DAY;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TimeUnit {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
}

impl TimeUnit {
    pub const ALL: [TimeUnit; 6] = [
        TimeUnit::Second,
        TimeUnit::Minute,
        TimeUnit::Hour,
        TimeUnit::Day,
        TimeUnit::Week,
        TimeUnit::Month,
    ];

    /// Stable identifier, used in saved drawings and translation keys.
    pub fn id(self) -> &'static str {
        match self {
            TimeUnit::Second => "second",
            TimeUnit::Minute => "minute",
            TimeUnit::Hour => "hour",
            TimeUnit::Day => "day",
            TimeUnit::Week => "week",
            TimeUnit::Month => "month",
        }
    }

    fn suffix(self) -> char {
        match self {
            TimeUnit::Second => 's',
            TimeUnit::Minute => 'm',
            TimeUnit::Hour => 'h',
            TimeUnit::Day => 'd',
            TimeUnit::Week => 'w',
            TimeUnit::Month => 'M',
        }
    }

    /// Length in milliseconds, or `None` for months, whose length varies.
    fn millis(self) -> Option<i64> {
        match self {
            TimeUnit::Second => Some(SECOND),
            TimeUnit::Minute => Some(MINUTE),
            TimeUnit::Hour => Some(HOUR),
            TimeUnit::Day => Some(DAY),
            TimeUnit::Week => Some(WEEK),
            TimeUnit::Month => None,
        }
    }
}

/// A candle period such as `15m`, `4h` or `1M`.
///
/// Any count is allowed, so custom periods like `45m` or `3h` can be built
/// from the periods an exchange provides (see [`Timeframe::best_source`]).
///
/// Buckets are aligned like this:
/// - periods that divide a day (`45m`, `4h`) start at multiples of the period since 00:00 UTC;
/// - shorter periods that do not divide a day (`7m`, `5h`) restart every day at 00:00 UTC,
///   so the day's last candle is shorter;
/// - longer fixed periods (`2d`) are counted from the Unix epoch;
/// - weeks start on Monday, months on the 1st, both at 00:00 UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Timeframe {
    count: u32,
    unit: TimeUnit,
}

impl Timeframe {
    /// `None` if `count` is zero.
    pub const fn new(count: u32, unit: TimeUnit) -> Option<Self> {
        if count == 0 {
            None
        } else {
            Some(Self { count, unit })
        }
    }

    pub fn count(self) -> u32 {
        self.count
    }

    pub fn unit(self) -> TimeUnit {
        self.unit
    }

    /// Fixed length in milliseconds, or `None` for months.
    pub fn duration_ms(self) -> Option<i64> {
        self.unit.millis().map(|unit| unit * i64::from(self.count))
    }

    /// Length used only for ordering timeframes; a month counts as 30 days.
    fn approx_ms(self) -> i64 {
        self.duration_ms()
            .unwrap_or(30 * DAY * i64::from(self.count))
    }

    /// Start of the candle that contains `time`.
    pub fn bucket_start(self, time: i64) -> i64 {
        match self.unit {
            TimeUnit::Month => {
                let (year, month) = year_month(time);
                let index = year * 12 + (month - 1);
                let start = index.div_euclid(i64::from(self.count)) * i64::from(self.count);
                month_start(start.div_euclid(12), start.rem_euclid(12) + 1)
            }
            TimeUnit::Week => {
                let length = WEEK * i64::from(self.count);
                FIRST_MONDAY + (time - FIRST_MONDAY).div_euclid(length) * length
            }
            _ => {
                let length = self.fixed_ms();
                if length < DAY && DAY % length != 0 {
                    let day = time.div_euclid(DAY) * DAY;
                    day + (time - day) / length * length
                } else {
                    time.div_euclid(length) * length
                }
            }
        }
    }

    /// Start of the candle after the one that contains `time`.
    pub fn next_bucket(self, time: i64) -> i64 {
        let start = self.bucket_start(time);
        match self.unit {
            TimeUnit::Month => {
                let (year, month) = year_month(start);
                let index = year * 12 + (month - 1) + i64::from(self.count);
                month_start(index.div_euclid(12), index.rem_euclid(12) + 1)
            }
            TimeUnit::Week => start + WEEK * i64::from(self.count),
            _ => {
                let length = self.fixed_ms();
                let next = start + length;
                if length < DAY && DAY % length != 0 {
                    next.min(start.div_euclid(DAY) * DAY + DAY)
                } else {
                    next
                }
            }
        }
    }

    /// Whether candles of this timeframe can be built by merging candles of `base`.
    ///
    /// Only bases that fit evenly into a day are used for fixed periods, weeks and
    /// months, because exchanges align those to 00:00 UTC; longer exchange periods
    /// (Binance's `3d`, for example) follow their own alignment.
    pub fn is_multiple_of(self, base: Timeframe) -> bool {
        if self == base {
            return true;
        }
        let base_fits_day = base.duration_ms().is_some_and(|b| b <= DAY && DAY % b == 0);
        match self.unit {
            TimeUnit::Month => {
                (base.unit == TimeUnit::Month && self.count.is_multiple_of(base.count))
                    || base_fits_day
            }
            TimeUnit::Week => {
                (base.unit == TimeUnit::Week && self.count.is_multiple_of(base.count))
                    || base_fits_day
            }
            _ => base_fits_day && self.fixed_ms() % base.fixed_ms() == 0,
        }
    }

    /// The longest timeframe in `available` that this one can be built from.
    pub fn best_source(self, available: &[Timeframe]) -> Option<Timeframe> {
        available
            .iter()
            .copied()
            .filter(|&base| self.is_multiple_of(base))
            .max_by_key(|base| base.approx_ms())
    }

    fn fixed_ms(self) -> i64 {
        self.duration_ms()
            .expect("only called for fixed-length timeframes")
    }
}

impl fmt::Display for Timeframe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.count, self.unit.suffix())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseTimeframeError(String);

impl fmt::Display for ParseTimeframeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid timeframe {:?}", self.0)
    }
}

impl std::error::Error for ParseTimeframeError {}

impl FromStr for Timeframe {
    type Err = ParseTimeframeError;

    /// Parses `45m`, `4h`, `1D`, `1W` or `1M`. Lowercase `m` is minutes, uppercase `M` is months.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let error = || ParseTimeframeError(s.to_owned());
        let s = s.trim();
        let suffix = s.chars().last().ok_or_else(error)?;
        let unit = match suffix {
            's' | 'S' => TimeUnit::Second,
            'm' => TimeUnit::Minute,
            'h' | 'H' => TimeUnit::Hour,
            'd' | 'D' => TimeUnit::Day,
            'w' | 'W' => TimeUnit::Week,
            'M' => TimeUnit::Month,
            _ => return Err(error()),
        };
        let count = s[..s.len() - 1].parse().map_err(|_| error())?;
        Timeframe::new(count, unit).ok_or_else(error)
    }
}

/// Calendar year and month (1–12) of a timestamp, in UTC.
fn year_month(time: i64) -> (i64, i64) {
    // Howard Hinnant's civil_from_days algorithm.
    let z = time.div_euclid(DAY) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month)
}

/// Timestamp of 00:00 UTC on the 1st of the given month.
fn month_start(year: i64, month: i64) -> i64 {
    // Howard Hinnant's days_from_civil algorithm.
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146_097 + doe - 719_468) * DAY
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tf(s: &str) -> Timeframe {
        s.parse().unwrap()
    }

    /// Unix milliseconds for a UTC date and time.
    fn at(year: i64, month: i64, day: i64, hour: i64, minute: i64) -> i64 {
        month_start(year, month) + (day - 1) * DAY + hour * HOUR + minute * MINUTE
    }

    #[test]
    fn parses_and_displays() {
        for s in ["1s", "45m", "4h", "1d", "1w", "1M", "3M"] {
            assert_eq!(tf(s).to_string(), s);
        }
        assert_eq!(tf("1D"), tf("1d"));
        assert_eq!(tf("1W"), tf("1w"));
        assert_ne!(tf("1m"), tf("1M"));
        for bad in ["", "m", "0m", "15x", "-1h", "1.5h"] {
            assert!(
                bad.parse::<Timeframe>().is_err(),
                "{bad:?} should not parse"
            );
        }
    }

    #[test]
    fn calendar_helpers_round_trip() {
        assert_eq!(month_start(1970, 1), 0);
        assert_eq!(month_start(2024, 3), 1_709_251_200_000);
        for (year, month) in [(1970, 1), (2000, 2), (2024, 2), (2024, 12), (2026, 10)] {
            assert_eq!(year_month(month_start(year, month)), (year, month));
            assert_eq!(
                year_month(month_start(year, month) - 1).1,
                (month + 10) % 12 + 1
            );
        }
    }

    #[test]
    fn intraday_buckets_divide_the_day() {
        let t = at(2026, 10, 30, 13, 37);
        assert_eq!(tf("15m").bucket_start(t), at(2026, 10, 30, 13, 30));
        // 13:37 is minute 817 of the day; 45m candles start at minute 810 (13:30).
        assert_eq!(tf("45m").bucket_start(t), at(2026, 10, 30, 13, 30));
        assert_eq!(tf("4h").bucket_start(t), at(2026, 10, 30, 12, 0));
        assert_eq!(tf("45m").next_bucket(t), at(2026, 10, 30, 14, 15));
    }

    #[test]
    fn uneven_intraday_buckets_restart_each_day() {
        // 7m does not divide 1440 minutes: 205 full candles, then a 5-minute one.
        let last = at(2026, 10, 30, 23, 55);
        assert_eq!(tf("7m").bucket_start(at(2026, 10, 30, 23, 58)), last);
        assert_eq!(tf("7m").next_bucket(last), at(2026, 10, 31, 0, 0));
        assert_eq!(
            tf("7m").bucket_start(at(2026, 10, 31, 0, 3)),
            at(2026, 10, 31, 0, 0)
        );
        assert_eq!(
            tf("5h").bucket_start(at(2026, 10, 31, 1, 0)),
            at(2026, 10, 31, 0, 0)
        );
    }

    #[test]
    fn weeks_start_on_monday() {
        // 2026-10-30 is a Friday; that week began on Monday 2026-10-26.
        let t = at(2026, 10, 30, 13, 37);
        assert_eq!(tf("1w").bucket_start(t), at(2026, 10, 26, 0, 0));
        assert_eq!(tf("1w").next_bucket(t), at(2026, 11, 2, 0, 0));
    }

    #[test]
    fn months_start_on_the_first() {
        let t = at(2026, 10, 30, 13, 37);
        assert_eq!(tf("1M").bucket_start(t), at(2026, 10, 1, 0, 0));
        assert_eq!(tf("1M").next_bucket(t), at(2026, 11, 1, 0, 0));
        assert_eq!(tf("3M").bucket_start(t), at(2026, 10, 1, 0, 0));
        assert_eq!(
            tf("3M").bucket_start(at(2026, 9, 30, 0, 0)),
            at(2026, 7, 1, 0, 0)
        );
        assert_eq!(tf("3M").next_bucket(t), at(2027, 1, 1, 0, 0));
    }

    #[test]
    fn picks_the_longest_usable_source() {
        let binance: Vec<Timeframe> = [
            "1m", "3m", "5m", "15m", "30m", "1h", "2h", "4h", "6h", "8h", "12h", "1d", "3d", "1w",
            "1M",
        ]
        .map(tf)
        .to_vec();
        let source = |s: &str| tf(s).best_source(&binance).map(|t| t.to_string());

        assert_eq!(source("4h").as_deref(), Some("4h"));
        assert_eq!(source("45m").as_deref(), Some("15m"));
        assert_eq!(source("7m").as_deref(), Some("1m"));
        assert_eq!(source("3h").as_deref(), Some("1h"));
        assert_eq!(source("10h").as_deref(), Some("2h"));
        assert_eq!(source("2d").as_deref(), Some("1d"));
        assert_eq!(source("6d").as_deref(), Some("1d"));
        assert_eq!(source("2w").as_deref(), Some("1w"));
        assert_eq!(source("3M").as_deref(), Some("1M"));
        assert_eq!(source("30s"), None);
    }
}
