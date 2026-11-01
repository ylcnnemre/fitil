//! The drawings of each symbol, kept in the user's data. A symbol's drawings show in
//! every timeframe; spot and futures pairs with the same name are different symbols.

use fitil_core::{Anchor, Drawing, DrawingKind, LineStyle, Market, Rgb};
use serde::{Deserialize, Serialize};

use crate::templates::with_user;
use crate::{MarketError, SharedUserData};

/// Version of the saved form, for later versions of Fitil to read older drawings.
const FORMAT_VERSION: u32 = 1;

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
    color: (u8, u8, u8),
    width: u8,
    line: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    levels: Vec<f64>,
    #[serde(default, skip_serializing_if = "is_false")]
    locked: bool,
}

fn is_false(value: &bool) -> bool {
    !value
}

impl SavedDrawing {
    fn new(drawing: &Drawing) -> Self {
        let Rgb(red, green, blue) = drawing.color;
        Self {
            kind: drawing.kind.id().to_owned(),
            points: drawing
                .points
                .iter()
                .map(|point| (point.time, point.price))
                .collect(),
            color: (red, green, blue),
            width: drawing.width,
            line: drawing.line.id().to_owned(),
            text: drawing.text.clone(),
            levels: drawing.levels.clone(),
            locked: drawing.locked,
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
        let (red, green, blue) = self.color;
        Some(Drawing {
            id,
            kind,
            points,
            color: Rgb(red, green, blue),
            width: self.width.clamp(1, 4),
            line: LineStyle::from_id(&self.line).unwrap_or_default(),
            text: self.text,
            levels: self
                .levels
                .into_iter()
                .filter(|level| level.is_finite())
                .collect(),
            locked: self.locked,
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

    fn drawings() -> Vec<Drawing> {
        let mut note = Drawing::new(1, DrawingKind::Text, vec![anchor(10, 100.0)]);
        note.text = "Destek".into();
        note.locked = true;
        let mut fib = Drawing::new(
            2,
            DrawingKind::FibRetracement,
            vec![anchor(0, 90.0), anchor(5, 120.0)],
        );
        fib.levels = vec![0.0, 0.5, 1.0];
        fib.color = Rgb::hex(0xFF6D00);
        fib.line = LineStyle::Dashed;
        fib.width = 3;
        vec![note, fib]
    }

    #[tokio::test]
    async fn drawings_are_saved_and_read_back() {
        let user = Arc::new(Mutex::new(UserData::open_in_memory().unwrap()));
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
        assert_eq!((line.width, line.line), (4, LineStyle::Solid));
        assert!(decode("{").is_empty());
    }
}
