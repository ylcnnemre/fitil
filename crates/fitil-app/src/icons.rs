//! Icons: most from the Lucide set (ISC license, see `assets/fonts/LICENSE-lucide.txt`),
//! drawn with its font, which is built into the program; the line tools' pictures are
//! drawn here, like TradingView's, as Lucide has none for most of them.

use fitil_core::DrawingKind;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::{Text, text};
use iced::{Element, Font, Point, Rectangle, Renderer, Theme, Vector, mouse};

pub const FONT_BYTES: &[u8] = include_bytes!("../../../assets/fonts/lucide.ttf");
const FONT: Font = Font::with_name("lucide");

// Code points of the icons used, from Lucide 1.66.0's `codepoints.json`.
pub const CURSOR: char = '\u{e1c3}'; // mouse-pointer-2
pub const RECTANGLE: char = '\u{e376}'; // rectangle-horizontal
pub const FIBONACCI: char = '\u{e184}'; // text-align-justify
pub const TEXT: char = '\u{e198}'; // type
pub const MEASURE: char = '\u{e14b}'; // ruler
pub const MAGNET: char = '\u{e2b5}'; // magnet
pub const LOCK: char = '\u{e10b}'; // lock
pub const UNLOCK: char = '\u{e10c}'; // lock-open
pub const EYE: char = '\u{e0ba}'; // eye
pub const EYE_OFF: char = '\u{e0bb}'; // eye-off
pub const TRASH: char = '\u{e18e}'; // trash-2
pub const UNDO: char = '\u{e2a1}'; // undo-2
pub const REDO: char = '\u{e2a0}'; // redo-2
pub const COPY: char = '\u{e09e}'; // copy
pub const SEARCH: char = '\u{e151}'; // search
pub const INDICATORS: char = '\u{e038}'; // activity

/// How tall a line of text is, as a share of its size: a font icon's height.
const LINE_HEIGHT: f32 = 1.3;
/// Drawn pictures are made on a square grid this many units wide.
const GRID: f32 = 18.0;
/// The circles marking a tool's points, and the width of the lines, in grid units.
const POINT_RADIUS: f32 = 2.0;
const STROKE: f32 = 1.2;

/// An icon `size` pixels tall.
pub fn icon<'a>(glyph: char, size: f32) -> Text<'a> {
    text(glyph.to_string()).font(FONT).size(size)
}

/// A drawing tool's icon, as big as a font icon of `size`. `on` gives it the colors of
/// a picked tool's button.
pub fn tool<'a, Message: 'a>(kind: DrawingKind, size: f32, on: bool) -> Element<'a, Message> {
    drawn(kind, size, on, false)
}

/// The icon of a group's button: its tool's, with a small corner mark that tells of the
/// menu behind it.
pub fn group<'a, Message: 'a>(kind: DrawingKind, size: f32, on: bool) -> Element<'a, Message> {
    drawn(kind, size, on, true)
}

fn drawn<'a, Message: 'a>(
    kind: DrawingKind,
    size: f32,
    on: bool,
    corner: bool,
) -> Element<'a, Message> {
    let glyph = match kind {
        DrawingKind::Rectangle => Some(RECTANGLE),
        DrawingKind::FibRetracement => Some(FIBONACCI),
        DrawingKind::Text => Some(TEXT),
        DrawingKind::Measure => Some(MEASURE),
        _ => None,
    };
    match glyph {
        Some(glyph) if !corner => icon(glyph, size).into(),
        _ => canvas::Canvas::new(ToolIcon { kind, on, corner })
            .width(size)
            .height(size * LINE_HEIGHT)
            .into(),
    }
}

/// A tool's picture: lines with small hollow circles where the tool's points go.
struct ToolIcon {
    kind: DrawingKind,
    on: bool,
    corner: bool,
}

impl<Message> canvas::Program<Message> for ToolIcon {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let palette = theme.extended_palette();
        let color = if self.on {
            palette.primary.base.text
        } else {
            palette.background.base.text
        };
        let mut frame = Frame::new(renderer, bounds.size());
        let scale = bounds.width.min(bounds.height) / GRID;
        let offset = Vector::new(
            (bounds.width - GRID * scale) / 2.0,
            (bounds.height - GRID * scale) / 2.0,
        );
        let place = |point: Point| Point::new(point.x * scale, point.y * scale) + offset;
        let sketch = sketch(self.kind);
        let radius = POINT_RADIUS * scale;
        let points: Vec<Point> = sketch.points.iter().map(|point| place(*point)).collect();
        let path = Path::new(|path| {
            for &(from, to) in &sketch.lines {
                for (start, end) in outside_circles(place(from), place(to), &points, radius) {
                    path.move_to(start);
                    path.line_to(end);
                }
            }
            for point in &points {
                path.circle(*point, radius);
            }
        });
        frame.stroke(
            &path,
            Stroke::default()
                .with_color(color)
                .with_width(STROKE * scale),
        );
        if self.corner {
            let (right, bottom) = (bounds.width - 0.5, bounds.height - 0.5);
            let mark = Path::new(|path| {
                path.move_to(Point::new(right, bottom - 4.0));
                path.line_to(Point::new(right, bottom));
                path.line_to(Point::new(right - 4.0, bottom));
                path.close();
            });
            frame.fill(&mark, color);
        }
        vec![frame.into_geometry()]
    }
}

/// A picture on the grid: lines, and the tool's points as circles over them.
#[derive(Default)]
struct Sketch {
    lines: Vec<(Point, Point)>,
    points: Vec<Point>,
}

impl Sketch {
    fn line(mut self, from: (f32, f32), to: (f32, f32)) -> Self {
        self.lines.push((from.into(), to.into()));
        self
    }

    /// Lines joining `corners` one after the other.
    fn path(mut self, corners: &[(f32, f32)]) -> Self {
        for pair in corners.windows(2) {
            self.lines.push((pair[0].into(), pair[1].into()));
        }
        self
    }

    fn points(mut self, points: &[(f32, f32)]) -> Self {
        self.points
            .extend(points.iter().map(|&point| Point::from(point)));
        self
    }
}

/// The picture of each line tool, on an 18 by 18 grid whose y grows downward.
fn sketch(kind: DrawingKind) -> Sketch {
    let sketch = Sketch::default();
    match kind {
        DrawingKind::TrendLine => sketch
            .line((3.0, 15.0), (15.0, 3.0))
            .points(&[(3.0, 15.0), (15.0, 3.0)]),
        DrawingKind::Ray => sketch
            .line((3.0, 15.0), (17.5, 0.5))
            .points(&[(3.0, 15.0), (9.0, 9.0)]),
        DrawingKind::InfoLine => sketch
            .line((2.5, 11.5), (11.0, 3.0))
            .path(&[
                (8.5, 10.0),
                (17.0, 10.0),
                (17.0, 17.0),
                (8.5, 17.0),
                (8.5, 10.0),
            ])
            .line((10.5, 12.5), (15.0, 12.5))
            .line((10.5, 14.5), (13.5, 14.5))
            .points(&[(2.5, 11.5), (11.0, 3.0)]),
        DrawingKind::ExtendedLine => sketch
            .line((0.5, 17.5), (17.5, 0.5))
            .points(&[(6.0, 12.0), (12.0, 6.0)]),
        DrawingKind::TrendAngle => {
            // The arc from the horizontal up to the line, in a few straight steps.
            let turn = 11.0_f32.atan2(12.0);
            let arc: Vec<(f32, f32)> = (0..=6)
                .map(|step| {
                    let at = turn * step as f32 / 6.0;
                    (3.0 + 8.0 * at.cos(), 15.0 - 8.0 * at.sin())
                })
                .collect();
            sketch
                .line((3.0, 15.0), (15.0, 4.0))
                .line((3.0, 15.0), (17.5, 15.0))
                .path(&arc)
                .points(&[(3.0, 15.0), (15.0, 4.0)])
        }
        DrawingKind::HorizontalLine => sketch.line((0.5, 9.0), (17.5, 9.0)).points(&[(9.0, 9.0)]),
        DrawingKind::HorizontalRay => sketch.line((4.0, 9.0), (17.5, 9.0)).points(&[(4.0, 9.0)]),
        DrawingKind::VerticalLine => sketch.line((9.0, 0.5), (9.0, 17.5)).points(&[(9.0, 9.0)]),
        DrawingKind::CrossLine => sketch
            .line((0.5, 9.0), (17.5, 9.0))
            .line((9.0, 0.5), (9.0, 17.5))
            .points(&[(9.0, 9.0)]),
        DrawingKind::ParallelChannel => sketch
            .line((2.5, 9.5), (12.5, 2.5))
            .line((5.5, 15.5), (15.5, 8.5))
            .points(&[(2.5, 9.5), (12.5, 2.5), (15.5, 8.5)]),
        DrawingKind::RegressionTrend => sketch
            .line((2.5, 12.0), (15.5, 6.0))
            .line((2.5, 7.5), (15.5, 1.5))
            .line((2.5, 16.5), (15.5, 10.5))
            .points(&[(2.5, 12.0), (15.5, 6.0)]),
        DrawingKind::FlatTopBottom => sketch
            .line((2.5, 12.0), (15.5, 2.5))
            .line((2.5, 15.5), (15.5, 15.5))
            .points(&[(2.5, 12.0), (15.5, 2.5), (15.5, 15.5)]),
        DrawingKind::DisjointChannel => sketch
            .line((2.5, 9.0), (10.5, 2.5))
            .line((7.5, 15.5), (15.5, 9.0))
            .points(&[(2.5, 9.0), (10.5, 2.5), (7.5, 15.5)]),
        DrawingKind::Pitchfork
        | DrawingKind::SchiffPitchfork
        | DrawingKind::ModifiedSchiffPitchfork => {
            pitchfork(sketch, kind, [(1.5, 11.0), (9.0, 3.0), (11.0, 15.0)])
        }
        DrawingKind::InsidePitchfork => {
            pitchfork(sketch, kind, [(2.0, 13.0), (7.0, 3.0), (12.0, 10.0)])
        }
        // Tools with a Lucide icon.
        DrawingKind::Rectangle
        | DrawingKind::FibRetracement
        | DrawingKind::Text
        | DrawingKind::Measure => sketch,
    }
}

/// A pitchfork from three points, built as the chart builds it, cut at the grid's edge.
fn pitchfork(sketch: Sketch, kind: DrawingKind, [a, b, c]: [(f32, f32); 3]) -> Sketch {
    let [a, b, c] = [a, b, c].map(Point::from);
    let (start, heading, [left, right]) = fitil_chart::pitchfork_frame(kind, a, b, c);
    let along = heading - start;
    let edge = |from: Point| {
        let room = |position: f32, step: f32| match step {
            step if step > 0.0 => (GRID - 0.5 - position) / step,
            step if step < 0.0 => (0.5 - position) / step,
            _ => f32::INFINITY,
        };
        let reach = room(from.x, along.x).min(room(from.y, along.y));
        let end = from + along * reach;
        (end.x, end.y)
    };
    let mut sketch = sketch
        .line((start.x, start.y), edge(start))
        .line((left.x, left.y), edge(left))
        .line((right.x, right.y), edge(right))
        .line((left.x, left.y), (right.x, right.y));
    if kind != DrawingKind::InsidePitchfork && start != a {
        sketch = sketch.line((a.x, a.y), (start.x, start.y));
    }
    sketch.points(&[(a.x, a.y), (b.x, b.y), (c.x, c.y)])
}

/// The pieces of the line from `from` to `to` outside every circle of `radius` around
/// `centers`, so that circles are drawn hollow.
fn outside_circles(from: Point, to: Point, centers: &[Point], radius: f32) -> Vec<(Point, Point)> {
    let along = to - from;
    let length = along.x * along.x + along.y * along.y;
    if length == 0.0 {
        return Vec::new();
    }
    // Where the line goes into and out of each circle, as shares of its length.
    let mut cuts: Vec<(f32, f32)> = centers
        .iter()
        .filter_map(|center| {
            let offset = from - *center;
            let half = offset.x * along.x + offset.y * along.y;
            let rest = offset.x * offset.x + offset.y * offset.y - radius * radius;
            let room = half * half - length * rest;
            (room > 0.0).then(|| {
                let root = room.sqrt();
                ((-half - root) / length, (-half + root) / length)
            })
        })
        .collect();
    cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut pieces = Vec::new();
    let mut start = 0.0_f32;
    for (into, out) in cuts {
        if into > start {
            pieces.push((from + along * start, from + along * into.min(1.0)));
        }
        start = start.max(out);
        if start >= 1.0 {
            return pieces;
        }
    }
    pieces.push((from + along * start, to));
    pieces
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_stop_at_the_circles_they_meet() {
        let (from, to) = (Point::new(0.0, 0.0), Point::new(10.0, 0.0));
        // Where each piece starts and ends along the line.
        let spans = |centers: &[Point]| -> Vec<(f32, f32)> {
            outside_circles(from, to, centers, 1.0)
                .into_iter()
                .map(|(start, end)| (start.x, end.x))
                .collect()
        };
        let same = |found: Vec<(f32, f32)>, expected: &[(f32, f32)]| {
            found.len() == expected.len()
                && found
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| (a.0 - b.0).abs() < 1e-4 && (a.1 - b.1).abs() < 1e-4)
        };
        // A circle in the middle splits the line; circles at its ends shorten it.
        assert!(same(
            spans(&[Point::new(5.0, 0.0)]),
            &[(0.0, 4.0), (6.0, 10.0)]
        ));
        assert!(same(spans(&[from, to]), &[(1.0, 9.0)]));
        // A circle away from the line leaves it whole.
        assert!(same(spans(&[Point::new(5.0, 3.0)]), &[(0.0, 10.0)]));
    }

    #[test]
    fn every_line_tool_has_a_picture() {
        for kind in crate::tools::ToolGroup::ALL
            .iter()
            .flat_map(|group| group.sections())
            .flat_map(|section| section.tools)
        {
            let sketch = sketch(*kind);
            assert!(
                !sketch.lines.is_empty() && !sketch.points.is_empty(),
                "{kind:?}"
            );
            let inside =
                |point: &Point| (0.0..=GRID).contains(&point.x) && (0.0..=GRID).contains(&point.y);
            assert!(
                sketch
                    .lines
                    .iter()
                    .all(|(from, to)| inside(from) && inside(to)),
                "{kind:?}"
            );
        }
    }
}
