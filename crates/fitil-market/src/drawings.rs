//! The drawings of each symbol, kept in the user's data, and the looks saved for each kind
//! of drawing. A symbol's drawings show in every timeframe unless their visibility says
//! otherwise; spot and futures pairs with the same name are different symbols.

use std::collections::{BTreeMap, HashMap};

use fitil_core::{
    Anchor, Drawing, DrawingKind, DrawingStyle, Extend, FIB_LEVELS, LabelAlign, LabelSide, Level,
    LevelLabels, LineStyle, Market, Rgb, TimeUnit, TrendLook, Visibility,
};
use fitil_store::StoredDrawingTemplate;
use serde::{Deserialize, Serialize};

use crate::templates::with_user;
use crate::{MarketError, SharedUserData};

/// Version of the saved form, for later versions of Fitil to read older drawings.
///
/// 1. The color, width, line style and levels as plain numbers.
/// 2. Levels with their own colors, fills, Fibonacci options and visibility.
const FORMAT_VERSION: u32 = 2;

#[derive(Serialize)]
struct SavedForm {
    version: u32,
    drawings: Vec<SavedDrawing>,
}

#[derive(Deserialize)]
struct ReadForm {
    /// Read one by one, so that one unreadable drawing does not lose the others.
    drawings: Vec<serde_json::Value>,
}

/// One drawing as saved.
#[derive(Serialize, Deserialize)]
struct SavedDrawing {
    kind: String,
    /// `[time, price]` pairs.
    points: Vec<(i64, f64)>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    text: String,
    #[serde(default, skip_serializing_if = "is_false")]
    locked: bool,
    #[serde(flatten)]
    style: SavedStyle,
}

fn is_false(value: &bool) -> bool {
    !value
}

/// A look as saved. What is missing, as in drawings saved by older versions, takes
/// Fitil's look for the drawing's kind.
#[derive(Serialize, Deserialize)]
struct SavedStyle {
    color: (u8, u8, u8),
    width: u8,
    line: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fill: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    opacity: Option<u8>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    levels: Vec<SavedLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    one_color: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    trend: Option<SavedTrend>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    extend: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reverse: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    labels: Option<SavedLabels>,
    /// By unit: the counts shown at, or nothing for none. Left out when the drawing shows
    /// on every timeframe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    visibility: Option<BTreeMap<String, Option<(u32, u32)>>>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum SavedLevel {
    /// As version 1 saved levels: the value alone, drawn in the drawing's color.
    Plain(f64),
    Full {
        value: f64,
        color: (u8, u8, u8),
        shown: bool,
    },
}

#[derive(Serialize, Deserialize)]
struct SavedTrend {
    shown: bool,
    color: (u8, u8, u8),
    line: String,
}

#[derive(Serialize, Deserialize)]
struct SavedLabels {
    levels: bool,
    percent: bool,
    prices: bool,
    align: String,
    side: String,
}

fn saved_rgb(Rgb(red, green, blue): Rgb) -> (u8, u8, u8) {
    (red, green, blue)
}

fn rgb((red, green, blue): (u8, u8, u8)) -> Rgb {
    Rgb(red, green, blue)
}

impl SavedStyle {
    /// The look of a drawing of `kind`, with what does not apply to the kind left out.
    fn new(style: &DrawingStyle, kind: DrawingKind) -> Self {
        let levels = kind.has_levels();
        let visibility = (style.visibility != Visibility::ALL).then(|| {
            TimeUnit::ALL
                .into_iter()
                .map(|unit| (unit.id().to_owned(), style.visibility.range(unit)))
                .collect()
        });
        Self {
            color: saved_rgb(style.color),
            width: style.width,
            line: style.line.id().to_owned(),
            fill: kind.has_fill().then_some(style.fill),
            opacity: kind.has_fill().then_some(style.opacity),
            levels: style
                .levels
                .iter()
                .map(|level| SavedLevel::Full {
                    value: level.value,
                    color: saved_rgb(level.color),
                    shown: level.shown,
                })
                .collect(),
            one_color: levels.then_some(style.one_color),
            trend: levels.then(|| SavedTrend {
                shown: style.trend.shown,
                color: saved_rgb(style.trend.color),
                line: style.trend.line.id().to_owned(),
            }),
            extend: levels.then(|| style.extend.id().to_owned()),
            reverse: levels.then_some(style.reverse),
            labels: levels.then(|| SavedLabels {
                levels: style.labels.levels,
                percent: style.labels.percent,
                prices: style.labels.prices,
                align: style.labels.align.id().to_owned(),
                side: style.labels.side.id().to_owned(),
            }),
            visibility,
        }
    }

    /// The look for a drawing of `kind`, with Fitil's look for what was not saved.
    fn style(self, kind: DrawingKind) -> DrawingStyle {
        let fitil = DrawingStyle::of(kind);
        let color = rgb(self.color);
        // Version 1 kept bare values, all drawn in the drawing's color. They are shown,
        // followed by the levels Fitil offers that they lack, hidden.
        let plain = self
            .levels
            .iter()
            .any(|level| matches!(level, SavedLevel::Plain(_)));
        let mut levels: Vec<Level> = self
            .levels
            .into_iter()
            .map(|level| match level {
                SavedLevel::Plain(value) => Level {
                    value,
                    color,
                    shown: true,
                },
                SavedLevel::Full {
                    value,
                    color,
                    shown,
                } => Level {
                    value,
                    color: rgb(color),
                    shown,
                },
            })
            .filter(|level| level.value.is_finite())
            .collect();
        if plain {
            let extra: Vec<Level> = FIB_LEVELS
                .iter()
                .filter(|offered| levels.iter().all(|level| level.value != offered.value))
                .map(|offered| Level {
                    shown: false,
                    ..*offered
                })
                .take(FIB_LEVELS.len().saturating_sub(levels.len()))
                .collect();
            levels.extend(extra);
        }
        if levels.is_empty() {
            levels = fitil.levels;
        }
        let mut visibility = Visibility::ALL;
        for (id, range) in self.visibility.into_iter().flatten() {
            if let Some(unit) = TimeUnit::ALL.into_iter().find(|unit| unit.id() == id) {
                visibility.set_range(unit, range);
            }
        }
        let labels = self.labels.map_or(fitil.labels, |labels| LevelLabels {
            levels: labels.levels,
            percent: labels.percent,
            prices: labels.prices,
            align: LabelAlign::from_id(&labels.align).unwrap_or_default(),
            side: LabelSide::from_id(&labels.side).unwrap_or_default(),
        });
        DrawingStyle {
            color,
            width: self.width.clamp(1, 4),
            line: LineStyle::from_id(&self.line).unwrap_or_default(),
            fill: self.fill.unwrap_or(fitil.fill),
            opacity: self
                .opacity
                .map_or(fitil.opacity, |opacity| opacity.min(100)),
            levels,
            one_color: self.one_color.unwrap_or(plain),
            trend: self.trend.map_or(fitil.trend, |trend| TrendLook {
                shown: trend.shown,
                color: rgb(trend.color),
                line: LineStyle::from_id(&trend.line).unwrap_or_default(),
            }),
            extend: self
                .extend
                .and_then(|id| Extend::from_id(&id))
                .unwrap_or(fitil.extend),
            reverse: self.reverse.unwrap_or(fitil.reverse),
            labels,
            visibility,
        }
    }
}

impl SavedDrawing {
    fn new(drawing: &Drawing) -> Self {
        Self {
            kind: drawing.kind.id().to_owned(),
            points: drawing
                .points
                .iter()
                .map(|point| (point.time, point.price))
                .collect(),
            text: drawing.text.clone(),
            locked: drawing.locked,
            style: SavedStyle::new(&drawing.style, drawing.kind),
        }
    }

    /// The drawing, if its kind is known and its points are whole and finite.
    fn drawing(self, id: u64) -> Option<Drawing> {
        let kind = DrawingKind::from_id(&self.kind)?;
        let points: Vec<Anchor> = self
            .points
            .into_iter()
            .map(|(time, price)| Anchor { time, price })
            .collect();
        if points.len() != kind.points() || points.iter().any(|point| !point.price.is_finite()) {
            return None;
        }
        Some(Drawing {
            id,
            kind,
            points,
            text: self.text,
            locked: self.locked,
            style: self.style.style(kind),
        })
    }
}

fn encode(drawings: &[Drawing]) -> String {
    let form = SavedForm {
        version: FORMAT_VERSION,
        drawings: drawings.iter().map(SavedDrawing::new).collect(),
    };
    serde_json::to_string(&form).expect("drawings always serialize")
}

/// Reads saved drawings, numbering them from 1. Drawings that cannot be read are left
/// out.
fn decode(body: &str) -> Vec<Drawing> {
    let Ok(form) = serde_json::from_str::<ReadForm>(body) else {
        return Vec::new();
    };
    form.drawings
        .into_iter()
        .filter_map(|value| serde_json::from_value::<SavedDrawing>(value).ok())
        .zip(1..)
        .filter_map(|(saved, id)| saved.drawing(id))
        .collect()
}

/// The drawings of `symbol` in `market`.
pub async fn load_drawings(
    user: SharedUserData,
    market: Market,
    symbol: String,
) -> Result<Vec<Drawing>, MarketError> {
    let body = with_user(user, move |user| user.drawings(market, &symbol)).await?;
    Ok(body.as_deref().map(decode).unwrap_or_default())
}

/// Saves the drawings of `symbol` in `market`, replacing what was saved before.
pub async fn save_drawings(
    user: SharedUserData,
    market: Market,
    symbol: String,
    drawings: Vec<Drawing>,
) -> Result<(), MarketError> {
    let body = (!drawings.is_empty()).then(|| encode(&drawings));
    with_user(user, move |user| {
        user.save_drawings(market, &symbol, body.as_deref())
    })
    .await
}

/// A look saved under a name, for one kind of drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawingTemplate {
    pub kind: DrawingKind,
    pub name: String,
    pub style: DrawingStyle,
}

/// The looks saved for drawings: what each kind's new drawings start with, and the named
/// templates.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DrawingLooks {
    pub defaults: HashMap<DrawingKind, DrawingStyle>,
    /// By kind, then by name.
    pub templates: Vec<DrawingTemplate>,
}

#[derive(Serialize)]
struct SavedLook {
    version: u32,
    style: SavedStyle,
}

#[derive(Deserialize)]
struct ReadLook {
    style: SavedStyle,
}

fn encode_look(style: &DrawingStyle, kind: DrawingKind) -> String {
    let look = SavedLook {
        version: FORMAT_VERSION,
        style: SavedStyle::new(style, kind),
    };
    serde_json::to_string(&look).expect("styles always serialize")
}

/// Reads a saved look for a drawing of `kind`, if it can be read.
fn decode_look(body: &str, kind: DrawingKind) -> Option<DrawingStyle> {
    let look = serde_json::from_str::<ReadLook>(body).ok()?;
    Some(look.style.style(kind))
}

/// Every saved look. Looks that cannot be read, or whose kind no longer exists, are left
/// out.
pub async fn load_drawing_looks(user: SharedUserData) -> Result<DrawingLooks, MarketError> {
    let (defaults, templates) = with_user(user, |user| {
        Ok((user.drawing_defaults()?, user.drawing_templates()?))
    })
    .await?;
    let defaults = defaults
        .into_iter()
        .filter_map(|(id, body)| {
            let kind = DrawingKind::from_id(&id)?;
            Some((kind, decode_look(&body, kind)?))
        })
        .collect();
    let templates = templates
        .into_iter()
        .filter_map(|stored| {
            let kind = DrawingKind::from_id(&stored.kind)?;
            Some(DrawingTemplate {
                kind,
                style: decode_look(&stored.body, kind)?,
                name: stored.name,
            })
        })
        .collect();
    Ok(DrawingLooks {
        defaults,
        templates,
    })
}

/// Saves a template, replacing any of its kind with the same name.
pub async fn save_drawing_template(
    user: SharedUserData,
    template: DrawingTemplate,
) -> Result<(), MarketError> {
    let stored = StoredDrawingTemplate {
        kind: template.kind.id().to_owned(),
        body: encode_look(&template.style, template.kind),
        name: template.name,
    };
    with_user(user, move |user| user.save_drawing_template(&stored)).await
}

pub async fn delete_drawing_template(
    user: SharedUserData,
    kind: DrawingKind,
    name: String,
) -> Result<(), MarketError> {
    with_user(user, move |user| {
        user.delete_drawing_template(kind.id(), &name)
    })
    .await
}

/// Saves the look new drawings of `kind` start with; `None` goes back to Fitil's.
pub async fn save_drawing_default(
    user: SharedUserData,
    kind: DrawingKind,
    style: Option<DrawingStyle>,
) -> Result<(), MarketError> {
    let body = style.map(|style| encode_look(&style, kind));
    with_user(user, move |user| {
        user.save_drawing_default(kind.id(), body.as_deref())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    use fitil_core::{ExchangeId, MarketKind};
    use fitil_store::UserData;

    fn market() -> Market {
        Market {
            exchange: ExchangeId::new("binance"),
            kind: MarketKind::Spot,
        }
    }

    fn anchor(time: i64, price: f64) -> Anchor {
        Anchor { time, price }
    }

    fn shared_user() -> SharedUserData {
        Arc::new(Mutex::new(UserData::open_in_memory().unwrap()))
    }

    fn drawings() -> Vec<Drawing> {
        let mut note = Drawing::new(1, DrawingKind::Text, vec![anchor(10, 100.0)]);
        note.text = "Destek".into();
        note.locked = true;
        let mut fib = Drawing::new(
            2,
            DrawingKind::FibRetracement,
            vec![anchor(0, 90.0), anchor(5, 120.0)],
        );
        let style = &mut fib.style;
        style.levels[3].shown = false;
        style.levels[11].color = Rgb::hex(0xFFEB3B);
        style.color = Rgb::hex(0xFF6D00);
        style.line = LineStyle::Dashed;
        style.width = 3;
        style.fill = false;
        style.opacity = 40;
        style.one_color = true;
        style.trend.shown = false;
        style.extend = Extend::Right;
        style.reverse = true;
        style.labels.percent = true;
        style.labels.align = LabelAlign::Right;
        style.labels.side = LabelSide::Top;
        style.visibility.set_range(TimeUnit::Hour, Some((1, 4)));
        style.visibility.set_range(TimeUnit::Minute, None);
        let mut box_ = Drawing::new(
            3,
            DrawingKind::Rectangle,
            vec![anchor(0, 1.0), anchor(1, 2.0)],
        );
        box_.style.opacity = 60;
        vec![note, fib, box_]
    }

    #[tokio::test]
    async fn drawings_are_saved_and_read_back() {
        let user = shared_user();
        let read = || load_drawings(user.clone(), market(), "BTCUSDT".into());
        assert!(read().await.unwrap().is_empty());

        save_drawings(user.clone(), market(), "BTCUSDT".into(), drawings())
            .await
            .unwrap();
        assert_eq!(read().await.unwrap(), drawings());

        // Removing every drawing leaves nothing behind.
        save_drawings(user.clone(), market(), "BTCUSDT".into(), Vec::new())
            .await
            .unwrap();
        assert!(read().await.unwrap().is_empty());
    }

    #[test]
    fn unreadable_drawings_are_left_out() {
        let body = r#"{
            "version": 1,
            "drawings": [
                {"kind":"spiral","points":[[0,1.0]],"color":[0,0,0],"width":1,"line":"solid"},
                {"kind":"ray","points":[[0,1.0]],"color":[0,0,0],"width":1,"line":"solid"},
                "not a drawing",
                {"kind":"horizontal-line","points":[[5,2.5]],"color":[1,2,3],"width":9,"line":"wavy"}
            ]
        }"#;
        let read = decode(body);
        assert_eq!(read.len(), 1);
        let line = &read[0];
        assert_eq!(line.kind, DrawingKind::HorizontalLine);
        assert_eq!((line.style.width, line.style.line), (4, LineStyle::Solid));
        assert!(decode("{").is_empty());
    }

    #[test]
    fn version_one_retracements_keep_their_look() {
        let body = r#"{
            "version": 1,
            "drawings": [
                {"kind":"fib-retracement","points":[[0,1.0],[5,2.0]],"color":[255,109,0],
                 "width":2,"line":"solid","levels":[0.0,0.5,0.618,1.0]},
                {"kind":"rectangle","points":[[0,1.0],[5,2.0]],"color":[0,0,0],"width":1,
                 "line":"solid"}
            ]
        }"#;
        let read = decode(body);
        let fib = &read[0].style;
        // The levels saved are shown in the drawing's color; Fitil's others are offered,
        // hidden.
        let shown: Vec<f64> = fib
            .levels
            .iter()
            .filter(|level| level.shown)
            .map(|level| level.value)
            .collect();
        assert_eq!(shown, [0.0, 0.5, 0.618, 1.0]);
        assert_eq!(fib.levels.len(), FIB_LEVELS.len());
        assert!(fib.one_color);
        assert!(fib.levels.iter().take(4).all(|l| l.color == fib.color));
        assert_eq!(fib.visibility, Visibility::ALL);
        // A rectangle gets Fitil's fill.
        let rectangle = &read[1].style;
        assert!(rectangle.fill);
        assert_eq!(rectangle.opacity, 15);
    }

    #[test]
    fn only_what_a_kind_uses_is_saved() {
        let line = Drawing::new(
            1,
            DrawingKind::TrendLine,
            vec![anchor(0, 1.0), anchor(1, 2.0)],
        );
        let body = encode(&[line]);
        for key in ["fill", "levels", "trend", "visibility"] {
            assert!(!body.contains(&format!("\"{key}\"")), "{key} in {body}");
        }
        assert!(body.contains(r#""version":2"#));
    }

    #[tokio::test]
    async fn looks_are_saved_by_kind() {
        let user = shared_user();
        assert_eq!(
            load_drawing_looks(user.clone()).await.unwrap(),
            DrawingLooks::default()
        );

        let mut style = DrawingStyle::of(DrawingKind::FibRetracement);
        style.levels.truncate(3);
        style.extend = Extend::Both;
        let template = DrawingTemplate {
            kind: DrawingKind::FibRetracement,
            name: "Kısa".into(),
            style: style.clone(),
        };
        save_drawing_template(user.clone(), template.clone())
            .await
            .unwrap();
        save_drawing_default(
            user.clone(),
            DrawingKind::FibRetracement,
            Some(style.clone()),
        )
        .await
        .unwrap();
        let looks = load_drawing_looks(user.clone()).await.unwrap();
        assert_eq!(looks.templates, [template]);
        assert_eq!(looks.defaults[&DrawingKind::FibRetracement], style);

        delete_drawing_template(user.clone(), DrawingKind::FibRetracement, "Kısa".into())
            .await
            .unwrap();
        save_drawing_default(user.clone(), DrawingKind::FibRetracement, None)
            .await
            .unwrap();
        assert_eq!(
            load_drawing_looks(user).await.unwrap(),
            DrawingLooks::default()
        );
    }
}
