//! Shared data types for Fitil: candles, symbols and timeframes.
//!
//! All timestamps are Unix milliseconds in UTC.

mod candle;
mod market;
mod timeframe;

pub use candle::{Candle, aggregate};
pub use market::{Market, SeriesKey, Symbol, Ticker, decimals_from_step};
pub use timeframe::{ParseTimeframeError, TimeUnit, Timeframe};
