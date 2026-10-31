//! The whole loading path against the real Binance API: the application layer
//! (`fitil-market`) driving the Binance adapter through the shared `Exchange` interface,
//! with an in-memory cache. Needs internet access, so it is skipped by default; run it
//! with `cargo test -p fitil-app -- --ignored`.

use std::sync::{Arc, Mutex};

use fitil_binance::Binance;
use fitil_core::{Candle, Market, MarketKind, SeriesKey, Timeframe};
use fitil_exchange::Exchange;
use fitil_market::{SharedStore, load_older, open_series};
use fitil_store::Store;

#[tokio::test]
#[ignore = "needs internet access"]
async fn open_reopen_and_page_back_against_binance() {
    let binance: Arc<dyn Exchange> = Arc::new(Binance::new().unwrap());
    binance.sync_clock().await.unwrap();
    let cache: SharedStore = Arc::new(Mutex::new(Store::open_in_memory().unwrap()));
    let source: Timeframe = "15m".parse().unwrap();
    let target: Timeframe = "45m".parse().unwrap();
    let step = source.duration_ms().unwrap();
    let key = SeriesKey {
        market: Market {
            exchange: fitil_binance::ID,
            kind: MarketKind::Spot,
        },
        symbol: "BTCUSDT".into(),
        timeframe: source,
    };
    let contiguous = |candles: &[Candle]| {
        candles
            .windows(2)
            .all(|pair| pair[1].open_time - pair[0].open_time == step)
    };

    // First open: one page downloaded; the open candle comes separately.
    let first = open_series(binance.clone(), cache.clone(), key.clone(), target)
        .await
        .unwrap();
    let count = first.candles.len();
    assert!((999..=1000).contains(&count), "{count}");
    assert_eq!(first.cached, count as u64);
    assert!(contiguous(&first.candles));
    let newest = first.candles.last().unwrap().open_time;
    assert_eq!(first.open.unwrap().open_time, source.next_bucket(newest));
    assert!(!first.complete);

    // Opening again uses the cache and adds at most a candle that just closed.
    let again = open_series(binance.clone(), cache.clone(), key.clone(), target)
        .await
        .unwrap();
    assert!(again.cached == first.cached || again.cached == first.cached + 1);

    // Going back downloads a full page that ends right before the first candle.
    let oldest = first.candles[0].open_time;
    let older = load_older(binance, cache, key, target, oldest)
        .await
        .unwrap();
    assert_eq!(older.candles.len(), 1000);
    assert_eq!(older.candles.last().unwrap().open_time + step, oldest);
    assert!(contiguous(&older.candles));
}
