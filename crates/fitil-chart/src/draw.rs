//! Drawing the chart and turning mouse input into [`ChartMessage`]s.

use std::time::{Duration, Instant};

use iced::alignment::{Horizontal, Vertical};
use iced::mouse;
use iced::widget::canvas::{self, Action, Event, Frame, Geometry, LineDash, Path, Stroke, Text};
use iced::{Color, Pixels, Point, Rectangle, Renderer, Size, Theme};

use crate::scale::{self, Mapper};
use crate::{Chart, ChartKind, ChartMessage, PriceScale};

const PRICE_AXIS_WIDTH: f32 = 84.0;
const TIME_AXIS_HEIGHT: f32 = 26.0;
const UP: Color = Color::from_rgb8(0x2E, 0xBD, 0x85);
const DOWN: Color = Color::from_rgb8(0xF6, 0x46, 0x5D);
/// Share of the plot height used by volume bars, at the bottom.
const VOLUME_SHARE: f64 = 0.18;
/// Space above and below the candles when the price range is fitted, as plot shares.
const TOP_MARGIN: f64 = 0.08;
const BOTTOM_MARGIN: f64 = 0.22;
/// Vertical drag, in pixels, before dragging the plot also moves the price range.
const VERTICAL_DRAG_THRESHOLD: f32 = 12.0;
const TEXT_SIZE: f32 = 12.0;
const DASH: [f32; 2] = [4.0, 4.0];
const DOUBLE_CLICK_TIME: Duration = Duration::from_millis(350);

/// Mouse state kept between events.
#[derive(Default)]
pub struct State {
    drag: Option<Drag>,
    /// Time and place of the last press, to recognise double clicks.
    last_press: Option<(Instant, Point)>,
    hovering: bool,
}

struct Drag {
    area: Area,
    last: Point,
    /// Vertical distance dragged so far.
    vertical: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Area {
    Plot,
    PriceAxis,
    TimeAxis,
}

/// Where the plot and the axes are inside the canvas.
struct Layout {
    size: Size,
    plot: Rectangle,
}

impl Layout {
    fn new(size: Size) -> Self {
        let plot = Rectangle {
            x: 0.0,
            y: 0.0,
            width: (size.width - PRICE_AXIS_WIDTH).max(1.0),
            height: (size.height - TIME_AXIS_HEIGHT).max(1.0),
        };
        Self { size, plot }
    }

    fn area(&self, point: Point) -> Option<Area> {
        if point.x < 0.0 || point.y < 0.0 || point.x > self.size.width || point.y > self.size.height
        {
            None
        } else if point.x > self.plot.width {
            Some(Area::PriceAxis)
        } else if point.y > self.plot.height {
            Some(Area::TimeAxis)
        } else {
            Some(Area::Plot)
        }
    }
}

/// What is on screen, worked out once per frame.
struct Visible {
    first: usize,
    last: usize,
    mapper: Mapper,
    /// Price range of the plot's full height, in scale units.
    low: f64,
    high: f64,
    max_volume: f64,
}

impl Chart {
    fn x(&self, layout: &Layout, index: usize) -> f32 {
        (f64::from(layout.plot.width) - (self.right - index as f64) * self.spacing) as f32
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
        let first = (self.right - f64::from(layout.plot.width) / self.spacing)
            .floor()
            .max(0.0) as usize;
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
        let (low, high) = shown.iter().fold((f64::MAX, f64::MIN), |(low, high), c| {
            let (l, h) = if line_only {
                (c.close, c.close)
            } else {
                (c.low, c.high)
            };
            (low.min(mapper.to_scale(l)), high.max(mapper.to_scale(h)))
        });
        let (low, high) = match self.price_range {
            Some(range) => range,
            None => {
                let span = if high > low {
                    high - low
                } else {
                    high.abs().max(1.0) * 0.01
                };
                let candles_share = 1.0 - TOP_MARGIN - BOTTOM_MARGIN;
                (
                    low - span * BOTTOM_MARGIN / candles_share,
                    high + span * TOP_MARGIN / candles_share,
                )
            }
        };
        self.shown_range.set(Some((low, high)));
        let max_volume = shown.iter().map(|c| c.volume).fold(0.0, f64::max);
        Some(Visible {
            first,
            last,
            mapper,
            low,
            high,
            max_volume,
        })
    }

    fn y(&self, layout: &Layout, visible: &Visible, price: f64) -> f32 {
        self.y_of(layout, visible, visible.mapper.to_scale(price))
    }

    fn y_of(&self, layout: &Layout, visible: &Visible, value: f64) -> f32 {
        let share = (visible.high - value) / (visible.high - visible.low);
        (share * f64::from(layout.plot.height)) as f32
    }

    fn value_at(&self, layout: &Layout, visible: &Visible, y: f32) -> f64 {
        visible.high - f64::from(y / layout.plot.height) * (visible.high - visible.low)
    }

    fn price_label(&self, visible: &Visible, value: f64) -> String {
        match self.scale {
            PriceScale::Percent => format!("{value:+.2}%"),
            _ => scale::price(visible.mapper.to_price(value), self.decimals),
        }
    }

    /// Grid, volume, candles and axis labels: everything that only changes with the
    /// data or the view, so it can be cached.
    fn draw_base(
        &self,
        frame: &mut Frame,
        theme: &Theme,
        layout: &Layout,
        visible: Option<&Visible>,
    ) {
        let palette = theme.extended_palette();
        let text = palette.background.base.text;
        let grid = text.scale_alpha(0.07);
        let plot = layout.plot;

        let price_ticks = visible.map(|v| {
            let count = (plot.height / 70.0).max(2.0) as usize;
            v.mapper.ticks(v.low, v.high, count)
        });
        let time_ticks = visible.map(|v| {
            scale::time_ticks(
                &self.candles,
                v.first,
                v.last,
                self.spacing,
                self.timeframe,
                &self.labels,
            )
        });

        if let (Some(v), Some(ticks)) = (visible, &price_ticks) {
            for &tick in ticks {
                let y = self.y_of(layout, v, tick);
                frame.stroke(
                    &Path::line(Point::new(0.0, y), Point::new(plot.width, y)),
                    Stroke::default().with_color(grid).with_width(1.0),
                );
            }
        }
        if let Some(ticks) = &time_ticks {
            for tick in ticks {
                let x = self.x(layout, tick.index);
                frame.stroke(
                    &Path::line(Point::new(x, 0.0), Point::new(x, plot.height)),
                    Stroke::default().with_color(grid).with_width(1.0),
                );
            }
        }

        if let Some(v) = visible {
            // With a price range set by hand, candles can reach past the plot; the
            // canvas does not clip by itself, so they would cover the widgets around it.
            frame.with_clip(plot, |frame| {
                self.draw_volume(frame, layout, v);
                match self.kind {
                    ChartKind::Candles | ChartKind::HeikinAshi => {
                        self.draw_candles(frame, layout, v)
                    }
                    ChartKind::Bars => self.draw_bars(frame, layout, v),
                    ChartKind::Line | ChartKind::Area => {
                        self.draw_line(frame, layout, v, palette.primary.base.color)
                    }
                }
            });
        }

        // Axis backgrounds also hide any bar reaching past the plot.
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
            Point::new(0.0, plot.height),
            Size::new(layout.size.width, TIME_AXIS_HEIGHT),
            background,
        );
        frame.stroke(
            &Path::line(
                Point::new(plot.width, 0.0),
                Point::new(plot.width, plot.height),
            ),
            border,
        );
        frame.stroke(
            &Path::line(
                Point::new(0.0, plot.height),
                Point::new(layout.size.width, plot.height),
            ),
            border,
        );

        if let (Some(v), Some(ticks)) = (visible, &price_ticks) {
            for &tick in ticks {
                let y = self.y_of(layout, v, tick);
                if y > 6.0 && y < plot.height - 6.0 {
                    frame.fill_text(label(
                        self.price_label(v, tick),
                        Point::new(plot.width + 8.0, y),
                        text.scale_alpha(0.75),
                        Horizontal::Left,
                    ));
                }
            }
        }
        if let Some(ticks) = time_ticks {
            for tick in ticks {
                let x = self.x(layout, tick.index);
                if x > 20.0 && x < plot.width - 20.0 {
                    let color = if tick.major {
                        text
                    } else {
                        text.scale_alpha(0.75)
                    };
                    frame.fill_text(label(
                        tick.label,
                        Point::new(x, plot.height + TIME_AXIS_HEIGHT / 2.0),
                        color,
                        Horizontal::Center,
                    ));
                }
            }
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

    /// The last price, the crosshair and the legend: redrawn on every frame.
    fn draw_overlay(
        &self,
        frame: &mut Frame,
        theme: &Theme,
        layout: &Layout,
        visible: Option<&Visible>,
        cursor: Option<Point>,
    ) {
        let palette = theme.extended_palette();
        let text = palette.background.base.text;
        let plot = layout.plot;
        let hovered = cursor
            .filter(|point| layout.area(*point) == Some(Area::Plot))
            .and_then(|point| self.index_at(layout, point.x).map(|index| (index, point)));

        if let (Some(v), Some(last)) = (visible, self.candles.last()) {
            let color = if last.close >= last.open { UP } else { DOWN };
            let y = self.y(layout, v, last.close);
            if (0.0..=plot.height).contains(&y) {
                frame.stroke(
                    &Path::line(Point::new(0.0, y), Point::new(plot.width, y)),
                    dashed(color.scale_alpha(0.8)),
                );
                let price = self.price_label(v, v.mapper.to_scale(last.close));
                let mut lines = vec![price];
                if self.timeframe.bucket_start(self.now) == last.open_time {
                    let left = self.timeframe.next_bucket(self.now) - self.now;
                    lines.push(scale::countdown(left, &self.labels));
                }
                axis_box(frame, layout, y, &lines, color, Color::WHITE);
            }
        }

        if let (Some(v), Some((index, point))) = (visible, hovered) {
            let x = self.x(layout, index);
            let line = dashed(text.scale_alpha(0.45));
            frame.stroke(
                &Path::line(Point::new(x, 0.0), Point::new(x, plot.height)),
                line,
            );
            frame.stroke(
                &Path::line(Point::new(0.0, point.y), Point::new(plot.width, point.y)),
                line,
            );

            let value = self.value_at(layout, v, point.y);
            let box_color = palette.background.strong.color;
            axis_box(
                frame,
                layout,
                point.y,
                &[self.price_label(v, value)],
                box_color,
                text,
            );

            let time =
                scale::crosshair_time(self.candles[index].open_time, self.timeframe, &self.labels);
            let width = time.chars().count() as f32 * 7.0 + 16.0;
            let left = (x - width / 2.0).clamp(0.0, (plot.width - width).max(0.0));
            frame.fill_rectangle(
                Point::new(left, plot.height + 1.0),
                Size::new(width, TIME_AXIS_HEIGHT - 2.0),
                box_color,
            );
            frame.fill_text(label(
                time,
                Point::new(left + width / 2.0, plot.height + TIME_AXIS_HEIGHT / 2.0),
                text,
                Horizontal::Center,
            ));
        }

        // Legend: the hovered candle, or the newest one.
        let shown = hovered
            .map(|(index, _)| index)
            .or(self.candles.len().checked_sub(1));
        frame.fill_text(Text {
            content: self.title.clone(),
            position: Point::new(10.0, 8.0),
            color: text,
            size: Pixels(15.0),
            ..Text::default()
        });
        if let Some(candle) = shown.map(|index| &self.candles[index]) {
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
            frame.fill_text(Text {
                content: values,
                position: Point::new(10.0, 30.0),
                color,
                size: Pixels(TEXT_SIZE),
                ..Text::default()
            });
        }
        if self.loading_older {
            frame.fill_text(Text {
                content: self.labels.loading.clone(),
                position: Point::new(10.0, 50.0),
                color: text.scale_alpha(0.6),
                size: Pixels(TEXT_SIZE),
                ..Text::default()
            });
        }
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

/// A filled label on the price axis, centered on `y`, with one or more lines.
fn axis_box(
    frame: &mut Frame,
    layout: &Layout,
    y: f32,
    lines: &[String],
    background: Color,
    color: Color,
) {
    let line_height = 16.0;
    let height = line_height * lines.len() as f32 + 4.0;
    let top = (y - line_height / 2.0 - 2.0).clamp(0.0, (layout.plot.height - height).max(0.0));
    frame.fill_rectangle(
        Point::new(layout.plot.width + 1.0, top),
        Size::new(PRICE_AXIS_WIDTH - 1.0, height),
        background,
    );
    for (i, line) in lines.iter().enumerate() {
        frame.fill_text(label(
            line.clone(),
            Point::new(
                layout.plot.width + 8.0,
                top + 2.0 + line_height * (i as f32 + 0.5),
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
        let layout = Layout::new(bounds.size());
        let Event::Mouse(event) = event else {
            return None;
        };
        match event {
            mouse::Event::ButtonPressed(mouse::Button::Left) => {
                let position = cursor.position_in(bounds)?;
                let area = layout.area(position)?;
                let now = Instant::now();
                let double = state.last_press.is_some_and(|(time, place)| {
                    now.duration_since(time) < DOUBLE_CLICK_TIME && place.distance(position) < 5.0
                });
                state.last_press = Some((now, position));
                if double {
                    state.last_press = None;
                    state.drag = None;
                    let reset = if area == Area::PriceAxis {
                        ChartMessage::ResetPrice
                    } else {
                        ChartMessage::ResetView
                    };
                    return Some(Action::publish(reset).and_capture());
                }
                state.drag = Some(Drag {
                    area,
                    last: position,
                    vertical: 0.0,
                });
                Some(Action::capture())
            }
            mouse::Event::ButtonReleased(mouse::Button::Left) => {
                state.drag.take().map(|_| Action::capture())
            }
            mouse::Event::CursorMoved { .. } => {
                let Some(drag) = &mut state.drag else {
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
                let message = match drag.area {
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
                    Area::PriceAxis => ChartMessage::ScalePrice {
                        factor: (-f64::from(delta.y) * 0.006).exp(),
                    },
                    Area::TimeAxis => ChartMessage::Zoom {
                        factor: (f64::from(delta.x) * 0.006).exp(),
                        anchor: 0.0,
                    },
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
        let layout = Layout::new(bounds.size());
        self.plot_width.set(f64::from(layout.plot.width));
        let visible = self.visible(&layout);

        let base = self.cache.draw(renderer, bounds.size(), |frame| {
            self.draw_base(frame, theme, &layout, visible.as_ref());
        });
        let mut overlay = Frame::new(renderer, bounds.size());
        self.draw_overlay(
            &mut overlay,
            theme,
            &layout,
            visible.as_ref(),
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
        if state.drag.is_some() {
            return mouse::Interaction::Grabbing;
        }
        let layout = Layout::new(bounds.size());
        match cursor
            .position_in(bounds)
            .and_then(|point| layout.area(point))
        {
            Some(Area::Plot) => mouse::Interaction::Crosshair,
            Some(Area::PriceAxis) => mouse::Interaction::ResizingVertically,
            Some(Area::TimeAxis) => mouse::Interaction::ResizingHorizontally,
            None => mouse::Interaction::default(),
        }
    }
}
