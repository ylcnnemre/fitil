//! Fitil's application layer: loading candle series, live updates and the watchlist.
//!
//! Decisions live here, not in the user interface: what to read from the cache and what
//! to download, how to fill gaps, what to do with a live candle, how to rank symbols.
//! This crate talks to exchanges only through `fitil_exchange::Exchange` and never
//! touches a screen, so all of it is tested with a fake exchange and no network.

mod board;
mod loader;
mod series;

#[cfg(test)]
mod fake;

pub use board::{Board, MarketSnapshot, load_market};
pub use loader::{Loaded, SharedStore, load_older, open_series, save_candle};
pub use series::{LiveUpdate, Series};

use fitil_exchange::ExchangeError;
use fitil_store::StoreError;

/// Why loading market data failed. Cloneable so it can travel in UI messages.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MarketError {
    #[error(transparent)]
    Exchange(#[from] ExchangeError),
    #[error("local cache: {0}")]
    Storage(String),
}

impl From<StoreError> for MarketError {
    fn from(error: StoreError) -> Self {
        MarketError::Storage(error.to_string())
    }
}
