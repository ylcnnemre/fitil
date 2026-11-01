//! Editing drawings: placing them point by point, picking, moving and changing them,
//! with undo and redo. Where drawings go on screen is in `shapes`.

use fitil_core::{Anchor, Drawing, DrawingKind};

use crate::Chart;

/// Changes kept for undo.
const HISTORY: usize = 100;
/// Bars a copy is moved to the right of its drawing, so that both can be seen.
const COPY_OFFSET: f64 = 5.0;
/// The length of a month, for timeframes counted in months.
const MONTH_MS: f64 = 30.44 * 86_400_000.0;

/// A drawing being placed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Draft {
    pub kind: DrawingKind,
    /// The points placed so far.
    pub points: Vec<Anchor>,
    /// Where the next point would go: under the cursor.
    pub next: Option<Anchor>,
}

impl Draft {
    /// The drawing as it would be with its next point placed, to show while placing.
    pub fn preview(&self) -> Drawing {
        let mut points = self.points.clone();
        points.extend(self.next);
        Drawing::new(0, self.kind, points)
    }
}

impl Chart {
    /// Shows a symbol's drawings, forgetting the selection and the changes to undo.
    pub fn set_drawings(&mut self, drawings: Vec<Drawing>) {
        self.drawings = drawings;
        self.selected = None;
        self.draft = None;
        self.undo.clear();
        self.redo.clear();
        self.moving = false;
        self.unsaved = false;
    }

    pub fn drawings(&self) -> &[Drawing] {
        &self.drawings
    }

    /// The drawings, if they changed since this was last asked: for saving them.
    pub fn take_changed_drawings(&mut self) -> Option<Vec<Drawing>> {
        std::mem::take(&mut self.unsaved).then(|| self.drawings.clone())
    }

    pub fn tool(&self) -> Option<DrawingKind> {
        self.tool
    }

    /// Picks the drawing tool for the next drawing, or none to pick and move drawings.
    pub fn set_tool(&mut self, tool: Option<DrawingKind>) {
        self.tool = tool;
        self.draft = None;
        if tool.is_some() {
            self.selected = None;
        }
    }

    pub fn magnet(&self) -> bool {
        self.magnet
    }

    pub fn set_magnet(&mut self, on: bool) {
        self.magnet = on;
    }

    pub fn drawings_hidden(&self) -> bool {
        self.drawings_hidden
    }

    pub fn set_drawings_hidden(&mut self, hidden: bool) {
        self.drawings_hidden = hidden;
        if hidden {
            self.selected = None;
            self.draft = None;
            self.tool = None;
        }
    }

    pub fn selected_drawing(&self) -> Option<&Drawing> {
        let id = self.selected?;
        self.drawings.iter().find(|drawing| drawing.id == id)
    }

    /// Changes the selected drawing: its look, its words or its levels.
    pub fn edit_selected(&mut self, change: impl FnOnce(&mut Drawing)) {
        let Some(index) = self.selected_index() else {
            return;
        };
        let mut edited = self.drawings[index].clone();
        change(&mut edited);
        if edited != self.drawings[index] {
            self.record();
            self.drawings[index] = edited;
            self.unsaved = true;
        }
    }

    pub fn delete_selected(&mut self) {
        if let Some(index) = self.selected_index() {
            self.record();
            self.drawings.remove(index);
            self.selected = None;
            self.unsaved = true;
        }
    }

    /// Adds a copy of the selected drawing a few bars to its right, and selects it.
    pub fn duplicate_selected(&mut self) {
        let Some(index) = self.selected_index() else {
            return;
        };
        let mut copy = self.drawings[index].clone();
        copy.id = self.next_drawing_id();
        copy.locked = false;
        for point in &mut copy.points {
            point.time = self.time_of(self.bar_of(point.time) + COPY_OFFSET);
        }
        self.record();
        self.selected = Some(copy.id);
        self.drawings.push(copy);
        self.unsaved = true;
    }

    pub fn clear_drawings(&mut self) {
        if !self.drawings.is_empty() {
            self.record();
            self.drawings.clear();
            self.selected = None;
            self.unsaved = true;
        }
    }

    /// Whether there are drawings and all of them are locked.
    pub fn all_locked(&self) -> bool {
        !self.drawings.is_empty() && self.drawings.iter().all(|drawing| drawing.locked)
    }

    /// Locks or unlocks every drawing.
    pub fn lock_all(&mut self, locked: bool) {
        if self.drawings.iter().any(|drawing| drawing.locked != locked) {
            self.record();
            for drawing in &mut self.drawings {
                drawing.locked = locked;
            }
            self.unsaved = true;
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) {
        if let Some(before) = self.undo.pop() {
            self.redo
                .push(std::mem::replace(&mut self.drawings, before));
            self.after_history_step();
        }
    }

    pub fn redo(&mut self) {
        if let Some(after) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.drawings, after));
            self.after_history_step();
        }
    }

    fn after_history_step(&mut self) {
        let selected = self.selected;
        self.selected = selected.filter(|id| self.drawings.iter().any(|d| d.id == *id));
        self.draft = None;
        self.moving = false;
        self.unsaved = true;
    }

    /// Adds a point to the drawing being placed, and finishes it once it has them all.
    pub(crate) fn place_point(&mut self, anchor: Anchor) {
        let Some(kind) = self.tool else {
            return;
        };
        let draft = self.draft.get_or_insert_with(|| Draft {
            kind,
            points: Vec::new(),
            next: None,
        });
        draft.points.push(anchor);
        draft.next = Some(anchor);
        if draft.points.len() < kind.points() {
            return;
        }
        let points = std::mem::take(&mut draft.points);
        self.draft = None;
        let id = self.next_drawing_id();
        let mut drawing = Drawing::new(id, kind, points);
        if kind == DrawingKind::Text {
            drawing.text = self.labels.note.clone();
        }
        self.record();
        self.drawings.push(drawing);
        // As in TradingView: the new drawing is picked and the tool put away.
        self.selected = Some(id);
        self.tool = None;
        self.unsaved = true;
    }

    /// Puts a drawing's points where a drag took them. The first move of a drag remembers
    /// the drawing as it was, so that the whole drag can be undone.
    pub(crate) fn move_drawing(&mut self, id: u64, points: Vec<Anchor>) {
        let Some(index) = self.drawings.iter().position(|drawing| drawing.id == id) else {
            return;
        };
        let drawing = &self.drawings[index];
        if drawing.locked || points.len() != drawing.points.len() {
            return;
        }
        if !self.moving {
            self.record();
            self.moving = true;
        }
        self.drawings[index].points = points;
    }

    pub(crate) fn cancel(&mut self) {
        if self.draft.take().is_some() || self.tool.take().is_some() {
            return;
        }
        self.selected = None;
    }

    /// Remembers the drawings as they are, so that the change about to be made can be
    /// undone.
    fn record(&mut self) {
        self.undo.push(self.drawings.clone());
        if self.undo.len() > HISTORY {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn selected_index(&self) -> Option<usize> {
        let id = self.selected?;
        self.drawings.iter().position(|drawing| drawing.id == id)
    }

    fn next_drawing_id(&self) -> u64 {
        self.drawings
            .iter()
            .map(|drawing| drawing.id)
            .max()
            .unwrap_or(0)
            + 1
    }

    /// The usual length of a bar, for times beyond the candles.
    fn bar_duration(&self) -> f64 {
        self.timeframe
            .duration_ms()
            .map_or(MONTH_MS, |ms| ms as f64)
    }

    /// The bar position of `time`: whole at candles' open times, a fraction of the way
    /// between them, and at the timeframe's pace before the first candle or after the
    /// last. The same times give the same places in every timeframe.
    pub(crate) fn bar_of(&self, time: i64) -> f64 {
        let (Some(first), Some(last)) = (self.candles.first(), self.candles.last()) else {
            return 0.0;
        };
        let duration = self.bar_duration();
        if time <= first.open_time {
            return (time - first.open_time) as f64 / duration;
        }
        if time >= last.open_time {
            let last_index = (self.candles.len() - 1) as f64;
            return last_index + (time - last.open_time) as f64 / duration;
        }
        let next = self
            .candles
            .partition_point(|candle| candle.open_time <= time);
        let (start, end) = (
            self.candles[next - 1].open_time,
            self.candles[next].open_time,
        );
        (next - 1) as f64 + (time - start) as f64 / (end - start) as f64
    }

    /// The time at bar position `bar`: the opposite of [`Chart::bar_of`].
    pub(crate) fn time_of(&self, bar: f64) -> i64 {
        let (Some(first), Some(last)) = (self.candles.first(), self.candles.last()) else {
            return 0;
        };
        let duration = self.bar_duration();
        let last_index = (self.candles.len() - 1) as f64;
        if bar <= 0.0 {
            return first.open_time + (bar * duration).round() as i64;
        }
        if bar >= last_index {
            return last.open_time + ((bar - last_index) * duration).round() as i64;
        }
        let index = bar.floor() as usize;
        let (start, end) = (
            self.candles[index].open_time,
            self.candles[index + 1].open_time,
        );
        start + ((bar - index as f64) * (end - start) as f64).round() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChartKind, ChartLabels, ChartMessage, PriceScale};
    use fitil_core::Candle;

    const HOUR: i64 = 3_600_000;

    fn labels() -> ChartLabels {
        ChartLabels {
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
        }
    }

    /// An hourly chart of candles at `hours`, which may leave gaps.
    fn chart(hours: &[i64]) -> Chart {
        let mut chart = Chart::new(
            "TEST".into(),
            "1h".parse().unwrap(),
            2,
            ChartKind::Candles,
            PriceScale::Linear,
            labels(),
        );
        chart.set_candles(
            hours
                .iter()
                .map(|&hour| Candle {
                    open_time: hour * HOUR,
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
        chart
    }

    fn at(hours: f64, price: f64) -> Anchor {
        Anchor {
            time: (hours * HOUR as f64) as i64,
            price,
        }
    }

    #[test]
    fn times_and_bars_map_both_ways_across_gaps() {
        // Hours 2 and 3 are missing.
        let chart = chart(&[0, 1, 4, 5]);
        assert_eq!(chart.bar_of(HOUR), 1.0);
        assert_eq!(chart.bar_of(4 * HOUR), 2.0);
        // Halfway through the gap is halfway between its candles.
        assert_eq!(chart.bar_of(5 * HOUR / 2), 1.5);
        // Beyond the candles, an hour is a bar.
        assert_eq!(chart.bar_of(8 * HOUR), 6.0);
        assert_eq!(chart.bar_of(-2 * HOUR), -2.0);
        for bar in [-3.0, 0.0, 0.5, 1.5, 2.25, 3.0, 7.5] {
            assert!(
                (chart.bar_of(chart.time_of(bar)) - bar).abs() < 1e-9,
                "{bar}"
            );
        }
    }

    #[test]
    fn drawings_are_placed_point_by_point() {
        let mut chart = chart(&[0, 1, 2, 3]);
        chart.set_tool(Some(DrawingKind::TrendLine));
        chart.update(ChartMessage::PlacePoint(at(0.0, 10.0)));
        chart.update(ChartMessage::PreviewPoint(at(2.0, 11.0)));
        assert_eq!(chart.draft.as_ref().unwrap().preview().points.len(), 2);
        assert!(chart.drawings.is_empty());

        chart.update(ChartMessage::PlacePoint(at(3.0, 12.0)));
        assert_eq!(chart.drawings.len(), 1);
        assert_eq!(chart.drawings[0].points, [at(0.0, 10.0), at(3.0, 12.0)]);
        // The new drawing is picked and the tool put away.
        assert_eq!(chart.selected, Some(chart.drawings[0].id));
        assert_eq!(chart.tool(), None);
        assert!(chart.take_changed_drawings().is_some());
        assert!(chart.take_changed_drawings().is_none());

        // One click is enough for a note, which gets its words.
        chart.set_tool(Some(DrawingKind::Text));
        chart.update(ChartMessage::PlacePoint(at(1.0, 10.5)));
        assert_eq!(chart.selected_drawing().unwrap().text, "Note");
    }

    #[test]
    fn a_drag_is_one_change_and_locked_drawings_stay() {
        let mut chart = chart(&[0, 1, 2, 3]);
        chart.set_tool(Some(DrawingKind::HorizontalLine));
        chart.update(ChartMessage::PlacePoint(at(1.0, 10.0)));
        let id = chart.drawings[0].id;
        chart.take_changed_drawings();

        for price in [10.5, 11.0, 11.5] {
            chart.update(ChartMessage::MoveDrawing {
                id,
                points: vec![at(1.0, price)],
            });
        }
        // Nothing to save until the drag ends.
        assert!(chart.take_changed_drawings().is_none());
        chart.update(ChartMessage::FinishMove);
        assert_eq!(
            chart.take_changed_drawings().unwrap()[0].points[0].price,
            11.5
        );

        // One undo takes back the whole drag, and redo puts it back.
        chart.undo();
        assert_eq!(chart.drawings[0].points[0].price, 10.0);
        chart.redo();
        assert_eq!(chart.drawings[0].points[0].price, 11.5);

        chart.lock_all(true);
        assert!(chart.all_locked());
        chart.update(ChartMessage::MoveDrawing {
            id,
            points: vec![at(1.0, 20.0)],
        });
        assert_eq!(chart.drawings[0].points[0].price, 11.5);
    }

    #[test]
    fn selected_drawings_are_changed_copied_and_deleted() {
        let mut chart = chart(&[0, 1, 2, 3]);
        chart.set_tool(Some(DrawingKind::Rectangle));
        chart.update(ChartMessage::PlacePoint(at(0.0, 9.0)));
        chart.update(ChartMessage::PlacePoint(at(1.0, 12.0)));

        chart.edit_selected(|drawing| drawing.width = 4);
        assert_eq!(chart.selected_drawing().unwrap().width, 4);

        chart.duplicate_selected();
        assert_eq!(chart.drawings.len(), 2);
        let copy = chart.selected_drawing().unwrap();
        assert_eq!(copy.points[0].time, at(5.0, 0.0).time);
        assert_ne!(copy.id, chart.drawings[0].id);

        chart.delete_selected();
        assert_eq!(chart.drawings.len(), 1);
        chart.clear_drawings();
        assert!(chart.drawings.is_empty());
        chart.undo();
        assert_eq!(chart.drawings.len(), 1);
    }

    #[test]
    fn esc_lets_go_of_one_thing_at_a_time() {
        let mut chart = chart(&[0, 1, 2, 3]);
        chart.set_tool(Some(DrawingKind::TrendLine));
        chart.update(ChartMessage::PlacePoint(at(0.0, 10.0)));
        chart.update(ChartMessage::Cancel);
        assert!(chart.draft.is_none() && chart.tool().is_some());
        chart.update(ChartMessage::Cancel);
        assert!(chart.tool().is_none());

        chart.set_drawings(vec![Drawing::new(
            7,
            DrawingKind::HorizontalLine,
            vec![at(0.0, 10.0)],
        )]);
        chart.update(ChartMessage::SelectDrawing(Some(7)));
        chart.update(ChartMessage::Cancel);
        assert!(chart.selected_drawing().is_none());
        // Drawings shown for a new symbol have nothing to save or undo.
        assert!(chart.take_changed_drawings().is_none() && !chart.can_undo());
    }
}
