//! Building blocks shared by the indicators: averages, deviations and rolling windows.
//!
//! Each takes one value at a time. NaN inputs are skipped (they return NaN and leave the
//! state alone), so building blocks can be chained: the average of a value that is not
//! defined yet is not defined either.

use std::collections::VecDeque;

use fitil_core::Candle;

/// The last `capacity` values, oldest first.
#[derive(Debug, Clone)]
pub struct Window {
    values: VecDeque<f64>,
    capacity: usize,
}

impl Window {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            values: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Adds a value and returns the one that fell out, if the window was full.
    pub fn push(&mut self, value: f64) -> Option<f64> {
        let evicted = if self.values.len() == self.capacity {
            self.values.pop_front()
        } else {
            None
        };
        self.values.push_back(value);
        evicted
    }

    pub fn is_full(&self) -> bool {
        self.values.len() == self.capacity
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = f64> + '_ {
        self.values.iter().copied()
    }

    pub fn max(&self) -> f64 {
        self.iter().fold(f64::NEG_INFINITY, f64::max)
    }

    pub fn min(&self) -> f64 {
        self.iter().fold(f64::INFINITY, f64::min)
    }

    fn mean(&self) -> f64 {
        self.iter().sum::<f64>() / self.values.len() as f64
    }
}

/// Simple moving average.
#[derive(Debug, Clone)]
pub struct Sma {
    window: Window,
    sum: f64,
}

impl Sma {
    pub fn new(length: usize) -> Self {
        Self {
            window: Window::new(length),
            sum: 0.0,
        }
    }

    pub fn next(&mut self, value: f64) -> f64 {
        if value.is_nan() {
            return f64::NAN;
        }
        if let Some(old) = self.window.push(value) {
            self.sum -= old;
        }
        self.sum += value;
        if self.window.is_full() {
            self.sum / self.window.capacity() as f64
        } else {
            f64::NAN
        }
    }
}

/// Exponential moving average with smoothing factor `alpha`, starting from the simple
/// average of the first `length` values, as TA-Lib and Wilder do.
#[derive(Debug, Clone)]
pub struct Ema {
    alpha: f64,
    seed: Option<Sma>,
    value: f64,
}

impl Ema {
    /// The usual EMA: alpha = 2 / (length + 1).
    pub fn new(length: usize) -> Self {
        Self::with_alpha(2.0 / (length.max(1) as f64 + 1.0), length)
    }

    /// Wilder's smoothing (often called RMA), used by RSI, ATR and ADX:
    /// alpha = 1 / length.
    pub fn wilder(length: usize) -> Self {
        Self::with_alpha(1.0 / length.max(1) as f64, length)
    }

    fn with_alpha(alpha: f64, length: usize) -> Self {
        Self {
            alpha,
            seed: Some(Sma::new(length)),
            value: f64::NAN,
        }
    }

    pub fn next(&mut self, value: f64) -> f64 {
        if value.is_nan() {
            return f64::NAN;
        }
        if let Some(seed) = &mut self.seed {
            let average = seed.next(value);
            if !average.is_nan() {
                self.value = average;
                self.seed = None;
            }
            return average;
        }
        self.value += self.alpha * (value - self.value);
        self.value
    }
}

/// Weighted moving average: the newest value weighs `length`, the oldest 1.
#[derive(Debug, Clone)]
pub struct Wma {
    window: Window,
}

impl Wma {
    pub fn new(length: usize) -> Self {
        Self {
            window: Window::new(length),
        }
    }

    pub fn next(&mut self, value: f64) -> f64 {
        if value.is_nan() {
            return f64::NAN;
        }
        self.window.push(value);
        if !self.window.is_full() {
            return f64::NAN;
        }
        let n = self.window.capacity() as f64;
        let weighted: f64 = self
            .window
            .iter()
            .enumerate()
            .map(|(i, value)| (i + 1) as f64 * value)
            .sum();
        weighted / (n * (n + 1.0) / 2.0)
    }
}

/// Population standard deviation over the last `length` values, like TradingView's
/// `ta.stdev`.
#[derive(Debug, Clone)]
pub struct Deviation {
    window: Window,
}

impl Deviation {
    pub fn new(length: usize) -> Self {
        Self {
            window: Window::new(length),
        }
    }

    pub fn next(&mut self, value: f64) -> f64 {
        if value.is_nan() {
            return f64::NAN;
        }
        self.window.push(value);
        if !self.window.is_full() {
            return f64::NAN;
        }
        let mean = self.window.mean();
        let variance = self.window.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
            / self.window.capacity() as f64;
        variance.sqrt()
    }
}

/// Mean absolute deviation from the average of the last `length` values (used by CCI).
#[derive(Debug, Clone)]
pub struct MeanDeviation {
    window: Window,
}

impl MeanDeviation {
    pub fn new(length: usize) -> Self {
        Self {
            window: Window::new(length),
        }
    }

    pub fn next(&mut self, value: f64) -> f64 {
        if value.is_nan() {
            return f64::NAN;
        }
        self.window.push(value);
        if !self.window.is_full() {
            return f64::NAN;
        }
        let mean = self.window.mean();
        self.window.iter().map(|v| (v - mean).abs()).sum::<f64>() / self.window.capacity() as f64
    }
}

/// Highest and lowest value over the last `length` values.
#[derive(Debug, Clone)]
pub struct Extremes {
    highs: Window,
    lows: Window,
}

impl Extremes {
    pub fn new(length: usize) -> Self {
        Self {
            highs: Window::new(length),
            lows: Window::new(length),
        }
    }

    /// The highest of the highs and lowest of the lows, or NaNs until the window is full.
    pub fn next(&mut self, high: f64, low: f64) -> (f64, f64) {
        if high.is_nan() || low.is_nan() {
            return (f64::NAN, f64::NAN);
        }
        self.highs.push(high);
        self.lows.push(low);
        if self.highs.is_full() {
            (self.highs.max(), self.lows.min())
        } else {
            (f64::NAN, f64::NAN)
        }
    }
}

/// A candle's range including any gap from the previous close.
#[derive(Debug, Clone, Default)]
pub struct TrueRange {
    previous_close: Option<f64>,
}

impl TrueRange {
    pub fn next(&mut self, candle: &Candle) -> f64 {
        let range = match self.previous_close {
            Some(close) => candle.high.max(close) - candle.low.min(close),
            None => candle.high - candle.low,
        };
        self.previous_close = Some(candle.close);
        range
    }
}

/// Average true range, with Wilder's smoothing.
#[derive(Debug, Clone)]
pub struct Atr {
    range: TrueRange,
    average: Ema,
}

impl Atr {
    pub fn new(length: usize) -> Self {
        Self {
            range: TrueRange::default(),
            average: Ema::wilder(length),
        }
    }

    pub fn next(&mut self, candle: &Candle) -> f64 {
        self.average.next(self.range.next(candle))
    }
}

/// The change from the previous value; NaN for the first.
#[derive(Debug, Clone, Default)]
pub struct Change {
    previous: Option<f64>,
}

impl Change {
    pub fn next(&mut self, value: f64) -> f64 {
        if value.is_nan() {
            return f64::NAN;
        }
        let change = self.previous.map_or(f64::NAN, |previous| value - previous);
        self.previous = Some(value);
        change
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(mut f: impl FnMut(f64) -> f64, values: &[f64]) -> Vec<f64> {
        values.iter().map(|&v| f(v)).collect()
    }

    fn close(a: &[f64], b: &[f64]) -> bool {
        a.len() == b.len()
            && a.iter()
                .zip(b)
                .all(|(x, y)| (x.is_nan() && y.is_nan()) || (x - y).abs() < 1e-9)
    }

    const NAN: f64 = f64::NAN;

    #[test]
    fn simple_and_weighted_averages() {
        let mut sma = Sma::new(3);
        assert!(close(
            &run(|v| sma.next(v), &[1.0, 2.0, 3.0, 4.0]),
            &[NAN, NAN, 2.0, 3.0]
        ));
        let mut wma = Wma::new(3);
        // (1·1 + 2·2 + 3·3) / 6 and (1·2 + 2·3 + 3·4) / 6.
        let expected = [NAN, NAN, 14.0 / 6.0, 20.0 / 6.0];
        assert!(close(
            &run(|v| wma.next(v), &[1.0, 2.0, 3.0, 4.0]),
            &expected
        ));
    }

    #[test]
    fn exponential_averages_start_from_the_simple_average() {
        let mut ema = Ema::new(3); // alpha 0.5
        assert!(close(
            &run(|v| ema.next(v), &[1.0, 2.0, 3.0, 5.0, 7.0]),
            &[NAN, NAN, 2.0, 3.5, 5.25]
        ));
        let mut wilder = Ema::wilder(2); // alpha 0.5
        assert!(close(
            &run(|v| wilder.next(v), &[2.0, 4.0, 8.0]),
            &[NAN, 3.0, 5.5]
        ));
    }

    #[test]
    fn nan_inputs_are_skipped() {
        let mut sma = Sma::new(2);
        assert!(close(
            &run(|v| sma.next(v), &[NAN, 1.0, NAN, 3.0]),
            &[NAN, NAN, NAN, 2.0]
        ));
    }

    #[test]
    fn deviations() {
        let mut deviation = Deviation::new(4);
        let out = run(|v| deviation.next(v), &[2.0, 4.0, 4.0, 4.0]);
        assert!((out[3] - 3f64.sqrt() / 2.0).abs() < 1e-12); // mean 3.5, variance 0.75
        let mut mean = MeanDeviation::new(4);
        let out = run(|v| mean.next(v), &[2.0, 4.0, 4.0, 4.0]);
        assert!((out[3] - 0.75).abs() < 1e-12);
    }

    #[test]
    fn true_range_includes_gaps() {
        let candle = |high, low, close| Candle {
            open_time: 0,
            open: close,
            high,
            low,
            close,
            volume: 0.0,
            quote_volume: None,
            trades: None,
            taker_buy_volume: None,
        };
        let mut range = TrueRange::default();
        assert_eq!(range.next(&candle(10.0, 8.0, 9.0)), 2.0);
        assert_eq!(range.next(&candle(14.0, 12.0, 13.0)), 5.0); // gap up from 9
        assert_eq!(range.next(&candle(13.0, 11.0, 12.0)), 2.0);
    }
}
