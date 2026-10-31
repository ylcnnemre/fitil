//! Every indicator Fitil offers: its parameters, outputs and how it is drawn.

use crate::Indicator;
use crate::indicators::{
    Anchor, AverageTrueRange, Bollinger, BuySellVolume, Cci, Cvd, Dmi, Donchian,
    ExponentialAverage, Ichimoku, Keltner, Macd, Obv, ParabolicSar, Rsi, SimpleAverage, StochRsi,
    Stochastic, SuperTrend, Volume, VolumeWeightedAverage, Vwap, WeightedAverage,
};
use crate::settings::{Params, Rgb, Source};

/// How indicators are grouped in the "add indicator" list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    MovingAverages,
    Bands,
    Trend,
    Momentum,
    Volatility,
    Volume,
}

impl Group {
    pub const ALL: [Group; 6] = [
        Group::MovingAverages,
        Group::Bands,
        Group::Trend,
        Group::Momentum,
        Group::Volatility,
        Group::Volume,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Group::MovingAverages => "moving-averages",
            Group::Bands => "bands",
            Group::Trend => "trend",
            Group::Momentum => "momentum",
            Group::Volatility => "volatility",
            Group::Volume => "volume",
        }
    }
}

/// Where an indicator is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// On the price chart, sharing its scale.
    Overlay,
    /// In a pane of its own below the price chart.
    Pane,
}

/// How an indicator's values are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueFormat {
    /// Like the symbol's prices.
    Price,
    Decimals(u32),
    /// Shortened, like volumes (1.23K).
    Volume,
}

/// How a histogram's bars are colored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shading {
    /// Always the output's color.
    Fixed,
    /// The output's color above zero, red below.
    Sign,
    /// Like the candle: green when it closed up, red when down.
    Candle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlotKind {
    Line,
    Histogram(Shading),
    Dots,
}

#[derive(Debug, Clone, Copy)]
pub struct OutputDef {
    /// Stable id, also used to translate the output's name.
    pub id: &'static str,
    pub kind: PlotKind,
    pub color: Rgb,
}

/// A shaded area between two outputs.
#[derive(Debug, Clone, Copy)]
pub struct FillDef {
    pub upper: usize,
    pub lower: usize,
    pub color: Rgb,
    /// The color where the outputs have crossed (`lower` above `upper`), if different.
    pub crossed: Option<Rgb>,
}

#[derive(Debug, Clone, Copy)]
pub struct ParamDef {
    /// Stable id, also used to translate the parameter's name.
    pub id: &'static str,
    pub kind: ParamKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParamKind {
    Integer {
        default: i64,
        min: i64,
        max: i64,
    },
    Float {
        default: f64,
        min: f64,
        max: f64,
        step: f64,
    },
    Source {
        default: Source,
    },
    Choice {
        options: &'static [&'static str],
        default: &'static str,
    },
}

/// One indicator of the catalog.
#[derive(Debug)]
pub struct Definition {
    /// Stable id, saved in templates and used to translate the indicator's name.
    pub id: &'static str,
    pub group: Group,
    pub placement: Placement,
    pub params: &'static [ParamDef],
    pub outputs: &'static [OutputDef],
    pub fills: &'static [FillDef],
    /// Horizontal guide lines, such as RSI's 70 and 30.
    pub levels: &'static [f64],
    /// A fixed value range for the pane, such as 0–100 for RSI.
    pub range: Option<(f64, f64)>,
    pub format: ValueFormat,
    /// Needs candles that include taker buy volume.
    pub needs_taker_volume: bool,
    build: fn(&Params) -> Box<dyn Indicator>,
    offsets: fn(&Params) -> Vec<i32>,
}

impl Definition {
    pub fn build(&self, params: &Params) -> Box<dyn Indicator> {
        (self.build)(params)
    }

    /// How many bars each output is shifted when drawn; positive shifts into the future.
    pub fn offsets(&self, params: &Params) -> Vec<i32> {
        let mut offsets = (self.offsets)(params);
        offsets.resize(self.outputs.len(), 0);
        offsets
    }
}

/// All indicators, in the order they are listed.
pub fn catalog() -> &'static [Definition] {
    &CATALOG
}

pub fn definition(id: &str) -> Option<&'static Definition> {
    CATALOG.iter().find(|definition| definition.id == id)
}

const MAX_LENGTH: i64 = 5000;

const fn integer(id: &'static str, default: i64) -> ParamDef {
    ParamDef {
        id,
        kind: ParamKind::Integer {
            default,
            min: 1,
            max: MAX_LENGTH,
        },
    }
}

const fn length(default: i64) -> ParamDef {
    integer("length", default)
}

const fn decimal(id: &'static str, default: f64, min: f64, max: f64, step: f64) -> ParamDef {
    ParamDef {
        id,
        kind: ParamKind::Float {
            default,
            min,
            max,
            step,
        },
    }
}

const fn source(default: Source) -> ParamDef {
    ParamDef {
        id: "source",
        kind: ParamKind::Source { default },
    }
}

const fn choice(
    id: &'static str,
    options: &'static [&'static str],
    default: &'static str,
) -> ParamDef {
    ParamDef {
        id,
        kind: ParamKind::Choice { options, default },
    }
}

const fn line(id: &'static str, color: u32) -> OutputDef {
    OutputDef {
        id,
        kind: PlotKind::Line,
        color: Rgb::hex(color),
    }
}

const fn histogram(id: &'static str, shading: Shading, color: u32) -> OutputDef {
    OutputDef {
        id,
        kind: PlotKind::Histogram(shading),
        color: Rgb::hex(color),
    }
}

const fn band_fill(color: u32) -> FillDef {
    FillDef {
        upper: 1,
        lower: 2,
        color: Rgb::hex(color),
        crossed: None,
    }
}

const BLUE: u32 = 0x2962FF;
const ORANGE: u32 = 0xFF6D00;
const GREEN: u32 = 0x089981;
const RED: u32 = 0xF23645;

const BANDS: [OutputDef; 3] = [
    line("basis", ORANGE),
    line("upper", BLUE),
    line("lower", BLUE),
];
const ANCHORS: &[&str] = &["day", "week", "month"];
const CVD_ANCHORS: &[&str] = &["day", "week", "month", "none"];

fn no_offsets(_: &Params) -> Vec<i32> {
    Vec::new()
}

/// Ichimoku draws the leading spans `displacement − 1` bars ahead and the lagging span
/// as far behind, as TradingView does.
fn ichimoku_offsets(params: &Params) -> Vec<i32> {
    let shift = params.length("displacement") as i32 - 1;
    vec![0, 0, -shift, shift, shift]
}

static CATALOG: [Definition; 22] = [
    Definition {
        id: "sma",
        group: Group::MovingAverages,
        placement: Placement::Overlay,
        params: &[length(9), source(Source::Close)],
        outputs: &[line("value", BLUE)],
        fills: &[],
        levels: &[],
        range: None,
        format: ValueFormat::Price,
        needs_taker_volume: false,
        build: |p| Box::new(SimpleAverage::new(p.source("source"), p.length("length"))),
        offsets: no_offsets,
    },
    Definition {
        id: "ema",
        group: Group::MovingAverages,
        placement: Placement::Overlay,
        params: &[length(9), source(Source::Close)],
        outputs: &[line("value", ORANGE)],
        fills: &[],
        levels: &[],
        range: None,
        format: ValueFormat::Price,
        needs_taker_volume: false,
        build: |p| {
            Box::new(ExponentialAverage::new(
                p.source("source"),
                p.length("length"),
            ))
        },
        offsets: no_offsets,
    },
    Definition {
        id: "wma",
        group: Group::MovingAverages,
        placement: Placement::Overlay,
        params: &[length(9), source(Source::Close)],
        outputs: &[line("value", 0x00BCD4)],
        fills: &[],
        levels: &[],
        range: None,
        format: ValueFormat::Price,
        needs_taker_volume: false,
        build: |p| Box::new(WeightedAverage::new(p.source("source"), p.length("length"))),
        offsets: no_offsets,
    },
    Definition {
        id: "vwma",
        group: Group::MovingAverages,
        placement: Placement::Overlay,
        params: &[length(20), source(Source::Close)],
        outputs: &[line("value", 0xE91E63)],
        fills: &[],
        levels: &[],
        range: None,
        format: ValueFormat::Price,
        needs_taker_volume: false,
        build: |p| {
            Box::new(VolumeWeightedAverage::new(
                p.source("source"),
                p.length("length"),
            ))
        },
        offsets: no_offsets,
    },
    Definition {
        id: "bollinger",
        group: Group::Bands,
        placement: Placement::Overlay,
        params: &[
            length(20),
            source(Source::Close),
            decimal("multiplier", 2.0, 0.1, 10.0, 0.1),
        ],
        outputs: &BANDS,
        fills: &[band_fill(BLUE)],
        levels: &[],
        range: None,
        format: ValueFormat::Price,
        needs_taker_volume: false,
        build: |p| {
            Box::new(Bollinger::new(
                p.source("source"),
                p.length("length"),
                p.number("multiplier"),
            ))
        },
        offsets: no_offsets,
    },
    Definition {
        id: "keltner",
        group: Group::Bands,
        placement: Placement::Overlay,
        params: &[
            length(20),
            source(Source::Close),
            decimal("multiplier", 2.0, 0.1, 10.0, 0.1),
            integer("atr-length", 10),
        ],
        outputs: &BANDS,
        fills: &[band_fill(BLUE)],
        levels: &[],
        range: None,
        format: ValueFormat::Price,
        needs_taker_volume: false,
        build: |p| {
            Box::new(Keltner::new(
                p.source("source"),
                p.length("length"),
                p.number("multiplier"),
                p.length("atr-length"),
            ))
        },
        offsets: no_offsets,
    },
    Definition {
        id: "donchian",
        group: Group::Bands,
        placement: Placement::Overlay,
        params: &[length(20)],
        outputs: &BANDS,
        fills: &[band_fill(BLUE)],
        levels: &[],
        range: None,
        format: ValueFormat::Price,
        needs_taker_volume: false,
        build: |p| Box::new(Donchian::new(p.length("length"))),
        offsets: no_offsets,
    },
    Definition {
        id: "supertrend",
        group: Group::Trend,
        placement: Placement::Overlay,
        params: &[
            integer("atr-length", 10),
            decimal("factor", 3.0, 0.1, 20.0, 0.1),
        ],
        outputs: &[line("up", GREEN), line("down", RED)],
        fills: &[],
        levels: &[],
        range: None,
        format: ValueFormat::Price,
        needs_taker_volume: false,
        build: |p| Box::new(SuperTrend::new(p.length("atr-length"), p.number("factor"))),
        offsets: no_offsets,
    },
    Definition {
        id: "parabolic-sar",
        group: Group::Trend,
        placement: Placement::Overlay,
        params: &[
            decimal("start", 0.02, 0.001, 1.0, 0.001),
            decimal("increment", 0.02, 0.001, 1.0, 0.001),
            decimal("maximum", 0.2, 0.001, 1.0, 0.01),
        ],
        outputs: &[OutputDef {
            id: "value",
            kind: PlotKind::Dots,
            color: Rgb::hex(BLUE),
        }],
        fills: &[],
        levels: &[],
        range: None,
        format: ValueFormat::Price,
        needs_taker_volume: false,
        build: |p| {
            Box::new(ParabolicSar::new(
                p.number("start"),
                p.number("increment"),
                p.number("maximum"),
            ))
        },
        offsets: no_offsets,
    },
    Definition {
        id: "dmi",
        group: Group::Trend,
        placement: Placement::Pane,
        params: &[integer("di-length", 14), integer("adx-smoothing", 14)],
        outputs: &[
            line("adx", 0xF50057),
            line("plus-di", BLUE),
            line("minus-di", ORANGE),
        ],
        fills: &[],
        levels: &[],
        range: None,
        format: ValueFormat::Decimals(2),
        needs_taker_volume: false,
        build: |p| Box::new(Dmi::new(p.length("di-length"), p.length("adx-smoothing"))),
        offsets: no_offsets,
    },
    Definition {
        id: "ichimoku",
        group: Group::Trend,
        placement: Placement::Overlay,
        params: &[
            integer("conversion", 9),
            integer("base", 26),
            integer("span-b", 52),
            integer("displacement", 26),
        ],
        outputs: &[
            line("conversion", BLUE),
            line("base", 0xB71C1C),
            line("lagging", 0x43A047),
            line("lead-a", 0xA5D6A7),
            line("lead-b", 0xEF9A9A),
        ],
        fills: &[FillDef {
            upper: 3,
            lower: 4,
            color: Rgb::hex(0x43A047),
            crossed: Some(Rgb::hex(0xF44336)),
        }],
        levels: &[],
        range: None,
        format: ValueFormat::Price,
        needs_taker_volume: false,
        build: |p| {
            Box::new(Ichimoku::new(
                p.length("conversion"),
                p.length("base"),
                p.length("span-b"),
            ))
        },
        offsets: ichimoku_offsets,
    },
    Definition {
        id: "rsi",
        group: Group::Momentum,
        placement: Placement::Pane,
        params: &[length(14), source(Source::Close)],
        outputs: &[line("value", 0x7E57C2)],
        fills: &[],
        levels: &[70.0, 50.0, 30.0],
        range: Some((0.0, 100.0)),
        format: ValueFormat::Decimals(2),
        needs_taker_volume: false,
        build: |p| Box::new(Rsi::new(p.source("source"), p.length("length"))),
        offsets: no_offsets,
    },
    Definition {
        id: "macd",
        group: Group::Momentum,
        placement: Placement::Pane,
        params: &[
            integer("fast", 12),
            integer("slow", 26),
            integer("signal", 9),
            source(Source::Close),
        ],
        outputs: &[
            histogram("histogram", Shading::Sign, 0x26A69A),
            line("macd", BLUE),
            line("signal", ORANGE),
        ],
        fills: &[],
        levels: &[0.0],
        range: None,
        format: ValueFormat::Price,
        needs_taker_volume: false,
        build: |p| {
            Box::new(Macd::new(
                p.source("source"),
                p.length("fast"),
                p.length("slow"),
                p.length("signal"),
            ))
        },
        offsets: no_offsets,
    },
    Definition {
        id: "stochastic",
        group: Group::Momentum,
        placement: Placement::Pane,
        params: &[
            integer("k-length", 14),
            integer("k-smoothing", 1),
            integer("d-smoothing", 3),
        ],
        outputs: &[line("k", BLUE), line("d", ORANGE)],
        fills: &[],
        levels: &[80.0, 50.0, 20.0],
        range: Some((0.0, 100.0)),
        format: ValueFormat::Decimals(2),
        needs_taker_volume: false,
        build: |p| {
            Box::new(Stochastic::new(
                p.length("k-length"),
                p.length("k-smoothing"),
                p.length("d-smoothing"),
            ))
        },
        offsets: no_offsets,
    },
    Definition {
        id: "stoch-rsi",
        group: Group::Momentum,
        placement: Placement::Pane,
        params: &[
            integer("k-smoothing", 3),
            integer("d-smoothing", 3),
            integer("rsi-length", 14),
            integer("stoch-length", 14),
            source(Source::Close),
        ],
        outputs: &[line("k", BLUE), line("d", ORANGE)],
        fills: &[],
        levels: &[80.0, 50.0, 20.0],
        range: Some((0.0, 100.0)),
        format: ValueFormat::Decimals(2),
        needs_taker_volume: false,
        build: |p| {
            Box::new(StochRsi::new(
                p.source("source"),
                p.length("rsi-length"),
                p.length("stoch-length"),
                p.length("k-smoothing"),
                p.length("d-smoothing"),
            ))
        },
        offsets: no_offsets,
    },
    Definition {
        id: "cci",
        group: Group::Momentum,
        placement: Placement::Pane,
        params: &[length(20), source(Source::Hlc3)],
        outputs: &[line("value", BLUE)],
        fills: &[],
        levels: &[100.0, 0.0, -100.0],
        range: None,
        format: ValueFormat::Decimals(2),
        needs_taker_volume: false,
        build: |p| Box::new(Cci::new(p.source("source"), p.length("length"))),
        offsets: no_offsets,
    },
    Definition {
        id: "atr",
        group: Group::Volatility,
        placement: Placement::Pane,
        params: &[length(14)],
        outputs: &[line("value", 0xB71C1C)],
        fills: &[],
        levels: &[],
        range: None,
        format: ValueFormat::Price,
        needs_taker_volume: false,
        build: |p| Box::new(AverageTrueRange::new(p.length("length"))),
        offsets: no_offsets,
    },
    Definition {
        id: "volume",
        group: Group::Volume,
        placement: Placement::Pane,
        params: &[],
        outputs: &[histogram("value", Shading::Candle, 0x26A69A)],
        fills: &[],
        levels: &[],
        range: None,
        format: ValueFormat::Volume,
        needs_taker_volume: false,
        build: |_| Box::new(Volume),
        offsets: no_offsets,
    },
    Definition {
        id: "buy-sell-volume",
        group: Group::Volume,
        placement: Placement::Pane,
        params: &[],
        outputs: &[
            histogram("buy", Shading::Fixed, 0x26A69A),
            histogram("sell", Shading::Fixed, 0xEF5350),
        ],
        fills: &[],
        levels: &[0.0],
        range: None,
        format: ValueFormat::Volume,
        needs_taker_volume: true,
        build: |_| Box::new(BuySellVolume),
        offsets: no_offsets,
    },
    Definition {
        id: "cvd",
        group: Group::Volume,
        placement: Placement::Pane,
        params: &[choice("anchor", CVD_ANCHORS, "day")],
        outputs: &[line("value", BLUE)],
        fills: &[],
        levels: &[0.0],
        range: None,
        format: ValueFormat::Volume,
        needs_taker_volume: true,
        build: |p| Box::new(Cvd::new(Anchor::from_id(p.text("anchor")))),
        offsets: no_offsets,
    },
    Definition {
        id: "obv",
        group: Group::Volume,
        placement: Placement::Pane,
        params: &[],
        outputs: &[line("value", BLUE)],
        fills: &[],
        levels: &[],
        range: None,
        format: ValueFormat::Volume,
        needs_taker_volume: false,
        build: |_| Box::new(Obv::default()),
        offsets: no_offsets,
    },
    Definition {
        id: "vwap",
        group: Group::Volume,
        placement: Placement::Overlay,
        params: &[choice("anchor", ANCHORS, "day"), source(Source::Hlc3)],
        outputs: &[line("value", BLUE)],
        fills: &[],
        levels: &[],
        range: None,
        format: ValueFormat::Price,
        needs_taker_volume: false,
        build: |p| {
            Box::new(Vwap::new(
                p.source("source"),
                Anchor::from_id(p.text("anchor")),
            ))
        },
        offsets: no_offsets,
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Settings;
    use crate::indicators::testing::random_walk;

    #[test]
    fn ids_are_unique_and_definitions_consistent() {
        let mut ids: Vec<&str> = catalog().iter().map(|d| d.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), catalog().len());

        for definition in catalog() {
            for fill in definition.fills {
                assert!(
                    fill.upper < definition.outputs.len() && fill.lower < definition.outputs.len()
                );
            }
            assert_eq!(
                definition.offsets(&Params::default()).len(),
                definition.outputs.len()
            );
            if definition.placement == Placement::Overlay {
                assert!(
                    definition.range.is_none() && definition.levels.is_empty(),
                    "{}",
                    definition.id
                );
            }
        }
    }

    #[test]
    fn every_indicator_produces_values() {
        let candles = random_walk(300, 12);
        for definition in catalog() {
            let settings = Settings::defaults(definition);
            let mut indicator = definition.build(&settings.params);
            let mut out = vec![f64::NAN; definition.outputs.len()];
            let mut defined = vec![false; definition.outputs.len()];
            for candle in &candles {
                indicator.next(candle, &mut out);
                for (seen, value) in defined.iter_mut().zip(&out) {
                    *seen |= value.is_finite();
                }
            }
            // SuperTrend shows one of its two lines at a time; both appear in 300 candles.
            assert!(defined.iter().all(|&d| d), "{} {defined:?}", definition.id);
        }
    }

    #[test]
    fn ichimoku_spans_are_shifted() {
        let ichimoku = definition("ichimoku").unwrap();
        let settings = Settings::defaults(ichimoku);
        assert_eq!(ichimoku.offsets(&settings.params), [0, 0, -25, 25, 25]);
    }
}
