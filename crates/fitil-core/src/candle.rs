use crate::Timeframe;

/// One OHLCV candle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candle {
    /// Unix milliseconds (UTC) when the candle opened.
    pub open_time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    /// Traded amount in the base asset (BTC in BTCUSDT).
    pub volume: f64,
    /// Traded amount in the quote asset (USDT in BTCUSDT).
    pub quote_volume: f64,
    pub trades: u64,
    /// Base-asset volume bought by market (taker) buyers. The rest of `volume` was sold
    /// by takers; this is what buy/sell volume and CVD are computed from.
    pub taker_buy_volume: f64,
}

impl Candle {
    /// Extends this candle with the next one in time.
    fn merge(&mut self, next: &Candle) {
        self.high = self.high.max(next.high);
        self.low = self.low.min(next.low);
        self.close = next.close;
        self.volume += next.volume;
        self.quote_volume += next.quote_volume;
        self.trades += next.trades;
        self.taker_buy_volume += next.taker_buy_volume;
    }
}

/// Merges candles, sorted by open time, into candles of the longer `target` timeframe.
///
/// The first and last results may be partial if `candles` does not start or end on a
/// `target` boundary.
pub fn aggregate(candles: &[Candle], target: Timeframe) -> Vec<Candle> {
    let mut merged: Vec<Candle> = Vec::new();
    for candle in candles {
        let start = target.bucket_start(candle.open_time);
        match merged.last_mut() {
            Some(last) if last.open_time == start => last.merge(candle),
            _ => merged.push(Candle {
                open_time: start,
                ..*candle
            }),
        }
    }
    merged
}

/// Like [`aggregate`], but skips the candles before the first `target` boundary, so the
/// first result is never partial. The last one may still be (it is the open candle).
pub fn aggregate_whole(candles: &[Candle], target: Timeframe) -> Vec<Candle> {
    let first_boundary = candles
        .iter()
        .position(|candle| target.bucket_start(candle.open_time) == candle.open_time)
        .unwrap_or(candles.len());
    aggregate(&candles[first_boundary..], target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_aggregation_skips_a_partial_start() {
        // Minutes 1 and 2 belong to the 0–3 bucket, which started before the data.
        let minutes: Vec<Candle> = (1..=7).map(|m| candle(m, 1.0, 2.0, 0.5, 1.5)).collect();
        let merged = aggregate_whole(&minutes, "3m".parse().unwrap());
        let starts: Vec<i64> = merged.iter().map(|c| c.open_time / 60_000).collect();
        assert_eq!(starts, [3, 6]);
        assert_eq!(merged[0].trades, 6);
        assert_eq!(merged[1].trades, 4); // still open: minutes 6 and 7
    }

    fn candle(minute: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            open_time: minute * 60_000,
            open,
            high,
            low,
            close,
            volume: 1.0,
            quote_volume: close,
            trades: 2,
            taker_buy_volume: 0.25,
        }
    }

    #[test]
    fn merges_into_longer_buckets() {
        let minutes = [
            candle(0, 10.0, 12.0, 9.0, 11.0),
            candle(1, 11.0, 15.0, 10.0, 14.0),
            candle(2, 14.0, 14.0, 8.0, 9.0),
            candle(3, 9.0, 10.0, 9.0, 10.0),
        ];
        let merged = aggregate(&minutes, "3m".parse().unwrap());

        assert_eq!(merged.len(), 2);
        let first = merged[0];
        assert_eq!(first.open_time, 0);
        assert_eq!(
            (first.open, first.high, first.low, first.close),
            (10.0, 15.0, 8.0, 9.0)
        );
        assert_eq!(first.volume, 3.0);
        assert_eq!(first.quote_volume, 34.0);
        assert_eq!(first.trades, 6);
        assert_eq!(first.taker_buy_volume, 0.75);
        assert_eq!(
            merged[1],
            Candle {
                open_time: 180_000,
                ..minutes[3]
            }
        );
    }

    #[test]
    fn empty_input_gives_no_candles() {
        assert!(aggregate(&[], "1h".parse().unwrap()).is_empty());
    }
}
