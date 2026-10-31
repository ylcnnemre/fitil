//! Momentum oscillators.

use fitil_core::Candle;

use crate::Indicator;
use crate::primitives::{Change, Ema, Extremes, MeanDeviation, Sma, Window};
use crate::settings::Source;

/// Wilder's RSI of a value, shared by [`Rsi`] and [`StochRsi`].
#[derive(Debug, Clone)]
struct RsiCore {
    change: Change,
    gains: Ema,
    losses: Ema,
}

impl RsiCore {
    fn new(length: usize) -> Self {
        Self {
            change: Change::default(),
            gains: Ema::wilder(length),
            losses: Ema::wilder(length),
        }
    }

    fn next(&mut self, value: f64) -> f64 {
        let change = self.change.next(value);
        if change.is_nan() {
            return f64::NAN;
        }
        let gain = self.gains.next(change.max(0.0));
        let loss = self.losses.next((-change).max(0.0));
        if gain.is_nan() || loss.is_nan() {
            f64::NAN
        } else if loss == 0.0 {
            100.0
        } else if gain == 0.0 {
            0.0
        } else {
            100.0 - 100.0 / (1.0 + gain / loss)
        }
    }
}

#[derive(Debug, Clone)]
pub struct Rsi {
    source: Source,
    core: RsiCore,
}

impl Rsi {
    pub fn new(source: Source, length: usize) -> Self {
        Self {
            source,
            core: RsiCore::new(length),
        }
    }
}

impl Indicator for Rsi {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        out[0] = self.core.next(self.source.of(candle));
    }
}

/// MACD. Outputs: histogram, MACD line, signal line.
#[derive(Debug, Clone)]
pub struct Macd {
    source: Source,
    fast: Ema,
    slow: Ema,
    signal: Ema,
}

impl Macd {
    pub fn new(source: Source, fast: usize, slow: usize, signal: usize) -> Self {
        Self {
            source,
            fast: Ema::new(fast),
            slow: Ema::new(slow),
            signal: Ema::new(signal),
        }
    }
}

impl Indicator for Macd {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        let value = self.source.of(candle);
        let macd = self.fast.next(value) - self.slow.next(value);
        let signal = self.signal.next(macd);
        out[0] = macd - signal;
        out[1] = macd;
        out[2] = signal;
    }
}

/// 100 · (value − lowest) / (highest − lowest); undefined when the range is empty.
fn stochastic(value: f64, highest: f64, lowest: f64) -> f64 {
    if highest > lowest {
        100.0 * (value - lowest) / (highest - lowest)
    } else {
        f64::NAN
    }
}

/// Stochastic oscillator. Outputs: %K, %D.
#[derive(Debug, Clone)]
pub struct Stochastic {
    extremes: Extremes,
    k: Sma,
    d: Sma,
}

impl Stochastic {
    pub fn new(k_length: usize, k_smoothing: usize, d_smoothing: usize) -> Self {
        Self {
            extremes: Extremes::new(k_length),
            k: Sma::new(k_smoothing),
            d: Sma::new(d_smoothing),
        }
    }
}

impl Indicator for Stochastic {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        let (highest, lowest) = self.extremes.next(candle.high, candle.low);
        let k = self.k.next(stochastic(candle.close, highest, lowest));
        out[0] = k;
        out[1] = self.d.next(k);
    }
}

/// The stochastic oscillator applied to RSI. Outputs: %K, %D.
#[derive(Debug, Clone)]
pub struct StochRsi {
    source: Source,
    rsi: RsiCore,
    window: Window,
    k: Sma,
    d: Sma,
}

impl StochRsi {
    pub fn new(
        source: Source,
        rsi_length: usize,
        stoch_length: usize,
        k_smoothing: usize,
        d_smoothing: usize,
    ) -> Self {
        Self {
            source,
            rsi: RsiCore::new(rsi_length),
            window: Window::new(stoch_length),
            k: Sma::new(k_smoothing),
            d: Sma::new(d_smoothing),
        }
    }
}

impl Indicator for StochRsi {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        let rsi = self.rsi.next(self.source.of(candle));
        let raw = if rsi.is_nan() {
            f64::NAN
        } else {
            self.window.push(rsi);
            if self.window.is_full() {
                stochastic(rsi, self.window.max(), self.window.min())
            } else {
                f64::NAN
            }
        };
        let k = self.k.next(raw);
        out[0] = k;
        out[1] = self.d.next(k);
    }
}

/// Commodity Channel Index.
#[derive(Debug, Clone)]
pub struct Cci {
    source: Source,
    average: Sma,
    deviation: MeanDeviation,
}

impl Cci {
    pub fn new(source: Source, length: usize) -> Self {
        Self {
            source,
            average: Sma::new(length),
            deviation: MeanDeviation::new(length),
        }
    }
}

impl Indicator for Cci {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        let value = self.source.of(candle);
        let average = self.average.next(value);
        let deviation = self.deviation.next(value);
        out[0] = if deviation > 0.0 {
            (value - average) / (0.015 * deviation)
        } else {
            f64::NAN
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::testing::{agree, closes, random_walk, run};

    /// StockCharts' worked example of a 14-day RSI ("Relative Strength Index",
    /// ChartSchool): the first value is 70.53 on the fifteenth close.
    #[test]
    fn rsi_matches_the_stockcharts_example() {
        let prices = [
            44.3389, 44.0902, 44.1497, 43.6124, 44.3278, 44.8264, 45.0955, 45.4245, 45.8433,
            46.0826, 45.8931, 46.0328, 45.6140, 46.2820, 46.2820, 46.0028, 46.0328, 46.4116,
            46.2222, 45.6439, 46.2122, 46.2521, 45.7137, 46.4515, 45.7835, 45.3548, 44.0288,
            44.1783, 44.2181, 44.5672, 43.4205, 42.6628, 43.1314,
        ];
        let expected = [
            70.53, 66.32, 66.55, 69.41, 66.36, 57.97, 62.93, 63.26, 56.06, 62.38, 54.71, 50.42,
            39.99, 41.46, 41.87, 45.46, 37.30, 33.08, 37.77,
        ];
        let values = run(Rsi::new(Source::Close, 14), 1, &closes(&prices));
        assert!(values[0][..14].iter().all(|v| v.is_nan()));
        // The example rounds to two decimals.
        assert!(
            agree(&values[0][14..], &expected, 0.005),
            "{:?}",
            &values[0][14..]
        );
    }

    #[test]
    fn macd_is_the_difference_of_two_averages() {
        let candles = random_walk(120, 8);
        let values = run(Macd::new(Source::Close, 12, 26, 9), 3, &candles);
        let mut fast = Ema::new(12);
        let mut slow = Ema::new(26);
        for (i, candle) in candles.iter().enumerate() {
            let expected = fast.next(candle.close) - slow.next(candle.close);
            assert!(agree(&[values[1][i]], &[expected], 1e-9));
            if !values[2][i].is_nan() {
                assert!((values[0][i] - (values[1][i] - values[2][i])).abs() < 1e-9);
            }
        }
        // The signal needs 26 + 9 − 1 candles.
        assert_eq!(values[2].iter().position(|v| !v.is_nan()), Some(33));
    }

    #[test]
    fn stochastics_stay_between_0_and_100() {
        let candles = random_walk(300, 9);
        let stochastic = run(Stochastic::new(14, 1, 3), 2, &candles);
        let stoch_rsi = run(StochRsi::new(Source::Close, 14, 14, 3, 3), 2, &candles);
        for series in stochastic.iter().chain(&stoch_rsi) {
            let defined: Vec<f64> = series.iter().copied().filter(|v| !v.is_nan()).collect();
            assert!(defined.len() > 200);
            // Running sums leave rounding noise far below what is ever shown.
            let outside = defined
                .iter()
                .find(|v| !(-1e-9..=100.0 + 1e-9).contains(*v));
            assert!(outside.is_none(), "{outside:?}");
        }
        // %K with no smoothing is the raw stochastic.
        let i = 50;
        let window = &candles[i - 13..=i];
        let high = window.iter().map(|c| c.high).fold(f64::MIN, f64::max);
        let low = window.iter().map(|c| c.low).fold(f64::MAX, f64::min);
        let expected = 100.0 * (candles[i].close - low) / (high - low);
        assert!((stochastic[0][i] - expected).abs() < 1e-9);
    }

    #[test]
    fn cci_uses_the_mean_deviation() {
        let candles = random_walk(60, 10);
        let values = run(Cci::new(Source::Hlc3, 20), 1, &candles);
        let i = 40;
        let typical: Vec<f64> = candles[i - 19..=i]
            .iter()
            .map(|c| Source::Hlc3.of(c))
            .collect();
        let mean = typical.iter().sum::<f64>() / 20.0;
        let deviation = typical.iter().map(|v| (v - mean).abs()).sum::<f64>() / 20.0;
        let expected = (typical[19] - mean) / (0.015 * deviation);
        assert!((values[0][i] - expected).abs() < 1e-9);
    }
}
