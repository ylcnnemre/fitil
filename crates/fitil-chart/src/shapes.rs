//! Where drawings go on screen: their lines, areas and words, drawn on the price pane and
//! picked with the mouse.

use fitil_core::{
    Anchor, Candle, Drawing, DrawingKind, LabelAlign, LabelSide, LevelLabels, LineStyle, Rgb,
};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::canvas::{Frame, LineDash, Path, Stroke, Text};
use iced::{Color, Pixels, Point, Rectangle, Size, Vector};

use crate::Chart;
use crate::draw::{Layout, Visible, without_negative_zero};
use crate::scale;

/// Pixels from a line within which a click picks it.
const PICK_DISTANCE: f32 = 6.0;
const HANDLE_RADIUS: f32 = 5.0;
/// How strongly areas are filled.
const FILL_ALPHA: f32 = 0.15;
const NOTE_SIZE: f32 = 14.0;
const LABEL_SIZE: f32 = 11.0;
/// About how wide a character is, as a share of the text size.
const CHAR_WIDTH: f32 = 0.55;
/// Rays and extended lines run this many plot sizes past their points, so that they
/// always reach the edge.
const REACH: f32 = 4.0;
/// Standard deviations between a regression line and its bands.
const DEVIATIONS: f64 = 2.0;
/// How far a trend angle's arc is from its first point, in pixels, at most.
const ARC_RADIUS: f32 = 28.0;
/// The height of a line of a label's words, as a share of their size.
const LINE_HEIGHT: f32 = 1.3;
/// A measurement's colors, going up and going down.
const MEASURE_UP: Color = Color::from_rgb8(0x29, 0x62, 0xFF);
const MEASURE_DOWN: Color = Color::from_rgb8(0xF2, 0x36, 0x45);

const DASHED: [f32; 2] = [6.0, 4.0];
const DOTTED: [f32; 2] = [2.0, 3.0];
const GUIDE: [f32; 2] = [4.0, 4.0];

/// A drawing as it is on screen.
pub(crate) struct Shape {
    pub color: Color,
    /// How the drawing's areas are filled: its color, as strong as its style says;
    /// transparent when they are not filled.
    pub fill: Color,
    /// Lines in the drawing's style.
    pub lines: Vec<(Point, Point)>,
    /// Lines in the drawing's style, each in a color of its own: a retracement's levels.
    pub tinted: Vec<(Point, Point, Color)>,
    /// Thin dashed lines: a channel's middle.
    pub guides: Vec<(Point, Point)>,
    /// A thin line in a color and style of its own: a retracement's move.
    pub trend: Option<(Point, Point, Color, LineStyle)>,
    /// Areas, each in its color. Transparent ones are not painted, but still pick the
    /// drawing when clicked.
    pub areas: Vec<(Vec<Point>, Color)>,
    pub labels: Vec<Label>,
    /// Where the drawing's points are, to drag them.
    pub handles: Vec<Point>,
}

/// Words a drawing shows, with the box they take.
pub(crate) struct Label {
    pub text: String,
    pub area: Rectangle,
    pub size: f32,
    pub color: Color,
    /// Painted behind the words.
    pub background: Option<Color>,
}

impl Label {
    /// Words whose box has its left edge at `x` and its middle at `y`.
    fn new(text: String, x: f32, y: f32, size: f32, color: Color) -> Self {
        let width = text_width(&text, size);
        let height = size + 6.0;
        Self {
            area: Rectangle::new(
                Point::new(x, y - height / 2.0),
                Size::new(width + 8.0, height),
            ),
            text,
            size,
            color,
            background: None,
        }
    }
}

fn text_width(text: &str, size: f32) -> f32 {
    text.lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as f32
        * size
        * CHAR_WIDTH
}

pub(crate) fn color(rgb: Rgb) -> Color {
    Color::from_rgb8(rgb.0, rgb.1, rgb.2)
}

/// Text that stands out on `background`: black on light colors, white on dark ones.
pub(crate) fn readable_on(background: Color) -> Color {
    let light = 0.299 * background.r + 0.587 * background.g + 0.114 * background.b;
    if light > 0.6 {
        Color::BLACK
    } else {
        Color::WHITE
    }
}

impl Shape {
    fn new(drawing: &Drawing) -> Self {
        let color = color(drawing.style.color);
        let strength = if drawing.style.fill {
            f32::from(drawing.style.opacity) / 100.0
        } else {
            0.0
        };
        Self {
            color,
            fill: color.scale_alpha(strength),
            lines: Vec::new(),
            tinted: Vec::new(),
            guides: Vec::new(),
            trend: None,
            areas: Vec::new(),
            labels: Vec::new(),
            handles: Vec::new(),
        }
    }

    /// Whether `point` is on the drawing: near a line, inside an area or on its words.
    fn contains(&self, point: Point, width: f32) -> bool {
        let reach = PICK_DISTANCE + width / 2.0;
        self.lines
            .iter()
            .chain(&self.guides)
            .copied()
            .chain(self.tinted.iter().map(|&(from, to, _)| (from, to)))
            .chain(self.trend.map(|(from, to, _, _)| (from, to)))
            .any(|(from, to)| distance_to_segment(point, from, to) <= reach)
            || self.areas.iter().any(|(area, _)| inside(point, area))
            || self.labels.iter().any(|label| label.area.contains(point))
    }

    fn paint(&self, frame: &mut Frame, width: f32, line: LineStyle) {
        for (area, fill) in &self.areas {
            if fill.a <= 0.0 {
                continue;
            }
            if let Some((first, rest)) = area.split_first() {
                let path = Path::new(|path| {
                    path.move_to(*first);
                    for point in rest {
                        path.line_to(*point);
                    }
                    path.close();
                });
                frame.fill(&path, *fill);
            }
        }
        let stroke_each = |frame: &mut Frame, lines: &[(Point, Point)], stroke: Stroke<'_>| {
            for run in runs(lines) {
                let path = Path::new(|path| {
                    path.move_to(run[0]);
                    for point in &run[1..] {
                        path.line_to(*point);
                    }
                });
                frame.stroke(&path, stroke);
            }
        };
        let styled = |color: Color, width: f32, line: LineStyle| Stroke {
            line_dash: LineDash {
                segments: dashes(line),
                offset: 0,
            },
            ..Stroke::default().with_color(color).with_width(width)
        };
        stroke_each(frame, &self.lines, styled(self.color, width, line));
        for &(from, to, color) in &self.tinted {
            stroke_each(frame, &[(from, to)], styled(color, width, line));
        }
        if let Some((from, to, color, line)) = self.trend {
            stroke_each(frame, &[(from, to)], styled(color, 1.0, line));
        }
        stroke_each(
            frame,
            &self.guides,
            Stroke {
                line_dash: LineDash {
                    segments: &GUIDE,
                    offset: 0,
                },
                ..Stroke::default()
                    .with_color(self.color.scale_alpha(0.7))
                    .with_width(1.0)
            },
        );
        for label in &self.labels {
            if let Some(background) = label.background {
                frame.fill_rectangle(label.area.position(), label.area.size(), background);
            }
            frame.fill_text(Text {
                content: label.text.clone(),
                position: Point::new(label.area.x + 4.0, label.area.center_y()),
                color: label.color,
                size: Pixels(label.size),
                align_x: Horizontal::Left.into(),
                align_y: Vertical::Center,
                ..Text::default()
            });
        }
    }

    /// Small circles on the drawing's points, showing that it is picked.
    fn paint_handles(&self, frame: &mut Frame, background: Color) {
        let handles = Path::new(|path| {
            for handle in &self.handles {
                path.circle(*handle, HANDLE_RADIUS);
            }
        });
        frame.fill(&handles, background);
        frame.stroke(
            &handles,
            Stroke::default().with_color(self.color).with_width(2.0),
        );
    }
}

/// The dashes of a line style.
fn dashes(line: LineStyle) -> &'static [f32] {
    match line {
        LineStyle::Solid => &[],
        LineStyle::Dashed => &DASHED,
        LineStyle::Dotted => &DOTTED,
    }
}

/// Lines as runs of points, a line that starts where the one before ends going on with it.
/// Each run is stroked as a path of its own: iced dashes a path as if it were one line,
/// so lines apart would be joined across their gaps, and a run cut into lines too short
/// for a dash would not show.
fn runs(lines: &[(Point, Point)]) -> Vec<Vec<Point>> {
    let mut runs: Vec<Vec<Point>> = Vec::new();
    for &(from, to) in lines {
        match runs.last_mut() {
            Some(run) if run.last() == Some(&from) => run.push(to),
            _ => runs.push(vec![from, to]),
        }
    }
    runs
}

fn distance_to_segment(point: Point, from: Point, to: Point) -> f32 {
    let along = to - from;
    let length = along.x * along.x + along.y * along.y;
    let t = if length > 0.0 {
        let offset = point - from;
        ((offset.x * along.x + offset.y * along.y) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    point.distance(from + along * t)
}

/// Whether `point` is inside the polygon `area`.
fn inside(point: Point, area: &[Point]) -> bool {
    let mut inside = false;
    let mut previous = match area.last() {
        Some(last) => *last,
        None => return false,
    };
    for &current in area {
        let crosses = (current.y > point.y) != (previous.y > point.y);
        if crosses {
            let x = previous.x
                + (point.y - previous.y) * (current.x - previous.x) / (current.y - previous.y);
            if point.x < x {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}

/// A point `reach` pixels past `to`, going from `from`; `to` when the two coincide.
fn beyond(from: Point, to: Point, reach: f32) -> Point {
    let along = to - from;
    let length = (along.x * along.x + along.y * along.y).sqrt();
    if length == 0.0 {
        to
    } else {
        to + along * (reach / length)
    }
}

/// `point` moved `reach` pixels along `along`; `point` itself when `along` is no move.
fn toward(point: Point, along: Vector, reach: f32) -> Point {
    beyond(point - along, point, reach)
}

fn halfway(a: Point, b: Point) -> Point {
    Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
}

/// The angle from `a` to `b` against the horizontal, in degrees: positive going up.
fn angle(a: Point, b: Point) -> f32 {
    (a.y - b.y).atan2(b.x - a.x).to_degrees()
}

/// A number without needless decimals: `0.618`, `0.5`, `1`.
fn trimmed(value: f64) -> String {
    let text = format!("{value:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// Where a pitchfork's median starts, the point it heads for, and the two points its
/// side lines go through.
pub fn pitchfork_frame(
    kind: DrawingKind,
    a: Point,
    b: Point,
    c: Point,
) -> (Point, Point, [Point; 2]) {
    match kind {
        DrawingKind::SchiffPitchfork => (Point::new(a.x, (a.y + b.y) / 2.0), halfway(b, c), [b, c]),
        DrawingKind::ModifiedSchiffPitchfork => (halfway(a, b), halfway(b, c), [b, c]),
        DrawingKind::InsidePitchfork => (halfway(a, b), c, [a, b]),
        _ => (a, halfway(b, c), [b, c]),
    }
}

/// The straight line that best fits some candles' closes.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Fit {
    /// The candles fitted, by index.
    first: usize,
    last: usize,
    /// The line's value at the first candle, and its change per candle.
    start: f64,
    slope: f64,
    /// How far the closes are from the line: their standard deviation from it.
    deviation: f64,
    /// Pearson's correlation of the closes with time, from -1 to 1.
    pearson: f64,
}

impl Fit {
    fn at(&self, bar: f64) -> f64 {
        self.start + self.slope * (bar - self.first as f64)
    }
}

/// The least-squares line through the closes of the candles between bars `from` and `to`,
/// either way round, if at least two candles are between them.
fn regression(candles: &[Candle], from: f64, to: f64) -> Option<Fit> {
    let last_index = candles.len().checked_sub(1)? as f64;
    let low = from.min(to).round().max(0.0);
    let high = from.max(to).round().min(last_index);
    if high - low < 1.0 {
        return None;
    }
    let (first, last) = (low as usize, high as usize);
    let closes = &candles[first..=last];
    let count = closes.len() as f64;
    let mean_x = (count - 1.0) / 2.0;
    let mean_y = closes.iter().map(|candle| candle.close).sum::<f64>() / count;
    let (mut xx, mut xy, mut yy) = (0.0, 0.0, 0.0);
    for (i, candle) in closes.iter().enumerate() {
        let (dx, dy) = (i as f64 - mean_x, candle.close - mean_y);
        xx += dx * dx;
        xy += dx * dy;
        yy += dy * dy;
    }
    let slope = xy / xx;
    let start = mean_y - slope * mean_x;
    let squares: f64 = closes
        .iter()
        .enumerate()
        .map(|(i, candle)| (candle.close - (start + slope * i as f64)).powi(2))
        .sum();
    Some(Fit {
        first,
        last,
        start,
        slope,
        deviation: (squares / (count - 1.0)).sqrt(),
        pearson: if yy > 0.0 { xy / (xx * yy).sqrt() } else { 0.0 },
    })
}

impl Chart {
    /// Where an anchor is on screen.
    pub(crate) fn point_of(&self, layout: &Layout, visible: &Visible, anchor: Anchor) -> Point {
        Point::new(
            self.x_at(layout, self.bar_of(anchor.time)),
            self.y(layout, visible, anchor.price),
        )
    }

    /// The time and price under a point of the price pane. With the magnet on, they are
    /// the open time of the nearest candle and its open, high, low or close nearest to
    /// the point.
    pub(crate) fn anchor_at(&self, layout: &Layout, visible: &Visible, point: Point) -> Anchor {
        let bar = self.right - f64::from(layout.plot.width - point.x) / self.spacing;
        if self.magnet && !self.candles.is_empty() {
            let last = (self.candles.len() - 1) as f64;
            let candle = &self.candles[bar.round().clamp(0.0, last) as usize];
            let gap = |price: f64| (self.y(layout, visible, price) - point.y).abs();
            let price = [candle.open, candle.high, candle.low, candle.close]
                .into_iter()
                .min_by(|a, b| gap(*a).total_cmp(&gap(*b)))
                .unwrap_or(candle.close);
            return Anchor {
                time: candle.open_time,
                price,
            };
        }
        Anchor {
            time: self.time_of(bar),
            price: visible
                .mapper
                .to_price(self.value_at(layout, visible, point.y)),
        }
    }

    /// A drawing's points moved by `delta` pixels, as a whole.
    pub(crate) fn moved_points(
        &self,
        layout: &Layout,
        visible: &Visible,
        points: &[Anchor],
        delta: Vector,
    ) -> Vec<Anchor> {
        let bars = f64::from(delta.x) / self.spacing;
        let value = -f64::from(delta.y / layout.plot.height) * (visible.high - visible.low);
        points
            .iter()
            .map(|point| Anchor {
                time: self.time_of(self.bar_of(point.time) + bars),
                price: visible
                    .mapper
                    .to_price(visible.mapper.to_scale(point.price) + value),
            })
            .collect()
    }

    /// The drawing under `point`, with which of its points if one: the picked drawing's
    /// points first, then the drawings from the topmost down.
    pub(crate) fn drawing_at(
        &self,
        layout: &Layout,
        visible: &Visible,
        point: Point,
    ) -> Option<(u64, Option<usize>)> {
        if let Some(drawing) = self.selected_drawing()
            && self.shows(drawing)
        {
            let shape = self.shape(layout, visible, drawing);
            let handle = shape
                .handles
                .iter()
                .position(|handle| handle.distance(point) <= HANDLE_RADIUS + 3.0);
            if handle.is_some() {
                return Some((drawing.id, handle));
            }
        }
        self.shown_drawings()
            .rev()
            .find(|drawing| {
                self.shape(layout, visible, drawing)
                    .contains(point, f32::from(drawing.style.width))
            })
            .map(|drawing| (drawing.id, None))
    }

    /// Whether `drawing` is on screen: drawings are not hidden, and its visibility takes
    /// the chart's timeframe.
    pub(crate) fn shows(&self, drawing: &Drawing) -> bool {
        !self.drawings_hidden && drawing.style.visibility.shows(self.timeframe)
    }

    /// The drawings on screen, bottom to top.
    fn shown_drawings(&self) -> impl DoubleEndedIterator<Item = &Drawing> {
        self.drawings.iter().filter(|drawing| self.shows(drawing))
    }

    /// The drawings, the one being placed and the handles of the picked one, on the
    /// price pane.
    pub(crate) fn draw_drawings(
        &self,
        frame: &mut Frame,
        layout: &Layout,
        visible: &Visible,
        background: Color,
    ) {
        frame.with_clip(layout.plot, |frame| {
            for drawing in self.shown_drawings() {
                let shape = self.shape(layout, visible, drawing);
                shape.paint(frame, f32::from(drawing.style.width), drawing.style.line);
                if self.selected == Some(drawing.id) {
                    shape.paint_handles(frame, background);
                }
            }
            if let Some(draft) = &self.draft {
                let preview = draft.preview();
                let shape = self.shape(layout, visible, &preview);
                shape.paint(frame, f32::from(preview.style.width), preview.style.line);
                shape.paint_handles(frame, background);
            }
        });
    }

    /// Prices the price axis shows for horizontal lines, with where and in what color.
    pub(crate) fn line_prices(
        &self,
        layout: &Layout,
        visible: &Visible,
    ) -> Vec<(f32, Anchor, Color)> {
        self.shown_drawings()
            .filter(|drawing| {
                matches!(
                    drawing.kind,
                    DrawingKind::HorizontalLine
                        | DrawingKind::HorizontalRay
                        | DrawingKind::CrossLine
                )
            })
            .filter_map(|drawing| {
                let anchor = *drawing.points.first()?;
                let y = self.y(layout, visible, anchor.price);
                (0.0..=layout.plot.height).contains(&y).then_some((
                    y,
                    anchor,
                    color(drawing.style.color),
                ))
            })
            .collect()
    }

    /// Times the time axis shows for vertical lines, with where and in what color.
    pub(crate) fn line_times(&self, layout: &Layout) -> Vec<(f32, i64, Color)> {
        self.shown_drawings()
            .filter(|drawing| {
                matches!(
                    drawing.kind,
                    DrawingKind::VerticalLine | DrawingKind::CrossLine
                )
            })
            .filter_map(|drawing| {
                let anchor = *drawing.points.first()?;
                let x = self.x_at(layout, self.bar_of(anchor.time));
                (0.0..=layout.plot.width).contains(&x).then_some((
                    x,
                    anchor.time,
                    color(drawing.style.color),
                ))
            })
            .collect()
    }

    /// Where a drawing is on screen. A drawing still being placed shows what it has.
    pub(crate) fn shape(&self, layout: &Layout, visible: &Visible, drawing: &Drawing) -> Shape {
        let mut shape = Shape::new(drawing);
        let p: Vec<Point> = drawing
            .points
            .iter()
            .map(|anchor| self.point_of(layout, visible, *anchor))
            .collect();
        shape.handles = p.clone();
        let reach = (layout.plot.width + layout.plot.height) * REACH;
        let plot = layout.plot;
        match (drawing.kind, p.as_slice()) {
            (DrawingKind::TrendLine, &[a, b]) => shape.lines.push((a, b)),
            (DrawingKind::Ray, &[a, b]) => shape.lines.push((a, beyond(a, b, reach))),
            (DrawingKind::InfoLine, &[a, b]) => self.info_shape(&mut shape, drawing, a, b),
            (DrawingKind::ExtendedLine, &[a, b]) => {
                shape.lines.push((beyond(b, a, reach), beyond(a, b, reach)))
            }
            (DrawingKind::TrendAngle, &[a, b]) => angle_shape(&mut shape, a, b),
            (DrawingKind::HorizontalLine, &[a]) => shape
                .lines
                .push((Point::new(0.0, a.y), Point::new(plot.width, a.y))),
            (DrawingKind::HorizontalRay, &[a]) => {
                shape.lines.push((a, Point::new(plot.width, a.y)))
            }
            (DrawingKind::VerticalLine, &[a]) => shape
                .lines
                .push((Point::new(a.x, 0.0), Point::new(a.x, plot.height))),
            (DrawingKind::CrossLine, &[a]) => {
                shape
                    .lines
                    .push((Point::new(0.0, a.y), Point::new(plot.width, a.y)));
                shape
                    .lines
                    .push((Point::new(a.x, 0.0), Point::new(a.x, plot.height)));
            }
            (DrawingKind::Rectangle, &[a, b]) => {
                let corners = vec![a, Point::new(b.x, a.y), b, Point::new(a.x, b.y)];
                for (i, corner) in corners.iter().enumerate() {
                    shape.lines.push((*corner, corners[(i + 1) % 4]));
                }
                shape.areas.push((corners, shape.fill));
            }
            (DrawingKind::ParallelChannel, &[a, b, c]) => {
                // The parallel line goes through the third point.
                let slope = if b.x == a.x {
                    0.0
                } else {
                    (b.y - a.y) / (b.x - a.x)
                };
                let shift = Vector::new(0.0, c.y - (a.y + (c.x - a.x) * slope));
                shape.lines.push((a, b));
                shape.lines.push((a + shift, b + shift));
                shape.guides.push((a + shift * 0.5, b + shift * 0.5));
                shape
                    .areas
                    .push((vec![a, b, b + shift, a + shift], shape.fill));
            }
            (DrawingKind::RegressionTrend, &[a, b]) => {
                self.regression_shape(&mut shape, layout, visible, drawing, a, b)
            }
            (DrawingKind::FlatTopBottom, &[a, b, c]) => {
                // The third point gives the flat line's price; its handle is where the
                // flat line ends.
                let (start, end) = (Point::new(a.x, c.y), Point::new(b.x, c.y));
                shape.lines.push((a, b));
                shape.lines.push((start, end));
                shape.areas.push((vec![a, b, end, start], shape.fill));
                shape.handles[2] = end;
            }
            (DrawingKind::DisjointChannel, &[a, b, c]) => {
                // The second line starts at the third point, as long as the first.
                let d = c + (b - a);
                shape.lines.push((a, b));
                shape.lines.push((c, d));
                shape.areas.push((vec![a, b, d, c], shape.fill));
            }
            (
                DrawingKind::Pitchfork
                | DrawingKind::SchiffPitchfork
                | DrawingKind::ModifiedSchiffPitchfork
                | DrawingKind::InsidePitchfork,
                &[a, b, c],
            ) => pitchfork_shape(&mut shape, drawing.kind, [a, b, c], reach),
            (DrawingKind::FibRetracement, &[a, b]) => {
                self.fib_shape(&mut shape, visible, layout, drawing, a, b)
            }
            (DrawingKind::Text, &[a]) => {
                let label = Label::new(drawing.text.clone(), a.x, a.y, NOTE_SIZE, shape.color);
                shape.labels.push(label);
            }
            (DrawingKind::Measure, &[a, b]) => self.measure_shape(&mut shape, drawing, a, b),
            // A drawing still being placed: what it has so far.
            (_, &[a, b, ..]) => shape.lines.push((a, b)),
            _ => {}
        }
        shape
    }

    /// Fibonacci levels: a line at each level shown, in its color, between the two points'
    /// times or on past them; the bands between neighboring levels filled in the color of
    /// the one farther from 0; labels with the level and its price; and the line of the
    /// move between the points.
    fn fib_shape(
        &self,
        shape: &mut Shape,
        visible: &Visible,
        layout: &Layout,
        drawing: &Drawing,
        a: Point,
        b: Point,
    ) {
        let style = &drawing.style;
        let plot = layout.plot;
        let (mut left, mut right) = (a.x.min(b.x), a.x.max(b.x));
        if style.extend.left() {
            left = left.min(0.0);
        }
        if style.extend.right() {
            right = right.max(plot.width);
        }
        // 0 is at the second point's price and 1 at the first's, or the other way round.
        let (mut one, mut zero) = (drawing.points[0].price, drawing.points[1].price);
        if style.reverse {
            std::mem::swap(&mut one, &mut zero);
        }
        if style.trend.shown {
            shape.trend = Some((a, b, color(style.trend.color), style.trend.line));
        }
        let mut levels: Vec<(f64, f32, Color)> = style
            .levels
            .iter()
            .filter(|level| level.shown)
            .map(|level| {
                let price = zero + (one - zero) * level.value;
                (
                    level.value,
                    self.y(layout, visible, price),
                    color(style.level_color(level)),
                )
            })
            .collect();
        levels.sort_by(|x, y| x.0.total_cmp(&y.0));
        let strength = if style.fill {
            f32::from(style.opacity) / 100.0
        } else {
            0.0
        };
        for pair in levels.windows(2) {
            let ((_, near, _), (_, far, tint)) = (pair[0], pair[1]);
            let band = vec![
                Point::new(left, near),
                Point::new(right, near),
                Point::new(right, far),
                Point::new(left, far),
            ];
            shape.areas.push((band, tint.scale_alpha(strength)));
        }
        // Labels are kept on the plot, however far the lines run.
        let (start, end) = (left.max(0.0), right.min(plot.width));
        for &(value, y, tint) in &levels {
            shape
                .tinted
                .push((Point::new(left, y), Point::new(right, y), tint));
            let price = zero + (one - zero) * value;
            let Some(text) = self.level_text(style.labels, value, price) else {
                continue;
            };
            let (width, height) = (text_width(&text, LABEL_SIZE) + 8.0, LABEL_SIZE + 6.0);
            let x = match style.labels.align {
                LabelAlign::Left => start - width - 2.0,
                LabelAlign::Center => (start + end - width) / 2.0,
                LabelAlign::Right => end + 2.0,
            };
            let middle = match style.labels.side {
                LabelSide::Top => y - height / 2.0,
                LabelSide::Middle => y,
                LabelSide::Bottom => y + height / 2.0,
            };
            let x = x.clamp(0.0, (plot.width - width).max(0.0));
            shape
                .labels
                .push(Label::new(text, x, middle, LABEL_SIZE, tint));
        }
    }

    /// The words of a level's label: the level, as a share or a percentage, and its price;
    /// `None` when the labels show neither.
    fn level_text(&self, labels: LevelLabels, value: f64, price: f64) -> Option<String> {
        let level = labels.levels.then(|| {
            if labels.percent {
                format!("{}%", trimmed(value * 100.0))
            } else {
                trimmed(value)
            }
        });
        let price = labels.prices.then(|| scale::price(price, self.decimals));
        match (level, price) {
            (Some(level), Some(price)) => Some(format!("{level} ({price})")),
            (level, price) => level.or(price),
        }
    }

    /// A box between the two points, colored by direction, with the price change, the
    /// candles and the time between them.
    fn measure_shape(&self, shape: &mut Shape, drawing: &Drawing, a: Point, b: Point) {
        let (start, end) = (drawing.points[0], drawing.points[1]);
        let rising = end.price >= start.price;
        shape.color = if rising { MEASURE_UP } else { MEASURE_DOWN };
        let corners = vec![a, Point::new(b.x, a.y), b, Point::new(a.x, b.y)];
        shape
            .areas
            .push((corners, shape.color.scale_alpha(FILL_ALPHA)));
        let middle = halfway(a, b);
        shape
            .lines
            .push((Point::new(middle.x, a.y), Point::new(middle.x, b.y)));
        shape
            .lines
            .push((Point::new(a.x, middle.y), Point::new(b.x, middle.y)));

        let text = self.change_text(start, end);
        let height = LABEL_SIZE * 2.0 + 10.0;
        let width = text_width(&text, LABEL_SIZE) + 8.0;
        // Under the box when going down, over it when going up.
        let edge = if rising {
            a.y.min(b.y) - 4.0 - height / 2.0
        } else {
            a.y.max(b.y) + 4.0 + height / 2.0
        };
        let mut label = Label::new(text, middle.x - width / 2.0, edge, LABEL_SIZE, Color::WHITE);
        label.area.height = height;
        label.area.y = edge - height / 2.0;
        label.background = Some(shape.color.scale_alpha(0.85));
        shape.labels.push(label);
    }

    /// Two lines of words: the price change from `start` to `end` with its share of the
    /// first price, then the candles and the time between them.
    fn change_text(&self, start: Anchor, end: Anchor) -> String {
        let change = end.price - start.price;
        let percent = if start.price == 0.0 {
            0.0
        } else {
            change / start.price * 100.0
        };
        let bars = (self.bar_of(end.time) - self.bar_of(start.time)).round();
        format!(
            "{:+.*} ({percent:+.2}%)\n{bars} {}, {}",
            self.decimals as usize,
            change,
            self.labels.bars,
            duration(end.time - start.time, &self.labels),
        )
    }

    /// A trend line with a box under it: the price change, the candles and the time
    /// between its points, and its angle.
    fn info_shape(&self, shape: &mut Shape, drawing: &Drawing, a: Point, b: Point) {
        shape.lines.push((a, b));
        let text = format!(
            "{}\n{}°",
            self.change_text(drawing.points[0], drawing.points[1]),
            without_negative_zero(format!("{:.2}", angle(a, b))),
        );
        let height = LABEL_SIZE * LINE_HEIGHT * 3.0 + 6.0;
        let width = text_width(&text, LABEL_SIZE) + 8.0;
        let top = a.y.max(b.y) + 8.0;
        let left = (a.x + b.x) / 2.0 - width / 2.0;
        let words = readable_on(shape.color);
        let mut label = Label::new(text, left, top + height / 2.0, LABEL_SIZE, words);
        label.area.y = top;
        label.area.height = height;
        label.background = Some(shape.color.scale_alpha(0.85));
        shape.labels.push(label);
    }

    /// The line that best fits the closes of the candles between the two points, bands
    /// two standard deviations above and below it, and Pearson's R under them. With
    /// fewer than two candles between the points, the line between them.
    fn regression_shape(
        &self,
        shape: &mut Shape,
        layout: &Layout,
        visible: &Visible,
        drawing: &Drawing,
        a: Point,
        b: Point,
    ) {
        let bars = [
            self.bar_of(drawing.points[0].time),
            self.bar_of(drawing.points[1].time),
        ];
        let Some(fit) = regression(&self.candles, bars[0], bars[1]) else {
            shape.lines.push((a, b));
            return;
        };
        let (first, last) = (fit.first as f64, fit.last as f64);
        let at = |bar: f64, offset: f64| {
            Point::new(
                self.x_at(layout, bar),
                self.y(layout, visible, fit.at(bar) + offset),
            )
        };
        let band = DEVIATIONS * fit.deviation;
        let line = |offset: f64| (at(first, offset), at(last, offset));
        let (middle, upper, lower) = (line(0.0), line(band), line(-band));
        shape.lines.extend([middle, upper, lower]);
        shape
            .areas
            .push((vec![upper.0, upper.1, lower.1, lower.0], shape.fill));
        // Each point's handle is on the line, at the candle the fit starts or ends at.
        shape.handles = bars
            .iter()
            .map(|bar| at(bar.round().clamp(first, last), 0.0))
            .collect();
        shape.labels.push(Label::new(
            without_negative_zero(format!("{:.2}", fit.pearson)),
            lower.0.x,
            lower.0.y + LABEL_SIZE,
            LABEL_SIZE,
            shape.color,
        ));
    }
}

/// A trend line, a guide going right from its first point, and an arc between the two
/// with the angle in degrees. The arc is drawn like the line, so that it shows.
fn angle_shape(shape: &mut Shape, a: Point, b: Point) {
    shape.lines.push((a, b));
    let degrees = angle(a, b);
    let radius = ARC_RADIUS.min(a.distance(b) * 0.6);
    let length = (b.x - a.x).abs().max(radius + 12.0);
    shape.guides.push((a, Point::new(a.x + length, a.y)));
    let turn = degrees.to_radians();
    let on_arc = |share: f32, reach: f32| {
        let at = turn * share;
        Point::new(a.x + reach * at.cos(), a.y - reach * at.sin())
    };
    const STEPS: usize = 16;
    for step in 0..STEPS {
        let (from, to) = (step as f32 / STEPS as f32, (step + 1) as f32 / STEPS as f32);
        shape.lines.push((on_arc(from, radius), on_arc(to, radius)));
    }
    // Beyond the arc's middle; the half angle is never more than a right angle, so the
    // words go right of the point.
    let place = on_arc(0.5, radius + 6.0);
    shape.labels.push(Label::new(
        format!("{}°", without_negative_zero(format!("{degrees:.2}"))),
        place.x,
        place.y,
        LABEL_SIZE,
        shape.color,
    ));
}

/// A pitchfork from its three points: the median and the lines through its side points
/// running on to the edge, the line between the side points, and the space between the
/// side lines filled. A guide joins the first point to a median starting elsewhere.
fn pitchfork_shape(shape: &mut Shape, kind: DrawingKind, [a, b, c]: [Point; 3], reach: f32) {
    let (start, heading, [left, right]) = pitchfork_frame(kind, a, b, c);
    let along = heading - start;
    shape.lines.push((start, toward(heading, along, reach)));
    let (left_end, right_end) = (toward(left, along, reach), toward(right, along, reach));
    shape.lines.push((left, left_end));
    shape.lines.push((right, right_end));
    shape.lines.push((left, right));
    if kind != DrawingKind::InsidePitchfork && start != a {
        shape.guides.push((a, start));
    }
    shape
        .areas
        .push((vec![left, left_end, right_end, right], shape.fill));
}

/// A span of time in its two largest units, such as `2d 5h` or `45m`; a second unit
/// that is zero is left out, as in `9h`.
fn duration(ms: i64, labels: &crate::ChartLabels) -> String {
    let minutes = ms.abs() / 60_000;
    let (days, hours, rest) = (minutes / 1440, minutes % 1440 / 60, minutes % 60);
    let (first, second) = if days > 0 {
        ((days, &labels.day_suffix), (hours, &labels.hour_suffix))
    } else if hours > 0 {
        ((hours, &labels.hour_suffix), (rest, &labels.minute_suffix))
    } else {
        return format!("{rest}{}", labels.minute_suffix);
    };
    if second.0 == 0 {
        format!("{}{}", first.0, first.1)
    } else {
        format!("{}{} {}{}", first.0, first.1, second.0, second.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_are_near_segments_and_inside_areas() {
        let (from, to) = (Point::new(0.0, 0.0), Point::new(10.0, 0.0));
        assert_eq!(distance_to_segment(Point::new(5.0, 3.0), from, to), 3.0);
        assert_eq!(distance_to_segment(Point::new(14.0, 3.0), from, to), 5.0);
        let square = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ];
        assert!(inside(Point::new(5.0, 5.0), &square));
        assert!(!inside(Point::new(15.0, 5.0), &square));
    }

    #[test]
    fn pitchforks_start_their_median_where_their_kind_says() {
        let (a, b, c) = (
            Point::new(0.0, 50.0),
            Point::new(40.0, 10.0),
            Point::new(60.0, 90.0),
        );
        // Halfway between the second and third points.
        let middle = Point::new(50.0, 50.0);
        let frame = |kind| pitchfork_frame(kind, a, b, c);
        assert_eq!(frame(DrawingKind::Pitchfork), (a, middle, [b, c]));
        assert_eq!(
            frame(DrawingKind::SchiffPitchfork),
            (Point::new(0.0, 30.0), middle, [b, c])
        );
        assert_eq!(
            frame(DrawingKind::ModifiedSchiffPitchfork),
            (Point::new(20.0, 30.0), middle, [b, c])
        );
        assert_eq!(
            frame(DrawingKind::InsidePitchfork),
            (Point::new(20.0, 30.0), c, [a, b])
        );

        // The side lines run the median's way; a guide shows a moved median's start.
        let heading = |(from, to): (Point, Point)| (to.y - from.y).atan2(to.x - from.x);
        for kind in [DrawingKind::Pitchfork, DrawingKind::SchiffPitchfork] {
            let mut shape = Shape::new(&Drawing::new(0, kind, Vec::new()));
            pitchfork_shape(&mut shape, kind, [a, b, c], 500.0);
            let median = heading(shape.lines[0]);
            assert!((heading(shape.lines[1]) - median).abs() < 1e-5);
            assert!((heading(shape.lines[2]) - median).abs() < 1e-5);
            assert_eq!(shape.lines[3], (b, c));
            assert_eq!(
                shape.guides.len(),
                usize::from(kind != DrawingKind::Pitchfork)
            );
        }
    }

    #[test]
    fn regression_fits_the_closes_between_two_bars() {
        let candles: Vec<Candle> = [1.0, 3.0, 2.0, 4.0, 100.0]
            .into_iter()
            .enumerate()
            .map(|(i, close)| Candle {
                open_time: i as i64,
                open: close,
                high: close,
                low: close,
                close,
                volume: 0.0,
                quote_volume: None,
                trades: None,
                taker_buy_volume: None,
            })
            .collect();
        // The fifth candle is past the bars asked for, given either way round.
        let fit = regression(&candles, 3.0, 0.0).unwrap();
        assert_eq!((fit.first, fit.last), (0, 3));
        assert!((fit.slope - 0.8).abs() < 1e-12);
        assert!((fit.at(0.0) - 1.3).abs() < 1e-12);
        assert!((fit.at(3.0) - 3.7).abs() < 1e-12);
        assert!((fit.deviation - 0.6_f64.sqrt()).abs() < 1e-12);
        assert!((fit.pearson - 0.8).abs() < 1e-12);

        // Bars past the candles are cut to them; fewer than two candles fit nothing.
        let span = |from, to| regression(&candles, from, to).map(|fit| (fit.first, fit.last));
        assert_eq!(span(3.0, 9.0), Some((3, 4)));
        assert_eq!(span(2.2, 1.8), None);
        assert_eq!(span(7.0, 9.0), None);
        assert_eq!(regression(&[], 0.0, 3.0), None);
    }

    #[test]
    fn angles_are_measured_up_from_the_horizontal() {
        let origin = Point::ORIGIN;
        assert!((angle(origin, Point::new(10.0, -10.0)) - 45.0).abs() < 1e-4);
        assert!((angle(origin, Point::new(10.0, 10.0)) + 45.0).abs() < 1e-4);
        assert!((angle(origin, Point::new(-10.0, 0.0)) - 180.0).abs() < 1e-4);

        let mut shape = Shape::new(&Drawing::new(0, DrawingKind::TrendAngle, Vec::new()));
        angle_shape(&mut shape, origin, Point::new(100.0, -100.0));
        assert_eq!(shape.labels[0].text, "45.00°");
        // The guide goes right as far as the line does; the arc runs from it to the line.
        assert_eq!(shape.guides, vec![(origin, Point::new(100.0, 0.0))]);
        let arc = &runs(&shape.lines)[1];
        assert!(angle(origin, arc[0]).abs() < 1e-3);
        assert!((angle(origin, *arc.last().unwrap()) - 45.0).abs() < 1e-3);
    }

    #[test]
    fn lines_that_join_are_stroked_together() {
        let p = |x: f32, y: f32| Point::new(x, y);
        let lines = [
            (p(0.0, 0.0), p(1.0, 0.0)),
            (p(1.0, 0.0), p(1.0, 1.0)),
            (p(5.0, 5.0), p(6.0, 5.0)),
        ];
        assert_eq!(
            runs(&lines),
            vec![
                vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0)],
                vec![p(5.0, 5.0), p(6.0, 5.0)],
            ]
        );
    }

    #[test]
    fn numbers_and_durations_are_short() {
        assert_eq!(trimmed(0.618), "0.618");
        assert_eq!(trimmed(0.5), "0.5");
        assert_eq!(trimmed(1.0), "1");
        let labels = crate::ChartLabels {
            open: String::new(),
            high: String::new(),
            low: String::new(),
            close: String::new(),
            volume: String::new(),
            months: std::array::from_fn(|_| String::new()),
            magnitudes: std::array::from_fn(|_| String::new()),
            day_suffix: "d".into(),
            hour_suffix: "h".into(),
            minute_suffix: "m".into(),
            loading: String::new(),
            note: String::new(),
            bars: String::new(),
        };
        assert_eq!(duration(2 * 86_400_000 + 5 * 3_600_000, &labels), "2d 5h");
        assert_eq!(duration(-(3 * 3_600_000 + 15 * 60_000), &labels), "3h 15m");
        assert_eq!(duration(45 * 60_000, &labels), "45m");
        assert_eq!(duration(9 * 3_600_000, &labels), "9h");
        assert_eq!(readable_on(Color::WHITE), Color::BLACK);
        assert_eq!(
            readable_on(Color::from_rgb8(0x29, 0x62, 0xFF)),
            Color::WHITE
        );
    }
}
