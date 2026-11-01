use std::cell::Cell;
use std::collections::BTreeMap;

use fitil_core::{Anchor, Candle, Drawing, DrawingKind, Timeframe};
use iced::widget::canvas;
use iced::{Element, Fill};

use crate::drawings::Draft;
use crate::study::{Study, StudyPlacement};

/// Shares of the height given to the price pane and to each new indicator pane: one
/// indicator pane takes a quarter of the chart, two a fifth each.
const PRICE_PANE_WEIGHT: f64 = 3.0;
const INDICATOR_PANE_WEIGHT: f64 = 1.0;
/// Panes are never dragged smaller than this, in pixels.
const MIN_PANE_HEIGHT: f64 = 30.0;

/// Pixels per bar when a chart opens.
const DEFAULT_SPACING: f64 = 8.0;
const MIN_SPACING: f64 = 1.0;
const MAX_SPACING: f64 = 60.0;
/// Empty bars kept to the right of the newest candle.
const RIGHT_MARGIN: f64 = 5.0;
/// Bars of the newest or oldest candles that always stay on screen.
const KEEP_VISIBLE: f64 = 3.0;
/// Older candles are wanted once fewer than this many are left of the view.
const LOAD_AHEAD: f64 = 150.0;

/// How candles are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChartKind {
    #[default]
    Candles,
    /// OHLC bars.
    Bars,
    Line,
    Area,
    HeikinAshi,
}

impl ChartKind {
    pub const ALL: [ChartKind; 5] = [
        ChartKind::Candles,
        ChartKind::Bars,
        ChartKind::Line,
        ChartKind::Area,
        ChartKind::HeikinAshi,
    ];
}

/// How prices map to height.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PriceScale {
    #[default]
    Linear,
    Logarithmic,
    /// Change from the first visible close, in percent.
    Percent,
}

/// Text the chart draws, in the user's language.
#[derive(Debug, Clone)]
pub struct ChartLabels {
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
    /// Short month names, January first.
    pub months: [String; 12],
    /// Suffixes for thousands, millions and billions.
    pub magnitudes: [String; 3],
    /// Suffix for days in a countdown, like the `d` in `2d 03:00:00`.
    pub day_suffix: String,
    /// Suffixes for hours and minutes in a duration, like the `h` and `m` in `5h 30m`.
    pub hour_suffix: String,
    pub minute_suffix: String,
    /// Shown while older candles load.
    pub loading: String,
    /// The words of a new note.
    pub note: String,
    /// Follows a number of candles, as in `42 bars`.
    pub bars: String,
}

/// What the user did on the chart, in chart terms.
#[derive(Debug, Clone, PartialEq)]
pub enum ChartMessage {
    /// Dragging the plot. Positive `bars` reveals older candles; `price` moves the price
    /// range by that fraction of its height.
    Pan { bars: f64, price: f64 },
    /// Changes the bar width by `factor`, keeping the bar `anchor` pixels left of the
    /// plot's right edge in place.
    Zoom { factor: f64, anchor: f64 },
    /// Stretches (`factor` > 1) or squeezes the price range around its middle.
    ScalePrice { factor: f64 },
    /// Fits the price range to the visible candles again.
    ResetPrice,
    /// Shows the newest candles at the default zoom.
    ResetView,
    /// Moves the edge above indicator pane `index` (0 is the topmost) down by `pixels`,
    /// or up when negative.
    MoveDivider { index: usize, pixels: f64 },
    /// A point placed with the drawing tool.
    PlacePoint(Anchor),
    /// Where the drawing tool's next point would go: under the cursor.
    PreviewPoint(Anchor),
    /// A click that picked a drawing, or none.
    SelectDrawing(Option<u64>),
    /// A drawing dragged to new points.
    MoveDrawing { id: u64, points: Vec<Anchor> },
    /// The end of a drag of a drawing.
    FinishMove,
    /// Drops the drawing being placed, or else the tool, or else the selection.
    Cancel,
}

/// A candle chart for one series.
pub struct Chart {
    pub(crate) title: String,
    pub(crate) timeframe: Timeframe,
    pub(crate) decimals: u32,
    /// Candles as received; `candles` holds what is drawn.
    raw: Vec<Candle>,
    pub(crate) candles: Vec<Candle>,
    pub(crate) kind: ChartKind,
    pub(crate) scale: PriceScale,
    pub(crate) labels: ChartLabels,
    /// Bar index at the plot's right edge; beyond the last bar leaves empty room.
    pub(crate) right: f64,
    /// Pixels per bar.
    pub(crate) spacing: f64,
    /// Price range in scale units set by the user; `None` fits the visible candles.
    pub(crate) price_range: Option<(f64, f64)>,
    /// Current time on the exchange's clock, for the countdown.
    pub(crate) now: i64,
    pub(crate) loading_older: bool,
    history_complete: bool,
    /// Indicators, in the order given.
    pub(crate) studies: Vec<Study>,
    /// Whether volume bars are drawn under the candles.
    pub(crate) show_volume: bool,
    /// Shares of the height of the price pane and of each indicator pane by study id;
    /// kept for studies that are gone, so a pane shown again keeps its height.
    price_weight: f64,
    pane_weights: BTreeMap<u64, f64>,
    /// The symbol's drawings, and what is being done with them.
    pub(crate) drawings: Vec<Drawing>,
    pub(crate) selected: Option<u64>,
    /// The drawing tool in use; without one, clicks pick drawings.
    pub(crate) tool: Option<DrawingKind>,
    /// A drawing being placed, point by point.
    pub(crate) draft: Option<Draft>,
    /// Points placed or dragged stick to the nearest price of the nearest candle.
    pub(crate) magnet: bool,
    pub(crate) drawings_hidden: bool,
    /// The drawings before each change, to undo it, and after each undo, to redo it.
    pub(crate) undo: Vec<Vec<Drawing>>,
    pub(crate) redo: Vec<Vec<Drawing>>,
    /// A drawing is being dragged; its points before the drag are on `undo`.
    pub(crate) moving: bool,
    /// The drawings changed since they were last handed out to be saved.
    pub(crate) unsaved: bool,
    /// Plot width, shown price range and height of all panes as of the last draw.
    pub(crate) plot_width: Cell<f64>,
    pub(crate) shown_range: Cell<Option<(f64, f64)>>,
    pub(crate) panes_height: Cell<f64>,
    pub(crate) cache: canvas::Cache,
}

impl Chart {
    pub fn new(
        title: String,
        timeframe: Timeframe,
        decimals: u32,
        kind: ChartKind,
        scale: PriceScale,
        labels: ChartLabels,
    ) -> Self {
        Self {
            title,
            timeframe,
            decimals,
            raw: Vec::new(),
            candles: Vec::new(),
            kind,
            scale,
            labels,
            right: RIGHT_MARGIN,
            spacing: DEFAULT_SPACING,
            price_range: None,
            now: 0,
            loading_older: false,
            history_complete: false,
            studies: Vec::new(),
            show_volume: true,
            price_weight: PRICE_PANE_WEIGHT,
            pane_weights: BTreeMap::new(),
            drawings: Vec::new(),
            selected: None,
            tool: None,
            draft: None,
            magnet: false,
            drawings_hidden: false,
            undo: Vec::new(),
            redo: Vec::new(),
            moving: false,
            unsaved: false,
            plot_width: Cell::new(800.0),
            shown_range: Cell::new(None),
            panes_height: Cell::new(600.0),
            cache: canvas::Cache::new(),
        }
    }

    pub fn view(&self) -> Element<'_, ChartMessage> {
        canvas(self).width(Fill).height(Fill).into()
    }

    /// Replaces the candles, sorted by open time.
    ///
    /// When older candles are added in front, the view stays on the same bars. When
    /// newer ones arrive while the newest candle is in view, the view follows them.
    pub fn set_candles(&mut self, candles: Vec<Candle>) {
        let old_len = self.raw.len() as f64;
        let added_in_front = match self.raw.first() {
            Some(first) => candles.partition_point(|c| c.open_time < first.open_time) as f64,
            None => 0.0,
        };
        let following = self.right >= old_len - 1.0;
        let new_len = candles.len() as f64;
        self.raw = candles;

        if old_len == 0.0 {
            self.right = new_len - 1.0 + RIGHT_MARGIN;
        } else {
            self.right += added_in_front;
            if following {
                self.right += (new_len - added_in_front - old_len).max(0.0);
            }
        }
        self.refresh();
    }

    /// Shows another series. Its candles start out empty and the view follows the newest
    /// once they arrive; the zoom and the look (chart type, scale, volume bars, pane
    /// heights) stay.
    pub fn replace_series(&mut self, title: String, timeframe: Timeframe, decimals: u32) {
        self.title = title;
        self.timeframe = timeframe;
        self.decimals = decimals;
        self.raw.clear();
        self.candles.clear();
        self.studies.clear();
        self.right = RIGHT_MARGIN;
        self.price_range = None;
        self.loading_older = false;
        self.history_complete = false;
        self.draft = None;
        self.shown_range.set(None);
        self.cache.clear();
    }

    pub fn set_kind(&mut self, kind: ChartKind) {
        self.kind = kind;
        self.refresh();
    }

    pub fn set_scale(&mut self, scale: PriceScale) {
        self.scale = scale;
        self.price_range = None;
        self.refresh();
    }

    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }

    pub fn set_labels(&mut self, labels: ChartLabels) {
        self.labels = labels;
        self.cache.clear();
    }

    /// Updates the clock used for the countdown; only the overlay is redrawn.
    pub fn set_now(&mut self, now: i64) {
        self.now = now;
    }

    pub fn set_loading_older(&mut self, loading: bool) {
        self.loading_older = loading;
    }

    pub fn set_history_complete(&mut self, complete: bool) {
        self.history_complete = complete;
    }

    /// Replaces the indicators. Their values line up with the candles last given to
    /// [`Chart::set_candles`].
    pub fn set_studies(&mut self, studies: Vec<Study>) {
        self.studies = studies;
        self.cache.clear();
    }

    pub fn set_show_volume(&mut self, show: bool) {
        self.show_volume = show;
        self.cache.clear();
    }

    pub(crate) fn overlays(&self) -> impl Iterator<Item = &Study> {
        self.studies
            .iter()
            .filter(|study| study.placement == StudyPlacement::Overlay)
    }

    /// The studies with a pane of their own, top to bottom.
    pub(crate) fn pane_studies(&self) -> impl Iterator<Item = &Study> {
        self.studies
            .iter()
            .filter(|study| study.placement == StudyPlacement::Pane)
    }

    /// Shares of the height: the price pane's, then each indicator pane's.
    pub(crate) fn pane_weights(&self) -> Vec<f64> {
        std::iter::once(self.price_weight)
            .chain(self.pane_studies().map(|study| {
                self.pane_weights
                    .get(&study.id)
                    .copied()
                    .unwrap_or(INDICATOR_PANE_WEIGHT)
            }))
            .collect()
    }

    /// Moves height between indicator pane `index` and the pane above it.
    fn move_divider(&mut self, index: usize, pixels: f64) {
        let ids: Vec<u64> = self.pane_studies().map(|study| study.id).collect();
        let mut weights = self.pane_weights();
        let (above, below) = (index, index + 1);
        if below >= weights.len() {
            return;
        }
        let per_weight = self.panes_height.get() / weights.iter().sum::<f64>();
        let lowest = MIN_PANE_HEIGHT - weights[above] * per_weight;
        let highest = weights[below] * per_weight - MIN_PANE_HEIGHT;
        if !per_weight.is_finite() || lowest > highest {
            return;
        }
        let moved = pixels.clamp(lowest, highest) / per_weight;
        weights[above] += moved;
        weights[below] -= moved;

        self.price_weight = weights[0];
        for (id, weight) in ids.into_iter().zip(&weights[1..]) {
            self.pane_weights.insert(id, *weight);
        }
    }

    /// Whether the view is close enough to the oldest candle that more should be loaded.
    pub fn wants_older(&self) -> bool {
        !self.history_complete
            && !self.loading_older
            && !self.raw.is_empty()
            && self.first_visible() < LOAD_AHEAD
    }

    pub fn update(&mut self, message: ChartMessage) {
        match message {
            ChartMessage::Pan { bars, price } => {
                self.right -= bars;
                if price != 0.0 {
                    let (low, high) = self.current_range();
                    let shift = (high - low) * price;
                    self.price_range = Some((low + shift, high + shift));
                }
            }
            ChartMessage::Zoom { factor, anchor } => {
                let anchored = self.right - anchor / self.spacing;
                self.spacing = (self.spacing * factor).clamp(MIN_SPACING, MAX_SPACING);
                self.right = anchored + anchor / self.spacing;
            }
            ChartMessage::ScalePrice { factor } => {
                let (low, high) = self.current_range();
                let middle = (low + high) / 2.0;
                let half = (high - low) / 2.0 / factor;
                self.price_range = Some((middle - half, middle + half));
            }
            ChartMessage::ResetPrice => self.price_range = None,
            ChartMessage::ResetView => {
                self.price_range = None;
                self.spacing = DEFAULT_SPACING;
                self.right = self.raw.len() as f64 - 1.0 + RIGHT_MARGIN;
            }
            ChartMessage::MoveDivider { index, pixels } => self.move_divider(index, pixels),
            // Drawings are drawn on every frame, so changing them redraws nothing else.
            ChartMessage::PlacePoint(anchor) => return self.place_point(anchor),
            ChartMessage::PreviewPoint(anchor) => {
                if let Some(draft) = &mut self.draft {
                    draft.next = Some(anchor);
                }
                return;
            }
            ChartMessage::SelectDrawing(id) => return self.selected = id,
            ChartMessage::MoveDrawing { id, points } => return self.move_drawing(id, points),
            ChartMessage::FinishMove => {
                if std::mem::take(&mut self.moving) {
                    self.unsaved = true;
                }
                return;
            }
            ChartMessage::Cancel => return self.cancel(),
        }
        self.clamp();
        self.cache.clear();
    }

    /// Index of the bar at the plot's left edge (may be negative or fractional).
    pub(crate) fn first_visible(&self) -> f64 {
        self.right - self.plot_width.get() / self.spacing
    }

    fn current_range(&self) -> (f64, f64) {
        self.price_range
            .or(self.shown_range.get())
            .unwrap_or((0.0, 1.0))
    }

    fn clamp(&mut self) {
        let len = self.raw.len() as f64;
        if len == 0.0 {
            return;
        }
        let visible = (self.plot_width.get() / self.spacing).max(KEEP_VISIBLE * 2.0);
        let lowest = KEEP_VISIBLE - 1.0;
        let highest = len - 1.0 + visible - KEEP_VISIBLE;
        self.right = self.right.clamp(lowest, highest.max(lowest));
    }

    fn refresh(&mut self) {
        self.candles = match self.kind {
            ChartKind::HeikinAshi => heikin_ashi(&self.raw),
            _ => self.raw.clone(),
        };
        self.cache.clear();
    }
}

/// Heikin Ashi candles: smoothed bodies that make trends easier to see.
fn heikin_ashi(candles: &[Candle]) -> Vec<Candle> {
    let mut smoothed: Vec<Candle> = Vec::with_capacity(candles.len());
    for candle in candles {
        let close = (candle.open + candle.high + candle.low + candle.close) / 4.0;
        let open = match smoothed.last() {
            Some(previous) => (previous.open + previous.close) / 2.0,
            None => (candle.open + candle.close) / 2.0,
        };
        smoothed.push(Candle {
            open,
            close,
            high: candle.high.max(open).max(close),
            low: candle.low.min(open).min(close),
            ..*candle
        });
    }
    smoothed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels() -> ChartLabels {
        ChartLabels {
            open: "O".into(),
            high: "H".into(),
            low: "L".into(),
            close: "C".into(),
            volume: "Vol".into(),
            months: std::array::from_fn(|i| format!("M{}", i + 1)),
            magnitudes: ["K", "M", "B"].map(String::from),
            day_suffix: "d".into(),
            hour_suffix: "h".into(),
            minute_suffix: "m".into(),
            loading: "…".into(),
            note: "Note".into(),
            bars: "bars".into(),
        }
    }

    fn candles(hours: std::ops::Range<i64>) -> Vec<Candle> {
        hours
            .map(|hour| Candle {
                open_time: hour * 3_600_000,
                open: 10.0,
                high: 12.0,
                low: 9.0,
                close: 11.0,
                volume: 1.0,
                quote_volume: Some(11.0),
                trades: Some(1),
                taker_buy_volume: Some(0.5),
            })
            .collect()
    }

    /// A chart with an 800-pixel plot (100 bars at the default zoom).
    fn chart() -> Chart {
        Chart::new(
            "TEST".into(),
            "1h".parse().unwrap(),
            2,
            ChartKind::Candles,
            PriceScale::Linear,
            labels(),
        )
    }

    #[test]
    fn opens_on_the_newest_candles() {
        let mut chart = chart();
        chart.set_candles(candles(0..500));
        assert_eq!(chart.right, 499.0 + RIGHT_MARGIN);
        assert!(!chart.wants_older());
    }

    #[test]
    fn older_candles_keep_the_view_in_place() {
        let mut chart = chart();
        chart.set_candles(candles(1000..1500));
        chart.update(ChartMessage::Pan {
            bars: 100.0,
            price: 0.0,
        });
        let before = chart.right;

        chart.set_candles(candles(0..1500));
        assert_eq!(chart.right, before + 1000.0);
    }

    #[test]
    fn new_candles_are_followed_only_when_in_view() {
        let mut chart = chart();
        chart.set_candles(candles(0..500));
        chart.set_candles(candles(0..501));
        assert_eq!(chart.right, 500.0 + RIGHT_MARGIN);

        chart.update(ChartMessage::Pan {
            bars: 200.0,
            price: 0.0,
        });
        let parked = chart.right;
        chart.set_candles(candles(0..502));
        assert_eq!(chart.right, parked);
    }

    #[test]
    fn zoom_keeps_the_anchor_bar_in_place() {
        let mut chart = chart();
        chart.set_candles(candles(0..500));
        let anchor = 240.0;
        let under_cursor = chart.right - anchor / chart.spacing;

        chart.update(ChartMessage::Zoom {
            factor: 2.0,
            anchor,
        });
        assert_eq!(chart.spacing, 16.0);
        assert!((chart.right - anchor / chart.spacing - under_cursor).abs() < 1e-9);

        chart.update(ChartMessage::Zoom {
            factor: 1000.0,
            anchor,
        });
        assert_eq!(chart.spacing, MAX_SPACING);
    }

    #[test]
    fn panning_stops_at_both_ends() {
        let mut chart = chart();
        chart.set_candles(candles(0..500));
        chart.update(ChartMessage::Pan {
            bars: 10_000.0,
            price: 0.0,
        });
        assert_eq!(chart.right, KEEP_VISIBLE - 1.0);
        assert!(chart.wants_older());

        chart.update(ChartMessage::Pan {
            bars: -10_000.0,
            price: 0.0,
        });
        assert_eq!(chart.right, 499.0 + 100.0 - KEEP_VISIBLE);
    }

    #[test]
    fn older_candles_are_wanted_near_the_left_edge_until_complete() {
        let mut chart = chart();
        chart.set_candles(candles(0..300));
        chart.update(ChartMessage::Pan {
            bars: 100.0,
            price: 0.0,
        });
        assert!(chart.wants_older()); // the view starts around bar 104

        chart.set_loading_older(true);
        assert!(!chart.wants_older());
        chart.set_loading_older(false);
        chart.set_history_complete(true);
        assert!(!chart.wants_older());
    }

    #[test]
    fn manual_price_range_scales_and_resets() {
        let mut chart = chart();
        chart.set_candles(candles(0..10));
        chart.shown_range.set(Some((100.0, 200.0)));

        chart.update(ChartMessage::ScalePrice { factor: 2.0 });
        assert_eq!(chart.price_range, Some((125.0, 175.0)));
        chart.update(ChartMessage::Pan {
            bars: 0.0,
            price: 0.5,
        });
        assert_eq!(chart.price_range, Some((150.0, 200.0)));
        chart.update(ChartMessage::ResetPrice);
        assert_eq!(chart.price_range, None);
    }

    fn pane_study(id: u64) -> Study {
        Study {
            id,
            label: format!("P{id}"),
            placement: StudyPlacement::Pane,
            plots: Vec::new(),
            fills: Vec::new(),
            levels: Vec::new(),
            range: None,
            format: crate::ValueFormat::Decimals(2),
        }
    }

    fn near(actual: &[f64], expected: &[f64]) -> bool {
        actual.len() == expected.len()
            && actual
                .iter()
                .zip(expected)
                .all(|(a, e)| (a - e).abs() < 1e-9)
    }

    #[test]
    fn indicator_panes_share_the_height_and_keep_it() {
        let mut chart = chart();
        chart.set_studies(vec![pane_study(1), pane_study(2)]);
        assert_eq!(chart.pane_weights(), [3.0, 1.0, 1.0]);

        // 500 pixels for a total weight of 5: 100 pixels per weight.
        chart.panes_height.set(500.0);
        chart.update(ChartMessage::MoveDivider {
            index: 1,
            pixels: -40.0,
        });
        assert!(near(&chart.pane_weights(), &[3.0, 0.6, 1.4]));

        // The first indicator pane is squeezed no lower than its minimum height.
        chart.update(ChartMessage::MoveDivider {
            index: 0,
            pixels: 1000.0,
        });
        assert!(near(&chart.pane_weights(), &[3.3, 0.3, 1.4]));
        chart.update(ChartMessage::MoveDivider {
            index: 5,
            pixels: 10.0,
        });
        assert!(near(&chart.pane_weights(), &[3.3, 0.3, 1.4]));

        // A pane shown again keeps its height; a new one gets the default.
        chart.set_studies(vec![pane_study(2), pane_study(3)]);
        assert!(near(&chart.pane_weights(), &[3.3, 1.4, 1.0]));
    }

    #[test]
    fn another_series_keeps_the_zoom_and_the_panes() {
        let mut chart = chart();
        chart.set_candles(candles(0..500));
        chart.set_studies(vec![pane_study(1)]);
        chart.panes_height.set(400.0);
        chart.update(ChartMessage::MoveDivider {
            index: 0,
            pixels: -50.0,
        });
        chart.update(ChartMessage::Zoom {
            factor: 2.0,
            anchor: 0.0,
        });

        chart.replace_series("OTHER".into(), "4h".parse().unwrap(), 4);
        assert!(chart.candles.is_empty() && chart.studies.is_empty());
        assert_eq!(chart.spacing, 16.0);
        chart.set_candles(candles(0..100));
        assert_eq!(chart.right, 99.0 + RIGHT_MARGIN);
        chart.set_studies(vec![pane_study(1)]);
        assert!(near(&chart.pane_weights(), &[2.5, 1.5]));
    }

    #[test]
    fn heikin_ashi_smooths_candles() {
        let raw = candles(0..3);
        let smoothed = heikin_ashi(&raw);
        assert_eq!(smoothed[0].open, 10.5);
        assert_eq!(smoothed[0].close, 10.5);
        assert_eq!(smoothed[1].open, 10.5);
        assert_eq!(smoothed[1].high, 12.0);
        assert_eq!(smoothed[1].open_time, raw[1].open_time);
    }
}
