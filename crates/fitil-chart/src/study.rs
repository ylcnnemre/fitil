//! Indicators as the chart draws them. The chart computes nothing: it is given values,
//! one per candle, and how to draw them.

use iced::Color;

/// Where a study is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudyPlacement {
    /// Over the candles, on the price scale.
    Overlay,
    /// In a pane of its own below the candles, with its own scale.
    Pane,
}

/// How a study's values are written on its axis and in its legend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueFormat {
    /// Like the symbol's prices.
    Price,
    Decimals(u32),
    /// Shortened, like volumes (1.23K).
    Volume,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlotStyle {
    Line,
    /// Bars from zero to each value.
    Histogram(BarColors),
    Dots,
}

/// How histogram bars are colored; the plot's color wherever nothing else is said.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BarColors {
    Fixed,
    /// `negative` for values below zero.
    Sign {
        negative: Color,
    },
    /// `falling` where the candle closed below its open.
    Candle {
        falling: Color,
    },
}

/// One series of a study.
#[derive(Debug, Clone, PartialEq)]
pub struct Plot {
    /// One per candle; NaN leaves a gap.
    pub values: Vec<f64>,
    /// Bars the plot is shifted by when drawn; positive moves it into the future.
    pub offset: i32,
    pub style: PlotStyle,
    pub color: Color,
    pub width: f32,
    pub visible: bool,
}

/// A shaded area between two plots of a study.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotFill {
    pub upper: usize,
    pub lower: usize,
    pub color: Color,
    /// The color where `lower` is above `upper`, if different.
    pub crossed: Option<Color>,
}

/// An indicator on the chart.
#[derive(Debug, Clone, PartialEq)]
pub struct Study {
    /// Stays the same while the indicator is on the chart, so its pane keeps its height.
    pub id: u64,
    /// Shown in the legend, such as `RSI 14`.
    pub label: String,
    pub placement: StudyPlacement,
    pub plots: Vec<Plot>,
    pub fills: Vec<PlotFill>,
    /// Guide lines across the pane, such as RSI's 70 and 30.
    pub levels: Vec<f64>,
    /// A fixed value range for the pane, such as 0–100; otherwise it fits the values.
    pub range: Option<(f64, f64)>,
    pub format: ValueFormat,
}
