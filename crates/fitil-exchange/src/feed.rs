use std::time::Duration;

use fitil_core::{Candle, Ticker, Timeframe};

/// Something to receive live updates for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Topic {
    /// 24-hour statistics for every symbol of the market.
    AllTickers,
    /// The open candle of one symbol.
    Candles {
        symbol: String,
        timeframe: Timeframe,
    },
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
