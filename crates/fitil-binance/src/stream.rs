//! Binance's WebSocket protocol. The reconnecting loop itself is shared
//! (`fitil_exchange::stream`).

use fitil_core::MarketKind;
use fitil_exchange::stream::{Incoming, Message, StreamProtocol};
use fitil_exchange::{FeedEvent, Topic};

use crate::wire;

/// Binance's combined-stream protocol: `SUBSCRIBE` / `UNSUBSCRIBE` requests, and
/// messages wrapped as `{"stream": …, "data": …}`. Binance pings and the WebSocket
/// library answers by itself.
pub struct BinanceStream {
    url: &'static str,
    next_id: u64,
}

impl BinanceStream {
    pub fn new(market: MarketKind) -> Self {
        let url = match market {
            MarketKind::Spot => "wss://stream.binance.com:9443/stream",
            // Since 2026-04-23, futures market streams are only served under `/market`.
            MarketKind::LinearPerpetual => "wss://fstream.binance.com/market/stream",
        };
        Self { url, next_id: 1 }
    }
}

impl StreamProtocol for BinanceStream {
    fn url(&self) -> String {
        self.url.to_owned()
    }

    fn stream_name(&self, topic: &Topic) -> String {
        match topic {
            // Every second, for the symbols that changed.
            Topic::AllTickers => "!miniTicker@arr".to_owned(),
            // Every one or two seconds, but only while the symbol trades.
            Topic::Candles { symbol, timeframe } => {
                format!("{}@kline_{timeframe}", symbol.to_lowercase())
            }
        }
    }

    fn request(&mut self, subscribe: bool, streams: &[String]) -> Vec<Message> {
        let method = if subscribe {
            "SUBSCRIBE"
        } else {
            "UNSUBSCRIBE"
        };
        let request =
            serde_json::json!({ "method": method, "params": streams, "id": self.next_id });
        self.next_id += 1;
        vec![Message::text(request.to_string())]
    }

    fn read(&mut self, message: &Message) -> Incoming {
        match message {
            Message::Text(text) => parse_message(text.as_str())
                .map(Incoming::event)
                .unwrap_or_default(),
            _ => Incoming::default(),
        }
    }
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

    const SPOT_STREAM: &str = include_str!("../tests/fixtures/binance/spot_stream.json");
    const FUTURES_STREAM: &str = include_str!("../tests/fixtures/binance/futures_stream.json");

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
                    assert!(candle.taker_buy_volume.is_some());
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
    fn stream_names_and_requests_follow_binance() {
        let mut protocol = BinanceStream::new(MarketKind::LinearPerpetual);
        let candles = Topic::Candles {
            symbol: "BTCUSDT".into(),
            timeframe: "1M".parse().unwrap(),
        };
        assert_eq!(protocol.stream_name(&candles), "btcusdt@kline_1M");
        assert_eq!(protocol.stream_name(&Topic::AllTickers), "!miniTicker@arr");
        assert_eq!(protocol.url(), "wss://fstream.binance.com/market/stream");

        let requests = protocol.request(true, &["a".into(), "b".into()]);
        let Message::Text(first) = &requests[0] else {
            panic!("expected a text request");
        };
        assert_eq!(
            first.as_str(),
            r#"{"id":1,"method":"SUBSCRIBE","params":["a","b"]}"#
        );
        let Message::Text(second) = &protocol.request(false, &["a".into()])[0] else {
            panic!("expected a text request");
        };
        assert_eq!(
            second.as_str(),
            r#"{"id":2,"method":"UNSUBSCRIBE","params":["a"]}"#
        );
    }
}
