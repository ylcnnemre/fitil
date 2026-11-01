//! Icons of the Lucide set (ISC license, see `assets/fonts/LICENSE-lucide.txt`), drawn
//! with its font, which is built into the program.

use fitil_core::DrawingKind;
use iced::Font;
use iced::widget::{Text, text};

pub const FONT_BYTES: &[u8] = include_bytes!("../../../assets/fonts/lucide.ttf");
const FONT: Font = Font::with_name("lucide");

// Code points of the icons used, from Lucide 1.66.0's `codepoints.json`.
pub const CURSOR: char = '\u{e1c3}'; // mouse-pointer-2
pub const TREND_LINE: char = '\u{e51d}'; // slash
pub const RAY: char = '\u{e493}'; // move-up-right
pub const EXTENDED_LINE: char = '\u{e1c4}'; // move-diagonal
pub const HORIZONTAL_LINE: char = '\u{e11c}'; // minus
pub const HORIZONTAL_RAY: char = '\u{e490}'; // move-right
pub const VERTICAL_LINE: char = '\u{e1c9}'; // separator-vertical
pub const RECTANGLE: char = '\u{e376}'; // rectangle-horizontal
pub const CHANNEL: char = '\u{e1bd}'; // equal
pub const FIBONACCI: char = '\u{e184}'; // text-align-justify
pub const TEXT: char = '\u{e198}'; // type
pub const MEASURE: char = '\u{e14b}'; // ruler
pub const MAGNET: char = '\u{e2b5}'; // magnet
pub const LOCK: char = '\u{e10b}'; // lock
pub const UNLOCK: char = '\u{e10c}'; // lock-open
pub const EYE: char = '\u{e0ba}'; // eye
pub const EYE_OFF: char = '\u{e0bb}'; // eye-off
pub const TRASH: char = '\u{e18e}'; // trash-2
pub const UNDO: char = '\u{e2a1}'; // undo-2
pub const REDO: char = '\u{e2a0}'; // redo-2
pub const COPY: char = '\u{e09e}'; // copy
pub const SEARCH: char = '\u{e151}'; // search
pub const INDICATORS: char = '\u{e038}'; // activity

/// An icon `size` pixels tall.
pub fn icon<'a>(glyph: char, size: f32) -> Text<'a> {
    text(glyph.to_string()).font(FONT).size(size)
}

pub fn for_tool(kind: DrawingKind) -> char {
    match kind {
        DrawingKind::TrendLine => TREND_LINE,
        DrawingKind::Ray => RAY,
        DrawingKind::ExtendedLine => EXTENDED_LINE,
        DrawingKind::HorizontalLine => HORIZONTAL_LINE,
        DrawingKind::HorizontalRay => HORIZONTAL_RAY,
        DrawingKind::VerticalLine => VERTICAL_LINE,
        DrawingKind::Rectangle => RECTANGLE,
        DrawingKind::ParallelChannel => CHANNEL,
        DrawingKind::FibRetracement => FIBONACCI,
        DrawingKind::Text => TEXT,
        DrawingKind::Measure => MEASURE,
    }
}
