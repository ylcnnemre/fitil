//! Checks against the real Binance API, through the shared `Exchange` interface. They
//! need internet access, so they are skipped by default; run them with
//! `cargo test -p fitil-binance -- --ignored`.

use std::time::Duration;

use fitil_binance::Binance;
use fitil_exchange::{CandleQuery, Exchange, FeedCommand, FeedEvent, Topic};
use futures::StreamExt;
use futures::channel::mpsc;

#[tokio::test]
#[ignore = "needs internet access"]
async fn rest_api_answers_for_both_markets() {
    let binance = Binance::new().unwrap();
    binance.sync_clock().await.unwrap();
    let offset = binance.clock_offset().unwrap();
    assert!(offset.abs() < 60_000, "clock offset {offset} ms");

    for &market in binance.markets() {
        let symbols = binance.symbols(market).await.unwrap();
        assert!(symbols.iter().any(|s| s.name == "BTCUSDT"), "{market:?}");

        let tickers = binance.tickers(market).await.unwrap();
        assert!(tickers.iter().any(|t| t.symbol == "BTCUSDT"), "{market:?}");

        let hour = "1h".parse().unwrap();
        let latest = binance
            .candles(market, "BTCUSDT", hour, CandleQuery::Latest { limit: 5 })
            .await
            .unwrap();
        assert_eq!(latest.len(), 5, "{market:?}");
        let end = latest[0].open_time - 1;
        let before = binance
            .candles(
                market,
                "BTCUSDT",
                hour,
                CandleQuery::Until { end, limit: 5 },
            )
            .await
            .unwrap();
        assert_eq!(
            before.last().unwrap().open_time,
            latest[0].open_time - 3_600_000
        );

        let usage = binance.usage(market).unwrap();
        assert!(
            usage.used > 0 && usage.used < usage.limit,
            "{market:?}: {usage:?}"
        );
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "needs internet access"]
async fn feed_delivers_tickers_and_candles() {
    let binance = Binance::new().unwrap();
    for &market in binance.markets() {
        let (commands, command_receiver) = mpsc::unbounded();
        let (event_sender, mut events) = mpsc::channel(64);
        let feed = tokio::spawn(binance.feed(market, command_receiver, event_sender));
        commands
            .unbounded_send(FeedCommand::Subscribe(Topic::AllTickers))
            .unwrap();
        commands
            .unbounded_send(FeedCommand::Subscribe(Topic::Candles {
                symbol: "BTCUSDT".into(),
                timeframe: "1m".parse().unwrap(),
            }))
            .unwrap();

        let (mut connected, mut tickers, mut candles) = (false, false, false);
        let deadline = tokio::time::sleep(Duration::from_secs(20));
        tokio::pin!(deadline);
        while !(connected && tickers && candles) {
            tokio::select! {
                event = events.next() => match event.expect("feed stopped") {
                    FeedEvent::Connected => connected = true,
                    FeedEvent::Tickers(list) => tickers |= !list.is_empty(),
                    FeedEvent::Candle { symbol, .. } => candles |= symbol == "BTCUSDT",
                    FeedEvent::Reconnecting { attempt, .. } => panic!("{market:?}: reconnecting ({attempt})"),
                },
                _ = &mut deadline => panic!("{market:?}: connected={connected} tickers={tickers} candles={candles}"),
            }
        }

        drop(events);
        drop(commands);
        tokio::time::timeout(Duration::from_secs(5), feed)
            .await
            .expect("feed should stop once its channels close")
            .unwrap();
    }
}
