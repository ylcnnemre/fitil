//! Drawings: lines, shapes and notes put on a chart. Their points are times and prices,
//! not places on the screen, so a drawing stays put whatever the zoom, the scroll or the
//! timeframe.

use crate::{TimeUnit, Timeframe};

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

    /// Whether the drawing has areas that can be filled, such as a channel's inside.
    pub fn has_fill(self) -> bool {
        matches!(
            self,
            DrawingKind::Rectangle
                | DrawingKind::ParallelChannel
                | DrawingKind::RegressionTrend
                | DrawingKind::FlatTopBottom
                | DrawingKind::DisjointChannel
                | DrawingKind::Pitchfork
                | DrawingKind::SchiffPitchfork
                | DrawingKind::ModifiedSchiffPitchfork
                | DrawingKind::InsidePitchfork
                | DrawingKind::FibRetracement
        )
    }

    /// Whether the drawing has levels, such as a Fibonacci retracement's 0.618.
    pub fn has_levels(self) -> bool {
        self == DrawingKind::FibRetracement
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

/// One level of a Fibonacci drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    /// A share of the drawing's move: 0.618 is 61.8% of it.
    pub value: f64,
    pub color: Rgb,
    pub shown: bool,
}

impl Level {
    pub const fn new(value: f64, color: u32, shown: bool) -> Self {
        Self {
            value,
            color: Rgb::hex(color),
            shown,
        }
    }
}

/// The levels a new Fibonacci retracement has, as in TradingView: the usual ones shown,
/// with more to turn on.
pub const FIB_LEVELS: [Level; 24] = [
    Level::new(0.0, 0x787B86, true),
    Level::new(0.236, 0xF23645, true),
    Level::new(0.382, 0xFF9800, true),
    Level::new(0.5, 0x4CAF50, true),
    Level::new(0.618, 0x089981, true),
    Level::new(0.786, 0x00BCD4, true),
    Level::new(1.0, 0x787B86, true),
    Level::new(1.618, 0x2962FF, true),
    Level::new(2.618, 0xF23645, true),
    Level::new(3.618, 0x9C27B0, true),
    Level::new(4.236, 0xE91E63, true),
    Level::new(0.65, 0x089981, false),
    Level::new(1.272, 0xFF9800, false),
    Level::new(1.414, 0x4CAF50, false),
    Level::new(2.0, 0x00BCD4, false),
    Level::new(2.272, 0x2962FF, false),
    Level::new(2.414, 0x9C27B0, false),
    Level::new(3.0, 0xE91E63, false),
    Level::new(3.272, 0xF23645, false),
    Level::new(3.414, 0xFF9800, false),
    Level::new(4.0, 0x4CAF50, false),
    Level::new(4.414, 0x089981, false),
    Level::new(4.618, 0x00BCD4, false),
    Level::new(4.764, 0x2962FF, false),
];

/// Which way a drawing's lines run on past its points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Extend {
    #[default]
    None,
    Left,
    Right,
    Both,
}

impl Extend {
    pub const ALL: [Extend; 4] = [Extend::None, Extend::Left, Extend::Right, Extend::Both];

    /// Stable identifier, used in saved drawings and translation keys.
    pub fn id(self) -> &'static str {
        match self {
            Extend::None => "none",
            Extend::Left => "left",
            Extend::Right => "right",
            Extend::Both => "both",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|extend| extend.id() == id)
    }

    pub fn left(self) -> bool {
        matches!(self, Extend::Left | Extend::Both)
    }

    pub fn right(self) -> bool {
        matches!(self, Extend::Right | Extend::Both)
    }
}

/// Where level labels go along their lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LabelAlign {
    /// Before the line's start.
    #[default]
    Left,
    Center,
    /// After the line's end.
    Right,
}

impl LabelAlign {
    pub const ALL: [LabelAlign; 3] = [LabelAlign::Left, LabelAlign::Center, LabelAlign::Right];

    /// Stable identifier, used in saved drawings and translation keys.
    pub fn id(self) -> &'static str {
        match self {
            LabelAlign::Left => "left",
            LabelAlign::Center => "center",
            LabelAlign::Right => "right",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|align| align.id() == id)
    }
}

/// Where level labels go against their lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LabelSide {
    /// Above the line.
    Top,
    /// Across it.
    #[default]
    Middle,
    /// Under it.
    Bottom,
}

impl LabelSide {
    pub const ALL: [LabelSide; 3] = [LabelSide::Top, LabelSide::Middle, LabelSide::Bottom];

    /// Stable identifier, used in saved drawings and translation keys.
    pub fn id(self) -> &'static str {
        match self {
            LabelSide::Top => "top",
            LabelSide::Middle => "middle",
            LabelSide::Bottom => "bottom",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|side| side.id() == id)
    }
}

/// What the labels of a drawing's levels show, and where.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelLabels {
    /// The level itself: its share, or with `percent` its percentage.
    pub levels: bool,
    pub percent: bool,
    /// The level's price.
    pub prices: bool,
    pub align: LabelAlign,
    pub side: LabelSide,
}

impl Default for LevelLabels {
    fn default() -> Self {
        Self {
            levels: true,
            percent: false,
            prices: true,
            align: LabelAlign::Left,
            side: LabelSide::Middle,
        }
    }
}

/// The line between a Fibonacci drawing's points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrendLook {
    pub shown: bool,
    pub color: Rgb,
    pub line: LineStyle,
}

impl Default for TrendLook {
    fn default() -> Self {
        Self {
            shown: true,
            color: Rgb::hex(0x787B86),
            line: LineStyle::Dashed,
        }
    }
}

/// The timeframes a drawing shows on: for each unit, a range of counts, or none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visibility {
    /// By unit, in the order of [`TimeUnit::ALL`].
    ranges: [Option<(u32, u32)>; 6],
}

impl Visibility {
    /// Every timeframe.
    pub const ALL: Visibility = Visibility {
        ranges: [
            Some(Visibility::limits(TimeUnit::Second)),
            Some(Visibility::limits(TimeUnit::Minute)),
            Some(Visibility::limits(TimeUnit::Hour)),
            Some(Visibility::limits(TimeUnit::Day)),
            Some(Visibility::limits(TimeUnit::Week)),
            Some(Visibility::limits(TimeUnit::Month)),
        ],
    };

    /// The counts a range of `unit` goes between, as in TradingView. A range up to the
    /// last one also takes longer timeframes of the unit, such as `90m`.
    pub const fn limits(unit: TimeUnit) -> (u32, u32) {
        match unit {
            TimeUnit::Second | TimeUnit::Minute => (1, 59),
            TimeUnit::Hour => (1, 24),
            TimeUnit::Day => (1, 366),
            TimeUnit::Week => (1, 52),
            TimeUnit::Month => (1, 12),
        }
    }

    /// The counts of `unit` the drawing shows at, if any.
    pub fn range(&self, unit: TimeUnit) -> Option<(u32, u32)> {
        self.ranges[unit as usize]
    }

    /// Shows the drawing at the counts of `unit` from `range.0` to `range.1`, kept within
    /// [`Visibility::limits`] and in order, or at none.
    pub fn set_range(&mut self, unit: TimeUnit, range: Option<(u32, u32)>) {
        let (low, high) = Self::limits(unit);
        self.ranges[unit as usize] = range.map(|(from, to)| {
            let (from, to) = (from.clamp(low, high), to.clamp(low, high));
            (from.min(to), from.max(to))
        });
    }

    pub fn shows(&self, timeframe: Timeframe) -> bool {
        let unit = timeframe.unit();
        let count = timeframe.count();
        self.range(unit)
            .is_some_and(|(from, to)| count >= from && (count <= to || to == Self::limits(unit).1))
    }
}

impl Default for Visibility {
    fn default() -> Self {
        Self::ALL
    }
}

/// How a drawing looks: all of it but where it is, its words and its lock. Templates and
/// the look new drawings start with are styles.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawingStyle {
    pub color: Rgb,
    /// Line width in pixels, 1–4.
    pub width: u8,
    pub line: LineStyle,
    /// Whether the drawing's areas are filled, and how strongly, in percent.
    pub fill: bool,
    pub opacity: u8,
    /// Fibonacci levels, as shares of the move from the second point back to the first: 0
    /// is the second point's price, 1 the first one's (the other way round with
    /// `reverse`). Empty for drawings without levels.
    pub levels: Vec<Level>,
    /// Every level in `color` rather than in its own.
    pub one_color: bool,
    pub trend: TrendLook,
    pub extend: Extend,
    pub reverse: bool,
    pub labels: LevelLabels,
    pub visibility: Visibility,
}

impl DrawingStyle {
    /// The look Fitil gives a new drawing of `kind`.
    pub fn of(kind: DrawingKind) -> Self {
        let levels = kind.has_levels();
        Self {
            color: Rgb::hex(0x2962FF),
            width: if levels { 1 } else { 2 },
            line: LineStyle::Solid,
            fill: kind.has_fill(),
            opacity: match kind {
                DrawingKind::Rectangle | DrawingKind::FibRetracement => 15,
                _ => 10,
            },
            levels: if levels {
                FIB_LEVELS.to_vec()
            } else {
                Vec::new()
            },
            one_color: false,
            trend: TrendLook::default(),
            extend: Extend::None,
            reverse: false,
            labels: LevelLabels::default(),
            visibility: Visibility::ALL,
        }
    }

    /// The color a level is drawn in.
    pub fn level_color(&self, level: &Level) -> Rgb {
        if self.one_color {
            self.color
        } else {
            level.color
        }
    }
}

/// One drawing on a symbol's chart.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawing {
    /// Tells drawings apart while a chart shows them.
    pub id: u64,
    pub kind: DrawingKind,
    /// As many as [`DrawingKind::points`] says.
    pub points: Vec<Anchor>,
    /// The words of a note.
    pub text: String,
    /// A locked drawing cannot be moved.
    pub locked: bool,
    pub style: DrawingStyle,
}

impl Drawing {
    /// A drawing in Fitil's look for its kind.
    pub fn new(id: u64, kind: DrawingKind, points: Vec<Anchor>) -> Self {
        Self {
            id,
            kind,
            points,
            text: String::new(),
            locked: false,
            style: DrawingStyle::of(kind),
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
        assert_eq!(fib.style.levels, FIB_LEVELS);
        assert!(fib.style.fill);
        assert!(fib.is_complete());
        let line = Drawing::new(2, DrawingKind::HorizontalLine, vec![anchor, anchor]);
        assert!(line.style.levels.is_empty());
        assert!(!line.style.fill);
        assert!(!line.is_complete());
        assert_eq!(Rgb::hex(0x2962FF), Rgb(0x29, 0x62, 0xFF));
    }

    #[test]
    fn fibonacci_starts_with_tradingviews_levels() {
        let shown: Vec<f64> = FIB_LEVELS
            .iter()
            .filter(|level| level.shown)
            .map(|level| level.value)
            .collect();
        assert_eq!(
            shown,
            [
                0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0, 1.618, 2.618, 3.618, 4.236
            ]
        );

        let mut style = DrawingStyle::of(DrawingKind::FibRetracement);
        let level = style.levels[1];
        assert_eq!(style.level_color(&level), Rgb::hex(0xF23645));
        style.one_color = true;
        assert_eq!(style.level_color(&level), style.color);
    }

    #[test]
    fn ids_of_the_style_choices_round_trip() {
        for extend in Extend::ALL {
            assert_eq!(Extend::from_id(extend.id()), Some(extend));
        }
        for align in LabelAlign::ALL {
            assert_eq!(LabelAlign::from_id(align.id()), Some(align));
        }
        for side in LabelSide::ALL {
            assert_eq!(LabelSide::from_id(side.id()), Some(side));
        }
        assert!(Extend::Both.left() && Extend::Both.right());
        assert!(Extend::Left.left() && !Extend::Left.right());
    }

    #[test]
    fn visibility_picks_the_timeframes_a_drawing_shows_on() {
        let tf = |text: &str| text.parse::<Timeframe>().unwrap();
        let mut visibility = Visibility::ALL;
        assert!(visibility.shows(tf("1m")) && visibility.shows(tf("1M")));

        // Hours from 1 to 4 only, and no minutes.
        visibility.set_range(TimeUnit::Hour, Some((4, 1)));
        visibility.set_range(TimeUnit::Minute, None);
        assert_eq!(visibility.range(TimeUnit::Hour), Some((1, 4)));
        assert!(visibility.shows(tf("1h")) && visibility.shows(tf("4h")));
        assert!(!visibility.shows(tf("6h")));
        assert!(!visibility.shows(tf("15m")));
        assert!(visibility.shows(tf("1d")));

        // A range up to the last count takes longer timeframes too.
        visibility.set_range(TimeUnit::Minute, Some((30, 500)));
        assert_eq!(visibility.range(TimeUnit::Minute), Some((30, 59)));
        assert!(visibility.shows(tf("90m")));
        assert!(!visibility.shows(tf("15m")));
    }
}
