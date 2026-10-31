//! The indicators on a chart ("studies"): which ones, with what settings, and their
//! values kept up to date with the candles.

use fitil_core::Candle;
use fitil_exchange::Features;
use fitil_indicators::{Calculation, Definition, PALETTE, Placement, Rgb, Settings};

/// Identifies a study for as long as its chart lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StudyId(u64);

impl StudyId {
    /// A number that stays the same for this study, such as for the chart to remember
    /// the height of its pane.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// One indicator on a chart.
pub struct Study {
    id: StudyId,
    settings: Settings,
    calculation: Calculation,
}

impl Study {
    pub fn id(&self) -> StudyId {
        self.id
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn definition(&self) -> &'static Definition {
        self.calculation.definition()
    }

    /// One series per output, aligned with the candles last given to
    /// [`Studies::update`] while the study was visible.
    pub fn values(&self) -> &[Vec<f64>] {
        self.calculation.values()
    }
}

/// The studies of one chart, in the order they were added.
#[derive(Default)]
pub struct Studies {
    list: Vec<Study>,
    next_id: u64,
}

impl Studies {
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &Study> {
        self.list.iter()
    }

    pub fn get(&self, id: StudyId) -> Option<&Study> {
        self.list.iter().find(|study| study.id == id)
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Adds an indicator with its default settings. Its values are computed at the next
    /// [`Studies::update`].
    ///
    /// A line drawn over the price, such as a moving average, gets a color no other such
    /// line uses yet, so that several averages can be told apart.
    pub fn add(&mut self, definition: &'static Definition) -> StudyId {
        let mut settings = Settings::defaults(definition);
        if let Some(color) = self.free_color(definition) {
            settings.outputs[0].color = color;
        }
        self.insert(definition, settings)
    }

    pub fn remove(&mut self, id: StudyId) -> bool {
        let before = self.list.len();
        self.list.retain(|study| study.id != id);
        self.list.len() != before
    }

    /// Changes a study's settings. Its values are computed again only if its parameters
    /// changed. Returns false for an unknown study or settings of another indicator.
    pub fn set_settings(&mut self, id: StudyId, settings: Settings) -> bool {
        let Some(study) = self.list.iter_mut().find(|study| study.id == id) else {
            return false;
        };
        if settings.indicator != study.settings.indicator {
            return false;
        }
        let Some(settings) = settings.normalized() else {
            return false;
        };
        if settings.params != study.settings.params {
            study.calculation = Calculation::new(study.definition(), settings.params.clone());
        }
        study.settings = settings;
        true
    }

    /// Shows or hides a study.
    pub fn set_visible(&mut self, id: StudyId, visible: bool) -> bool {
        match self.list.iter_mut().find(|study| study.id == id) {
            Some(study) => {
                study.settings.visible = visible;
                true
            }
            None => false,
        }
    }

    /// Brings the values of every visible study up to date with `candles`. Hidden studies
    /// are not computed; they catch up when shown again.
    pub fn update(&mut self, candles: &[Candle]) {
        for study in &mut self.list {
            if study.settings.visible {
                study.calculation.update(candles);
            }
        }
    }

    /// Makes every study start over at the next update, for candles of another series.
    pub fn reset(&mut self) {
        for study in &mut self.list {
            study.calculation.reset();
        }
    }

    /// The settings of every study, as a template saves them.
    pub fn settings(&self) -> Vec<Settings> {
        self.list
            .iter()
            .map(|study| study.settings.clone())
            .collect()
    }

    /// Replaces the studies with ones made from `settings` (such as a template's), leaving
    /// out indicators that no longer exist or that the exchange cannot feed.
    pub fn replace(&mut self, settings: &[Settings], features: Features) {
        self.list.clear();
        for settings in settings {
            if let Some(settings) = settings.clone().normalized()
                && let Some(definition) = settings.definition()
                && is_available(definition, features)
            {
                self.insert(definition, settings);
            }
        }
    }

    fn insert(&mut self, definition: &'static Definition, settings: Settings) -> StudyId {
        let id = StudyId(self.next_id);
        self.next_id += 1;
        self.list.push(Study {
            id,
            calculation: Calculation::new(definition, settings.params.clone()),
            settings,
        });
        id
    }

    /// For a single line over the price whose own color another such line already uses:
    /// the first palette color none of them uses.
    fn free_color(&self, definition: &Definition) -> Option<Rgb> {
        if !is_single_overlay(definition) {
            return None;
        }
        let used: Vec<Rgb> = self
            .list
            .iter()
            .filter(|study| is_single_overlay(study.definition()))
            .map(|study| study.settings.outputs[0].color)
            .collect();
        if !used.contains(&definition.outputs[0].color) {
            return None;
        }
        PALETTE.into_iter().find(|color| !used.contains(color))
    }
}

fn is_single_overlay(definition: &Definition) -> bool {
    definition.placement == Placement::Overlay && definition.outputs.len() == 1
}

/// Whether the exchange provides the data `definition` needs; indicators it cannot feed
/// are not offered.
pub fn is_available(definition: &Definition, features: Features) -> bool {
    !definition.needs_taker_volume || features.taker_volume
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::candle;
    use fitil_indicators::{Value, definition};

    const ALL: Features = Features {
        taker_volume: true,
        full_history: true,
    };

    fn candles(count: i64) -> Vec<Candle> {
        (0..count).map(candle).collect()
    }

    #[test]
    fn repeated_averages_get_their_own_colors() {
        let mut studies = Studies::default();
        let sma = definition("sma").unwrap();
        let first = studies.add(sma);
        let second = studies.add(sma);
        let rsi = studies.add(definition("rsi").unwrap());

        let color = |id| studies.get(id).unwrap().settings().outputs[0].color;
        assert_eq!(color(first), sma.outputs[0].color);
        assert_ne!(color(second), color(first));
        // Indicators in their own pane keep their colors.
        assert_eq!(color(rsi), definition("rsi").unwrap().outputs[0].color);
    }

    #[test]
    fn values_follow_the_candles_and_the_settings() {
        let mut studies = Studies::default();
        let id = studies.add(definition("sma").unwrap());
        studies.update(&candles(30));
        let values = studies.get(id).unwrap().values()[0].clone();
        assert_eq!(values.len(), 30);
        assert!(values[7].is_nan() && values[8].is_finite()); // a 9-candle average

        let mut settings = studies.get(id).unwrap().settings().clone();
        settings.params.set("length", Value::Number(3.0));
        assert!(studies.set_settings(id, settings));
        studies.update(&candles(30));
        assert!(studies.get(id).unwrap().values()[0][2].is_finite());

        // Settings of another indicator are refused.
        let other = Settings::defaults(definition("ema").unwrap());
        assert!(!studies.set_settings(id, other));
    }

    #[test]
    fn hidden_studies_catch_up_when_shown() {
        let mut studies = Studies::default();
        let id = studies.add(definition("ema").unwrap());
        studies.update(&candles(20));
        assert!(studies.set_visible(id, false));
        studies.update(&candles(25));
        assert_eq!(studies.get(id).unwrap().values()[0].len(), 20);

        studies.set_visible(id, true);
        studies.update(&candles(25));
        assert_eq!(studies.get(id).unwrap().values()[0].len(), 25);
    }

    #[test]
    fn replacing_leaves_out_what_the_exchange_cannot_feed() {
        let cvd = Settings::defaults(definition("cvd").unwrap());
        let rsi = Settings::defaults(definition("rsi").unwrap());
        let mut gone = rsi.clone();
        gone.indicator = "gone".into();
        let template = [cvd, gone, rsi];

        let mut studies = Studies::default();
        studies.add(definition("sma").unwrap());
        studies.replace(&template, ALL);
        let ids: Vec<&str> = studies.iter().map(|s| s.definition().id).collect();
        assert_eq!(ids, ["cvd", "rsi"]);

        let no_taker_volume = Features {
            taker_volume: false,
            ..ALL
        };
        studies.replace(&template, no_taker_volume);
        let ids: Vec<&str> = studies.iter().map(|s| s.definition().id).collect();
        assert_eq!(ids, ["rsi"]);
        assert_eq!(studies.settings().len(), 1);
    }

    #[test]
    fn removing_a_study() {
        let mut studies = Studies::default();
        let id = studies.add(definition("atr").unwrap());
        assert!(studies.remove(id));
        assert!(!studies.remove(id));
        assert!(studies.is_empty());
    }
}
