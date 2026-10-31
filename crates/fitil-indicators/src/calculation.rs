use fitil_core::Candle;

use crate::Indicator;
use crate::catalog::Definition;
use crate::settings::Params;

/// An indicator's values for a run of candles, kept up to date as the candles change.
///
/// Every candle but the newest is treated as final. When only the newest candle changes
/// (a live update) or a new one is added (the previous one closed), only those candles
/// are computed: the newest on a copy of the indicator's state, so the next update can
/// start from the same point. Anything else (older candles added, a different symbol, a
/// final candle that changed after all) is computed from scratch.
pub struct Calculation {
    definition: &'static Definition,
    params: Params,
    /// The indicator after the last final candle.
    state: Box<dyn Indicator>,
    /// How many candles `state` has taken, the first one's open time and the last one.
    settled: usize,
    first_time: Option<i64>,
    last_settled: Option<Candle>,
    /// One series per output, as long as the candles last given.
    values: Vec<Vec<f64>>,
    scratch: Vec<f64>,
}

impl Calculation {
    pub fn new(definition: &'static Definition, params: Params) -> Self {
        let outputs = definition.outputs.len();
        Self {
            state: definition.build(&params),
            definition,
            params,
            settled: 0,
            first_time: None,
            last_settled: None,
            values: vec![Vec::new(); outputs],
            scratch: vec![f64::NAN; outputs],
        }
    }

    pub fn definition(&self) -> &'static Definition {
        self.definition
    }

    pub fn params(&self) -> &Params {
        &self.params
    }

    /// One series per output, aligned with the candles last given to [`Self::update`].
    pub fn values(&self) -> &[Vec<f64>] {
        &self.values
    }

    /// Starts over at the next update, for when the candles belong to another series.
    pub fn reset(&mut self) {
        self.first_time = None;
    }

    /// Brings the values up to date with `candles`, sorted by open time.
    pub fn update(&mut self, candles: &[Candle]) {
        let Some(newest) = candles.last() else {
            self.start_over();
            return;
        };
        let final_count = candles.len() - 1;
        let continues = self.first_time == Some(candles[0].open_time)
            && final_count >= self.settled
            && (self.settled == 0
                || Some(&candles[self.settled - 1]) == self.last_settled.as_ref());
        if !continues {
            self.start_over();
            self.first_time = Some(candles[0].open_time);
        }

        // Drop the previous newest value, then add the candles that became final.
        for series in &mut self.values {
            series.truncate(self.settled);
        }
        for candle in &candles[self.settled..final_count] {
            self.state.next(candle, &mut self.scratch);
            push(&mut self.values, &self.scratch);
        }
        if final_count > self.settled {
            self.settled = final_count;
            self.last_settled = Some(candles[final_count - 1]);
        }

        let mut probe = self.state.clone();
        probe.next(newest, &mut self.scratch);
        push(&mut self.values, &self.scratch);
    }

    fn start_over(&mut self) {
        self.state = self.definition.build(&self.params);
        self.settled = 0;
        self.first_time = None;
        self.last_settled = None;
        for series in &mut self.values {
            series.clear();
        }
    }
}

fn push(values: &mut [Vec<f64>], row: &[f64]) {
    for (series, value) in values.iter_mut().zip(row) {
        series.push(*value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::testing::random_walk;
    use crate::{Settings, catalog};

    /// Values computed from scratch, for comparison.
    fn fresh(
        definition: &'static Definition,
        params: &Params,
        candles: &[Candle],
    ) -> Vec<Vec<f64>> {
        let mut calculation = Calculation::new(definition, params.clone());
        calculation.update(candles);
        calculation.values().to_vec()
    }

    fn same(a: &[Vec<f64>], b: &[Vec<f64>]) -> bool {
        a.len() == b.len()
            && a.iter().zip(b).all(|(x, y)| {
                x.len() == y.len()
                    && x.iter()
                        .zip(y)
                        .all(|(p, q)| (p.is_nan() && q.is_nan()) || p == q)
            })
    }

    #[test]
    fn live_updates_match_a_full_recomputation_for_every_indicator() {
        let history = random_walk(260, 13);
        for definition in catalog() {
            let params = Settings::defaults(definition).params;
            let mut calculation = Calculation::new(definition, params.clone());
            let mut candles = history[..200].to_vec();
            calculation.update(&candles);

            for (i, closed) in history.iter().enumerate().skip(200) {
                // The newest candle changes a few times before it closes.
                let mut open = *closed;
                for step in 0..3 {
                    open.close = closed.open + (closed.close - closed.open) * step as f64 / 2.0;
                    open.high = open.high.max(open.close);
                    open.low = open.low.min(open.close);
                    candles.truncate(i);
                    candles.push(open);
                    calculation.update(&candles);
                    assert!(
                        same(calculation.values(), &fresh(definition, &params, &candles)),
                        "{} at candle {i}, step {step}",
                        definition.id
                    );
                }
            }
        }
    }

    #[test]
    fn older_candles_and_other_series_start_over() {
        let history = random_walk(300, 14);
        let definition = crate::definition("rsi").unwrap();
        let params = Settings::defaults(definition).params;
        let mut calculation = Calculation::new(definition, params.clone());

        calculation.update(&history[100..]);
        // Older candles in front: everything shifts.
        calculation.update(&history);
        assert!(same(
            calculation.values(),
            &fresh(definition, &params, &history)
        ));

        // Another symbol whose candles happen to start at the same time.
        let other = random_walk(300, 15);
        calculation.reset();
        calculation.update(&other);
        assert!(same(
            calculation.values(),
            &fresh(definition, &params, &other)
        ));

        // A candle taken as final that turns out different, such as a longer candle
        // built before all of its shorter candles had arrived.
        let mut corrected = other.clone();
        corrected[298].close *= 1.01;
        calculation.update(&corrected);
        assert!(same(
            calculation.values(),
            &fresh(definition, &params, &corrected)
        ));

        calculation.update(&[]);
        assert!(calculation.values().iter().all(Vec::is_empty));
    }
}
