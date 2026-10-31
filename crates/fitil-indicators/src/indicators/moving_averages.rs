use fitil_core::Candle;

use crate::Indicator;
use crate::primitives::{Ema, Sma, Window, Wma};
use crate::settings::Source;

#[derive(Debug, Clone)]
pub struct SimpleAverage {
    source: Source,
    average: Sma,
}

impl SimpleAverage {
    pub fn new(source: Source, length: usize) -> Self {
        Self {
            source,
            average: Sma::new(length),
        }
    }
}

impl Indicator for SimpleAverage {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        out[0] = self.average.next(self.source.of(candle));
    }
}

#[derive(Debug, Clone)]
pub struct ExponentialAverage {
    source: Source,
    average: Ema,
}

impl ExponentialAverage {
    pub fn new(source: Source, length: usize) -> Self {
        Self {
            source,
            average: Ema::new(length),
        }
    }
}

impl Indicator for ExponentialAverage {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        out[0] = self.average.next(self.source.of(candle));
    }
}

#[derive(Debug, Clone)]
pub struct WeightedAverage {
    source: Source,
    average: Wma,
}

impl WeightedAverage {
    pub fn new(source: Source, length: usize) -> Self {
        Self {
            source,
            average: Wma::new(length),
        }
    }
}

impl Indicator for WeightedAverage {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        out[0] = self.average.next(self.source.of(candle));
    }
}

/// The average price weighted by volume over the last `length` candles.
#[derive(Debug, Clone)]
pub struct VolumeWeightedAverage {
    source: Source,
    weighted: Window,
    volumes: Window,
}

impl VolumeWeightedAverage {
    pub fn new(source: Source, length: usize) -> Self {
        Self {
            source,
            weighted: Window::new(length),
            volumes: Window::new(length),
        }
    }
}

impl Indicator for VolumeWeightedAverage {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        self.weighted.push(self.source.of(candle) * candle.volume);
        self.volumes.push(candle.volume);
        let volume: f64 = self.volumes.iter().sum();
        out[0] = if self.volumes.is_full() && volume > 0.0 {
            self.weighted.iter().sum::<f64>() / volume
        } else {
            f64::NAN
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::testing::{agree, closes, random_walk, run};

    /// StockCharts' worked example of a 10-day EMA ("Moving Averages – Simple and
    /// Exponential", ChartSchool): the first value is the 10-day SMA, 22.22.
    #[test]
    fn ema_matches_the_stockcharts_example() {
        let prices = [
            22.2734, 22.1940, 22.0847, 22.1741, 22.1840, 22.1344, 22.2337, 22.4323, 22.2436,
            22.2933, 22.1542, 22.3926, 22.3816, 22.6109, 23.3558, 24.0519, 23.7530, 23.8324,
            23.9516, 23.6338, 23.8225, 23.8722, 23.6537, 23.1870, 23.0976, 23.3260, 22.6805,
            23.0976, 22.4025, 22.1725,
        ];
        let expected = [
            22.22, 22.21, 22.24, 22.27, 22.33, 22.52, 22.80, 22.97, 23.13, 23.28, 23.34, 23.43,
            23.51, 23.54, 23.47, 23.40, 23.39, 23.26, 23.23, 23.08, 22.92,
        ];
        let values = run(
            ExponentialAverage::new(Source::Close, 10),
            1,
            &closes(&prices),
        );
        assert!(values[0][..9].iter().all(|v| v.is_nan()));
        // The example rounds to two decimals.
        assert!(
            agree(&values[0][9..], &expected, 0.005),
            "{:?}",
            &values[0][9..]
        );
    }

    #[test]
    fn vwma_weights_prices_by_volume() {
        let candles = random_walk(60, 1);
        let values = run(VolumeWeightedAverage::new(Source::Close, 5), 1, &candles);
        for i in 4..candles.len() {
            let window = &candles[i - 4..=i];
            let volume: f64 = window.iter().map(|c| c.volume).sum();
            let expected = window.iter().map(|c| c.close * c.volume).sum::<f64>() / volume;
            assert!((values[0][i] - expected).abs() < 1e-9);
        }
    }
}
