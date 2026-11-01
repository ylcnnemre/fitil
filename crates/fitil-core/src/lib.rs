//! Shared data types for Fitil: candles, symbols, timeframes and drawings.
//!
//! All timestamps are Unix milliseconds in UTC. This crate knows nothing about
//! networks, storage or screens; every other crate builds on it.

mod candle;
mod drawing;
mod market;
mod timeframe;

pub use candle::{Candle, aggregate, aggregate_whole};
pub use drawing::{Anchor, Drawing, DrawingKind, FIB_LEVELS, LineStyle, Rgb};
pub use market::{ExchangeId, Market, MarketKind, SeriesKey, Symbol, Ticker, decimals_from_step};
pub use timeframe::{ParseTimeframeError, TimeUnit, Timeframe};
