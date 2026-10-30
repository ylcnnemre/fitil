//! Checks against the real Binance API. They need internet access, so they are
//! skipped by default; run them with `cargo test -p fitil-exchange -- --ignored`.

use std::time::Duration;

use fitil_core::Market;
use fitil_exchange::binance::{BinanceClient, FeedCommand, FeedEvent, Topic, run_feed};
use futures::StreamExt;
use futures::channel::mpsc;

#[tokio::test]
#[ignore = "needs internet access"]
async fn rest_api_answers_for_both_markets() {
    for market in Market::ALL {
        let client = BinanceClient::new(market).unwrap();
        let offset = client.sync_clock().await.unwrap();
        assert!(
            offset.abs() < 60_000,
            "{market:?}: clock offset {offset} ms"
        );

        let info = client.exchange_info().await.unwrap();
        assert!(
            info.symbols.iter().any(|s| s.name == "BTCUSDT"),
            "{market:?}"
        );

        let tickers = client.tickers().await.unwrap();
        assert!(tickers.iter().any(|t| t.symbol == "BTCUSDT"), "{market:?}");

        let hour = "1h".parse().unwrap();
        let latest = client
            .candles("BTCUSDT", hour, None, None, 5)
            .await
            .unwrap();
        assert_eq!(latest.len(), 5, "{market:?}");
        let before = client
            .candles("BTCUSDT", hour, None, Some(latest[0].open_time - 1), 5)
            .await
            .unwrap();
        assert_eq!(
            before.last().unwrap().open_time,
            latest[0].open_time - 3_600_000
        );

        let (used, limit) = client.weight_usage();
        assert!(
            used > 0 && used < limit,
            "{market:?}: weight {used}/{limit}"
        );
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "needs internet access"]
async fn feed_delivers_tickers_and_candles() {
    for market in Market::ALL {
        let (commands, command_rx) = mpsc::unbounded();
        let (event_tx, mut events) = mpsc::channel(64);
        let feed = tokio::spawn(run_feed(market, command_rx, event_tx));
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
