use std::cell::Cell;

use fitil_core::{Candle, Timeframe};
use iced::widget::canvas;
use iced::{Element, Fill};

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
    /// Shown while older candles load.
    pub loading: String,
}

/// What the user did on the chart, in chart terms.
#[derive(Debug, Clone, Copy, PartialEq)]
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
    /// Plot width and shown price range as of the last draw.
    pub(crate) plot_width: Cell<f64>,
    pub(crate) shown_range: Cell<Option<(f64, f64)>>,
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
            plot_width: Cell::new(800.0),
            shown_range: Cell::new(None),
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
            loading: "…".into(),
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
