//! Fitil's application layer: candle series, live updates, finding symbols and the
//! indicators on a chart.
//!
//! Decisions live here, not in the user interface: what to read from the cache and what
//! to download, how to fill gaps, what to do with a live candle, how to rank symbols,
//! which indicators an exchange can feed. This crate talks to exchanges only through
//! `fitil_exchange::Exchange` and never touches a screen, so all of it is tested with a
//! fake exchange and no network.

mod board;
mod drawings;
mod loader;
mod series;
mod studies;
mod templates;

#[cfg(test)]
mod fake;

pub use board::{Board, MarketSnapshot, Match, load_market, search_markets};
pub use drawings::{
    DrawingLooks, DrawingTemplate, delete_drawing_template, load_drawing_looks, load_drawings,
    save_drawing_default, save_drawing_template, save_drawings,
};
pub use loader::{Loaded, SharedStore, load_older, open_series, save_candle};
pub use series::{LiveUpdate, Series};
pub use studies::{Studies, Study, StudyId, is_available};
pub use templates::{
    SharedUserData, TEMPLATE_NAME_MAX, Template, delete_template, load_templates, save_template,
};

use fitil_exchange::ExchangeError;
use fitil_store::StoreError;

/// Why loading market data failed. Cloneable so it can travel in UI messages.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MarketError {
    #[error(transparent)]
    Exchange(#[from] ExchangeError),
    #[error("local storage: {0}")]
    Storage(String),
}

impl From<StoreError> for MarketError {
    fn from(error: StoreError) -> Self {
        MarketError::Storage(error.to_string())
    }
}
