//! The Fitil chart engine and drawing tools.
//!
//! [`Chart`] holds the candles, the indicators to draw ([`Study`]), the user's drawings
//! and the view (which bars and prices are visible, how tall each indicator pane is). Its
//! canvas turns mouse input into [`ChartMessage`]s; the application passes them back to
//! [`Chart::update`], so the view can also be saved or shared between charts.

mod chart;
mod draw;
mod drawings;
mod scale;
mod shapes;
mod study;

pub use chart::{Chart, ChartKind, ChartLabels, ChartMessage, PriceScale};
pub use shapes::pitchfork_frame;
pub use study::{BarColors, Plot, PlotFill, PlotStyle, Study, StudyPlacement, ValueFormat};
