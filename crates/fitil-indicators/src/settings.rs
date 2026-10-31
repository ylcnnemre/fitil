//! An indicator's settings: parameter values and how each output is drawn. Settings are
//! saved in templates, so they serialize to JSON and are checked against the catalog
//! when read back.

use std::collections::BTreeMap;

use fitil_core::Candle;
use serde::{Deserialize, Serialize};

use crate::catalog::{Definition, ParamKind, definition};

/// Which price of a candle an indicator reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Open,
    High,
    Low,
    Close,
    /// (high + low) / 2
    Hl2,
    /// (high + low + close) / 3
    Hlc3,
    /// (open + high + low + close) / 4
    Ohlc4,
}

impl Source {
    pub const ALL: [Source; 7] = [
        Source::Open,
        Source::High,
        Source::Low,
        Source::Close,
        Source::Hl2,
        Source::Hlc3,
        Source::Ohlc4,
    ];

    pub fn of(self, candle: &Candle) -> f64 {
        match self {
            Source::Open => candle.open,
            Source::High => candle.high,
            Source::Low => candle.low,
            Source::Close => candle.close,
            Source::Hl2 => (candle.high + candle.low) / 2.0,
            Source::Hlc3 => (candle.high + candle.low + candle.close) / 3.0,
            Source::Ohlc4 => (candle.open + candle.high + candle.low + candle.close) / 4.0,
        }
    }

    /// Stable identifier, used in saved settings and translation keys.
    pub fn id(self) -> &'static str {
        match self {
            Source::Open => "open",
            Source::High => "high",
            Source::Low => "low",
            Source::Close => "close",
            Source::Hl2 => "hl2",
            Source::Hlc3 => "hlc3",
            Source::Ohlc4 => "ohlc4",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|source| source.id() == id)
    }
}

/// A parameter value: a number, or text for sources and choices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Number(f64),
    Text(String),
}

/// Parameter values by parameter id.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Params(BTreeMap<String, Value>);

impl Params {
    pub fn get(&self, id: &str) -> Option<&Value> {
        self.0.get(id)
    }

    pub fn set(&mut self, id: &str, value: Value) {
        self.0.insert(id.to_owned(), value);
    }

    pub fn number(&self, id: &str) -> f64 {
        match self.0.get(id) {
            Some(Value::Number(number)) => *number,
            _ => f64::NAN,
        }
    }

    /// A whole-number parameter such as a length; at least 1.
    pub fn length(&self, id: &str) -> usize {
        let number = self.number(id);
        if number.is_finite() {
            number.round().max(1.0) as usize
        } else {
            1
        }
    }

    pub fn source(&self, id: &str) -> Source {
        match self.0.get(id) {
            Some(Value::Text(text)) => Source::from_id(text).unwrap_or(Source::Close),
            _ => Source::Close,
        }
    }

    pub fn text(&self, id: &str) -> &str {
        match self.0.get(id) {
            Some(Value::Text(text)) => text,
            _ => "",
        }
    }
}

/// A color as red, green and blue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// A color from `0xRRGGBB`.
    pub const fn hex(value: u32) -> Self {
        Self((value >> 16) as u8, (value >> 8) as u8, value as u8)
    }
}

/// Colors offered when choosing an output's color, and given in turn to new lines.
pub const PALETTE: [Rgb; 12] = [
    Rgb::hex(0x2962FF),
    Rgb::hex(0xFF6D00),
    Rgb::hex(0x00BCD4),
    Rgb::hex(0xE91E63),
    Rgb::hex(0x7E57C2),
    Rgb::hex(0xFFEB3B),
    Rgb::hex(0x4CAF50),
    Rgb::hex(0xF23645),
    Rgb::hex(0x089981),
    Rgb::hex(0x9E9E9E),
    Rgb::hex(0xFFFFFF),
    Rgb::hex(0x795548),
];

/// How one output of an indicator is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputStyle {
    pub color: Rgb,
    /// Line width in pixels, 1–4.
    pub width: u8,
    pub visible: bool,
}

/// Everything about one indicator on a chart, as saved in templates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// The indicator's id in the catalog, such as `rsi`.
    pub indicator: String,
    pub params: Params,
    /// One per output of the indicator.
    pub outputs: Vec<OutputStyle>,
    pub visible: bool,
}

impl Settings {
    pub fn defaults(definition: &Definition) -> Self {
        let mut params = Params::default();
        for param in definition.params {
            params.set(param.id, param.kind.default_value());
        }
        Self {
            indicator: definition.id.to_owned(),
            params,
            outputs: definition
                .outputs
                .iter()
                .map(|output| OutputStyle {
                    color: output.color,
                    width: 1,
                    visible: true,
                })
                .collect(),
            visible: true,
        }
    }

    /// The catalog entry of this indicator, if it exists.
    pub fn definition(&self) -> Option<&'static Definition> {
        definition(&self.indicator)
    }

    /// Settings made fit for the current catalog: unknown or invalid parameters get their
    /// defaults, numbers are kept in range, and there is one style per output. `None` for
    /// an indicator that does not exist (any more).
    pub fn normalized(mut self) -> Option<Self> {
        let definition = self.definition()?;
        let defaults = Settings::defaults(definition);
        let mut params = Params::default();
        for param in definition.params {
            let value = self
                .params
                .get(param.id)
                .and_then(|value| param.kind.accept(value))
                .unwrap_or_else(|| param.kind.default_value());
            params.set(param.id, value);
        }
        self.params = params;
        self.outputs.truncate(definition.outputs.len());
        self.outputs
            .extend_from_slice(&defaults.outputs[self.outputs.len()..]);
        for output in &mut self.outputs {
            output.width = output.width.clamp(1, 4);
        }
        Some(self)
    }
}

impl ParamKind {
    pub fn default_value(&self) -> Value {
        match self {
            ParamKind::Integer { default, .. } => Value::Number(*default as f64),
            ParamKind::Float { default, .. } => Value::Number(*default),
            ParamKind::Source { default } => Value::Text(default.id().to_owned()),
            ParamKind::Choice { default, .. } => Value::Text((*default).to_owned()),
        }
    }

    /// `value` made valid for this kind of parameter, or `None` if it cannot be.
    pub fn accept(&self, value: &Value) -> Option<Value> {
        match (self, value) {
            (ParamKind::Integer { min, max, .. }, Value::Number(number)) if number.is_finite() => {
                Some(Value::Number(
                    number.round().clamp(*min as f64, *max as f64),
                ))
            }
            (ParamKind::Float { min, max, .. }, Value::Number(number)) if number.is_finite() => {
                Some(Value::Number(number.clamp(*min, *max)))
            }
            (ParamKind::Source { .. }, Value::Text(text)) => {
                Source::from_id(text).map(|source| Value::Text(source.id().to_owned()))
            }
            (ParamKind::Choice { options, .. }, Value::Text(text)) => {
                options.contains(&text.as_str()).then(|| value.clone())
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_cover_every_parameter_and_output() {
        for definition in crate::catalog() {
            let settings = Settings::defaults(definition);
            assert_eq!(
                settings.outputs.len(),
                definition.outputs.len(),
                "{}",
                definition.id
            );
            for param in definition.params {
                assert!(
                    settings.params.get(param.id).is_some(),
                    "{} {}",
                    definition.id,
                    param.id
                );
            }
            assert_eq!(settings.clone().normalized(), Some(settings));
        }
    }

    #[test]
    fn settings_survive_a_round_trip_through_json() {
        let mut settings = Settings::defaults(definition("bollinger").unwrap());
        settings.params.set("length", Value::Number(50.0));
        settings.outputs[1].color = Rgb::hex(0x123456);
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains(r#""length":50.0"#), "{json}");
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back, settings);
    }

    #[test]
    fn bad_saved_settings_are_repaired() {
        let json = r#"{
            "indicator": "rsi",
            "params": { "length": 99999, "source": "bogus", "unknown": 3 },
            "outputs": [],
            "visible": false
        }"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        let fixed = settings.normalized().unwrap();
        assert_eq!(fixed.params.length("length"), 5000);
        assert_eq!(fixed.params.source("source"), Source::Close);
        assert!(fixed.params.get("unknown").is_none());
        assert_eq!(fixed.outputs.len(), 1);
        assert!(!fixed.visible);

        let gone: Settings =
            serde_json::from_str(r#"{"indicator":"gone","params":{},"outputs":[],"visible":true}"#)
                .unwrap();
        assert_eq!(gone.normalized(), None);
    }

    #[test]
    fn sources_read_the_right_price() {
        let candle = Candle {
            open_time: 0,
            open: 1.0,
            high: 4.0,
            low: 0.0,
            close: 3.0,
            volume: 0.0,
            quote_volume: None,
            trades: None,
            taker_buy_volume: None,
        };
        assert_eq!(Source::Hl2.of(&candle), 2.0);
        assert!((Source::Hlc3.of(&candle) - 7.0 / 3.0).abs() < 1e-12);
        assert_eq!(Source::Ohlc4.of(&candle), 2.0);
        assert_eq!(Source::from_id("hlc3"), Some(Source::Hlc3));
    }
}
