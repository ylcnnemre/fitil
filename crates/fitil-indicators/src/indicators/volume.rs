//! Volume-based indicators, and ATR.

use fitil_core::{Candle, TimeUnit, Timeframe};

use crate::Indicator;
use crate::primitives::{Atr, Change};
use crate::settings::Source;

/// When a cumulative indicator starts over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    Never,
    Day,
    Week,
    Month,
}

impl Anchor {
    /// From a choice parameter: `none`, `day`, `week` or `month`.
    pub fn from_id(id: &str) -> Self {
        match id {
            "week" => Anchor::Week,
            "month" => Anchor::Month,
            "none" => Anchor::Never,
            _ => Anchor::Day,
        }
    }

    /// The period `time` falls in; a change means starting over.
    fn period(self, time: i64) -> i64 {
        let unit = match self {
            Anchor::Never => return 0,
            Anchor::Day => TimeUnit::Day,
            Anchor::Week => TimeUnit::Week,
            Anchor::Month => TimeUnit::Month,
        };
        Timeframe::new(1, unit)
            .expect("1 is a valid count")
            .bucket_start(time)
    }
}

#[derive(Debug, Clone)]
pub struct AverageTrueRange {
    atr: Atr,
}

impl AverageTrueRange {
    pub fn new(length: usize) -> Self {
        Self {
            atr: Atr::new(length),
        }
    }
}

impl Indicator for AverageTrueRange {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        out[0] = self.atr.next(candle);
    }
}

#[derive(Debug, Clone)]
pub struct Volume;

impl Indicator for Volume {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        out[0] = candle.volume;
    }
}

/// Volume bought and sold by market (taker) orders. Outputs: buy volume, and sell volume
/// as a negative number so it is drawn below zero.
#[derive(Debug, Clone)]
pub struct BuySellVolume;

impl Indicator for BuySellVolume {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        match candle.taker_buy_volume {
            Some(buy) => {
                out[0] = buy;
                out[1] = -(candle.volume - buy);
            }
            None => {
                out[0] = f64::NAN;
                out[1] = f64::NAN;
            }
        }
    }
}

/// Cumulative volume delta: taker buys minus taker sells, added up since the anchor.
#[derive(Debug, Clone)]
pub struct Cvd {
    anchor: Anchor,
    period: Option<i64>,
    total: f64,
}

impl Cvd {
    pub fn new(anchor: Anchor) -> Self {
        Self {
            anchor,
            period: None,
            total: 0.0,
        }
    }
}

impl Indicator for Cvd {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        let Some(buy) = candle.taker_buy_volume else {
            out[0] = f64::NAN;
            return;
        };
        let period = self.anchor.period(candle.open_time);
        if self.period != Some(period) {
            self.period = Some(period);
            self.total = 0.0;
        }
        self.total += buy - (candle.volume - buy);
        out[0] = self.total;
    }
}

/// On-balance volume: volume added on up closes and taken away on down closes.
#[derive(Debug, Clone, Default)]
pub struct Obv {
    change: Change,
    total: f64,
}

impl Indicator for Obv {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        let change = self.change.next(candle.close);
        if change > 0.0 {
            self.total += candle.volume;
        } else if change < 0.0 {
            self.total -= candle.volume;
        }
        out[0] = self.total;
    }
}

/// Volume-weighted average price since the anchor (the start of the day by default).
#[derive(Debug, Clone)]
pub struct Vwap {
    source: Source,
    anchor: Anchor,
    period: Option<i64>,
    weighted: f64,
    volume: f64,
}

impl Vwap {
    pub fn new(source: Source, anchor: Anchor) -> Self {
        Self {
            source,
            anchor,
            period: None,
            weighted: 0.0,
            volume: 0.0,
        }
    }
}

impl Indicator for Vwap {
    fn next(&mut self, candle: &Candle, out: &mut [f64]) {
        let period = self.anchor.period(candle.open_time);
        if self.period != Some(period) {
            self.period = Some(period);
            self.weighted = 0.0;
            self.volume = 0.0;
        }
        self.weighted += self.source.of(candle) * candle.volume;
        self.volume += candle.volume;
        out[0] = if self.volume > 0.0 {
            self.weighted / self.volume
        } else {
            f64::NAN
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::testing::{candle, random_walk, run};

    const DAY: i64 = 86_400_000;

    #[test]
    fn buy_and_sell_volume_split_the_volume() {
        let candles = random_walk(20, 11);
        let values = run(BuySellVolume, 2, &candles);
        for (i, c) in candles.iter().enumerate() {
            assert!((values[0][i] - values[1][i] - c.volume).abs() < 1e-9);
        }
        let mut bare = candles[0];
        bare.taker_buy_volume = None;
        assert!(run(BuySellVolume, 2, &[bare])[0][0].is_nan());
    }

    #[test]
    fn cvd_and_vwap_start_over_each_day() {
        // Three candles: two on day 0, one on day 1.
        let mut candles = vec![
            candle(0, 10.0, 10.0, 10.0, 10.0, 10.0),
            candle(1, 20.0, 20.0, 20.0, 20.0, 30.0),
            candle(0, 40.0, 40.0, 40.0, 40.0, 5.0),
        ];
        candles[2].open_time = DAY;
        for (candle, buy) in candles.iter_mut().zip([6.0, 10.0, 5.0]) {
            candle.taker_buy_volume = Some(buy);
        }

        let cvd = run(Cvd::new(Anchor::Day), 1, &candles);
        // +2 (6 bought, 4 sold), then −10 (10 bought, 20 sold), then a new day: +5.
        assert_eq!(cvd[0], [2.0, -8.0, 5.0]);
        let never = run(Cvd::new(Anchor::Never), 1, &candles);
        assert_eq!(never[0], [2.0, -8.0, -3.0]);

        let vwap = run(Vwap::new(Source::Close, Anchor::Day), 1, &candles);
        assert_eq!(vwap[0], [10.0, (100.0 + 600.0) / 40.0, 40.0]);
    }

    #[test]
    fn obv_adds_and_subtracts_volume() {
        let closes = [10.0, 11.0, 11.0, 9.0];
        let candles: Vec<Candle> = closes
            .iter()
            .enumerate()
            .map(|(i, &c)| candle(i as i64, c, c, c, c, 10.0 * (i + 1) as f64))
            .collect();
        assert_eq!(
            run(Obv::default(), 1, &candles)[0],
            [0.0, 20.0, 20.0, -20.0]
        );
    }
}
