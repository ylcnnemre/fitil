//! Loading market data: symbol lists from the exchange, candles from the local cache
//! first and from the exchange for whatever is missing.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use fitil_core::{Candle, SeriesKey, Symbol, Ticker, Timeframe};
use fitil_exchange::binance::{BinanceClient, MAX_CANDLES_PER_REQUEST};
use fitil_store::{SeriesRange, Store};

use crate::error::AppError;

pub type SharedStore = Arc<Mutex<Store>>;

/// A cache this many requests behind is downloaded afresh instead of being filled.
const MAX_FILL_REQUESTS: i64 = 50;

/// Where the candle cache lives: the local (non-roaming) app data folder.
pub fn cache_path() -> Option<PathBuf> {
    let folder = if cfg!(target_os = "linux") {
        "fitil"
    } else {
        "Fitil"
    };
    directories::BaseDirs::new().map(|dirs| dirs.data_local_dir().join(folder).join("cache.sqlite"))
}

pub fn open_store() -> Result<SharedStore, AppError> {
    let path = cache_path().ok_or_else(|| AppError::Storage("no data folder".into()))?;
    Ok(Arc::new(Mutex::new(Store::open(&path)?)))
}

#[derive(Debug, Clone)]
pub struct MarketSnapshot {
    pub symbols: Vec<Symbol>,
    pub tickers: Vec<Ticker>,
}

pub async fn load_market(client: Arc<BinanceClient>) -> Result<MarketSnapshot, AppError> {
    client.sync_clock().await?;
    let info = client.exchange_info().await?;
    let tickers = client.tickers().await?;
    Ok(MarketSnapshot {
        symbols: info.symbols,
        tickers,
    })
}

/// Candles of the displayed timeframe brought into memory by each load.
const WINDOW_BARS: i64 = 1000;

/// How far back in time one load reaches for `target`.
fn window_span(target: Timeframe) -> i64 {
    target.duration_ms().unwrap_or(31 * 86_400_000) * WINDOW_BARS
}

/// A stretch of a series, ready to show.
#[derive(Debug, Clone)]
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

/// Brings a series up to date: downloads the newest candles if nothing is cached,
/// otherwise fills in what was missed since the newest cached candle.
pub async fn open_series(
    client: Arc<BinanceClient>,
    store: SharedStore,
    key: SeriesKey,
    target: Timeframe,
) -> Result<Loaded, AppError> {
    let cached = with_store(&store, {
        let key = key.clone();
        move |store| store.range(&key)
    })
    .await?;

    let request_span =
        key.timeframe.duration_ms().unwrap_or(31 * 86_400_000) * MAX_CANDLES_PER_REQUEST as i64;
    match cached {
        Some(range) if client.now() - range.newest <= MAX_FILL_REQUESTS * request_span => {
            fill_forward(&client, &store, &key, range.newest).await?;
        }
        stale => {
            if stale.is_some() {
                with_store(&store, {
                    let key = key.clone();
                    move |store| store.clear_series(&key)
                })
                .await?;
            }
            download_latest(&client, &store, &key).await?;
        }
    }

    // The live feed only sends a candle when it trades, so fetch the open one now.
    let latest = client
        .candles(&key.symbol, key.timeframe, None, None, 1)
        .await?;
    let now = client.now();
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
    client: Arc<BinanceClient>,
    store: SharedStore,
    key: SeriesKey,
    target: Timeframe,
    before: i64,
) -> Result<Loaded, AppError> {
    let span = window_span(target);
    let cached = with_store(&store, {
        let key = key.clone();
        move |store| store.candles(&key, before - span, before - 1)
    })
    .await?;
    if cached.is_empty() {
        download_older(&client, &store, &key).await?;
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

/// Downloads one page of candles before the oldest cached one, unless the cache
/// already reaches the symbol's first candle.
async fn download_older(
    client: &BinanceClient,
    store: &SharedStore,
    key: &SeriesKey,
) -> Result<(), AppError> {
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

    let page = client
        .candles(
            &key.symbol,
            key.timeframe,
            None,
            Some(range.oldest - 1),
            MAX_CANDLES_PER_REQUEST,
        )
        .await?;
    let reached_start = page.len() < MAX_CANDLES_PER_REQUEST;
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

pub async fn save_candle(
    store: SharedStore,
    key: SeriesKey,
    candle: Candle,
) -> Result<(), AppError> {
    with_store(&store, move |store| store.save_candles(&key, &[candle])).await
}

/// Saves the newest page of closed candles.
async fn download_latest(
    client: &BinanceClient,
    store: &SharedStore,
    key: &SeriesKey,
) -> Result<(), AppError> {
    let page = client
        .candles(
            &key.symbol,
            key.timeframe,
            None,
            None,
            MAX_CANDLES_PER_REQUEST,
        )
        .await?;
    let reached_start = page.len() < MAX_CANDLES_PER_REQUEST;
    let closed = closed_only(client, key.timeframe, page);
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
    client: &BinanceClient,
    store: &SharedStore,
    key: &SeriesKey,
    mut newest: i64,
) -> Result<(), AppError> {
    loop {
        let start = key.timeframe.next_bucket(newest);
        if key.timeframe.next_bucket(start) > client.now() {
            // The candle after `newest` is still open.
            return Ok(());
        }
        let page = client
            .candles(
                &key.symbol,
                key.timeframe,
                Some(start),
                None,
                MAX_CANDLES_PER_REQUEST,
            )
            .await?;
        let full = page.len() == MAX_CANDLES_PER_REQUEST;
        let closed = closed_only(client, key.timeframe, page);
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

fn closed_only(
    client: &BinanceClient,
    timeframe: Timeframe,
    mut candles: Vec<Candle>,
) -> Vec<Candle> {
    let now = client.now();
    candles.retain(|candle| timeframe.next_bucket(candle.open_time) <= now);
    candles
}

/// Runs a cache operation on a blocking thread so the UI stays responsive.
async fn with_store<T: Send + 'static>(
    store: &SharedStore,
    operation: impl FnOnce(&mut Store) -> fitil_store::Result<T> + Send + 'static,
) -> Result<T, AppError> {
    let store = Arc::clone(store);
    tokio::task::spawn_blocking(move || {
        let mut store = store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        operation(&mut store)
    })
    .await
    .map_err(|error| AppError::Storage(error.to_string()))?
    .map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fitil_core::Market;

    /// Runs the whole load cycle against the real Binance API with an in-memory cache.
    /// Run it with `cargo test -p fitil-app -- --ignored`.
    #[tokio::test]
    #[ignore = "needs internet access"]
    async fn open_reopen_and_page_back_against_binance() {
        let client = Arc::new(BinanceClient::new(Market::Spot).unwrap());
        client.sync_clock().await.unwrap();
        let store: SharedStore = Arc::new(Mutex::new(Store::open_in_memory().unwrap()));
        let source: Timeframe = "15m".parse().unwrap();
        let target: Timeframe = "45m".parse().unwrap();
        let step = source.duration_ms().unwrap();
        let key = SeriesKey {
            market: Market::Spot,
            symbol: "BTCUSDT".into(),
            timeframe: source,
        };
        let contiguous = |candles: &[Candle]| {
            candles
                .windows(2)
                .all(|pair| pair[1].open_time - pair[0].open_time == step)
        };

        // First open: nothing cached, one page downloaded; the open candle comes separately.
        let first = open_series(client.clone(), store.clone(), key.clone(), target)
            .await
            .unwrap();
        let count = first.candles.len();
        assert!((999..=1000).contains(&count), "{count}");
        assert_eq!(first.cached, count as u64);
        assert!(contiguous(&first.candles));
        let newest = first.candles.last().unwrap().open_time;
        let open = first.open.expect("the open candle is fetched");
        assert_eq!(open.open_time, source.next_bucket(newest));
        assert!(!first.complete);

        // Opening again uses the cache and adds at most a candle that just closed.
        let again = open_series(client.clone(), store.clone(), key.clone(), target)
            .await
            .unwrap();
        assert!(again.cached == first.cached || again.cached == first.cached + 1);
        assert!(contiguous(&again.candles));

        // Going back downloads a full page that ends right before the first candle.
        let oldest = first.candles[0].open_time;
        let older = load_older(client.clone(), store.clone(), key.clone(), target, oldest)
            .await
            .unwrap();
        assert_eq!(older.candles.len(), MAX_CANDLES_PER_REQUEST);
        assert_eq!(older.candles.last().unwrap().open_time + step, oldest);
        assert!(contiguous(&older.candles));
        assert!(!older.complete);

        // Asking again is answered from the cache.
        let cached_before = older.cached;
        let again = load_older(client, store, key, target, oldest)
            .await
            .unwrap();
        assert_eq!(again.cached, cached_before);
        assert_eq!(again.candles, older.candles);
    }
}
