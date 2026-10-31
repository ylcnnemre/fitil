//! The Fitil chart engine and drawing tools.
//!
//! [`Chart`] holds the candles and the view (which bars and prices are visible).
//! Its canvas turns mouse input into [`ChartMessage`]s; the application passes them
//! back to [`Chart::update`], so the view can also be saved or shared between charts.

mod chart;
mod draw;
mod scale;

pub use chart::{Chart, ChartKind, ChartLabels, ChartMessage, PriceScale};
