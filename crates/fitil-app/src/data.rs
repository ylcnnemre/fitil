//! Loading market data: symbol lists from the exchange, candles from the local cache
//! first and from the exchange for whatever is missing.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use fitil_core::{Candle, SeriesKey, Symbol, Ticker, Timeframe, aggregate};
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

/// The state of a cached series after a load.
#[derive(Debug, Clone)]
pub struct Loaded {
    /// Candles that were already cached before this load.
    pub from_cache: u64,
    /// Candles downloaded by this load.
    pub downloaded: u64,
    pub range: Option<SeriesRange>,
    /// How many candles of the displayed timeframe the cache makes up.
    pub target_count: u64,
    /// Whether the cache reaches back to the symbol's first candle.
    pub complete: bool,
    /// Closed candles inside the displayed timeframe's current candle.
    pub recent: Vec<Candle>,
    /// The candle still open on the exchange, when this load fetched it.
    pub open: Option<Candle>,
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
    let (from_cache, downloaded) = match cached {
        Some(range) if client.now() - range.newest <= MAX_FILL_REQUESTS * request_span => {
            let downloaded = fill_forward(&client, &store, &key, range.newest).await?;
            (range.count, downloaded)
        }
        stale => {
            if stale.is_some() {
                with_store(&store, {
                    let key = key.clone();
                    move |store| store.clear_series(&key)
                })
                .await?;
            }
            (0, download_latest(&client, &store, &key).await?)
        }
    };

    // The live feed only sends a candle when it trades, so fetch the open one now.
    let latest = client
        .candles(&key.symbol, key.timeframe, None, None, 1)
        .await?;
    let now = client.now();
    let open = latest
        .into_iter()
        .find(|candle| key.timeframe.next_bucket(candle.open_time) > now);

    let mut loaded = summarize(&client, &store, key, target, from_cache, downloaded).await?;
    loaded.open = open;
    Ok(loaded)
}

/// Downloads one more page of candles before the oldest cached one.
pub async fn load_older(
    client: Arc<BinanceClient>,
    store: SharedStore,
    key: SeriesKey,
    target: Timeframe,
) -> Result<Loaded, AppError> {
    let (range, start) = with_store(&store, {
        let key = key.clone();
        move |store| Ok((store.range(&key)?, store.history_start(&key)?))
    })
    .await?;
    let Some(range) = range else {
        return open_series(client, store, key, target).await;
    };
    if start == Some(range.oldest) {
        return summarize(&client, &store, key, target, range.count, 0).await;
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
    let downloaded = page.len() as u64;
    let reached_start = page.len() < MAX_CANDLES_PER_REQUEST;
    let first = page.first().map_or(range.oldest, |candle| candle.open_time);
    with_store(&store, {
        let key = key.clone();
        move |store| {
            store.save_candles(&key, &page)?;
            if reached_start {
                store.set_history_start(&key, first)?;
            }
            Ok(())
        }
    })
    .await?;

    summarize(&client, &store, key, target, range.count, downloaded).await
}

pub async fn save_candle(
    store: SharedStore,
    key: SeriesKey,
    candle: Candle,
) -> Result<(), AppError> {
    with_store(&store, move |store| store.save_candles(&key, &[candle])).await
}

/// Saves the newest page of closed candles and returns how many were saved.
async fn download_latest(
    client: &BinanceClient,
    store: &SharedStore,
    key: &SeriesKey,
) -> Result<u64, AppError> {
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
    let saved = closed.len() as u64;
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
    .await?;
    Ok(saved)
}

/// Downloads the closed candles after `newest` and returns how many were saved.
async fn fill_forward(
    client: &BinanceClient,
    store: &SharedStore,
    key: &SeriesKey,
    mut newest: i64,
) -> Result<u64, AppError> {
    let mut saved = 0;
    loop {
        let start = key.timeframe.next_bucket(newest);
        if key.timeframe.next_bucket(start) > client.now() {
            // The candle after `newest` is still open.
            return Ok(saved);
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
            return Ok(saved);
        };
        newest = last.open_time;
        saved += closed.len() as u64;
        with_store(store, {
            let key = key.clone();
            move |store| store.save_candles(&key, &closed)
        })
        .await?;
        if !full {
            return Ok(saved);
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

async fn summarize(
    client: &BinanceClient,
    store: &SharedStore,
    key: SeriesKey,
    target: Timeframe,
    from_cache: u64,
    downloaded: u64,
) -> Result<Loaded, AppError> {
    let bucket = target.bucket_start(client.now());
    with_store(store, move |store| {
        let range = store.range(&key)?;
        let complete = match range {
            Some(range) => store.history_start(&key)? == Some(range.oldest),
            None => false,
        };
        let target_count = match range {
            Some(range) if target != key.timeframe => {
                let all = store.candles(&key, range.oldest, range.newest)?;
                aggregate(&all, target).len() as u64
            }
            Some(range) => range.count,
            None => 0,
        };
        let recent = store.candles(&key, bucket, i64::MAX)?;
        Ok(Loaded {
            from_cache,
            downloaded,
            range,
            target_count,
            complete,
            recent,
            open: None,
        })
    })
    .await
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
        let contiguous =
            |range: SeriesRange| (range.newest - range.oldest) / step + 1 == range.count as i64;

        // First open: nothing cached, one page downloaded; the open candle is left out.
        let first = open_series(client.clone(), store.clone(), key.clone(), target)
            .await
            .unwrap();
        let range = first.range.unwrap();
        assert_eq!(first.from_cache, 0);
        assert_eq!(first.downloaded, range.count);
        assert!((999..=1000).contains(&range.count), "{}", range.count);
        assert!(contiguous(range));
        let open = first.open.expect("the open candle is fetched");
        assert_eq!(open.open_time, source.next_bucket(range.newest));
        assert!((range.count / 3..=range.count / 3 + 2).contains(&first.target_count));
        assert!(!first.complete);

        // Opening again uses the cache and downloads at most a candle that just closed.
        let again = open_series(client.clone(), store.clone(), key.clone(), target)
            .await
            .unwrap();
        assert_eq!(again.from_cache, range.count);
        assert!(again.downloaded <= 1);
        assert!(contiguous(again.range.unwrap()));

        // Paging back adds a full page right before the oldest candle.
        let older = load_older(client.clone(), store.clone(), key.clone(), target)
            .await
            .unwrap();
        let older_range = older.range.unwrap();
        assert_eq!(older.downloaded, MAX_CANDLES_PER_REQUEST as u64);
        assert_eq!(
            older_range.oldest,
            range.oldest - MAX_CANDLES_PER_REQUEST as i64 * step
        );
        assert!(contiguous(older_range));
    }
}
