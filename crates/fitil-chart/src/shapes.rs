//! Where drawings go on screen: their lines, areas and words, drawn on the price pane and
//! picked with the mouse.

use fitil_core::{Anchor, Drawing, DrawingKind, LineStyle, Rgb};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::canvas::{Frame, LineDash, Path, Stroke, Text};
use iced::{Color, Pixels, Point, Rectangle, Size, Vector};

use crate::Chart;
use crate::draw::{Layout, Visible};
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
/// A measurement's colors, going up and going down.
const MEASURE_UP: Color = Color::from_rgb8(0x29, 0x62, 0xFF);
const MEASURE_DOWN: Color = Color::from_rgb8(0xF2, 0x36, 0x45);

const DASHED: [f32; 2] = [6.0, 4.0];
const DOTTED: [f32; 2] = [2.0, 3.0];
const GUIDE: [f32; 2] = [4.0, 4.0];

/// A drawing as it is on screen.
pub(crate) struct Shape {
    pub color: Color,
    /// Lines in the drawing's style.
    pub lines: Vec<(Point, Point)>,
    /// Thin dashed lines: a channel's middle, a retracement's move.
    pub guides: Vec<(Point, Point)>,
    /// Filled areas, each with how strongly it is filled.
    pub areas: Vec<(Vec<Point>, f32)>,
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
    fn new(color: Color) -> Self {
        Self {
            color,
            lines: Vec::new(),
            guides: Vec::new(),
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
            .any(|&(from, to)| distance_to_segment(point, from, to) <= reach)
            || self.areas.iter().any(|(area, _)| inside(point, area))
            || self.labels.iter().any(|label| label.area.contains(point))
    }

    fn paint(&self, frame: &mut Frame, width: f32, line: LineStyle) {
        for (area, alpha) in &self.areas {
            if let Some((first, rest)) = area.split_first() {
                let path = Path::new(|path| {
                    path.move_to(*first);
                    for point in rest {
                        path.line_to(*point);
                    }
                    path.close();
                });
                frame.fill(&path, self.color.scale_alpha(*alpha));
            }
        }
        let segments = |lines: &[(Point, Point)]| {
            Path::new(|path| {
                for &(from, to) in lines {
                    path.move_to(from);
                    path.line_to(to);
                }
            })
        };
        let segments_style = match line {
            LineStyle::Solid => &[][..],
            LineStyle::Dashed => &DASHED[..],
            LineStyle::Dotted => &DOTTED[..],
        };
        frame.stroke(
            &segments(&self.lines),
            Stroke {
                line_dash: LineDash {
                    segments: segments_style,
                    offset: 0,
                },
                ..Stroke::default().with_color(self.color).with_width(width)
            },
        );
        frame.stroke(
            &segments(&self.guides),
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

/// A number without needless decimals: `0.618`, `0.5`, `1`.
fn trimmed(value: f64) -> String {
    let text = format!("{value:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
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
        if self.drawings_hidden {
            return None;
        }
        if let Some(drawing) = self.selected_drawing() {
            let shape = self.shape(layout, visible, drawing);
            let handle = shape
                .handles
                .iter()
                .position(|handle| handle.distance(point) <= HANDLE_RADIUS + 3.0);
            if handle.is_some() {
                return Some((drawing.id, handle));
            }
        }
        self.drawings
            .iter()
            .rev()
            .find(|drawing| {
                self.shape(layout, visible, drawing)
                    .contains(point, f32::from(drawing.width))
            })
            .map(|drawing| (drawing.id, None))
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
            if !self.drawings_hidden {
                for drawing in &self.drawings {
                    let shape = self.shape(layout, visible, drawing);
                    shape.paint(frame, f32::from(drawing.width), drawing.line);
                    if self.selected == Some(drawing.id) {
                        shape.paint_handles(frame, background);
                    }
                }
            }
            if let Some(draft) = &self.draft {
                let preview = draft.preview();
                let shape = self.shape(layout, visible, &preview);
                shape.paint(frame, f32::from(preview.width), preview.line);
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
        if self.drawings_hidden {
            return Vec::new();
        }
        self.drawings
            .iter()
            .filter(|drawing| {
                matches!(
                    drawing.kind,
                    DrawingKind::HorizontalLine | DrawingKind::HorizontalRay
                )
            })
            .filter_map(|drawing| {
                let anchor = *drawing.points.first()?;
                let y = self.y(layout, visible, anchor.price);
                (0.0..=layout.plot.height)
                    .contains(&y)
                    .then_some((y, anchor, color(drawing.color)))
            })
            .collect()
    }

    /// Where a drawing is on screen. A drawing still being placed shows what it has.
    pub(crate) fn shape(&self, layout: &Layout, visible: &Visible, drawing: &Drawing) -> Shape {
        let mut shape = Shape::new(color(drawing.color));
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
            (DrawingKind::ExtendedLine, &[a, b]) => {
                shape.lines.push((beyond(b, a, reach), beyond(a, b, reach)))
            }
            (DrawingKind::HorizontalLine, &[a]) => shape
                .lines
                .push((Point::new(0.0, a.y), Point::new(plot.width, a.y))),
            (DrawingKind::HorizontalRay, &[a]) => {
                shape.lines.push((a, Point::new(plot.width, a.y)))
            }
            (DrawingKind::VerticalLine, &[a]) => shape
                .lines
                .push((Point::new(a.x, 0.0), Point::new(a.x, plot.height))),
            (DrawingKind::Rectangle, &[a, b]) => {
                let corners = vec![a, Point::new(b.x, a.y), b, Point::new(a.x, b.y)];
                for (i, corner) in corners.iter().enumerate() {
                    shape.lines.push((*corner, corners[(i + 1) % 4]));
                }
                shape.areas.push((corners, FILL_ALPHA));
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
                    .push((vec![a, b, b + shift, a + shift], FILL_ALPHA * 0.7));
            }
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

    /// Fibonacci levels: a line at each, between the two points' times, with its share and
    /// price, and the bands between them lightly filled.
    fn fib_shape(
        &self,
        shape: &mut Shape,
        visible: &Visible,
        layout: &Layout,
        drawing: &Drawing,
        a: Point,
        b: Point,
    ) {
        let (left, right) = (a.x.min(b.x), a.x.max(b.x));
        let (from, to) = (drawing.points[0].price, drawing.points[1].price);
        shape.guides.push((a, b));
        let mut previous: Option<f32> = None;
        for (i, level) in drawing.levels.iter().enumerate() {
            let price = to + (from - to) * level;
            let y = self.y(layout, visible, price);
            shape
                .lines
                .push((Point::new(left, y), Point::new(right, y)));
            if let Some(above) = previous {
                let alpha = if i % 2 == 0 { 0.06 } else { 0.12 };
                let band = vec![
                    Point::new(left, above),
                    Point::new(right, above),
                    Point::new(right, y),
                    Point::new(left, y),
                ];
                shape.areas.push((band, alpha));
            }
            previous = Some(y);
            let text = format!(
                "{} ({})",
                trimmed(*level),
                scale::price(price, self.decimals)
            );
            // Just above its line, inside the retracement, so the edge of the plot does
            // not cut it off.
            shape.labels.push(Label::new(
                text,
                left,
                y - LABEL_SIZE * 0.8,
                LABEL_SIZE,
                shape.color,
            ));
        }
    }

    /// A box between the two points, colored by direction, with the price change, the
    /// candles and the time between them.
    fn measure_shape(&self, shape: &mut Shape, drawing: &Drawing, a: Point, b: Point) {
        let (start, end) = (drawing.points[0], drawing.points[1]);
        let rising = end.price >= start.price;
        shape.color = if rising { MEASURE_UP } else { MEASURE_DOWN };
        let corners = vec![a, Point::new(b.x, a.y), b, Point::new(a.x, b.y)];
        shape.areas.push((corners, FILL_ALPHA));
        let middle = Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
        shape
            .lines
            .push((Point::new(middle.x, a.y), Point::new(middle.x, b.y)));
        shape
            .lines
            .push((Point::new(a.x, middle.y), Point::new(b.x, middle.y)));

        let change = end.price - start.price;
        let percent = if start.price == 0.0 {
            0.0
        } else {
            change / start.price * 100.0
        };
        let bars = (self.bar_of(end.time) - self.bar_of(start.time)).round();
        let text = format!(
            "{:+.*} ({percent:+.2}%)\n{bars} {}, {}",
            self.decimals as usize,
            change,
            self.labels.bars,
            duration(end.time - start.time, &self.labels),
        );
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
