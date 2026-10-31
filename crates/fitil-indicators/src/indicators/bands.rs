//! Bands around a middle line. Outputs: basis, upper, lower.

use fitil_core::Candle;

use crate::Indicator;
use crate::primitives::{Atr, Deviation, Ema, Extremes, Sma};
use crate::settings::Source;

/// Bollinger Bands: a simple average plus and minus a multiple of the standard deviation.
#[derive(Debug, Clone)]
pub struct Bollinger {
    source: Source,
    average: Sma,
    deviation: Deviation,
    multiplier: f64,
}

impl Bollinger {
    pub fn new(source: Source, length: usize, multiplier: f64) -> Self {
        Self {
            source,
            average: Sma::new(length),
            deviation: Deviation::new(length),
            multiplier,
        }
    }
}

impl Indicator for Bollinger {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        let value = self.source.of(candle);
        let basis = self.average.next(value);
        let width = self.multiplier * self.deviation.next(value);
        out[0] = basis;
        out[1] = basis + width;
        out[2] = basis - width;
    }
}

/// Keltner Channels: an exponential average plus and minus a multiple of the ATR,
/// as TradingView draws them by default.
#[derive(Debug, Clone)]
pub struct Keltner {
    source: Source,
    average: Ema,
    range: Atr,
    multiplier: f64,
}

impl Keltner {
    pub fn new(source: Source, length: usize, multiplier: f64, atr_length: usize) -> Self {
        Self {
            source,
            average: Ema::new(length),
            range: Atr::new(atr_length),
            multiplier,
        }
    }
}

impl Indicator for Keltner {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        let basis = self.average.next(self.source.of(candle));
        let width = self.multiplier * self.range.next(candle);
        out[0] = basis;
        out[1] = basis + width;
        out[2] = basis - width;
    }
}

/// Donchian Channels: the highest high and lowest low, and their midpoint.
#[derive(Debug, Clone)]
pub struct Donchian {
    extremes: Extremes,
}

impl Donchian {
    pub fn new(length: usize) -> Self {
        Self {
            extremes: Extremes::new(length),
        }
    }
}

impl Indicator for Donchian {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        let (high, low) = self.extremes.next(candle.high, candle.low);
        out[0] = (high + low) / 2.0;
        out[1] = high;
        out[2] = low;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::testing::{random_walk, run};

    #[test]
    fn bollinger_uses_the_population_deviation() {
        let candles = random_walk(80, 2);
        let values = run(Bollinger::new(Source::Close, 20, 2.0), 3, &candles);
        for i in 19..candles.len() {
            let window: Vec<f64> = candles[i - 19..=i].iter().map(|c| c.close).collect();
            let mean = window.iter().sum::<f64>() / 20.0;
            let deviation = (window.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / 20.0).sqrt();
            assert!((values[0][i] - mean).abs() < 1e-9);
            assert!((values[1][i] - (mean + 2.0 * deviation)).abs() < 1e-9);
            assert!((values[2][i] - (mean - 2.0 * deviation)).abs() < 1e-9);
        }
        assert!(values[1][18].is_nan());
    }

    #[test]
    fn donchian_tracks_the_extremes() {
        let candles = random_walk(50, 3);
        let values = run(Donchian::new(10), 3, &candles);
        for i in 9..candles.len() {
            let window = &candles[i - 9..=i];
            let high = window.iter().map(|c| c.high).fold(f64::MIN, f64::max);
            let low = window.iter().map(|c| c.low).fold(f64::MAX, f64::min);
            assert_eq!((values[1][i], values[2][i]), (high, low));
            assert_eq!(values[0][i], (high + low) / 2.0);
        }
    }

    #[test]
    fn keltner_is_symmetric_around_its_basis() {
        let candles = random_walk(60, 4);
        let values = run(Keltner::new(Source::Close, 20, 2.0, 10), 3, &candles);
        let last = candles.len() - 1;
        let half = values[1][last] - values[0][last];
        assert!(half > 0.0);
        assert!((values[0][last] - values[2][last] - half).abs() < 1e-9);
    }
}
