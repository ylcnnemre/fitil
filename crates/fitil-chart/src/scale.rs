//! Price scales, axis ticks and label text.

use chrono::{Datelike, Local, TimeZone, Timelike};
use fitil_core::{Candle, Timeframe};

use crate::{ChartLabels, PriceScale};

/// Converts prices to values on the chosen price scale and back.
#[derive(Debug, Clone, Copy)]
pub struct Mapper {
    pub scale: PriceScale,
    /// The price shown as 0% on the percent scale: the first visible close.
    pub base: f64,
}

impl Mapper {
    pub fn to_scale(self, price: f64) -> f64 {
        match self.scale {
            PriceScale::Linear => price,
            PriceScale::Logarithmic => price.max(f64::MIN_POSITIVE).ln(),
            PriceScale::Percent => (price / self.base - 1.0) * 100.0,
        }
    }

    pub fn to_price(self, value: f64) -> f64 {
        match self.scale {
            PriceScale::Linear => value,
            PriceScale::Logarithmic => value.exp(),
            PriceScale::Percent => self.base * (1.0 + value / 100.0),
        }
    }

    /// Round values between `low` and `high` (scale units) for grid lines, about `count`.
    pub fn ticks(self, low: f64, high: f64, count: usize) -> Vec<f64> {
        match self.scale {
            PriceScale::Linear | PriceScale::Percent => linear_ticks(low, high, count),
            PriceScale::Logarithmic => {
                let (low, high) = (low.exp(), high.exp());
                let prices = if high / low > 10.0 {
                    decade_ticks(low, high)
                } else {
                    linear_ticks(low, high, count)
                };
                prices.into_iter().map(f64::ln).collect()
            }
        }
    }
}

/// A round step close to `raw`: 1, 2, 2.5 or 5 times a power of ten.
pub fn nice_step(raw: f64) -> f64 {
    let magnitude = 10f64.powf(raw.log10().floor());
    let nice = match raw / magnitude {
        f if f <= 1.0 => 1.0,
        f if f <= 2.0 => 2.0,
        f if f <= 2.5 => 2.5,
        f if f <= 5.0 => 5.0,
        _ => 10.0,
    };
    nice * magnitude
}

pub fn linear_ticks(low: f64, high: f64, count: usize) -> Vec<f64> {
    // Also rejects NaN, which compares false with everything.
    let ordered = high > low;
    if !ordered || count == 0 || !low.is_finite() || !high.is_finite() {
        return Vec::new();
    }
    let step = nice_step((high - low) / count as f64);
    let first = (low / step).ceil() as i64;
    let last = (high / step).floor() as i64;
    (first..=last).map(|i| i as f64 * step).collect()
}

/// 1, 2 and 5 times each power of ten between `low` and `high`.
fn decade_ticks(low: f64, high: f64) -> Vec<f64> {
    let mut ticks = Vec::new();
    let mut magnitude = 10f64.powf(low.log10().floor());
    while magnitude <= high {
        for factor in [1.0, 2.0, 5.0] {
            let value = magnitude * factor;
            if value >= low && value <= high {
                ticks.push(value);
            }
        }
        magnitude *= 10.0;
    }
    ticks
}

/// A price with the symbol's decimals.
pub fn price(value: f64, decimals: u32) -> String {
    format!("{value:.*}", decimals as usize)
}

/// A volume shortened with the label's magnitudes, e.g. `1.23K`.
pub fn volume(value: f64, labels: &ChartLabels) -> String {
    let [thousand, million, billion] = &labels.magnitudes;
    match value.abs() {
        v if v >= 1e9 => format!("{:.2}{billion}", value / 1e9),
        v if v >= 1e6 => format!("{:.2}{million}", value / 1e6),
        v if v >= 1e3 => format!("{:.2}{thousand}", value / 1e3),
        v if v >= 1.0 => format!("{value:.2}"),
        _ => format!("{value:.4}"),
    }
}

/// Time left, as `05:09`, `03:12:05` or with days in front (`2d 03:12:05`).
pub fn countdown(ms: i64, labels: &ChartLabels) -> String {
    let total = ms.max(0) / 1000;
    let (days, hours, minutes, seconds) = (
        total / 86_400,
        total % 86_400 / 3_600,
        total % 3_600 / 60,
        total % 60,
    );
    if days > 0 {
        format!(
            "{days}{} {hours:02}:{minutes:02}:{seconds:02}",
            labels.day_suffix
        )
    } else if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

/// A local date and time for the crosshair, e.g. `31 Eki 2026 14:00`.
pub fn crosshair_time(time: i64, timeframe: Timeframe, labels: &ChartLabels) -> String {
    let Some(local) = LocalTime::at(time) else {
        return String::new();
    };
    let date = format!(
        "{} {} {}",
        local.day,
        labels.months[local.month as usize - 1],
        local.year
    );
    if timeframe.duration_ms().is_some_and(|ms| ms < 86_400_000) {
        format!(
            "{date} {:02}:{:02}",
            local.minute_of_day / 60,
            local.minute_of_day % 60
        )
    } else {
        date
    }
}

/// A label under the time axis.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeTick {
    pub index: usize,
    pub label: String,
    /// Starts a new month or year.
    pub major: bool,
}

/// How far apart time labels are, from fine to coarse.
#[derive(Debug, Clone, Copy)]
enum Step {
    Minutes(u32),
    Days(u32),
    Months(u32),
    Years(i32),
}

const STEPS: [Step; 25] = [
    Step::Minutes(1),
    Step::Minutes(2),
    Step::Minutes(5),
    Step::Minutes(10),
    Step::Minutes(15),
    Step::Minutes(30),
    Step::Minutes(60),
    Step::Minutes(120),
    Step::Minutes(180),
    Step::Minutes(240),
    Step::Minutes(360),
    Step::Minutes(720),
    Step::Days(1),
    Step::Days(2),
    Step::Days(5),
    Step::Days(10),
    Step::Days(15),
    Step::Months(1),
    Step::Months(2),
    Step::Months(3),
    Step::Months(6),
    Step::Years(1),
    Step::Years(2),
    Step::Years(5),
    Step::Years(10),
];

/// Pixels needed between two time labels.
const LABEL_GAP: f64 = 90.0;

impl Step {
    fn approx_ms(self) -> f64 {
        const DAY: f64 = 86_400_000.0;
        match self {
            Step::Minutes(n) => f64::from(n) * 60_000.0,
            Step::Days(n) => f64::from(n) * DAY,
            Step::Months(n) => f64::from(n) * 30.0 * DAY,
            Step::Years(n) => f64::from(n) * 365.0 * DAY,
        }
    }
}

/// The calendar fields of a timestamp in the computer's time zone.
#[derive(Debug, Clone, Copy)]
struct LocalTime {
    year: i32,
    month: u32,
    day: u32,
    minute_of_day: u32,
}

impl LocalTime {
    fn at(time: i64) -> Option<Self> {
        let local = Local.timestamp_millis_opt(time).single()?;
        Some(Self {
            year: local.year(),
            month: local.month(),
            day: local.day(),
            minute_of_day: local.hour() * 60 + local.minute(),
        })
    }
}

/// Labels for the bars `first..=last` that start a new period, spaced to stay readable.
pub fn time_ticks(
    candles: &[Candle],
    first: usize,
    last: usize,
    spacing: f64,
    timeframe: Timeframe,
    labels: &ChartLabels,
) -> Vec<TimeTick> {
    let bar_ms = timeframe
        .duration_ms()
        .map_or(30.0 * 86_400_000.0, |ms| ms as f64);
    let step = STEPS
        .into_iter()
        .find(|step| step.approx_ms() >= bar_ms && step.approx_ms() / bar_ms * spacing >= LABEL_GAP)
        .unwrap_or(Step::Years(10));

    let mut ticks: Vec<TimeTick> = Vec::new();
    let mut previous = first
        .checked_sub(1)
        .and_then(|i| candles.get(i))
        .and_then(|candle| LocalTime::at(candle.open_time));
    for (index, candle) in candles.iter().enumerate().take(last + 1).skip(first) {
        let Some(current) = LocalTime::at(candle.open_time) else {
            continue;
        };
        if let Some(previous) = previous
            && let Some((label, major)) = boundary(previous, current, step, labels)
        {
            let too_close = ticks
                .last()
                .is_some_and(|tick| (index - tick.index) as f64 * spacing < LABEL_GAP * 0.6);
            if !too_close {
                ticks.push(TimeTick {
                    index,
                    label,
                    major,
                });
            } else if major && let Some(last) = ticks.last_mut() {
                // A month or year beats a nearby day or hour.
                *last = TimeTick {
                    index,
                    label,
                    major,
                };
            }
        }
        previous = Some(current);
    }
    ticks
}

/// The label for `current` if it starts a new `step` period after `previous`.
fn boundary(
    previous: LocalTime,
    current: LocalTime,
    step: Step,
    labels: &ChartLabels,
) -> Option<(String, bool)> {
    let new_year = current.year != previous.year;
    let new_month = new_year || current.month != previous.month;
    let new_day = new_month || current.day != previous.day;
    let crossed = match step {
        Step::Minutes(n) => new_day || current.minute_of_day / n != previous.minute_of_day / n,
        Step::Days(n) => new_month || (current.day - 1) / n != (previous.day - 1) / n,
        Step::Months(n) => new_year || (current.month - 1) / n != (previous.month - 1) / n,
        Step::Years(n) => current.year.div_euclid(n) != previous.year.div_euclid(n),
    };
    if !crossed {
        return None;
    }
    Some(if new_year {
        (current.year.to_string(), true)
    } else if new_month {
        (labels.months[current.month as usize - 1].clone(), true)
    } else if new_day {
        (current.day.to_string(), false)
    } else {
        (
            format!(
                "{:02}:{:02}",
                current.minute_of_day / 60,
                current.minute_of_day % 60
            ),
            false,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn labels() -> ChartLabels {
        ChartLabels {
            open: "O".into(),
            high: "H".into(),
            low: "L".into(),
            close: "C".into(),
            volume: "Vol".into(),
            months: [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ]
            .map(String::from),
            magnitudes: ["K", "M", "B"].map(String::from),
            day_suffix: "d".into(),
            hour_suffix: "h".into(),
            minute_suffix: "m".into(),
            loading: "Loading…".into(),
            note: "Note".into(),
            bars: "bars".into(),
        }
    }

    fn local_ms(year: i32, month: u32, day: u32, hour: u32) -> i64 {
        Local
            .with_ymd_and_hms(year, month, day, hour, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis()
    }

    fn hourly(from: i64, count: usize) -> Vec<Candle> {
        (0..count)
            .map(|i| Candle {
                open_time: from + i as i64 * 3_600_000,
                open: 1.0,
                high: 1.0,
                low: 1.0,
                close: 1.0,
                volume: 1.0,
                quote_volume: Some(1.0),
                trades: Some(1),
                taker_buy_volume: Some(0.5),
            })
            .collect()
    }

    #[test]
    fn steps_are_round() {
        assert_eq!(nice_step(0.37), 0.5);
        assert_eq!(nice_step(130.0), 200.0);
        assert_eq!(nice_step(2.2), 2.5);
        assert_eq!(nice_step(7.0), 10.0);
        assert_eq!(
            linear_ticks(76_310.0, 77_240.0, 5),
            [76_400.0, 76_600.0, 76_800.0, 77_000.0, 77_200.0]
        );
        assert!(linear_ticks(5.0, 5.0, 5).is_empty());
    }

    #[test]
    fn scales_round_trip() {
        for scale in [
            PriceScale::Linear,
            PriceScale::Logarithmic,
            PriceScale::Percent,
        ] {
            let mapper = Mapper { scale, base: 200.0 };
            for price in [0.5, 200.0, 77_000.0] {
                let back = mapper.to_price(mapper.to_scale(price));
                assert!((back - price).abs() < 1e-9 * price, "{scale:?} {price}");
            }
        }
        let percent = Mapper {
            scale: PriceScale::Percent,
            base: 200.0,
        };
        assert_eq!(percent.to_scale(250.0), 25.0);
    }

    #[test]
    fn log_ticks_span_decades() {
        let mapper = Mapper {
            scale: PriceScale::Logarithmic,
            base: 1.0,
        };
        let ticks: Vec<f64> = mapper
            .ticks(3_000f64.ln(), 120_000f64.ln(), 6)
            .into_iter()
            .map(|t| t.exp().round())
            .collect();
        assert_eq!(ticks, [5_000.0, 10_000.0, 20_000.0, 50_000.0, 100_000.0]);
    }

    #[test]
    fn short_numbers() {
        let labels = labels();
        assert_eq!(volume(1_234.0, &labels), "1.23K");
        assert_eq!(volume(5_600_000.0, &labels), "5.60M");
        assert_eq!(volume(2.5e9, &labels), "2.50B");
        assert_eq!(volume(12.346, &labels), "12.35");
        assert_eq!(volume(0.12346, &labels), "0.1235");
        assert_eq!(countdown(309_000, &labels), "05:09");
        assert_eq!(countdown(184_325_000, &labels), "2d 03:12:05");
        assert_eq!(price(76_913.7, 2), "76913.70");
    }

    #[test]
    fn hours_get_times_and_new_days_get_dates() {
        let candles = hourly(local_ms(2026, 10, 30, 20), 10);
        let ticks = time_ticks(&candles, 0, 9, 40.0, "1h".parse().unwrap(), &labels());
        let shown: Vec<&str> = ticks.iter().map(|t| t.label.as_str()).collect();
        // 40 px per bar: a label every 3 hours; midnight shows the day instead.
        assert_eq!(shown, ["21:00", "31", "03:00"]);
    }

    #[test]
    fn a_new_month_is_a_major_label() {
        let candles = hourly(local_ms(2026, 10, 31, 22), 4);
        let ticks = time_ticks(&candles, 0, 3, 40.0, "1h".parse().unwrap(), &labels());
        assert_eq!(
            ticks,
            [TimeTick {
                index: 2,
                label: "Nov".into(),
                major: true
            }]
        );
    }
}
