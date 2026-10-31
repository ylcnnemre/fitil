//! Technical indicator calculations for Fitil.
//!
//! Pure computation: candles go in, series of values come out. Nothing here knows about
//! screens, networks or storage.
//!
//! Every indicator is a small state machine fed one candle at a time ([`Indicator`]).
//! [`Calculation`] keeps an indicator's values up to date as candles change: when only
//! the newest candle changes, as it does many times a second on a live chart, only that
//! candle is computed again.
//!
//! [`catalog`] lists every indicator with its parameters and outputs. Names shown to
//! users are not here: the application translates the ids (`rsi`, `length`, `upper`…).

mod calculation;
mod catalog;
mod indicators;
mod primitives;
mod settings;

pub use calculation::Calculation;
pub use catalog::{
    Definition, FillDef, Group, OutputDef, ParamDef, ParamKind, Placement, PlotKind, Shading,
    ValueFormat, catalog, definition,
};
pub use settings::{OutputStyle, PALETTE, Params, Rgb, Settings, Source, Value};

use fitil_core::Candle;

/// An indicator fed one candle at a time.
pub trait Indicator: IndicatorClone + Send + Sync {
    /// Takes the next candle and writes this indicator's values for it into `out`, one
    /// per output, with `f64::NAN` where a value is not defined (yet).
    fn next(&mut self, candle: &Candle, out: &mut [f64]);
}

/// Lets boxed indicators be cloned, so a calculation can try the newest candle on a copy
/// of its state.
pub trait IndicatorClone {
    fn clone_box(&self) -> Box<dyn Indicator>;
}

impl<T: Indicator + Clone + 'static> IndicatorClone for T {
    fn clone_box(&self) -> Box<dyn Indicator> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Indicator> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
