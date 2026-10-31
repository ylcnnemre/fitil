//! A fake exchange for tests: one hourly series generated on demand, with a clock that
//! tests move forward.

use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};

use async_trait::async_trait;
use fitil_core::{
    Candle, ExchangeId, Market, MarketKind, SeriesKey, Symbol, Ticker, TimeUnit, Timeframe,
};
use fitil_exchange::{
    CandleQuery, Exchange, ExchangeError, Features, FeedCommand, FeedEvent, RequestUsage,
};
use futures::channel::mpsc;
use futures::future::BoxFuture;

pub const HOUR: i64 = 3_600_000;
pub const FAKE: ExchangeId = ExchangeId::new("fake");

static HOURLY: [Timeframe; 1] = [Timeframe::new(1, TimeUnit::Hour).unwrap()];

/// The fake exchange's only series.
pub fn key() -> SeriesKey {
    SeriesKey {
        market: Market {
            exchange: FAKE,
            kind: MarketKind::Spot,
        },
        symbol: "BTCUSDT".into(),
        timeframe: HOURLY[0],
    }
}

/// The candle of `hour`; prices rise by one each hour so candles are easy to tell apart.
pub fn candle(hour: i64) -> Candle {
    let open = 100.0 + hour as f64;
    Candle {
        open_time: hour * HOUR,
        open,
        high: open + 2.0,
        low: open - 2.0,
        close: open + 1.0,
        volume: 1.0,
        quote_volume: Some(open),
        trades: Some(1),
        taker_buy_volume: Some(0.5),
    }
}

/// Hourly candles from hour 0 up to the current hour, which is still open.
pub struct FakeExchange {
    current_hour: AtomicI64,
    max_candles: usize,
    requests: AtomicUsize,
}

impl FakeExchange {
    /// `hours` candles (the last one open), served at most `max_candles` per request.
    pub fn hourly(hours: i64, max_candles: usize) -> Self {
        Self {
            current_hour: AtomicI64::new(hours - 1),
            max_candles,
            requests: AtomicUsize::new(0),
        }
    }

    pub fn advance(&self, hours: i64) {
        self.current_hour.fetch_add(hours, Ordering::Relaxed);
    }

    /// Candle requests answered so far.
    pub fn requests(&self) -> usize {
        self.requests.load(Ordering::Relaxed)
    }

    fn current(&self) -> i64 {
        self.current_hour.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl Exchange for FakeExchange {
    fn id(&self) -> ExchangeId {
        FAKE
    }

    fn name(&self) -> &'static str {
        "Fake"
    }

    fn markets(&self) -> &'static [MarketKind] {
        &[MarketKind::Spot]
    }

    fn timeframes(&self, _market: MarketKind) -> &'static [Timeframe] {
        &HOURLY
    }

    fn max_candles(&self, _market: MarketKind) -> usize {
        self.max_candles
    }

    fn features(&self) -> Features {
        Features::default()
    }

    /// Halfway through the current hour.
    fn now(&self) -> i64 {
        self.current() * HOUR + HOUR / 2
    }

    fn clock_offset(&self) -> Option<i64> {
        None
    }

    fn usage(&self, _market: MarketKind) -> Option<RequestUsage> {
        None
    }

    async fn sync_clock(&self) -> Result<(), ExchangeError> {
        Ok(())
    }

    async fn symbols(&self, _market: MarketKind) -> Result<Vec<Symbol>, ExchangeError> {
        Ok(Vec::new())
    }

    async fn tickers(&self, _market: MarketKind) -> Result<Vec<Ticker>, ExchangeError> {
        Ok(Vec::new())
    }

    async fn candles(
        &self,
        _market: MarketKind,
        _symbol: &str,
        _timeframe: Timeframe,
        query: CandleQuery,
    ) -> Result<Vec<Candle>, ExchangeError> {
        self.requests.fetch_add(1, Ordering::Relaxed);
        let current = self.current();
        let (first, last) = match query {
            CandleQuery::Latest { limit } => (current - limit as i64 + 1, current),
            CandleQuery::From { start, limit } => {
                let first = start.div_euclid(HOUR) + i64::from(start.rem_euclid(HOUR) > 0);
                (first, (first + limit as i64 - 1).min(current))
            }
            CandleQuery::Until { end, limit } => {
                let last = end.div_euclid(HOUR).min(current);
                (last - limit as i64 + 1, last)
            }
        };
        Ok((first.max(0)..=last).map(candle).collect())
    }

    fn feed(
        &self,
        _market: MarketKind,
        _commands: mpsc::UnboundedReceiver<FeedCommand>,
        _events: mpsc::Sender<FeedEvent>,
    ) -> BoxFuture<'static, ()> {
        Box::pin(async {})
    }
}
