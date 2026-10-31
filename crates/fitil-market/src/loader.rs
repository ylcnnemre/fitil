//! Loading candles: from the local cache first, from the exchange for whatever is missing.

use std::sync::{Arc, Mutex};

use fitil_core::{Candle, SeriesKey, Timeframe};
use fitil_exchange::{CandleQuery, Exchange};
use fitil_store::{SeriesRange, Store};

use crate::MarketError;

pub type SharedStore = Arc<Mutex<Store>>;

/// Candles of the displayed timeframe brought into memory by each load.
const WINDOW_BARS: i64 = 1000;

/// A cache this many requests behind is downloaded afresh instead of being filled.
const MAX_FILL_REQUESTS: i64 = 50;

/// A stretch of a series, ready to show.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    /// Closed candles, oldest first, without gaps.
    pub candles: Vec<Candle>,
    /// The candle still open on the exchange, when this load fetched it.
    pub open: Option<Candle>,
    /// Whether `candles` reaches back to the symbol's first candle.
    pub complete: bool,
    /// Candles in the cache for this series.
    pub cached: u64,
}

/// How far back in time one load reaches when `target` is displayed.
fn window_span(target: Timeframe) -> i64 {
    target.duration_ms().unwrap_or(31 * 86_400_000) * WINDOW_BARS
}

/// Brings a series up to date and returns its newest stretch.
///
/// With nothing cached, the newest page is downloaded. Otherwise only the candles closed
/// since the newest cached one are; a cache too far behind is downloaded afresh.
pub async fn open_series(
    exchange: Arc<dyn Exchange>,
    store: SharedStore,
    key: SeriesKey,
    target: Timeframe,
) -> Result<Loaded, MarketError> {
    let cached = with_store(&store, {
        let key = key.clone();
        move |store| store.range(&key)
    })
    .await?;

    let max = exchange.max_candles(key.market.kind);
    let request_span = key.timeframe.duration_ms().unwrap_or(31 * 86_400_000) * max as i64;
    match cached {
        Some(range) if exchange.now() - range.newest <= MAX_FILL_REQUESTS * request_span => {
            fill_forward(exchange.as_ref(), &store, &key, range.newest).await?;
        }
        stale => {
            if stale.is_some() {
                with_store(&store, {
                    let key = key.clone();
                    move |store| store.clear_series(&key)
                })
                .await?;
            }
            download_latest(exchange.as_ref(), &store, &key).await?;
        }
    }

    // The live feed only sends a candle when it trades, so fetch the open one now.
    let latest = exchange
        .candles(
            key.market.kind,
            &key.symbol,
            key.timeframe,
            CandleQuery::Latest { limit: 1 },
        )
        .await?;
    let now = exchange.now();
    let open = latest
        .into_iter()
        .find(|candle| key.timeframe.next_bucket(candle.open_time) > now);

    let span = window_span(target);
    with_store(&store, move |store| {
        let Some(range) = store.range(&key)? else {
            return Ok(Loaded {
                candles: Vec::new(),
                open,
                complete: false,
                cached: 0,
            });
        };
        let candles = store.candles(&key, range.newest - span + 1, range.newest)?;
        let complete = reaches_start(store, &key, range, &candles, range.newest)?;
        Ok(Loaded {
            candles,
            open,
            complete,
            cached: range.count,
        })
    })
    .await
}

/// The candles before `before`: from the cache when it has them, otherwise one more
/// page downloaded from the exchange.
pub async fn load_older(
    exchange: Arc<dyn Exchange>,
    store: SharedStore,
    key: SeriesKey,
    target: Timeframe,
    before: i64,
) -> Result<Loaded, MarketError> {
    let span = window_span(target);
    let cached = with_store(&store, {
        let key = key.clone();
        move |store| store.candles(&key, before - span, before - 1)
    })
    .await?;
    if cached.is_empty() {
        download_older(exchange.as_ref(), &store, &key).await?;
    }

    with_store(&store, move |store| {
        let Some(range) = store.range(&key)? else {
            return Ok(Loaded {
                candles: Vec::new(),
                open: None,
                complete: true,
                cached: 0,
            });
        };
        let candles = store.candles(&key, before - span, before - 1)?;
        let complete = reaches_start(store, &key, range, &candles, before)?;
        Ok(Loaded {
            candles,
            open: None,
            complete,
            cached: range.count,
        })
    })
    .await
}

pub async fn save_candle(
    store: SharedStore,
    key: SeriesKey,
    candle: Candle,
) -> Result<(), MarketError> {
    with_store(&store, move |store| store.save_candles(&key, &[candle])).await
}

/// Whether `candles` (or, if empty, the point `before`) is at the symbol's first candle.
fn reaches_start(
    store: &Store,
    key: &SeriesKey,
    range: SeriesRange,
    candles: &[Candle],
    before: i64,
) -> fitil_store::Result<bool> {
    let at_oldest = candles.first().map_or(before <= range.oldest, |candle| {
        candle.open_time == range.oldest
    });
    Ok(at_oldest && store.history_start(key)? == Some(range.oldest))
}

/// Saves the newest page of closed candles.
async fn download_latest(
    exchange: &dyn Exchange,
    store: &SharedStore,
    key: &SeriesKey,
) -> Result<(), MarketError> {
    let max = exchange.max_candles(key.market.kind);
    let page = exchange
        .candles(
            key.market.kind,
            &key.symbol,
            key.timeframe,
            CandleQuery::Latest { limit: max },
        )
        .await?;
    let reached_start = page.len() < max;
    let closed = closed_only(exchange, key.timeframe, page);
    with_store(store, {
        let key = key.clone();
        move |store| {
            store.save_candles(&key, &closed)?;
            if reached_start && let Some(first) = closed.first() {
                store.set_history_start(&key, first.open_time)?;
            }
            Ok(())
        }
    })
    .await
}

/// Downloads and saves the closed candles after `newest`.
async fn fill_forward(
    exchange: &dyn Exchange,
    store: &SharedStore,
    key: &SeriesKey,
    mut newest: i64,
) -> Result<(), MarketError> {
    let max = exchange.max_candles(key.market.kind);
    loop {
        let start = key.timeframe.next_bucket(newest);
        if key.timeframe.next_bucket(start) > exchange.now() {
            // The candle after `newest` is still open.
            return Ok(());
        }
        let page = exchange
            .candles(
                key.market.kind,
                &key.symbol,
                key.timeframe,
                CandleQuery::From { start, limit: max },
            )
            .await?;
        let full = page.len() == max;
        let closed = closed_only(exchange, key.timeframe, page);
        let Some(last) = closed.last() else {
            return Ok(());
        };
        newest = last.open_time;
        with_store(store, {
            let key = key.clone();
            move |store| store.save_candles(&key, &closed)
        })
        .await?;
        if !full {
            return Ok(());
        }
    }
}

/// Downloads one page of candles before the oldest cached one, unless the cache
/// already reaches the symbol's first candle.
async fn download_older(
    exchange: &dyn Exchange,
    store: &SharedStore,
    key: &SeriesKey,
) -> Result<(), MarketError> {
    let (range, start) = with_store(store, {
        let key = key.clone();
        move |store| Ok((store.range(&key)?, store.history_start(&key)?))
    })
    .await?;
    let Some(range) = range else {
        return Ok(());
    };
    if start == Some(range.oldest) {
        return Ok(());
    }

    let max = exchange.max_candles(key.market.kind);
    let page = exchange
        .candles(
            key.market.kind,
            &key.symbol,
            key.timeframe,
            CandleQuery::Until {
                end: range.oldest - 1,
                limit: max,
            },
        )
        .await?;
    let reached_start = page.len() < max;
    let first = page.first().map_or(range.oldest, |candle| candle.open_time);
    with_store(store, {
        let key = key.clone();
        move |store| {
            store.save_candles(&key, &page)?;
            if reached_start {
                store.set_history_start(&key, first)?;
            }
            Ok(())
        }
    })
    .await
}

fn closed_only(
    exchange: &dyn Exchange,
    timeframe: Timeframe,
    mut candles: Vec<Candle>,
) -> Vec<Candle> {
    let now = exchange.now();
    candles.retain(|candle| timeframe.next_bucket(candle.open_time) <= now);
    candles
}

/// Runs a cache operation on a blocking thread so the async runtime stays responsive.
async fn with_store<T: Send + 'static>(
    store: &SharedStore,
    operation: impl FnOnce(&mut Store) -> fitil_store::Result<T> + Send + 'static,
) -> Result<T, MarketError> {
    let store = Arc::clone(store);
    tokio::task::spawn_blocking(move || {
        let mut store = store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        operation(&mut store)
    })
    .await
    .map_err(|error| MarketError::Storage(error.to_string()))?
    .map_err(MarketError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{FakeExchange, HOUR, key};

    fn store() -> SharedStore {
        Arc::new(Mutex::new(Store::open_in_memory().unwrap()))
    }

    fn hour() -> Timeframe {
        "1h".parse().unwrap()
    }

    fn contiguous(candles: &[Candle]) -> bool {
        candles
            .windows(2)
            .all(|pair| pair[1].open_time - pair[0].open_time == HOUR)
    }

    #[tokio::test]
    async fn first_open_downloads_the_newest_page() {
        // 100 hourly candles, the last still open; the exchange returns 10 per request.
        let exchange = Arc::new(FakeExchange::hourly(100, 10));
        let loaded = open_series(exchange.clone(), store(), key(), hour())
            .await
            .unwrap();

        assert_eq!(loaded.candles.len(), 9); // the newest page, minus the open candle
        assert_eq!(loaded.cached, 9);
        assert_eq!(loaded.candles.last().unwrap().open_time, 98 * HOUR);
        assert_eq!(loaded.open.unwrap().open_time, 99 * HOUR);
        assert!(!loaded.complete);
    }

    #[tokio::test]
    async fn reopening_downloads_only_what_closed_since() {
        let exchange = Arc::new(FakeExchange::hourly(100, 10));
        let store = store();
        open_series(exchange.clone(), store.clone(), key(), hour())
            .await
            .unwrap();

        // Five hours pass, as if the app (or the connection) had been away.
        exchange.advance(5);
        let requests_before = exchange.requests();
        let loaded = open_series(exchange.clone(), store, key(), hour())
            .await
            .unwrap();

        assert_eq!(loaded.cached, 14);
        assert_eq!(loaded.candles.last().unwrap().open_time, 103 * HOUR);
        assert!(contiguous(&loaded.candles));
        // One page for the gap, one for the open candle.
        assert_eq!(exchange.requests() - requests_before, 2);
    }

    #[tokio::test]
    async fn a_cache_far_behind_is_downloaded_afresh() {
        let exchange = Arc::new(FakeExchange::hourly(100, 10));
        let store = store();
        open_series(exchange.clone(), store.clone(), key(), hour())
            .await
            .unwrap();

        exchange.advance(10 * 51); // more than 50 requests' worth
        let loaded = open_series(exchange.clone(), store, key(), hour())
            .await
            .unwrap();
        assert_eq!(loaded.cached, 9);
        assert_eq!(loaded.candles.last().unwrap().open_time, (98 + 510) * HOUR);
    }

    #[tokio::test]
    async fn older_pages_come_from_the_cache_then_the_exchange_until_the_start() {
        let exchange = Arc::new(FakeExchange::hourly(25, 10));
        let store = store();
        let first = open_series(exchange.clone(), store.clone(), key(), hour())
            .await
            .unwrap();
        let mut oldest = first.candles[0].open_time;
        assert_eq!(oldest, 15 * HOUR);

        let page = load_older(exchange.clone(), store.clone(), key(), hour(), oldest)
            .await
            .unwrap();
        assert_eq!(page.candles.len(), 10);
        assert_eq!(page.candles.last().unwrap().open_time + HOUR, oldest);
        assert!(!page.complete);
        oldest = page.candles[0].open_time;

        // Only five candles are left before hour 5: the exchange's history starts there.
        let page = load_older(exchange.clone(), store.clone(), key(), hour(), oldest)
            .await
            .unwrap();
        assert_eq!(page.candles.len(), 5);
        assert_eq!(page.candles[0].open_time, 0);
        assert!(page.complete);

        // Asking again is answered from the cache, without a request.
        let requests = exchange.requests();
        let again = load_older(exchange.clone(), store, key(), hour(), oldest)
            .await
            .unwrap();
        assert_eq!(again, page);
        assert_eq!(exchange.requests(), requests);
    }
}
