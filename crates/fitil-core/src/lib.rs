//! Shared data types for Fitil: candles, symbols and timeframes.
//!
//! All timestamps are Unix milliseconds in UTC. This crate knows nothing about
//! networks, storage or screens; every other crate builds on it.

mod candle;
mod market;
mod timeframe;

pub use candle::{Candle, aggregate, aggregate_whole};
pub use market::{ExchangeId, Market, MarketKind, SeriesKey, Symbol, Ticker, decimals_from_step};
pub use timeframe::{ParseTimeframeError, TimeUnit, Timeframe};
