//! The exchange interface every Fitil exchange adapter implements, with shared tools.
//!
//! The rest of Fitil talks to exchanges only through [`Exchange`]. Everything that
//! differs between exchanges — symbol formats, timeframe names, limits, message
//! formats, pings — stays inside the adapter crates (`fitil-binance`, …).
//!
//! Adapters can build on the shared tools here: [`stream::run_stream`] (a reconnecting
//! WebSocket loop), [`RateLimiter`], [`Clock`] and [`http_client`]. Only public market
//! data is read: no API keys and no orders.

mod clock;
mod error;
mod feed;
mod rate_limit;
pub mod stream;

pub use clock::{Clock, local_now};
pub use error::ExchangeError;
pub use feed::{FeedCommand, FeedEvent, Topic};
pub use rate_limit::RateLimiter;

use std::time::Duration;

use async_trait::async_trait;
use fitil_core::{Candle, ExchangeId, MarketKind, Symbol, Ticker, Timeframe};
use futures::channel::mpsc;
use futures::future::BoxFuture;

/// One exchange's public market data.
#[async_trait]
pub trait Exchange: Send + Sync + 'static {
    /// Stable identifier, such as `binance`.
    fn id(&self) -> ExchangeId;

    /// Name shown to the user, such as `Binance`.
    fn name(&self) -> &'static str;

    /// The kinds of market this exchange offers, in the order they are shown.
    fn markets(&self) -> &'static [MarketKind];

    /// Timeframes the exchange serves directly for `market`. Other timeframes are built
    /// from these, so only list ones aligned the way Fitil aligns them: days start at
    /// 00:00 UTC, weeks on Monday, months on the 1st. Leave out the rest (HTX's UTC+8
    /// days, Hyperliquid's Thursday weeks and 30-day "months"); Fitil builds those from
    /// shorter candles. Where an exchange offers a UTC variant (OKX's `1Dutc`), use it.
    fn timeframes(&self, market: MarketKind) -> &'static [Timeframe];

    /// The most candles one request can return.
    fn max_candles(&self, market: MarketKind) -> usize;

    fn features(&self) -> Features;

    /// Current time on the exchange's clock, in Unix milliseconds.
    fn now(&self) -> i64;

    /// Exchange clock minus local clock, if the exchange reports its time.
    fn clock_offset(&self) -> Option<i64>;

    /// How much of the request budget is used, if the exchange reports it.
    fn usage(&self, market: MarketKind) -> Option<RequestUsage>;

    /// Measures the exchange's clock. Exchanges without a time endpoint do nothing.
    async fn sync_clock(&self) -> Result<(), ExchangeError>;

    /// Symbols that are currently trading in `market`.
    async fn symbols(&self, market: MarketKind) -> Result<Vec<Symbol>, ExchangeError>;

    /// 24-hour statistics for every symbol of `market`.
    async fn tickers(&self, market: MarketKind) -> Result<Vec<Ticker>, ExchangeError>;

    /// Candles of `symbol`, oldest first. `timeframe` must be one of
    /// [`Exchange::timeframes`]; the newest candle may still be open.
    async fn candles(
        &self,
        market: MarketKind,
        symbol: &str,
        timeframe: Timeframe,
        query: CandleQuery,
    ) -> Result<Vec<Candle>, ExchangeError>;

    /// Streams live data for `market` into `events` until either channel closes.
    ///
    /// The feed reconnects by itself. Updates sent while it was disconnected are lost,
    /// so after a [`FeedEvent::Connected`] that follows [`FeedEvent::Reconnecting`],
    /// callers fetch whatever they may have missed.
    fn feed(
        &self,
        market: MarketKind,
        commands: mpsc::UnboundedReceiver<FeedCommand>,
        events: mpsc::Sender<FeedEvent>,
    ) -> BoxFuture<'static, ()>;
}

/// Which candles to fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandleQuery {
    /// The newest `limit` candles.
    Latest { limit: usize },
    /// Up to `limit` candles opening at or after `start`.
    From { start: i64, limit: usize },
    /// The newest `limit` candles opening at or before `end`.
    Until { end: i64, limit: usize },
}

/// What an exchange provides beyond the basics, so features it lacks can be hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Features {
    /// Candles include the volume bought by takers (needed for buy/sell volume and CVD).
    pub taker_volume: bool,
    /// Older candles can be requested page by page back to a symbol's first candle.
    pub full_history: bool,
}

/// Request budget used in the exchange's current window, and the limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestUsage {
    pub used: u32,
    pub limit: u32,
}

/// An HTTP client for exchange APIs, using rustls with the `ring` crypto provider.
pub fn http_client() -> Result<reqwest::Client, ExchangeError> {
    install_crypto_provider();
    Ok(reqwest::Client::builder()
        .user_agent(concat!("Fitil/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()?)
}

/// Makes `ring` the TLS crypto provider for this process. Safe to call repeatedly.
pub fn install_crypto_provider() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // This only fails if a provider is already installed, which is just as good.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}
