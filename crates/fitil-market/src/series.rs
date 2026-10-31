//! One displayed series: the closed candles in memory, the open candle and what is
//! loading. Every decision about loaded pages and live updates is made here.

use fitil_core::{Candle, SeriesKey, Timeframe, aggregate_whole};

use crate::{Loaded, MarketError};

/// What a live candle update means for the caller.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LiveUpdate {
    /// The open candle changed: redraw.
    Changed,
    /// A candle closed and was added: redraw, and save it to the cache.
    Closed(Candle),
    /// Candles were missed (for example while disconnected): load the newest ones again.
    Reload,
}

/// A series shown in `target`, built from the series `key` the exchange serves.
#[derive(Debug, Clone)]
pub struct Series {
    key: SeriesKey,
    target: Timeframe,
    /// Closed candles of `key`, oldest first, without gaps.
    candles: Vec<Candle>,
    /// The open candle of `key`.
    live: Option<Candle>,
    /// Whether `candles` starts at the symbol's first candle.
    complete: bool,
    /// Candles of `key` in the cache.
    cached: u64,
    loading: bool,
    loading_older: bool,
}

impl Series {
    pub fn new(key: SeriesKey, target: Timeframe) -> Self {
        Self {
            key,
            target,
            candles: Vec::new(),
            live: None,
            complete: false,
            cached: 0,
            loading: false,
            loading_older: false,
        }
    }

    pub fn key(&self) -> &SeriesKey {
        &self.key
    }

    /// The timeframe shown.
    pub fn target(&self) -> Timeframe {
        self.target
    }

    /// The timeframe downloaded from the exchange to build [`Series::target`].
    pub fn source(&self) -> Timeframe {
        self.key.timeframe
    }

    pub fn candles(&self) -> &[Candle] {
        &self.candles
    }

    pub fn cached(&self) -> u64 {
        self.cached
    }

    pub fn complete(&self) -> bool {
        self.complete
    }

    pub fn loading(&self) -> bool {
        self.loading
    }

    pub fn loading_older(&self) -> bool {
        self.loading_older
    }

    /// Whether a feed update for `symbol` in `timeframe` belongs to this series.
    pub fn is_fed_by(&self, symbol: &str, timeframe: Timeframe) -> bool {
        self.key.symbol == symbol && self.key.timeframe == timeframe
    }

    /// Marks the newest candles as loading. Returns false if they already are.
    pub fn begin_load(&mut self) -> bool {
        !std::mem::replace(&mut self.loading, true)
    }

    /// Marks older candles as loading and returns the open time to load before, unless
    /// something is loading already or the history is complete.
    pub fn begin_load_older(&mut self) -> Option<i64> {
        if self.loading || self.loading_older || self.complete {
            return None;
        }
        let first = self.candles.first()?.open_time;
        self.loading_older = true;
        Some(first)
    }

    /// Takes in the newest candles. Older candles already in memory stay when they connect.
    pub fn finish_load(&mut self, result: Result<Loaded, MarketError>) -> Result<(), MarketError> {
        self.loading = false;
        let loaded = result?;
        // Keep the feed's open candle unless the fetched one is newer.
        if let Some(open) = loaded.open
            && self.live.is_none_or(|live| live.open_time < open.open_time)
        {
            self.live = Some(open);
        }
        self.cached = loaded.cached;

        let Some(first_new) = loaded.candles.first().map(|candle| candle.open_time) else {
            self.candles = loaded.candles;
            self.complete = loaded.complete;
            return Ok(());
        };
        let keep = self
            .candles
            .partition_point(|candle| candle.open_time < first_new);
        let connects =
            keep > 0 && self.source().next_bucket(self.candles[keep - 1].open_time) == first_new;
        if connects {
            self.candles.truncate(keep);
            self.candles.extend(loaded.candles);
        } else {
            self.candles = loaded.candles;
            self.complete = loaded.complete;
        }
        Ok(())
    }

    /// Takes in older candles, which go in front when they connect.
    pub fn finish_load_older(
        &mut self,
        result: Result<Loaded, MarketError>,
    ) -> Result<(), MarketError> {
        self.loading_older = false;
        let older = result?;
        self.cached = older.cached;
        let connects = match (older.candles.last(), self.candles.first()) {
            (Some(last), Some(first)) => {
                self.source().next_bucket(last.open_time) == first.open_time
            }
            // Nothing older: only the "complete" flag is news.
            (None, _) => true,
            (Some(_), None) => false,
        };
        if connects {
            let mut candles = older.candles;
            candles.append(&mut self.candles);
            self.candles = candles;
            self.complete = older.complete;
        }
        Ok(())
    }

    /// Applies a candle update from the live feed.
    pub fn apply_live(&mut self, candle: Candle, closed: bool) -> LiveUpdate {
        if !closed {
            self.live = Some(candle);
            return LiveUpdate::Changed;
        }
        self.live = None;
        // While loading, the load will bring this candle along.
        let Some(last) = self.candles.last().filter(|_| !self.loading) else {
            return LiveUpdate::Changed;
        };
        if candle.open_time <= last.open_time {
            return LiveUpdate::Changed;
        }
        if self.source().next_bucket(last.open_time) != candle.open_time {
            return LiveUpdate::Reload;
        }
        self.candles.push(candle);
        self.cached += 1;
        LiveUpdate::Closed(candle)
    }

    /// The candles to draw: the closed ones and the open one, in the displayed timeframe.
    pub fn shown(&self) -> Vec<Candle> {
        let mut source = self.candles.clone();
        if let Some(live) = self.live
            && source
                .last()
                .is_none_or(|last| last.open_time < live.open_time)
        {
            source.push(live);
        }
        if self.target == self.source() {
            source
        } else {
            aggregate_whole(&source, self.target)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{HOUR, candle, key};

    fn series() -> Series {
        Series::new(key(), "1h".parse().unwrap())
    }

    fn loaded(hours: std::ops::Range<i64>, complete: bool) -> Loaded {
        Loaded {
            candles: hours.clone().map(candle).collect(),
            open: Some(candle(hours.end)),
            complete,
            cached: hours.count() as u64,
        }
    }

    fn hours(candles: &[Candle]) -> Vec<i64> {
        candles.iter().map(|c| c.open_time / HOUR).collect()
    }

    #[test]
    fn a_first_load_shows_closed_and_open_candles() {
        let mut series = series();
        assert!(series.begin_load());
        assert!(!series.begin_load());
        series.finish_load(Ok(loaded(10..13, false))).unwrap();

        assert!(!series.loading());
        assert_eq!(hours(series.candles()), [10, 11, 12]);
        assert_eq!(hours(&series.shown()), [10, 11, 12, 13]);
    }

    #[test]
    fn live_candles_update_close_and_detect_gaps() {
        let mut series = series();
        series.finish_load(Ok(loaded(10..13, false))).unwrap();

        let mut open = candle(13);
        open.close = 999.0;
        assert_eq!(series.apply_live(open, false), LiveUpdate::Changed);
        assert_eq!(series.shown().last().unwrap().close, 999.0);

        assert_eq!(
            series.apply_live(candle(13), true),
            LiveUpdate::Closed(candle(13))
        );
        assert_eq!(hours(series.candles()), [10, 11, 12, 13]);
        assert_eq!(series.cached(), 4);

        // Hour 14 never arrived: a gap.
        assert_eq!(series.apply_live(candle(15), true), LiveUpdate::Reload);
        // A repeat of a known candle changes nothing.
        assert_eq!(series.apply_live(candle(12), true), LiveUpdate::Changed);
        assert_eq!(hours(series.candles()), [10, 11, 12, 13]);
    }

    #[test]
    fn reloading_keeps_older_candles_that_connect() {
        let mut series = series();
        series.finish_load(Ok(loaded(10..13, false))).unwrap();
        series.begin_load_older();
        series.finish_load_older(Ok(loaded(5..10, false))).unwrap();

        // After a reconnect, the newest window (12..16) is loaded again.
        series.begin_load();
        series.finish_load(Ok(loaded(12..16, false))).unwrap();
        assert_eq!(hours(series.candles()), (5..16).collect::<Vec<_>>());

        // A window that does not connect replaces what was there.
        series.finish_load(Ok(loaded(40..42, false))).unwrap();
        assert_eq!(hours(series.candles()), [40, 41]);
    }

    #[test]
    fn older_candles_go_in_front_until_the_history_is_complete() {
        let mut series = series();
        series.finish_load(Ok(loaded(10..13, false))).unwrap();

        assert_eq!(series.begin_load_older(), Some(10 * HOUR));
        assert_eq!(series.begin_load_older(), None); // already loading
        series.finish_load_older(Ok(loaded(5..10, false))).unwrap();
        assert_eq!(hours(series.candles()), (5..13).collect::<Vec<_>>());

        series.begin_load_older();
        series.finish_load_older(Ok(loaded(0..5, true))).unwrap();
        assert!(series.complete());
        assert_eq!(series.begin_load_older(), None);

        // Older candles that do not connect are dropped.
        let mut other = Series::new(key(), "1h".parse().unwrap());
        other.finish_load(Ok(loaded(10..13, false))).unwrap();
        other.begin_load_older();
        other.finish_load_older(Ok(loaded(0..3, true))).unwrap();
        assert_eq!(hours(other.candles()), [10, 11, 12]);
        assert!(!other.complete());
    }

    #[test]
    fn errors_end_the_load_and_are_passed_on() {
        let mut series = series();
        series.begin_load();
        let error = MarketError::Storage("disk full".into());
        assert_eq!(series.finish_load(Err(error.clone())), Err(error));
        assert!(!series.loading());
    }

    #[test]
    fn longer_timeframes_are_built_from_the_source() {
        let mut series = Series::new(key(), "3h".parse().unwrap());
        series.finish_load(Ok(loaded(1..8, false))).unwrap();

        // Hours 1 and 2 belong to a bucket that started before the data, so they are
        // skipped; 3–5 make a candle, and 6–8 one that is still open.
        let shown = series.shown();
        assert_eq!(hours(&shown), [3, 6]);
        assert_eq!(shown[0].volume, 3.0);
        assert_eq!(shown[1].close, candle(8).close);
    }
}
