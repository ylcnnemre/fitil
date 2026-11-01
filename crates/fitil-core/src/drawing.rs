//! Drawings: lines, shapes and notes put on a chart. Their points are times and prices,
//! not places on the screen, so a drawing stays put whatever the zoom, the scroll or the
//! timeframe.

/// A color as red, green and blue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// A color from `0xRRGGBB`.
    pub const fn hex(value: u32) -> Self {
        Self((value >> 16) as u8, (value >> 8) as u8, value as u8)
    }
}

/// A point of a drawing: a moment (Unix milliseconds) and a price.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchor {
    pub time: i64,
    pub price: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DrawingKind {
    /// A line between two points.
    TrendLine,
    /// A line from the first point through the second, on to the edge.
    Ray,
    /// A trend line with the price change, the candles, the time and the angle between
    /// its points.
    InfoLine,
    /// A line through two points, edge to edge.
    ExtendedLine,
    /// A trend line with its angle to the horizontal.
    TrendAngle,
    HorizontalLine,
    /// A horizontal line from its point on to the right.
    HorizontalRay,
    VerticalLine,
    /// A horizontal and a vertical line through one point.
    CrossLine,
    /// A box with two opposite corners.
    Rectangle,
    /// A line between two points and a parallel one through the third.
    ParallelChannel,
    /// The straight line that best fits the closes between two times, with bands two
    /// standard deviations away.
    RegressionTrend,
    /// A line between two points and a horizontal one at the third point's price.
    FlatTopBottom,
    /// A line between two points and a parallel one as long, starting at the third.
    DisjointChannel,
    /// Andrews' pitchfork: a median line from the first point to halfway between the
    /// other two, and parallels through those.
    Pitchfork,
    /// A pitchfork whose median starts at the first point's time, halfway between the
    /// first two points' prices.
    SchiffPitchfork,
    /// A pitchfork whose median starts halfway between the first two points.
    ModifiedSchiffPitchfork,
    /// A pitchfork whose median runs from halfway between the first two points through
    /// the third, with parallels through the first two.
    InsidePitchfork,
    /// Levels between two prices, such as 0.618 of the move.
    FibRetracement,
    /// A note.
    Text,
    /// The price change, the candles and the time between two points.
    Measure,
}

impl DrawingKind {
    pub const ALL: [DrawingKind; 21] = [
        DrawingKind::TrendLine,
        DrawingKind::Ray,
        DrawingKind::InfoLine,
        DrawingKind::ExtendedLine,
        DrawingKind::TrendAngle,
        DrawingKind::HorizontalLine,
        DrawingKind::HorizontalRay,
        DrawingKind::VerticalLine,
        DrawingKind::CrossLine,
        DrawingKind::Rectangle,
        DrawingKind::ParallelChannel,
        DrawingKind::RegressionTrend,
        DrawingKind::FlatTopBottom,
        DrawingKind::DisjointChannel,
        DrawingKind::Pitchfork,
        DrawingKind::SchiffPitchfork,
        DrawingKind::ModifiedSchiffPitchfork,
        DrawingKind::InsidePitchfork,
        DrawingKind::FibRetracement,
        DrawingKind::Text,
        DrawingKind::Measure,
    ];

    /// How many points placing one takes.
    pub fn points(self) -> usize {
        match self {
            DrawingKind::HorizontalLine
            | DrawingKind::HorizontalRay
            | DrawingKind::VerticalLine
            | DrawingKind::CrossLine
            | DrawingKind::Text => 1,
            DrawingKind::ParallelChannel
            | DrawingKind::FlatTopBottom
            | DrawingKind::DisjointChannel
            | DrawingKind::Pitchfork
            | DrawingKind::SchiffPitchfork
            | DrawingKind::ModifiedSchiffPitchfork
            | DrawingKind::InsidePitchfork => 3,
            _ => 2,
        }
    }

    /// Stable identifier, used in saved drawings and translation keys.
    pub fn id(self) -> &'static str {
        match self {
            DrawingKind::TrendLine => "trend-line",
            DrawingKind::Ray => "ray",
            DrawingKind::InfoLine => "info-line",
            DrawingKind::ExtendedLine => "extended-line",
            DrawingKind::TrendAngle => "trend-angle",
            DrawingKind::HorizontalLine => "horizontal-line",
            DrawingKind::HorizontalRay => "horizontal-ray",
            DrawingKind::VerticalLine => "vertical-line",
            DrawingKind::CrossLine => "cross-line",
            DrawingKind::Rectangle => "rectangle",
            DrawingKind::ParallelChannel => "parallel-channel",
            DrawingKind::RegressionTrend => "regression-trend",
            DrawingKind::FlatTopBottom => "flat-top-bottom",
            DrawingKind::DisjointChannel => "disjoint-channel",
            DrawingKind::Pitchfork => "pitchfork",
            DrawingKind::SchiffPitchfork => "schiff-pitchfork",
            DrawingKind::ModifiedSchiffPitchfork => "modified-schiff-pitchfork",
            DrawingKind::InsidePitchfork => "inside-pitchfork",
            DrawingKind::FibRetracement => "fib-retracement",
            DrawingKind::Text => "text",
            DrawingKind::Measure => "measure",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.id() == id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LineStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

impl LineStyle {
    pub const ALL: [LineStyle; 3] = [LineStyle::Solid, LineStyle::Dashed, LineStyle::Dotted];

    /// Stable identifier, used in saved drawings and translation keys.
    pub fn id(self) -> &'static str {
        match self {
            LineStyle::Solid => "solid",
            LineStyle::Dashed => "dashed",
            LineStyle::Dotted => "dotted",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|style| style.id() == id)
    }
}

/// Fibonacci retracement levels a new retracement shows.
pub const FIB_LEVELS: [f64; 7] = [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0];

/// One drawing on a symbol's chart.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawing {
    /// Tells drawings apart while a chart shows them.
    pub id: u64,
    pub kind: DrawingKind,
    /// As many as [`DrawingKind::points`] says.
    pub points: Vec<Anchor>,
    pub color: Rgb,
    /// Line width in pixels, 1–4.
    pub width: u8,
    pub line: LineStyle,
    /// The words of a note.
    pub text: String,
    /// Fibonacci levels, as shares of the move from the second point back to the first:
    /// 0 is the second point's price, 1 the first one's.
    pub levels: Vec<f64>,
    /// A locked drawing cannot be moved.
    pub locked: bool,
}

impl Drawing {
    /// A drawing in the default look.
    pub fn new(id: u64, kind: DrawingKind, points: Vec<Anchor>) -> Self {
        Self {
            id,
            kind,
            points,
            color: Rgb::hex(0x2962FF),
            width: 2,
            line: LineStyle::Solid,
            text: String::new(),
            levels: if kind == DrawingKind::FibRetracement {
                FIB_LEVELS.to_vec()
            } else {
                Vec::new()
            },
            locked: false,
        }
    }

    /// Whether the drawing has the points its kind needs, so it can be drawn.
    pub fn is_complete(&self) -> bool {
        self.points.len() == self.kind.points()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        for kind in DrawingKind::ALL {
            assert_eq!(DrawingKind::from_id(kind.id()), Some(kind));
        }
        for style in LineStyle::ALL {
            assert_eq!(LineStyle::from_id(style.id()), Some(style));
        }
        assert_eq!(DrawingKind::from_id("spiral"), None);
    }

    #[test]
    fn kinds_take_their_points() {
        assert_eq!(DrawingKind::CrossLine.points(), 1);
        assert_eq!(DrawingKind::InfoLine.points(), 2);
        assert_eq!(DrawingKind::RegressionTrend.points(), 2);
        for kind in [
            DrawingKind::ParallelChannel,
            DrawingKind::FlatTopBottom,
            DrawingKind::DisjointChannel,
            DrawingKind::Pitchfork,
            DrawingKind::SchiffPitchfork,
            DrawingKind::ModifiedSchiffPitchfork,
            DrawingKind::InsidePitchfork,
        ] {
            assert_eq!(kind.points(), 3, "{kind:?}");
        }
    }

    #[test]
    fn new_drawings_have_their_kind_defaults() {
        let anchor = Anchor {
            time: 0,
            price: 1.0,
        };
        let fib = Drawing::new(1, DrawingKind::FibRetracement, vec![anchor, anchor]);
        assert_eq!(fib.levels, FIB_LEVELS);
        assert!(fib.is_complete());
        let line = Drawing::new(2, DrawingKind::HorizontalLine, vec![anchor, anchor]);
        assert!(line.levels.is_empty());
        assert!(!line.is_complete());
        assert_eq!(Rgb::hex(0x2962FF), Rgb(0x29, 0x62, 0xFF));
    }
}
