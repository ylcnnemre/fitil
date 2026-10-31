//! Trend indicators. They follow TradingView's definitions, so their values match what
//! traders already know.

use fitil_core::Candle;

use crate::Indicator;
use crate::primitives::{Atr, Change, Ema, Extremes, TrueRange};

/// SuperTrend: a line below the price while it rises and above it while it falls,
/// `factor` ATRs away from the candle's midpoint. Outputs: rising line, falling line
/// (each NaN while the other is shown).
#[derive(Debug, Clone)]
pub struct SuperTrend {
    factor: f64,
    atr: Atr,
    previous: Option<SuperTrendState>,
}

#[derive(Debug, Clone, Copy)]
struct SuperTrendState {
    lower: f64,
    upper: f64,
    close: f64,
    rising: bool,
}

impl SuperTrend {
    pub fn new(atr_length: usize, factor: f64) -> Self {
        Self {
            factor,
            atr: Atr::new(atr_length),
            previous: None,
        }
    }
}

impl Indicator for SuperTrend {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        let atr = self.atr.next(candle);
        if atr.is_nan() {
            out[0] = f64::NAN;
            out[1] = f64::NAN;
            return;
        }
        let middle = (candle.high + candle.low) / 2.0;
        let mut lower = middle - self.factor * atr;
        let mut upper = middle + self.factor * atr;
        let rising = match self.previous {
            // TradingView starts in a falling trend.
            None => false,
            Some(previous) => {
                // The bands only move towards the price while the price stays inside them.
                if !(lower > previous.lower || previous.close < previous.lower) {
                    lower = previous.lower;
                }
                if !(upper < previous.upper || previous.close > previous.upper) {
                    upper = previous.upper;
                }
                if previous.rising {
                    candle.close >= lower
                } else {
                    candle.close > upper
                }
            }
        };
        self.previous = Some(SuperTrendState {
            lower,
            upper,
            close: candle.close,
            rising,
        });
        out[0] = if rising { lower } else { f64::NAN };
        out[1] = if rising { f64::NAN } else { upper };
    }
}

/// Parabolic SAR ("stop and reverse"), as TradingView computes it.
#[derive(Debug, Clone)]
pub struct ParabolicSar {
    start: f64,
    increment: f64,
    maximum: f64,
    /// The previous two candles, newest first.
    previous: [Option<Candle>; 2],
    state: Option<SarState>,
}

#[derive(Debug, Clone, Copy)]
struct SarState {
    result: f64,
    /// The extreme price of the current trend.
    extreme: f64,
    acceleration: f64,
    rising: bool,
}

impl ParabolicSar {
    pub fn new(start: f64, increment: f64, maximum: f64) -> Self {
        Self {
            start,
            increment,
            maximum,
            previous: [None, None],
            state: None,
        }
    }
}

impl Indicator for ParabolicSar {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        let [Some(prior), before_prior] = self.previous else {
            self.previous = [Some(*candle), None];
            out[0] = f64::NAN;
            return;
        };

        let mut first_trend_bar = false;
        let mut state = match self.state {
            Some(state) => state,
            None => {
                // The second candle sets the first trend.
                first_trend_bar = true;
                if candle.close > prior.close {
                    SarState {
                        result: prior.low,
                        extreme: candle.high,
                        acceleration: self.start,
                        rising: true,
                    }
                } else {
                    SarState {
                        result: prior.high,
                        extreme: candle.low,
                        acceleration: self.start,
                        rising: false,
                    }
                }
            }
        };

        state.result += state.acceleration * (state.extreme - state.result);
        if state.rising {
            if state.result > candle.low {
                first_trend_bar = true;
                state.rising = false;
                state.result = candle.high.max(state.extreme);
                state.extreme = candle.low;
                state.acceleration = self.start;
            }
        } else if state.result < candle.high {
            first_trend_bar = true;
            state.rising = true;
            state.result = candle.low.min(state.extreme);
            state.extreme = candle.high;
            state.acceleration = self.start;
        }

        if !first_trend_bar {
            let new_extreme = if state.rising {
                candle.high > state.extreme
            } else {
                candle.low < state.extreme
            };
            if new_extreme {
                state.extreme = if state.rising {
                    candle.high
                } else {
                    candle.low
                };
                state.acceleration = (state.acceleration + self.increment).min(self.maximum);
            }
        }

        // The SAR never goes beyond the previous two candles' range.
        if state.rising {
            state.result = state.result.min(prior.low);
            if let Some(before) = before_prior {
                state.result = state.result.min(before.low);
            }
        } else {
            state.result = state.result.max(prior.high);
            if let Some(before) = before_prior {
                state.result = state.result.max(before.high);
            }
        }

        out[0] = state.result;
        self.state = Some(state);
        self.previous = [Some(*candle), Some(prior)];
    }
}

/// Directional Movement Index. Outputs: ADX, +DI, −DI.
#[derive(Debug, Clone)]
pub struct Dmi {
    high: Change,
    low: Change,
    range: TrueRange,
    first: bool,
    true_range: Ema,
    plus: Ema,
    minus: Ema,
    adx: Ema,
    last_plus: f64,
    last_minus: f64,
}

impl Dmi {
    pub fn new(di_length: usize, adx_smoothing: usize) -> Self {
        Self {
            high: Change::default(),
            low: Change::default(),
            range: TrueRange::default(),
            first: true,
            true_range: Ema::wilder(di_length),
            plus: Ema::wilder(di_length),
            minus: Ema::wilder(di_length),
            adx: Ema::wilder(adx_smoothing),
            last_plus: f64::NAN,
            last_minus: f64::NAN,
        }
    }
}

impl Indicator for Dmi {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        let up = self.high.next(candle.high);
        let down = -self.low.next(candle.low);
        let range = self.range.next(candle);
        // Like TradingView's `ta.tr`, the first candle has no true range.
        let range = if std::mem::replace(&mut self.first, false) {
            f64::NAN
        } else {
            range
        };
        let (plus_move, minus_move) = if up.is_nan() || down.is_nan() {
            (f64::NAN, f64::NAN)
        } else {
            (
                if up > down && up > 0.0 { up } else { 0.0 },
                if down > up && down > 0.0 { down } else { 0.0 },
            )
        };

        let average_range = self.true_range.next(range);
        let plus = 100.0 * self.plus.next(plus_move) / average_range;
        let minus = 100.0 * self.minus.next(minus_move) / average_range;
        // A flat market (no range at all) keeps the last values.
        let plus = if plus.is_finite() {
            plus
        } else {
            self.last_plus
        };
        let minus = if minus.is_finite() {
            minus
        } else {
            self.last_minus
        };
        self.last_plus = plus;
        self.last_minus = minus;

        let sum = plus + minus;
        let spread = (plus - minus).abs() / if sum == 0.0 { 1.0 } else { sum };
        out[0] = 100.0 * self.adx.next(spread);
        out[1] = plus;
        out[2] = minus;
    }
}

/// Ichimoku Cloud. Outputs: conversion line, base line, lagging span, leading span A,
/// leading span B. The spans are drawn shifted in time (see the catalog's offsets).
#[derive(Debug, Clone)]
pub struct Ichimoku {
    conversion: Extremes,
    base: Extremes,
    span_b: Extremes,
}

impl Ichimoku {
    pub fn new(conversion: usize, base: usize, span_b: usize) -> Self {
        Self {
            conversion: Extremes::new(conversion),
            base: Extremes::new(base),
            span_b: Extremes::new(span_b),
        }
    }
}

impl Indicator for Ichimoku {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        let middle = |(high, low): (f64, f64)| (high + low) / 2.0;
        let conversion = middle(self.conversion.next(candle.high, candle.low));
        let base = middle(self.base.next(candle.high, candle.low));
        out[0] = conversion;
        out[1] = base;
        out[2] = candle.close;
        out[3] = (conversion + base) / 2.0;
        out[4] = middle(self.span_b.next(candle.high, candle.low));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::testing::{candle, random_walk, run};

    #[test]
    fn supertrend_stays_on_the_right_side_of_the_price() {
        let candles = random_walk(400, 5);
        let values = run(SuperTrend::new(10, 3.0), 2, &candles);
        let mut flips = 0;
        for (i, c) in candles.iter().enumerate() {
            let (up, down) = (values[0][i], values[1][i]);
            // The first true range is the candle's own range, so the 10-candle ATR (and the
            // SuperTrend) starts on the tenth candle.
            if i < 9 {
                assert!(up.is_nan() && down.is_nan());
                continue;
            }
            assert!(up.is_nan() != down.is_nan(), "exactly one line at {i}");
            if !up.is_nan() {
                assert!(up <= c.close, "rising line below the close at {i}");
            } else {
                assert!(down >= c.close, "falling line above the close at {i}");
            }
            if i > 9 && values[0][i - 1].is_nan() != up.is_nan() {
                flips += 1;
            }
        }
        assert!(flips > 2, "a random walk should change trend a few times");
    }

    #[test]
    fn parabolic_sar_starts_on_the_second_candle_and_flips() {
        let candles = [
            candle(0, 10.0, 11.0, 9.0, 10.5, 1.0),
            candle(1, 10.5, 12.0, 10.0, 11.5, 1.0),
            candle(2, 11.5, 13.0, 11.0, 12.5, 1.0),
            candle(3, 12.5, 12.6, 8.0, 8.5, 1.0),
        ];
        let values = run(ParabolicSar::new(0.02, 0.02, 0.2), 1, &candles)[0].clone();
        assert!(values[0].is_nan());
        // Rising from the second candle: the first SAR sits at the first candle's low,
        // nudged towards the high (9 + 0.02 · (12 − 9)) and capped by the prior low.
        assert!((values[1] - 9.0).abs() < 1e-9, "{values:?}");
        assert!(values[2] < candles[2].low);
        // The crash through the SAR flips it above the price.
        assert!(values[3] > candles[3].high, "{values:?}");
    }

    #[test]
    fn dmi_values_stay_in_range() {
        let candles = random_walk(300, 6);
        let values = run(Dmi::new(14, 14), 3, &candles);
        for series in &values {
            let defined: Vec<f64> = series.iter().copied().filter(|v| !v.is_nan()).collect();
            assert!(defined.len() > 250);
            assert!(defined.iter().all(|v| (0.0..=100.0).contains(v)));
        }
        // ADX needs two smoothings, so it starts later than the DI lines.
        let first = |series: &[f64]| series.iter().position(|v| !v.is_nan()).unwrap();
        assert_eq!(first(&values[1]), 14);
        assert_eq!(first(&values[0]), 27);
    }

    #[test]
    fn ichimoku_lines_are_channel_midpoints() {
        let candles = random_walk(100, 7);
        let values = run(Ichimoku::new(9, 26, 52), 5, &candles);
        let i = 80;
        let midpoint = |n: usize| {
            let window = &candles[i + 1 - n..=i];
            let high = window.iter().map(|c| c.high).fold(f64::MIN, f64::max);
            let low = window.iter().map(|c| c.low).fold(f64::MAX, f64::min);
            (high + low) / 2.0
        };
        assert!((values[0][i] - midpoint(9)).abs() < 1e-9);
        assert!((values[1][i] - midpoint(26)).abs() < 1e-9);
        assert_eq!(values[2][i], candles[i].close);
        assert!((values[3][i] - (midpoint(9) + midpoint(26)) / 2.0).abs() < 1e-9);
        assert!((values[4][i] - midpoint(52)).abs() < 1e-9);
    }
}
