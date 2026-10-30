//! Live market data over Binance's WebSocket streams.

use std::collections::BTreeSet;
use std::time::Duration;

use fitil_core::{Candle, Market, Ticker, Timeframe};
use futures::channel::mpsc;
use futures::{Sink, SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use super::{endpoints, wire};

/// With no message for this long the connection counts as dead. Ticker streams
/// update every second, so silence means the network is gone.
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);

const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(30);

/// Something to receive live updates for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Topic {
    /// 24-hour statistics for every symbol, sent each second for the ones that changed.
    AllTickers,
    /// The open candle of one symbol, sent every one or two seconds.
    Candles {
        symbol: String,
        timeframe: Timeframe,
    },
}

impl Topic {
    fn stream_name(&self) -> String {
        match self {
            Topic::AllTickers => "!miniTicker@arr".to_owned(),
            Topic::Candles { symbol, timeframe } => {
                format!("{}@kline_{timeframe}", symbol.to_lowercase())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum FeedCommand {
    Subscribe(Topic),
    Unsubscribe(Topic),
}

#[derive(Debug, Clone)]
pub enum FeedEvent {
    Connected,
    /// The connection dropped; the feed reconnects after `delay`.
    Reconnecting {
        attempt: u32,
        delay: Duration,
    },
    Tickers(Vec<Ticker>),
    Candle {
        symbol: String,
        timeframe: Timeframe,
        candle: Candle,
        /// Whether this is the candle's final state.
        closed: bool,
    },
}

/// Streams live data for `market` into `events` until either channel closes.
///
/// Connects once there is at least one topic. When the connection drops (Binance also
/// closes every connection after 24 hours), it reconnects with growing delays and
/// subscribes to every topic again. Updates sent while disconnected are lost, so after a
/// [`FeedEvent::Connected`] that follows [`FeedEvent::Reconnecting`], callers should
/// fetch whatever they may have missed.
pub async fn run_feed(
    market: Market,
    mut commands: mpsc::UnboundedReceiver<FeedCommand>,
    mut events: mpsc::Sender<FeedEvent>,
) {
    crate::install_crypto_provider();
    let url = endpoints(market).stream;
    let mut streams = BTreeSet::new();
    let mut next_id = 1;
    let mut attempt = 0;

    loop {
        while streams.is_empty() {
            let Some(command) = commands.next().await else {
                return;
            };
            apply(&mut streams, &command);
        }

        if let Ok((socket, _)) = tokio_tungstenite::connect_async(url).await {
            let (mut write, mut read) = socket.split();
            let all = streams.iter().cloned().collect();
            if send_request(&mut write, "SUBSCRIBE", all, &mut next_id)
                .await
                .is_ok()
            {
                attempt = 0;
                if events.send(FeedEvent::Connected).await.is_err() {
                    return;
                }
                loop {
                    tokio::select! {
                        message = tokio::time::timeout(IDLE_TIMEOUT, read.next()) => match message {
                            Ok(Some(Ok(Message::Text(text)))) => {
                                if let Some(event) = parse_message(text.as_str())
                                    && events.send(event).await.is_err()
                                {
                                    return;
                                }
                            }
                            // Pings are answered automatically while reading.
                            Ok(Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Binary(_) | Message::Frame(_)))) => {}
                            Ok(Some(Ok(Message::Close(_)) | Err(_)) | None) | Err(_) => break,
                        },
                        command = commands.next() => {
                            let Some(command) = command else {
                                return;
                            };
                            if let Some((method, stream)) = apply(&mut streams, &command)
                                && send_request(&mut write, method, vec![stream], &mut next_id).await.is_err()
                            {
                                break;
                            }
                        }
                    }
                }
            }
        }

        attempt += 1;
        let delay = reconnect_delay(attempt);
        if events
            .send(FeedEvent::Reconnecting { attempt, delay })
            .await
            .is_err()
        {
            return;
        }
        tokio::time::sleep(delay).await;
    }
}

/// Updates the subscribed stream names; returns the request to send if anything changed.
fn apply(streams: &mut BTreeSet<String>, command: &FeedCommand) -> Option<(&'static str, String)> {
    match command {
        FeedCommand::Subscribe(topic) => {
            let name = topic.stream_name();
            streams.insert(name.clone()).then_some(("SUBSCRIBE", name))
        }
        FeedCommand::Unsubscribe(topic) => {
            let name = topic.stream_name();
            streams.remove(&name).then_some(("UNSUBSCRIBE", name))
        }
    }
}

async fn send_request<S>(
    write: &mut S,
    method: &str,
    params: Vec<String>,
    next_id: &mut u64,
) -> Result<(), S::Error>
where
    S: Sink<Message> + Unpin,
{
    let request = serde_json::json!({ "method": method, "params": params, "id": *next_id });
    *next_id += 1;
    write.send(Message::text(request.to_string())).await
}

/// 1, 2, 4, 8, 16, then 30 seconds.
fn reconnect_delay(attempt: u32) -> Duration {
    let seconds = 1u64 << attempt.saturating_sub(1).min(5);
    Duration::from_secs(seconds).min(MAX_RECONNECT_DELAY)
}

fn parse_message(text: &str) -> Option<FeedEvent> {
    // Replies to SUBSCRIBE requests have no `stream` field and are skipped here.
    let envelope: wire::Envelope = serde_json::from_str(text).ok()?;
    if envelope.stream == "!miniTicker@arr" {
        let tickers: Vec<wire::MiniTicker> = serde_json::from_value(envelope.data).ok()?;
        let tickers = tickers
            .into_iter()
            .filter_map(|ticker| ticker.into_ticker().ok())
            .collect();
        Some(FeedEvent::Tickers(tickers))
    } else if envelope.stream.contains("@kline_") {
        let event: wire::KlineEvent = serde_json::from_value(envelope.data).ok()?;
        let update = event.into_stream_candle().ok()?;
        Some(FeedEvent::Candle {
            symbol: update.symbol,
            timeframe: update.timeframe,
            candle: update.candle,
            closed: update.closed,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPOT_STREAM: &str = include_str!("../../tests/fixtures/binance/spot_stream.json");
    const FUTURES_STREAM: &str = include_str!("../../tests/fixtures/binance/futures_stream.json");

    fn recorded_messages(json: &str) -> Vec<String> {
        let messages: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        messages.iter().map(|m| m.to_string()).collect()
    }

    #[test]
    fn recorded_messages_parse() {
        for json in [SPOT_STREAM, FUTURES_STREAM] {
            let messages = recorded_messages(json);

            match parse_message(&messages[0]) {
                Some(FeedEvent::Candle {
                    symbol,
                    timeframe,
                    candle,
                    closed,
                }) => {
                    assert_eq!(symbol, "BTCUSDT");
                    assert_eq!(timeframe.to_string(), "1m");
                    assert_eq!(candle.open_time % 60_000, 0);
                    assert!(candle.low <= candle.close && candle.close <= candle.high);
                    assert!(!closed);
                }
                other => panic!("expected a candle, got {other:?}"),
            }

            match parse_message(&messages[1]) {
                Some(FeedEvent::Tickers(tickers)) => {
                    assert_eq!(tickers.len(), 2);
                    assert!(tickers.iter().all(|t| t.last_price > 0.0));
                }
                other => panic!("expected tickers, got {other:?}"),
            }
        }
    }

    #[test]
    fn subscription_replies_are_ignored() {
        assert!(parse_message(r#"{"result":null,"id":1}"#).is_none());
        assert!(parse_message("not json").is_none());
    }

    #[test]
    fn stream_names_follow_binance() {
        let candles = Topic::Candles {
            symbol: "BTCUSDT".into(),
            timeframe: "1M".parse().unwrap(),
        };
        assert_eq!(candles.stream_name(), "btcusdt@kline_1M");
        assert_eq!(Topic::AllTickers.stream_name(), "!miniTicker@arr");
    }

    #[test]
    fn only_changes_produce_requests() {
        let mut streams = BTreeSet::new();
        let subscribe = FeedCommand::Subscribe(Topic::AllTickers);
        assert_eq!(
            apply(&mut streams, &subscribe),
            Some(("SUBSCRIBE", "!miniTicker@arr".to_owned()))
        );
        assert_eq!(apply(&mut streams, &subscribe), None);
        let unsubscribe = FeedCommand::Unsubscribe(Topic::AllTickers);
        assert_eq!(
            apply(&mut streams, &unsubscribe),
            Some(("UNSUBSCRIBE", "!miniTicker@arr".to_owned()))
        );
        assert_eq!(apply(&mut streams, &unsubscribe), None);
    }

    #[test]
    fn reconnect_delays_grow_then_cap() {
        let delays: Vec<u64> = (1..=8).map(|n| reconnect_delay(n).as_secs()).collect();
        assert_eq!(delays, [1, 2, 4, 8, 16, 30, 30, 30]);
    }
}
