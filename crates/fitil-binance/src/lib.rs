//! The Binance adapter: Spot and USDⓈ-M perpetual futures market data.

mod rest;
mod stream;
mod wire;

use std::sync::Arc;

use async_trait::async_trait;
use fitil_core::{Candle, ExchangeId, MarketKind, Symbol, Ticker, TimeUnit, Timeframe};
use fitil_exchange::stream::run_stream;
use fitil_exchange::{
    CandleQuery, Clock, Exchange, ExchangeError, Features, FeedCommand, FeedEvent, RequestUsage,
    http_client,
};
use futures::channel::mpsc;
use futures::future::BoxFuture;

use rest::RestClient;
use stream::BinanceStream;

pub const ID: ExchangeId = ExchangeId::new("binance");

/// Most candles requested at once. Futures allow 1500, but 1000 costs less weight per candle.
const MAX_CANDLES: usize = 1000;

const MARKETS: [MarketKind; 2] = [MarketKind::Spot, MarketKind::LinearPerpetual];

const fn tf(count: u32, unit: TimeUnit) -> Timeframe {
    Timeframe::new(count, unit).unwrap()
}

static TIMEFRAMES: [Timeframe; 16] = [
    tf(1, TimeUnit::Second),
    tf(1, TimeUnit::Minute),
    tf(3, TimeUnit::Minute),
    tf(5, TimeUnit::Minute),
    tf(15, TimeUnit::Minute),
    tf(30, TimeUnit::Minute),
    tf(1, TimeUnit::Hour),
    tf(2, TimeUnit::Hour),
    tf(4, TimeUnit::Hour),
    tf(6, TimeUnit::Hour),
    tf(8, TimeUnit::Hour),
    tf(12, TimeUnit::Hour),
    tf(1, TimeUnit::Day),
    tf(3, TimeUnit::Day),
    tf(1, TimeUnit::Week),
    tf(1, TimeUnit::Month),
];

pub struct Binance {
    clock: Arc<Clock>,
    spot: RestClient,
    futures: RestClient,
}

impl Binance {
    pub fn new() -> Result<Self, ExchangeError> {
        let http = http_client()?;
        let clock = Arc::new(Clock::default());
        Ok(Self {
            spot: RestClient::new(MarketKind::Spot, http.clone(), clock.clone()),
            futures: RestClient::new(MarketKind::LinearPerpetual, http, clock.clone()),
            clock,
        })
    }

    fn rest(&self, market: MarketKind) -> &RestClient {
        match market {
            MarketKind::Spot => &self.spot,
            MarketKind::LinearPerpetual => &self.futures,
        }
    }
}

#[async_trait]
impl Exchange for Binance {
    fn id(&self) -> ExchangeId {
        ID
    }

    fn name(&self) -> &'static str {
        "Binance"
    }

    fn markets(&self) -> &'static [MarketKind] {
        &MARKETS
    }

    fn timeframes(&self, market: MarketKind) -> &'static [Timeframe] {
        match market {
            MarketKind::Spot => &TIMEFRAMES,
            // Futures have no 1-second candles.
            MarketKind::LinearPerpetual => &TIMEFRAMES[1..],
        }
    }

    fn max_candles(&self, _market: MarketKind) -> usize {
        MAX_CANDLES
    }

    fn features(&self) -> Features {
        Features {
            taker_volume: true,
            full_history: true,
        }
    }

    fn now(&self) -> i64 {
        self.clock.now()
    }

    fn clock_offset(&self) -> Option<i64> {
        Some(self.clock.offset())
    }

    fn usage(&self, market: MarketKind) -> Option<RequestUsage> {
        Some(self.rest(market).usage())
    }

    async fn sync_clock(&self) -> Result<(), ExchangeError> {
        self.spot.sync_clock().await
    }

    async fn symbols(&self, market: MarketKind) -> Result<Vec<Symbol>, ExchangeError> {
        self.rest(market).symbols().await
    }

    async fn tickers(&self, market: MarketKind) -> Result<Vec<Ticker>, ExchangeError> {
        self.rest(market).tickers().await
    }

    async fn candles(
        &self,
        market: MarketKind,
        symbol: &str,
        timeframe: Timeframe,
        query: CandleQuery,
    ) -> Result<Vec<Candle>, ExchangeError> {
        self.rest(market).candles(symbol, timeframe, query).await
    }

    fn feed(
        &self,
        market: MarketKind,
        commands: mpsc::UnboundedReceiver<FeedCommand>,
        events: mpsc::Sender<FeedEvent>,
    ) -> BoxFuture<'static, ()> {
        Box::pin(run_stream(BinanceStream::new(market), commands, events))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn futures_have_no_one_second_candles() {
        let binance = Binance::new().unwrap();
        let names = |market| -> Vec<String> {
            binance
                .timeframes(market)
                .iter()
                .map(|t| t.to_string())
                .collect()
        };
        let spot = names(MarketKind::Spot);
        let futures = names(MarketKind::LinearPerpetual);
        assert_eq!(spot.first().map(String::as_str), Some("1s"));
        assert_eq!(futures.first().map(String::as_str), Some("1m"));
        assert_eq!(spot.len(), futures.len() + 1);
        assert_eq!(spot.last().map(String::as_str), Some("1M"));
    }
}
