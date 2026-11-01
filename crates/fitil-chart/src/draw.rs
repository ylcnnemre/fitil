//! Drawing the chart and turning mouse input into [`ChartMessage`]s.

use std::ops::RangeInclusive;
use std::time::{Duration, Instant};

use fitil_core::Anchor;
use iced::alignment::{Horizontal, Vertical};
use iced::mouse;
use iced::widget::canvas::{self, Action, Event, Frame, Geometry, LineDash, Path, Stroke, Text};
use iced::{Color, Pixels, Point, Rectangle, Renderer, Size, Theme};

use crate::scale::{self, Mapper};
use crate::shapes::readable_on;
use crate::study::{BarColors, Plot, PlotFill, PlotStyle, Study, ValueFormat};
use crate::{Chart, ChartKind, ChartMessage, PriceScale};

const PRICE_AXIS_WIDTH: f32 = 84.0;
const TIME_AXIS_HEIGHT: f32 = 26.0;
const UP: Color = Color::from_rgb8(0x2E, 0xBD, 0x85);
const DOWN: Color = Color::from_rgb8(0xF6, 0x46, 0x5D);
/// Share of the price pane's height used by volume bars, at the bottom.
const VOLUME_SHARE: f64 = 0.18;
/// Space above and below the candles when the price range is fitted, as pane shares.
/// The space below holds the volume bars; without them it is as small as the top's.
const TOP_MARGIN: f64 = 0.08;
const BOTTOM_MARGIN: f64 = 0.22;
/// Space above and below the values of an indicator pane, as shares of their range.
const PANE_MARGIN: f64 = 0.1;
/// Pixels on either side of the edge above an indicator pane where it can be dragged.
const DIVIDER_GRAB: f32 = 4.0;
/// Vertical drag, in pixels, before dragging the plot also moves the price range.
const VERTICAL_DRAG_THRESHOLD: f32 = 12.0;
const TEXT_SIZE: f32 = 12.0;
/// About how wide a character is, as a share of the text size, to set the pieces of a
/// legend side by side.
const CHAR_WIDTH: f32 = 0.55;
const LEGEND_GAP: f32 = 8.0;
const LEGEND_LINE_HEIGHT: f32 = 18.0;
/// Axis labels closer than this, in pixels, would overlap.
const LABEL_SPACING: f32 = 14.0;
/// Height of a line of text in a box on an axis.
const AXIS_LINE_HEIGHT: f32 = 16.0;
const DASH: [f32; 2] = [4.0, 4.0];
const DOUBLE_CLICK_TIME: Duration = Duration::from_millis(350);
/// Pixels the mouse may move between press and release and still make a click.
const CLICK_SLOP: f32 = 4.0;

/// Mouse state kept between events.
#[derive(Default)]
pub struct State {
    drag: Option<Drag>,
    /// Time and place of the last press, to recognise double clicks.
    last_press: Option<(Instant, Point)>,
    hovering: bool,
}

struct Drag {
    what: Dragged,
    last: Point,
    /// Vertical distance dragged so far.
    vertical: f32,
}

enum Dragged {
    /// The view, by the area pressed: panning, scaling an axis or resizing a pane.
    View(Area),
    /// A drawing by one of its points, or whole; with its points when the drag began.
    Drawing {
        id: u64,
        handle: Option<usize>,
        points: Vec<Anchor>,
        origin: Point,
    },
    /// A press with the drawing tool: let go somewhere else, it places a point there.
    Placing { origin: Point },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Area {
    Plot,
    PriceAxis,
    TimeAxis,
    /// An indicator pane, by its position from the top.
    Pane(usize),
    PaneAxis(usize),
    /// The edge above an indicator pane, dragged to resize it.
    Divider(usize),
}

/// Where the panes and the axes are inside the canvas.
pub(crate) struct Layout {
    size: Size,
    /// The price pane.
    pub(crate) plot: Rectangle,
    /// The indicator panes, top to bottom, as wide as the price pane.
    panes: Vec<Rectangle>,
}

impl Layout {
    /// Shares the height by `weights`: the price pane's first, then each indicator pane's.
    fn new(size: Size, weights: &[f64]) -> Self {
        let width = (size.width - PRICE_AXIS_WIDTH).max(1.0);
        let height = f64::from(panes_height(size));
        let total: f64 = weights.iter().sum();
        let mut panes = Vec::with_capacity(weights.len());
        let (mut sum, mut top) = (0.0, 0.0);
        for weight in weights {
            sum += weight;
            // Whole pixels, so the edges between panes stay sharp.
            let bottom = (height * sum / total).round() as f32;
            panes.push(Rectangle {
                x: 0.0,
                y: top,
                width,
                height: (bottom - top).max(1.0),
            });
            top = bottom;
        }
        let plot = if panes.is_empty() {
            Rectangle::new(Point::ORIGIN, Size::new(width, height as f32))
        } else {
            panes.remove(0)
        };
        Self { size, plot, panes }
    }

    /// The top of the time axis, below every pane.
    fn bottom(&self) -> f32 {
        let last = self.panes.last().unwrap_or(&self.plot);
        last.y + last.height
    }

    fn area(&self, point: Point) -> Option<Area> {
        if point.x < 0.0 || point.y < 0.0 || point.x > self.size.width || point.y > self.size.height
        {
            return None;
        }
        if let Some(index) = self
            .panes
            .iter()
            .position(|pane| (point.y - pane.y).abs() <= DIVIDER_GRAB)
        {
            return Some(Area::Divider(index));
        }
        if point.y > self.bottom() {
            return Some(Area::TimeAxis);
        }
        let axis = point.x > self.plot.width;
        Some(
            match self.panes.iter().rposition(|pane| point.y >= pane.y) {
                Some(index) if axis => Area::PaneAxis(index),
                Some(index) => Area::Pane(index),
                None if axis => Area::PriceAxis,
                None => Area::Plot,
            },
        )
    }
}

/// The height shared by the panes: all of it but the time axis.
fn panes_height(size: Size) -> f32 {
    (size.height - TIME_AXIS_HEIGHT).max(1.0)
}

/// What is on screen, worked out once per frame.
pub(crate) struct Visible {
    /// The first and last candle in view.
    first: usize,
    last: usize,
    /// The bars at the plot's edges, which may lie beyond the candles.
    first_bar: i64,
    last_bar: i64,
    pub(crate) mapper: Mapper,
    /// Price range of the price pane's full height, in scale units.
    pub(crate) low: f64,
    pub(crate) high: f64,
    max_volume: f64,
}

/// An indicator pane as drawn in this frame.
struct PaneView<'a> {
    study: &'a Study,
    rect: Rectangle,
    /// The values at the pane's bottom and top.
    low: f64,
    high: f64,
}

impl PaneView<'_> {
    fn y(&self, value: f64) -> f32 {
        let share = (self.high - value) / (self.high - self.low);
        self.rect.y + (share * f64::from(self.rect.height)) as f32
    }

    fn value_at(&self, y: f32) -> f64 {
        let share = f64::from((y - self.rect.y) / self.rect.height);
        self.high - share * (self.high - self.low)
    }
}

/// Where the grid lines and labels of the axes go, worked out once per frame.
#[derive(Default)]
struct Ticks {
    /// In price scale units.
    price: Vec<f64>,
    /// One list per indicator pane.
    panes: Vec<Vec<f64>>,
    time: Vec<scale::TimeTick>,
}

/// Where the crosshair is and what its boxes on the axes say.
struct Crosshair {
    /// The hovered bar's position, and the cursor's height.
    x: f32,
    y: f32,
    /// The pane the cursor is in.
    pane: Rectangle,
    value: String,
    time: String,
    time_left: f32,
    time_width: f32,
}

/// The values of `plot` drawn between the bars at the plot's edges, if any.
fn plot_indices(plot: &Plot, visible: &Visible, candles: usize) -> Option<RangeInclusive<usize>> {
    let len = plot.values.len().min(candles) as i64;
    let offset = i64::from(plot.offset);
    let first = (visible.first_bar - offset).max(0);
    let last = (visible.last_bar - offset).min(len - 1);
    (first <= last).then_some(first as usize..=last as usize)
}

impl Chart {
    fn layout(&self, size: Size) -> Layout {
        Layout::new(size, &self.pane_weights())
    }

    /// The horizontal position of `bar`, which may lie between or beyond the candles.
    pub(crate) fn x_at(&self, layout: &Layout, bar: f64) -> f32 {
        (f64::from(layout.plot.width) - (self.right - bar) * self.spacing) as f32
    }

    fn x(&self, layout: &Layout, index: usize) -> f32 {
        self.x_at(layout, index as f64)
    }

    /// The bar under a horizontal position in the plot.
    fn index_at(&self, layout: &Layout, x: f32) -> Option<usize> {
        if self.candles.is_empty() {
            return None;
        }
        let index = (self.right - f64::from(layout.plot.width - x) / self.spacing).round();
        Some(index.clamp(0.0, (self.candles.len() - 1) as f64) as usize)
    }

    fn visible(&self, layout: &Layout) -> Option<Visible> {
        let last_index = self.candles.len().checked_sub(1)?;
        let left = self.right - f64::from(layout.plot.width) / self.spacing;
        let first = left.floor().max(0.0) as usize;
        let last = (self.right.ceil().max(0.0) as usize).min(last_index);
        if first > last {
            return None;
        }
        let shown = &self.candles[first..=last];
        let mapper = Mapper {
            scale: self.scale,
            base: shown[0].close,
        };
        let line_only = matches!(self.kind, ChartKind::Line | ChartKind::Area);
        let (mut low, mut high) = shown.iter().fold((f64::MAX, f64::MIN), |(low, high), c| {
            let (l, h) = if line_only {
                (c.close, c.close)
            } else {
                (c.low, c.high)
            };
            (low.min(mapper.to_scale(l)), high.max(mapper.to_scale(h)))
        });
        let mut visible = Visible {
            first,
            last,
            first_bar: left.floor() as i64,
            last_bar: self.right.ceil() as i64,
            mapper,
            low: 0.0,
            high: 0.0,
            max_volume: shown.iter().map(|c| c.volume).fold(0.0, f64::max),
        };

        // Indicators drawn over the candles stay in view too.
        for plot in self
            .overlays()
            .flat_map(|study| &study.plots)
            .filter(|plot| plot.visible)
        {
            let Some(indices) = plot_indices(plot, &visible, self.candles.len()) else {
                continue;
            };
            for &value in &plot.values[indices] {
                let drawable = self.scale != PriceScale::Logarithmic || value > 0.0;
                if value.is_finite() && drawable {
                    low = low.min(mapper.to_scale(value));
                    high = high.max(mapper.to_scale(value));
                }
            }
        }

        (visible.low, visible.high) = match self.price_range {
            Some(range) => range,
            None => {
                let span = if high > low {
                    high - low
                } else {
                    high.abs().max(1.0) * 0.01
                };
                let bottom_margin = if self.show_volume {
                    BOTTOM_MARGIN
                } else {
                    TOP_MARGIN
                };
                let candles_share = 1.0 - TOP_MARGIN - bottom_margin;
                (
                    low - span * bottom_margin / candles_share,
                    high + span * TOP_MARGIN / candles_share,
                )
            }
        };
        self.shown_range.set(Some((visible.low, visible.high)));
        Some(visible)
    }

    fn pane_views(&self, layout: &Layout, visible: Option<&Visible>) -> Vec<PaneView<'_>> {
        self.pane_studies()
            .zip(&layout.panes)
            .map(|(study, rect)| {
                let (low, high) = visible
                    .and_then(|v| self.pane_range(study, v))
                    .unwrap_or((0.0, 1.0));
                PaneView {
                    study,
                    rect: *rect,
                    low,
                    high,
                }
            })
            .collect()
    }

    /// The value range of a pane: its fixed range, or its visible values with their guide
    /// lines (and zero, where histogram bars start).
    fn pane_range(&self, study: &Study, visible: &Visible) -> Option<(f64, f64)> {
        if let Some((low, high)) = study.range {
            let margin = (high - low) * PANE_MARGIN / 2.0;
            return Some((low - margin, high + margin));
        }
        let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
        let mut bars = false;
        for plot in study.plots.iter().filter(|plot| plot.visible) {
            let Some(indices) = plot_indices(plot, visible, self.candles.len()) else {
                continue;
            };
            let mut found = false;
            for &value in plot.values[indices].iter().filter(|v| v.is_finite()) {
                low = low.min(value);
                high = high.max(value);
                found = true;
            }
            if found && matches!(plot.style, PlotStyle::Histogram(_)) {
                bars = true;
                low = low.min(0.0);
                high = high.max(0.0);
            }
        }
        if low > high {
            return None;
        }
        for &level in &study.levels {
            low = low.min(level);
            high = high.max(level);
        }
        let span = if high > low {
            high - low
        } else {
            high.abs().max(1.0) * 0.01
        };
        // Bars rising from zero (like volume) start at the bottom of the pane.
        let below = if bars && low == 0.0 {
            0.0
        } else {
            span * PANE_MARGIN
        };
        Some((low - below, high + span * PANE_MARGIN))
    }

    /// The height of `price` in the price pane.
    pub(crate) fn y(&self, layout: &Layout, visible: &Visible, price: f64) -> f32 {
        self.y_of(layout, visible, visible.mapper.to_scale(price))
    }

    fn y_of(&self, layout: &Layout, visible: &Visible, value: f64) -> f32 {
        let share = (visible.high - value) / (visible.high - visible.low);
        (share * f64::from(layout.plot.height)) as f32
    }

    /// The value, in price scale units, at height `y` of the price pane.
    pub(crate) fn value_at(&self, layout: &Layout, visible: &Visible, y: f32) -> f64 {
        visible.high - f64::from(y / layout.plot.height) * (visible.high - visible.low)
    }

    fn price_label(&self, visible: &Visible, value: f64) -> String {
        match self.scale {
            PriceScale::Percent => format!("{value:+.2}%"),
            _ => scale::price(visible.mapper.to_price(value), self.decimals),
        }
    }

    /// An indicator's value as its axis and legend write it.
    fn format_value(&self, format: ValueFormat, value: f64) -> String {
        let text = match format {
            ValueFormat::Price => scale::price(value, self.decimals),
            ValueFormat::Decimals(decimals) => format!("{value:.*}", decimals as usize),
            ValueFormat::Volume => scale::volume(value, &self.labels),
        };
        without_negative_zero(text)
    }

    /// The grid lines of every axis.
    fn ticks(&self, layout: &Layout, visible: Option<&Visible>, panes: &[PaneView]) -> Ticks {
        let Some(v) = visible else {
            return Ticks::default();
        };
        let price_count = (layout.plot.height / 70.0).max(2.0) as usize;
        Ticks {
            price: v.mapper.ticks(v.low, v.high, price_count),
            panes: panes
                .iter()
                .map(|pane| {
                    let count = (pane.rect.height / 30.0).max(3.0) as usize;
                    scale::linear_ticks(pane.low, pane.high, count)
                })
                .collect(),
            time: scale::time_ticks(
                &self.candles,
                v.first,
                v.last,
                self.spacing,
                self.timeframe,
                &self.labels,
            ),
        }
    }

    /// Grid, volume, candles and indicators: everything that only changes with the data
    /// or the view, so it can be cached.
    fn draw_base(
        &self,
        frame: &mut Frame,
        theme: &Theme,
        layout: &Layout,
        visible: Option<&Visible>,
        panes: &[PaneView],
        ticks: &Ticks,
    ) {
        let palette = theme.extended_palette();
        let text = palette.background.base.text;
        let grid = Stroke::default()
            .with_color(text.scale_alpha(0.07))
            .with_width(1.0);
        let plot = layout.plot;
        let bottom = layout.bottom();

        let horizontal = |frame: &mut Frame, y: f32| {
            frame.stroke(
                &Path::line(Point::new(0.0, y), Point::new(plot.width, y)),
                grid,
            );
        };
        if let Some(v) = visible {
            for &tick in &ticks.price {
                horizontal(frame, self.y_of(layout, v, tick));
            }
        }
        for (pane, pane_ticks) in panes.iter().zip(&ticks.panes) {
            for &tick in pane_ticks {
                horizontal(frame, pane.y(tick));
            }
        }
        for tick in &ticks.time {
            let x = self.x(layout, tick.index);
            frame.stroke(&Path::line(Point::new(x, 0.0), Point::new(x, bottom)), grid);
        }

        if let Some(v) = visible {
            // With a price range set by hand, candles can reach past the plot; the
            // canvas does not clip by itself, so they would cover the widgets around it.
            frame.with_clip(plot, |frame| {
                if self.show_volume {
                    self.draw_volume(frame, layout, v);
                }
                match self.kind {
                    ChartKind::Candles | ChartKind::HeikinAshi => {
                        self.draw_candles(frame, layout, v)
                    }
                    ChartKind::Bars => self.draw_bars(frame, layout, v),
                    ChartKind::Line | ChartKind::Area => {
                        self.draw_line(frame, layout, v, palette.primary.base.color)
                    }
                }
                for study in self.overlays() {
                    self.draw_study(frame, layout, v, study, &|value| self.y(layout, v, value));
                }
            });
            for pane in panes {
                frame.with_clip(pane.rect, |frame| {
                    for &level in &pane.study.levels {
                        let y = pane.y(level);
                        frame.stroke(
                            &Path::line(Point::new(0.0, y), Point::new(plot.width, y)),
                            dashed(text.scale_alpha(0.3)),
                        );
                    }
                    self.draw_study(frame, layout, v, pane.study, &|value| pane.y(value));
                });
            }
        }

        // Axis backgrounds also hide anything reaching past the plot.
        let background = palette.background.base.color;
        let border = Stroke::default()
            .with_color(text.scale_alpha(0.2))
            .with_width(1.0);
        frame.fill_rectangle(
            Point::new(plot.width, 0.0),
            Size::new(PRICE_AXIS_WIDTH, layout.size.height),
            background,
        );
        frame.fill_rectangle(
            Point::new(0.0, bottom),
            Size::new(layout.size.width, layout.size.height - bottom),
            background,
        );
        frame.stroke(
            &Path::line(Point::new(plot.width, 0.0), Point::new(plot.width, bottom)),
            border,
        );
        for y in layout.panes.iter().map(|pane| pane.y).chain([bottom]) {
            frame.stroke(
                &Path::line(Point::new(0.0, y), Point::new(layout.size.width, y)),
                border,
            );
        }
    }

    fn body_width(&self) -> f32 {
        ((self.spacing * 0.7).floor() as f32).max(1.0)
    }

    fn draw_volume(&self, frame: &mut Frame, layout: &Layout, v: &Visible) {
        if v.max_volume <= 0.0 {
            return;
        }
        let height = f64::from(layout.plot.height) * VOLUME_SHARE;
        let width = self.body_width();
        for (rising, color) in [(true, UP), (false, DOWN)] {
            let bars = Path::new(|path| {
                for index in v.first..=v.last {
                    let candle = &self.candles[index];
                    if (candle.close >= candle.open) != rising {
                        continue;
                    }
                    let bar = (candle.volume / v.max_volume * height) as f32;
                    let x = self.x(layout, index);
                    path.rectangle(
                        Point::new(x - width / 2.0, layout.plot.height - bar),
                        Size::new(width, bar),
                    );
                }
            });
            frame.fill(&bars, color.scale_alpha(0.25));
        }
    }

    fn draw_candles(&self, frame: &mut Frame, layout: &Layout, v: &Visible) {
        let width = self.body_width();
        for (rising, color) in [(true, UP), (false, DOWN)] {
            let mut wicks = Vec::new();
            let bodies = Path::new(|path| {
                for index in v.first..=v.last {
                    let candle = &self.candles[index];
                    if (candle.close >= candle.open) != rising {
                        continue;
                    }
                    let x = self.x(layout, index).round();
                    wicks.push((
                        x,
                        self.y(layout, v, candle.high),
                        self.y(layout, v, candle.low),
                    ));
                    let top = self.y(layout, v, candle.open.max(candle.close));
                    let bottom = self.y(layout, v, candle.open.min(candle.close));
                    path.rectangle(
                        Point::new(x - (width / 2.0).floor(), top),
                        Size::new(width, (bottom - top).max(1.0)),
                    );
                }
            });
            let wicks = Path::new(|path| {
                for (x, high, low) in wicks {
                    path.move_to(Point::new(x + 0.5, high));
                    path.line_to(Point::new(x + 0.5, low));
                }
            });
            frame.stroke(&wicks, Stroke::default().with_color(color).with_width(1.0));
            if width > 2.0 {
                frame.fill(&bodies, color);
            }
        }
    }

    fn draw_bars(&self, frame: &mut Frame, layout: &Layout, v: &Visible) {
        let tick = (self.body_width() / 2.0).max(1.0);
        for (rising, color) in [(true, UP), (false, DOWN)] {
            let bars = Path::new(|path| {
                for index in v.first..=v.last {
                    let candle = &self.candles[index];
                    if (candle.close >= candle.open) != rising {
                        continue;
                    }
                    let x = self.x(layout, index).round() + 0.5;
                    path.move_to(Point::new(x, self.y(layout, v, candle.high)));
                    path.line_to(Point::new(x, self.y(layout, v, candle.low)));
                    let open = self.y(layout, v, candle.open);
                    path.move_to(Point::new(x - tick, open));
                    path.line_to(Point::new(x, open));
                    let close = self.y(layout, v, candle.close);
                    path.move_to(Point::new(x, close));
                    path.line_to(Point::new(x + tick, close));
                }
            });
            frame.stroke(&bars, Stroke::default().with_color(color).with_width(1.0));
        }
    }

    fn draw_line(&self, frame: &mut Frame, layout: &Layout, v: &Visible, color: Color) {
        let points: Vec<Point> = (v.first..=v.last)
            .map(|index| {
                Point::new(
                    self.x(layout, index),
                    self.y(layout, v, self.candles[index].close),
                )
            })
            .collect();
        let (Some(first), Some(last)) = (points.first(), points.last()) else {
            return;
        };
        if self.kind == ChartKind::Area {
            let area = Path::new(|path| {
                path.move_to(Point::new(first.x, layout.plot.height));
                for point in &points {
                    path.line_to(*point);
                }
                path.line_to(Point::new(last.x, layout.plot.height));
                path.close();
            });
            frame.fill(&area, color.scale_alpha(0.15));
        }
        let line = Path::new(|path| {
            path.move_to(*first);
            for point in &points[1..] {
                path.line_to(*point);
            }
        });
        frame.stroke(&line, Stroke::default().with_color(color).with_width(1.5));
    }

    /// A study's shaded areas and plots; `y` gives the height of a value.
    fn draw_study(
        &self,
        frame: &mut Frame,
        layout: &Layout,
        visible: &Visible,
        study: &Study,
        y: &dyn Fn(f64) -> f32,
    ) {
        for fill in &study.fills {
            let (Some(upper), Some(lower)) =
                (study.plots.get(fill.upper), study.plots.get(fill.lower))
            else {
                continue;
            };
            if upper.visible
                && lower.visible
                && let Some(indices) = plot_indices(upper, visible, self.candles.len())
            {
                self.draw_fill(frame, layout, fill, upper, lower, indices, y);
            }
        }
        for plot in study.plots.iter().filter(|plot| plot.visible) {
            if let Some(indices) = plot_indices(plot, visible, self.candles.len()) {
                self.draw_plot(frame, layout, plot, indices, y);
            }
        }
    }

    fn draw_plot(
        &self,
        frame: &mut Frame,
        layout: &Layout,
        plot: &Plot,
        indices: RangeInclusive<usize>,
        y: &dyn Fn(f64) -> f32,
    ) {
        let x = |index: usize| self.x_at(layout, index as f64 + f64::from(plot.offset));
        let points = || {
            indices.clone().filter_map(|index| {
                let value = plot.values[index];
                value.is_finite().then_some((index, value))
            })
        };
        match plot.style {
            PlotStyle::Line => {
                let line = Path::new(|path| {
                    let mut previous = None;
                    for (index, value) in points() {
                        let point = Point::new(x(index), y(value));
                        // A gap in the values is a gap in the line.
                        if previous == Some(index.wrapping_sub(1)) {
                            path.line_to(point);
                        } else {
                            path.move_to(point);
                        }
                        previous = Some(index);
                    }
                });
                frame.stroke(
                    &line,
                    Stroke::default()
                        .with_color(plot.color)
                        .with_width(plot.width),
                );
            }
            PlotStyle::Dots => {
                let dots = Path::new(|path| {
                    for (index, value) in points() {
                        path.circle(Point::new(x(index), y(value)), plot.width + 1.0);
                    }
                });
                frame.fill(&dots, plot.color);
            }
            PlotStyle::Histogram(colors) => {
                let width = self.body_width();
                let zero = y(0.0);
                let (other, alpha) = match colors {
                    BarColors::Fixed => (plot.color, 1.0),
                    BarColors::Sign { negative } => (negative, 1.0),
                    // Bars colored like the candles are softened, as volume bars are.
                    BarColors::Candle { falling } => (falling, 0.5),
                };
                let in_plot_color = |index: usize, value: f64| match colors {
                    BarColors::Fixed => true,
                    BarColors::Sign { .. } => value >= 0.0,
                    BarColors::Candle { .. } => self
                        .candles
                        .get(index)
                        .is_none_or(|candle| candle.close >= candle.open),
                };
                for (first_color, color) in [(true, plot.color), (false, other)] {
                    let bars = Path::new(|path| {
                        for (index, value) in points() {
                            if in_plot_color(index, value) != first_color {
                                continue;
                            }
                            let end = y(value);
                            let (top, bottom) = (end.min(zero), end.max(zero));
                            path.rectangle(
                                Point::new(x(index) - width / 2.0, top),
                                Size::new(width, (bottom - top).max(1.0)),
                            );
                        }
                    });
                    frame.fill(&bars, color.scale_alpha(alpha));
                }
            }
        }
    }

    /// The area between two plots, in its crossed color wherever the lower one is above.
    #[allow(clippy::too_many_arguments)]
    fn draw_fill(
        &self,
        frame: &mut Frame,
        layout: &Layout,
        fill: &PlotFill,
        upper: &Plot,
        lower: &Plot,
        indices: RangeInclusive<usize>,
        y: &dyn Fn(f64) -> f32,
    ) {
        // Both plots are shifted alike; the upper one's offset places the area.
        let x = |index: usize| self.x_at(layout, index as f64 + f64::from(upper.offset));
        let mut area: Vec<(Point, Point)> = Vec::new();
        let mut color = fill.color;
        // The last point taken: its x, upper and lower values.
        let mut previous: Option<(f32, f64, f64)> = None;
        for index in indices {
            let (high, low) = match (upper.values.get(index), lower.values.get(index)) {
                (Some(&high), Some(&low)) if high.is_finite() && low.is_finite() => (high, low),
                _ => {
                    fill_area(frame, &mut area, color);
                    previous = None;
                    continue;
                }
            };
            let here = match fill.crossed {
                Some(crossed) if low > high => crossed,
                _ => fill.color,
            };
            let point_x = x(index);
            if here != color {
                // End the area where the lines cross and start the next one there.
                if let Some((x0, high0, low0)) = previous {
                    let (before, after) = (high0 - low0, high - low);
                    let t = if before == after {
                        0.0
                    } else {
                        before / (before - after)
                    };
                    let crossing = Point::new(
                        x0 + t as f32 * (point_x - x0),
                        y(high0 + t * (high - high0)),
                    );
                    area.push((crossing, crossing));
                    fill_area(frame, &mut area, color);
                    area.push((crossing, crossing));
                } else {
                    fill_area(frame, &mut area, color);
                }
                color = here;
            }
            area.push((Point::new(point_x, y(high)), Point::new(point_x, y(low))));
            previous = Some((point_x, high, low));
        }
        fill_area(frame, &mut area, color);
    }

    /// Axis labels, the last price, the crosshair and the legends: redrawn on every
    /// frame.
    ///
    /// The axis labels are here rather than with the cached base: text is drawn after
    /// every shape, so a label under a box on the axis would show through it. Labels
    /// under a box are left out instead.
    #[allow(clippy::too_many_arguments)]
    fn draw_overlay(
        &self,
        frame: &mut Frame,
        theme: &Theme,
        layout: &Layout,
        visible: Option<&Visible>,
        panes: &[PaneView],
        ticks: &Ticks,
        cursor: Option<Point>,
    ) {
        let palette = theme.extended_palette();
        let text = palette.background.base.text;
        let plot = layout.plot;
        let bottom = layout.bottom();
        let hovered = cursor
            .and_then(|point| layout.area(point).map(|area| (area, point)))
            .filter(|(area, _)| matches!(area, Area::Plot | Area::Pane(_)))
            .and_then(|(area, point)| {
                self.index_at(layout, point.x)
                    .map(|index| (index, area, point))
            });

        let last_price = visible.zip(self.candles.last()).and_then(|(v, last)| {
            let y = self.y(layout, v, last.close);
            if !(0.0..=plot.height).contains(&y) {
                return None;
            }
            let color = if last.close >= last.open { UP } else { DOWN };
            let mut lines = vec![self.price_label(v, v.mapper.to_scale(last.close))];
            if self.timeframe.bucket_start(self.now) == last.open_time {
                let left = self.timeframe.next_bucket(self.now) - self.now;
                lines.push(scale::countdown(left, &self.labels));
            }
            Some((y, lines, color))
        });
        let crosshair = visible.zip(hovered).map(|(v, (index, area, point))| {
            let (pane, value) = match area {
                Area::Pane(i) if i < panes.len() => (
                    panes[i].rect,
                    self.format_value(panes[i].study.format, panes[i].value_at(point.y)),
                ),
                _ => (plot, self.price_label(v, self.value_at(layout, v, point.y))),
            };
            let time =
                scale::crosshair_time(self.candles[index].open_time, self.timeframe, &self.labels);
            let x = self.x(layout, index);
            let time_width = time.chars().count() as f32 * 7.0 + 16.0;
            Crosshair {
                x,
                y: point.y,
                pane,
                value,
                time_left: (x - time_width / 2.0).clamp(0.0, (plot.width - time_width).max(0.0)),
                time_width,
                time,
            }
        });

        // The prices of horizontal lines drawn by the user, on the axis.
        let line_prices: Vec<(f32, Anchor, Color)> = visible
            .map(|v| self.line_prices(layout, v))
            .unwrap_or_default();

        // Axis labels, except where a box goes.
        let mut boxes = Vec::new();
        if let Some((y, lines, _)) = &last_price {
            boxes.push(axis_box_span(plot, *y, lines.len()));
        }
        if let Some(crosshair) = &crosshair {
            boxes.push(axis_box_span(crosshair.pane, crosshair.y, 1));
        }
        for (y, _, _) in &line_prices {
            boxes.push(axis_box_span(plot, *y, 1));
        }
        let free = |y: f32| {
            boxes.iter().all(|&(top, end)| {
                y + AXIS_LINE_HEIGHT / 2.0 < top || y - AXIS_LINE_HEIGHT / 2.0 > end
            })
        };
        let axis_label = |frame: &mut Frame, content: String, y: f32| {
            frame.fill_text(label(
                content,
                Point::new(plot.width + 8.0, y),
                text.scale_alpha(0.75),
                Horizontal::Left,
            ));
        };
        if let Some(v) = visible {
            for &tick in &ticks.price {
                let y = self.y_of(layout, v, tick);
                if y > 6.0 && y < plot.height - 6.0 && free(y) {
                    axis_label(frame, self.price_label(v, tick), y);
                }
            }
        }
        for (pane, pane_ticks) in panes.iter().zip(&ticks.panes) {
            // Guide lines, such as RSI's 70 and 30, are labeled too.
            let mut values: Vec<f64> = pane_ticks
                .iter()
                .chain(&pane.study.levels)
                .copied()
                .collect();
            values.sort_by(f64::total_cmp);
            let mut previous = f32::INFINITY;
            for value in values {
                let y = pane.y(value);
                let inside = y > pane.rect.y + 6.0 && y < pane.rect.y + pane.rect.height - 6.0;
                if inside && previous - y >= LABEL_SPACING && free(y) {
                    axis_label(frame, self.format_value(pane.study.format, value), y);
                    previous = y;
                }
            }
        }
        for tick in &ticks.time {
            let x = self.x(layout, tick.index);
            let half = tick.label.chars().count() as f32 * TEXT_SIZE * CHAR_WIDTH / 2.0 + 4.0;
            let under_box = crosshair.as_ref().is_some_and(|crosshair| {
                x + half > crosshair.time_left
                    && x - half < crosshair.time_left + crosshair.time_width
            });
            if x > 20.0 && x < plot.width - 20.0 && !under_box {
                let color = if tick.major {
                    text
                } else {
                    text.scale_alpha(0.75)
                };
                frame.fill_text(label(
                    tick.label.clone(),
                    Point::new(x, bottom + TIME_AXIS_HEIGHT / 2.0),
                    color,
                    Horizontal::Center,
                ));
            }
        }

        if let Some(v) = visible {
            let background = palette.background.base.color;
            self.draw_drawings(frame, layout, v, background);
            for (y, anchor, color) in &line_prices {
                let price = self.price_label(v, v.mapper.to_scale(anchor.price));
                axis_box(frame, plot, *y, &[price], *color, readable_on(*color));
            }
        }

        if let Some((y, lines, color)) = last_price {
            frame.stroke(
                &Path::line(Point::new(0.0, y), Point::new(plot.width, y)),
                dashed(color.scale_alpha(0.8)),
            );
            axis_box(frame, plot, y, &lines, color, Color::WHITE);
        }

        if let Some(crosshair) = crosshair {
            let line = dashed(text.scale_alpha(0.45));
            frame.stroke(
                &Path::line(
                    Point::new(crosshair.x, 0.0),
                    Point::new(crosshair.x, bottom),
                ),
                line,
            );
            frame.stroke(
                &Path::line(
                    Point::new(0.0, crosshair.y),
                    Point::new(plot.width, crosshair.y),
                ),
                line,
            );
            let box_color = palette.background.strong.color;
            axis_box(
                frame,
                crosshair.pane,
                crosshair.y,
                &[crosshair.value],
                box_color,
                text,
            );
            frame.fill_rectangle(
                Point::new(crosshair.time_left, bottom + 1.0),
                Size::new(crosshair.time_width, TIME_AXIS_HEIGHT - 2.0),
                box_color,
            );
            frame.fill_text(label(
                crosshair.time,
                Point::new(
                    crosshair.time_left + crosshair.time_width / 2.0,
                    bottom + TIME_AXIS_HEIGHT / 2.0,
                ),
                text,
                Horizontal::Center,
            ));
        }

        // Legends: the hovered candle, or the newest one.
        let shown = hovered
            .map(|(index, _, _)| index)
            .or(self.candles.len().checked_sub(1));
        let backdrop = palette.background.base.color.scale_alpha(0.7);
        legend_line(
            frame,
            Point::new(10.0, 8.0),
            15.0,
            &[(self.title.clone(), text)],
            backdrop,
        );
        let mut line_y = 50.0;
        if let Some(index) = shown {
            let candle = &self.candles[index];
            let l = &self.labels;
            let d = self.decimals;
            let values = format!(
                "{} {}   {} {}   {} {}   {} {}   {} {}",
                l.open,
                scale::price(candle.open, d),
                l.high,
                scale::price(candle.high, d),
                l.low,
                scale::price(candle.low, d),
                l.close,
                scale::price(candle.close, d),
                l.volume,
                scale::volume(candle.volume, l),
            );
            let color = if candle.close >= candle.open {
                UP
            } else {
                DOWN
            };
            legend_line(
                frame,
                Point::new(10.0, 30.0),
                TEXT_SIZE,
                &[(values, color)],
                backdrop,
            );
            for study in self.overlays() {
                let pieces = self.study_legend(study, index, text);
                legend_line(
                    frame,
                    Point::new(10.0, line_y),
                    TEXT_SIZE,
                    &pieces,
                    backdrop,
                );
                line_y += LEGEND_LINE_HEIGHT;
            }
            for pane in panes {
                let pieces = self.study_legend(pane.study, index, text);
                let position = Point::new(10.0, pane.rect.y + 6.0);
                legend_line(frame, position, TEXT_SIZE, &pieces, backdrop);
            }
        }
        if self.loading_older {
            let loading = (self.labels.loading.clone(), text.scale_alpha(0.6));
            legend_line(
                frame,
                Point::new(10.0, line_y),
                TEXT_SIZE,
                &[loading],
                backdrop,
            );
        }
    }

    /// A study's label and its values at bar `index`, each in its plot's color.
    fn study_legend(&self, study: &Study, index: usize, color: Color) -> Vec<(String, Color)> {
        let mut pieces = vec![(study.label.clone(), color.scale_alpha(0.8))];
        for plot in study.plots.iter().filter(|plot| plot.visible) {
            // The value drawn at this bar: a shifted plot shows an earlier or later one.
            let value = index
                .checked_add_signed(-(plot.offset as isize))
                .and_then(|i| plot.values.get(i))
                .filter(|value| value.is_finite());
            if let Some(&value) = value {
                pieces.push((self.format_value(study.format, value), plot.color));
            }
        }
        pieces
    }
}

/// Writes `pieces` side by side from `position`, over a faint backdrop so that the lines
/// drawn under them do not hide them.
fn legend_line(
    frame: &mut Frame,
    position: Point,
    size: f32,
    pieces: &[(String, Color)],
    backdrop: Color,
) {
    let widths: Vec<f32> = pieces
        .iter()
        .map(|(content, _)| content.chars().count() as f32 * size * CHAR_WIDTH)
        .collect();
    let width = widths.iter().sum::<f32>() + LEGEND_GAP * widths.len().saturating_sub(1) as f32;
    frame.fill_rectangle(
        Point::new(position.x - 4.0, position.y - 2.0),
        Size::new(width + 8.0, size + 6.0),
        backdrop,
    );
    let mut x = position.x;
    for ((content, color), width) in pieces.iter().zip(widths) {
        frame.fill_text(Text {
            content: content.clone(),
            position: Point::new(x, position.y),
            color: *color,
            size: Pixels(size),
            ..Text::default()
        });
        x += width + LEGEND_GAP;
    }
}

/// Fills the area between the upper and lower points gathered so far, and starts over.
fn fill_area(frame: &mut Frame, area: &mut Vec<(Point, Point)>, color: Color) {
    if area.len() >= 2 {
        let path = Path::new(|path| {
            path.move_to(area[0].0);
            for (upper, _) in &area[1..] {
                path.line_to(*upper);
            }
            for (_, lower) in area.iter().rev() {
                path.line_to(*lower);
            }
            path.close();
        });
        frame.fill(&path, color);
    }
    area.clear();
}

/// `-0.00` and the like, written as `0.00`.
fn without_negative_zero(text: String) -> String {
    match text.strip_prefix('-') {
        Some(rest) if !rest.chars().any(|c| matches!(c, '1'..='9')) => rest.to_owned(),
        _ => text,
    }
}

fn label(content: String, position: Point, color: Color, align_x: Horizontal) -> Text {
    Text {
        content,
        position,
        color,
        size: Pixels(TEXT_SIZE),
        align_x: align_x.into(),
        align_y: Vertical::Center,
        ..Text::default()
    }
}

fn dashed(color: Color) -> Stroke<'static> {
    Stroke {
        line_dash: LineDash {
            segments: &DASH,
            offset: 0,
        },
        ..Stroke::default().with_color(color).with_width(1.0)
    }
}

/// The top and bottom of a box with `lines` lines on the axis of `pane`, centered on `y`
/// and kept inside the pane.
fn axis_box_span(pane: Rectangle, y: f32, lines: usize) -> (f32, f32) {
    let height = AXIS_LINE_HEIGHT * lines as f32 + 4.0;
    let lowest = (pane.y + pane.height - height).max(pane.y);
    let top = (y - AXIS_LINE_HEIGHT / 2.0 - 2.0).clamp(pane.y, lowest);
    (top, top + height)
}

/// A filled label on the axis of `pane`, centered on `y`, with one or more lines.
fn axis_box(
    frame: &mut Frame,
    pane: Rectangle,
    y: f32,
    lines: &[String],
    background: Color,
    color: Color,
) {
    let (top, end) = axis_box_span(pane, y, lines.len());
    frame.fill_rectangle(
        Point::new(pane.width + 1.0, top),
        Size::new(PRICE_AXIS_WIDTH - 1.0, end - top),
        background,
    );
    for (i, line) in lines.iter().enumerate() {
        frame.fill_text(label(
            line.clone(),
            Point::new(
                pane.width + 8.0,
                top + 2.0 + AXIS_LINE_HEIGHT * (i as f32 + 0.5),
            ),
            color,
            Horizontal::Left,
        ));
    }
}

impl canvas::Program<ChartMessage> for Chart {
    type State = State;

    fn update(
        &self,
        state: &mut State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<ChartMessage>> {
        let layout = self.layout(bounds.size());
        let Event::Mouse(event) = event else {
            return None;
        };
        match event {
            mouse::Event::ButtonPressed(mouse::Button::Left) => {
                let position = cursor.position_in(bounds)?;
                let area = layout.area(position)?;
                let start = |what| Drag {
                    what,
                    last: position,
                    vertical: 0.0,
                };
                // On the price pane, a press places a point with the drawing tool, or
                // picks a drawing to drag.
                if area == Area::Plot
                    && let Some(v) = self.visible(&layout)
                {
                    if self.tool.is_some() {
                        state.last_press = None;
                        state.drag = Some(start(Dragged::Placing { origin: position }));
                        let anchor = self.anchor_at(&layout, &v, position);
                        return Some(
                            Action::publish(ChartMessage::PlacePoint(anchor)).and_capture(),
                        );
                    }
                    if let Some((id, handle)) = self.drawing_at(&layout, &v, position) {
                        let points = self
                            .drawings
                            .iter()
                            .find(|drawing| drawing.id == id)
                            .map(|drawing| drawing.points.clone())
                            .unwrap_or_default();
                        state.last_press = None;
                        state.drag = Some(start(Dragged::Drawing {
                            id,
                            handle,
                            points,
                            origin: position,
                        }));
                        let select = ChartMessage::SelectDrawing(Some(id));
                        return Some(Action::publish(select).and_capture());
                    }
                }
                let now = Instant::now();
                let double = state.last_press.is_some_and(|(time, place)| {
                    now.duration_since(time) < DOUBLE_CLICK_TIME && place.distance(position) < 5.0
                });
                state.last_press = Some((now, position));
                if double {
                    state.last_press = None;
                    state.drag = None;
                    let reset = match area {
                        Area::PriceAxis => ChartMessage::ResetPrice,
                        Area::Plot | Area::Pane(_) | Area::TimeAxis => ChartMessage::ResetView,
                        Area::PaneAxis(_) | Area::Divider(_) => return Some(Action::capture()),
                    };
                    return Some(Action::publish(reset).and_capture());
                }
                state.drag = Some(start(Dragged::View(area)));
                // A press beside the picked drawing lets go of it.
                if self.selected.is_some() {
                    return Some(Action::publish(ChartMessage::SelectDrawing(None)).and_capture());
                }
                Some(Action::capture())
            }
            mouse::Event::ButtonReleased(mouse::Button::Left) => {
                let drag = state.drag.take()?;
                match drag.what {
                    Dragged::Drawing { .. } => {
                        Some(Action::publish(ChartMessage::FinishMove).and_capture())
                    }
                    // Let go away from where it was pressed, the tool places a point
                    // there too: drawings can be dragged out as well as clicked.
                    Dragged::Placing { origin } => {
                        let position = cursor.position_in(bounds)?;
                        let v = self.visible(&layout)?;
                        if position.distance(origin) <= CLICK_SLOP {
                            return Some(Action::capture());
                        }
                        let anchor = self.anchor_at(&layout, &v, position);
                        Some(Action::publish(ChartMessage::PlacePoint(anchor)).and_capture())
                    }
                    Dragged::View(_) => Some(Action::capture()),
                }
            }
            mouse::Event::CursorMoved { .. } => {
                let Some(drag) = &mut state.drag else {
                    // While a drawing is being placed, its next point follows the cursor.
                    if self.draft.is_some()
                        && let Some(position) = cursor.position_in(bounds)
                        && layout.area(position) == Some(Area::Plot)
                        && let Some(v) = self.visible(&layout)
                    {
                        state.hovering = true;
                        let anchor = self.anchor_at(&layout, &v, position);
                        return Some(Action::publish(ChartMessage::PreviewPoint(anchor)));
                    }
                    // Redraw the crosshair while the cursor is over the chart, and once
                    // more when it leaves so the crosshair disappears.
                    let over = cursor.is_over(bounds);
                    let changed = over || state.hovering;
                    state.hovering = over;
                    return changed.then(Action::request_redraw);
                };
                let position = cursor.position()? - bounds.position();
                let position = Point::new(position.x, position.y);
                let delta = position - drag.last;
                drag.last = position;
                let area = match &drag.what {
                    Dragged::View(area) => *area,
                    Dragged::Drawing {
                        id,
                        handle,
                        points,
                        origin,
                    } => {
                        let v = self.visible(&layout)?;
                        let moved = match handle {
                            Some(handle) => {
                                let mut moved = points.clone();
                                if let Some(point) = moved.get_mut(*handle) {
                                    *point = self.anchor_at(&layout, &v, position);
                                }
                                moved
                            }
                            None => self.moved_points(&layout, &v, points, position - *origin),
                        };
                        let message = ChartMessage::MoveDrawing {
                            id: *id,
                            points: moved,
                        };
                        return Some(Action::publish(message).and_capture());
                    }
                    Dragged::Placing { .. } => {
                        let v = self.visible(&layout)?;
                        let anchor = self.anchor_at(&layout, &v, position);
                        return Some(
                            Action::publish(ChartMessage::PreviewPoint(anchor)).and_capture(),
                        );
                    }
                };
                let message = match area {
                    Area::Plot => {
                        drag.vertical += delta.y;
                        let moves_price = self.price_range.is_some()
                            || drag.vertical.abs() > VERTICAL_DRAG_THRESHOLD;
                        let price = if moves_price {
                            f64::from(delta.y / layout.plot.height)
                        } else {
                            0.0
                        };
                        ChartMessage::Pan {
                            bars: f64::from(delta.x) / self.spacing,
                            price,
                        }
                    }
                    Area::Pane(_) => ChartMessage::Pan {
                        bars: f64::from(delta.x) / self.spacing,
                        price: 0.0,
                    },
                    Area::PriceAxis => ChartMessage::ScalePrice {
                        factor: (-f64::from(delta.y) * 0.006).exp(),
                    },
                    Area::TimeAxis => ChartMessage::Zoom {
                        factor: (f64::from(delta.x) * 0.006).exp(),
                        anchor: 0.0,
                    },
                    Area::Divider(index) => ChartMessage::MoveDivider {
                        index,
                        pixels: f64::from(delta.y),
                    },
                    Area::PaneAxis(_) => return Some(Action::capture()),
                };
                Some(Action::publish(message).and_capture())
            }
            mouse::Event::CursorLeft => {
                state.hovering = false;
                Some(Action::request_redraw())
            }
            mouse::Event::WheelScrolled { delta } => {
                let position = cursor.position_in(bounds)?;
                let lines = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => f64::from(*y),
                    mouse::ScrollDelta::Pixels { y, .. } => f64::from(*y) / 60.0,
                };
                let anchor = f64::from((layout.plot.width - position.x).max(0.0));
                Some(
                    Action::publish(ChartMessage::Zoom {
                        factor: 1.15f64.powf(lines),
                        anchor,
                    })
                    .and_capture(),
                )
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let layout = self.layout(bounds.size());
        self.plot_width.set(f64::from(layout.plot.width));
        self.panes_height
            .set(f64::from(panes_height(bounds.size())));
        let visible = self.visible(&layout);
        let panes = self.pane_views(&layout, visible.as_ref());
        let ticks = self.ticks(&layout, visible.as_ref(), &panes);

        let base = self.cache.draw(renderer, bounds.size(), |frame| {
            self.draw_base(frame, theme, &layout, visible.as_ref(), &panes, &ticks);
        });
        let mut overlay = Frame::new(renderer, bounds.size());
        self.draw_overlay(
            &mut overlay,
            theme,
            &layout,
            visible.as_ref(),
            &panes,
            &ticks,
            cursor.position_in(bounds),
        );
        vec![base, overlay.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if let Some(drag) = &state.drag {
            return match drag.what {
                Dragged::View(Area::Divider(_)) => mouse::Interaction::ResizingVertically,
                Dragged::Placing { .. } => mouse::Interaction::Crosshair,
                _ => mouse::Interaction::Grabbing,
            };
        }
        let layout = self.layout(bounds.size());
        let position = cursor.position_in(bounds);
        match position.and_then(|point| layout.area(point)) {
            Some(Area::Plot) => {
                // A drawing under the cursor can be picked.
                let over_drawing = self.tool.is_none()
                    && position
                        .zip(self.visible(&layout))
                        .is_some_and(|(point, v)| self.drawing_at(&layout, &v, point).is_some());
                if over_drawing {
                    mouse::Interaction::Pointer
                } else {
                    mouse::Interaction::Crosshair
                }
            }
            Some(Area::Pane(_)) => mouse::Interaction::Crosshair,
            Some(Area::PriceAxis | Area::Divider(_)) => mouse::Interaction::ResizingVertically,
            Some(Area::TimeAxis) => mouse::Interaction::ResizingHorizontally,
            Some(Area::PaneAxis(_)) | None => mouse::Interaction::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChartLabels;
    use crate::study::StudyPlacement;
    use fitil_core::{Candle, Drawing, DrawingKind};

    fn chart(candles: usize, studies: Vec<Study>) -> Chart {
        let labels = ChartLabels {
            open: "O".into(),
            high: "H".into(),
            low: "L".into(),
            close: "C".into(),
            volume: "V".into(),
            months: std::array::from_fn(|i| format!("M{}", i + 1)),
            magnitudes: ["K", "M", "B"].map(String::from),
            day_suffix: "d".into(),
            hour_suffix: "h".into(),
            minute_suffix: "m".into(),
            loading: "…".into(),
            note: "Note".into(),
            bars: "bars".into(),
        };
        let mut chart = Chart::new(
            "TEST".into(),
            "1h".parse().unwrap(),
            2,
            ChartKind::Candles,
            PriceScale::Linear,
            labels,
        );
        chart.set_candles(
            (0..candles as i64)
                .map(|hour| Candle {
                    open_time: hour * 3_600_000,
                    open: 10.0,
                    high: 12.0,
                    low: 9.0,
                    close: 11.0,
                    volume: 1.0,
                    quote_volume: None,
                    trades: None,
                    taker_buy_volume: None,
                })
                .collect(),
        );
        chart.set_studies(studies);
        chart
    }

    fn study(placement: StudyPlacement, plots: Vec<Plot>) -> Study {
        Study {
            id: 1,
            label: "S".into(),
            placement,
            plots,
            fills: Vec::new(),
            levels: Vec::new(),
            range: None,
            format: ValueFormat::Decimals(2),
        }
    }

    fn plot(values: Vec<f64>, style: PlotStyle) -> Plot {
        Plot {
            values,
            offset: 0,
            style,
            color: Color::WHITE,
            width: 1.0,
            visible: true,
        }
    }

    /// An 800 by 600 pixel area for the panes.
    fn size() -> Size {
        Size::new(800.0 + PRICE_AXIS_WIDTH, 600.0 + TIME_AXIS_HEIGHT)
    }

    #[test]
    fn panes_split_the_height_and_have_draggable_edges() {
        let layout = Layout::new(size(), &[3.0, 1.0]);
        assert_eq!((layout.plot.height, layout.panes[0].y), (450.0, 450.0));
        assert_eq!(layout.bottom(), 600.0);

        assert_eq!(layout.area(Point::new(100.0, 100.0)), Some(Area::Plot));
        assert_eq!(
            layout.area(Point::new(100.0, 452.0)),
            Some(Area::Divider(0))
        );
        assert_eq!(layout.area(Point::new(100.0, 500.0)), Some(Area::Pane(0)));
        assert_eq!(
            layout.area(Point::new(850.0, 500.0)),
            Some(Area::PaneAxis(0))
        );
        assert_eq!(layout.area(Point::new(100.0, 610.0)), Some(Area::TimeAxis));
    }

    #[test]
    fn indicators_over_the_price_stay_in_view() {
        let high = |chart: &Chart| {
            let layout = chart.layout(size());
            chart.visible(&layout).unwrap().high
        };
        let without = high(&chart(50, Vec::new()));
        let line = plot(vec![20.0; 50], PlotStyle::Line);
        let with = high(&chart(
            50,
            vec![study(StudyPlacement::Overlay, vec![line.clone()])],
        ));
        assert!(without < 20.0 && with > 20.0, "{without} {with}");

        let hidden = Plot {
            visible: false,
            ..line
        };
        let ignored = high(&chart(
            50,
            vec![study(StudyPlacement::Overlay, vec![hidden])],
        ));
        assert_eq!(ignored, without);
    }

    #[test]
    fn pane_ranges_fit_values_guides_and_zero() {
        let pane_range = |study: Study| {
            let chart = chart(50, vec![study]);
            let layout = chart.layout(size());
            let visible = chart.visible(&layout).unwrap();
            let pane = &chart.pane_views(&layout, Some(&visible))[0];
            (pane.low, pane.high)
        };

        let mut fixed = study(
            StudyPlacement::Pane,
            vec![plot(vec![50.0; 50], PlotStyle::Line)],
        );
        fixed.range = Some((0.0, 100.0));
        assert_eq!(pane_range(fixed), (-5.0, 105.0));

        // Volume-like bars start at the bottom.
        let bars = PlotStyle::Histogram(BarColors::Fixed);
        let volume = study(StudyPlacement::Pane, vec![plot(vec![10.0; 50], bars)]);
        assert_eq!(pane_range(volume), (0.0, 11.0));

        let mut guided = study(
            StudyPlacement::Pane,
            vec![plot(vec![50.0; 50], PlotStyle::Line)],
        );
        guided.levels = vec![0.0, 100.0];
        assert_eq!(pane_range(guided), (-10.0, 110.0));
    }

    #[test]
    fn shifted_plots_draw_other_values() {
        let chart = chart(50, Vec::new());
        let layout = chart.layout(size());
        let visible = chart.visible(&layout).unwrap();
        let mut shifted = plot(vec![1.0; 50], PlotStyle::Line);
        // The view ends five bars after the last candle.
        assert_eq!(plot_indices(&shifted, &visible, 50), Some(0..=49));
        shifted.offset = 26;
        assert_eq!(visible.last_bar, 54);
        assert_eq!(plot_indices(&shifted, &visible, 50), Some(0..=28));
        shifted.offset = -200;
        assert_eq!(plot_indices(&shifted, &visible, 50), None);
    }

    #[test]
    fn drawings_are_picked_where_they_are_drawn() {
        const HOUR: i64 = 3_600_000;
        let mut chart = chart(50, Vec::new());
        let layout = chart.layout(size());
        let visible = chart.visible(&layout).unwrap();
        let anchor = Anchor {
            time: 40 * HOUR,
            price: 11.5,
        };
        chart.set_drawings(vec![Drawing::new(
            1,
            DrawingKind::HorizontalLine,
            vec![anchor],
        )]);
        let point = chart.point_of(&layout, &visible, anchor);
        let near = Point::new(point.x - 200.0, point.y + 3.0);
        assert_eq!(chart.drawing_at(&layout, &visible, near), Some((1, None)));
        let far = Point::new(point.x, point.y + 30.0);
        assert_eq!(chart.drawing_at(&layout, &visible, far), None);

        // Once picked, its point can be grabbed.
        chart.update(ChartMessage::SelectDrawing(Some(1)));
        assert_eq!(
            chart.drawing_at(&layout, &visible, point),
            Some((1, Some(0)))
        );

        // The time and price under a point are those it was drawn at.
        let back = chart.anchor_at(&layout, &visible, point);
        assert_eq!(back.time, anchor.time);
        assert!((back.price - anchor.price).abs() < 1e-6);

        // With the magnet, a point sticks to the nearest price of the nearest candle.
        chart.set_magnet(true);
        let above_high = Point::new(point.x + 2.0, chart.y(&layout, &visible, 12.2));
        let stuck = chart.anchor_at(&layout, &visible, above_high);
        assert_eq!((stuck.time, stuck.price), (40 * HOUR, 12.0));

        // Hidden drawings cannot be picked.
        chart.set_drawings_hidden(true);
        assert_eq!(chart.drawing_at(&layout, &visible, near), None);
    }

    #[test]
    fn negative_zero_loses_its_sign() {
        assert_eq!(without_negative_zero("-0.00".into()), "0.00");
        assert_eq!(without_negative_zero("-0.01".into()), "-0.01");
        assert_eq!(without_negative_zero("-0.0000".into()), "0.0000");
        assert_eq!(without_negative_zero("12.5".into()), "12.5");
    }
}
