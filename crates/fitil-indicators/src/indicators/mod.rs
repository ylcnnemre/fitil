//! The indicators themselves, grouped like the catalog.

mod bands;
mod momentum;
mod moving_averages;
mod trend;
mod volume;

pub use bands::{Bollinger, Donchian, Keltner};
pub use momentum::{Cci, Macd, Rsi, StochRsi, Stochastic};
pub use moving_averages::{
    ExponentialAverage, SimpleAverage, VolumeWeightedAverage, WeightedAverage,
};
pub use trend::{Dmi, Ichimoku, ParabolicSar, SuperTrend};
pub use volume::{Anchor, AverageTrueRange, BuySellVolume, Cvd, Obv, Volume, Vwap};

#[cfg(test)]
pub(crate) mod testing {
    use fitil_core::Candle;

    use crate::Indicator;

    /// A candle at minute `index` with the given prices.
    pub fn candle(index: i64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
        Candle {
            open_time: index * 60_000,
            open,
            high,
            low,
            close,
            volume,
            quote_volume: Some(volume * close),
            trades: Some(1),
            taker_buy_volume: Some(volume * 0.6),
        }
    }

    /// Candles whose close is each of `closes`, with a small range around it.
    pub fn closes(closes: &[f64]) -> Vec<Candle> {
        closes
            .iter()
            .enumerate()
            .map(|(i, &close)| candle(i as i64, close, close + 0.5, close - 0.5, close, 100.0))
            .collect()
    }

    /// A deterministic, bumpy price series of `count` candles.
    pub fn random_walk(count: usize, seed: u64) -> Vec<Candle> {
        let mut state = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let mut next = move || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 33) as f64 / (1u64 << 31) as f64
        };
        let mut price = 100.0;
        (0..count)
            .map(|i| {
                let open = price;
                let close = (open * (1.0 + (next() - 0.5) * 0.04)).max(1.0);
                let high = open.max(close) * (1.0 + next() * 0.01);
                let low = open.min(close) * (1.0 - next() * 0.01);
                let volume = 50.0 + next() * 100.0;
                price = close;
                let mut candle = candle(i as i64, open, high, low, close, volume);
                candle.taker_buy_volume = Some(volume * next());
                candle
            })
            .collect()
    }

    /// Feeds every candle and collects each output's values.
    pub fn run(mut indicator: impl Indicator, outputs: usize, candles: &[Candle]) -> Vec<Vec<f64>> {
        let mut values = vec![Vec::with_capacity(candles.len()); outputs];
        let mut out = vec![f64::NAN; outputs];
        for candle in candles {
            indicator.next(candle, &mut out);
            for (series, value) in values.iter_mut().zip(&out) {
                series.push(*value);
            }
        }
        values
    }

    /// Whether two series agree, NaNs included, to within `tolerance`.
    pub fn agree(actual: &[f64], expected: &[f64], tolerance: f64) -> bool {
        actual.len() == expected.len()
            && actual
                .iter()
                .zip(expected)
                .all(|(a, e)| (a.is_nan() && e.is_nan()) || (a - e).abs() <= tolerance)
    }
}
