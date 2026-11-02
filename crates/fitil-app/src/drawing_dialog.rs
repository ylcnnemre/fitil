//! A drawing's settings window, as in TradingView: its look, its points and the
//! timeframes it shows on, with templates of looks. Changes show on the chart at once;
//! the window then keeps them or puts the drawing back as it was.

use std::collections::BTreeMap;
use std::ops::Range;

use chrono::{Local, NaiveDate, NaiveDateTime, TimeZone, Timelike};
use fitil_core::{
    Drawing, DrawingKind, Extend, LabelAlign, LabelSide, LineStyle, Rgb, TimeUnit, Visibility,
};
use fitil_indicators::PALETTE;
use fitil_market::Template;
use iced::widget::{
    Column, button, checkbox, column, container, pick_list, row, rule, scrollable, slider, space,
    text, text_input,
};
use iced::{Element, Fill};

use crate::dialogs::{frame, swatch};
use crate::flyout::flyout;
use crate::i18n::Translations;
use crate::view::{Choice, menu_box, menu_row};
use crate::{Fitil, Message, icons, studies};

/// The box of a template's name, focused when it shows.
pub const TEMPLATE_NAME_ID: &str = "drawing-template-name";

const WIDTH: f32 = 600.0;
const LABEL_WIDTH: f32 = 170.0;
const MENU_WIDTH: f32 = 260.0;
/// How dates are shown and typed, in local time.
const TIME_FORMAT: &str = "%Y-%m-%d %H:%M";
const TIME_FORMAT_SECONDS: &str = "%Y-%m-%d %H:%M:%S";
const DATE_FORMAT: &str = "%Y-%m-%d";

/// The window's pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Style,
    Coordinates,
    Visibility,
}

impl Tab {
    const ALL: [Tab; 3] = [Tab::Style, Tab::Coordinates, Tab::Visibility];

    fn key(self) -> &'static str {
        match self {
            Tab::Style => "drawing-tab-style",
            Tab::Coordinates => "drawing-tab-coordinates",
            Tab::Visibility => "drawing-tab-visibility",
        }
    }
}

/// One of a drawing's colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorTarget {
    /// The drawing's own color: its lines, or a retracement's levels in one color.
    Line,
    /// The line of a retracement's move.
    Trend,
    Level(usize),
}

/// A setting typed as text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Field {
    Level(usize),
    Price(usize),
    Time(usize),
    /// The ends of the counts of a unit's timeframes the drawing shows on.
    From(TimeUnit),
    To(TimeUnit),
}

/// A change made in the window.
#[derive(Debug, Clone, PartialEq)]
pub enum Setting {
    Color(ColorTarget, Rgb),
    Width(u8),
    Line(LineStyle),
    Fill(bool),
    Opacity(u8),
    Text(String),
    OneColor(bool),
    TrendShown(bool),
    TrendLine(LineStyle),
    Extend(Extend),
    Reverse(bool),
    LabelLevels(bool),
    LabelPercent(bool),
    LabelPrices(bool),
    LabelAlign(LabelAlign),
    LabelSide(LabelSide),
    LevelShown(usize, bool),
    /// Text typed in a field, put into the drawing once it can be read.
    Typed(Field, String),
    /// Whether the drawing shows on the timeframes of a unit.
    UnitShown(TimeUnit, bool),
}

/// What is done from the template menu.
#[derive(Debug, Clone, PartialEq)]
pub enum Look {
    /// Opens or closes the menu.
    Menu(bool),
    /// The name to save the look under, as typed; the first starts the typing.
    Naming(String),
    Save,
    Apply(String),
    Delete(String),
    /// Saves the look as the one new drawings of the kind start with.
    SaveDefault,
    /// Gives the drawing the look new drawings of its kind start with.
    ApplyDefault,
    /// Gives the drawing Fitil's own look for its kind.
    Fitil,
}

/// The window's state. The drawing it changes is kept by the chart.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DrawingWindow {
    pub tab: Tab,
    /// Text typed in fields, kept as typed until it can be read.
    pub drafts: BTreeMap<Field, String>,
    /// The color whose picker is open.
    pub picker: Option<ColorTarget>,
    pub templates_open: bool,
    /// The name of the template to save, once saving one was chosen.
    pub template_name: Option<String>,
}

impl DrawingWindow {
    /// Makes `setting` on `drawing`.
    pub fn apply(&mut self, drawing: &mut Drawing, setting: Setting) {
        let style = &mut drawing.style;
        match setting {
            Setting::Color(target, color) => {
                match target {
                    ColorTarget::Line => style.color = color,
                    ColorTarget::Trend => style.trend.color = color,
                    ColorTarget::Level(index) => {
                        if let Some(level) = style.levels.get_mut(index) {
                            level.color = color;
                        }
                    }
                }
                self.picker = None;
            }
            Setting::Width(width) => style.width = width,
            Setting::Line(line) => style.line = line,
            Setting::Fill(fill) => style.fill = fill,
            Setting::Opacity(opacity) => style.opacity = opacity.min(100),
            Setting::Text(words) => drawing.text = words,
            Setting::OneColor(one) => style.one_color = one,
            Setting::TrendShown(shown) => style.trend.shown = shown,
            Setting::TrendLine(line) => style.trend.line = line,
            Setting::Extend(extend) => style.extend = extend,
            Setting::Reverse(reverse) => style.reverse = reverse,
            Setting::LabelLevels(shown) => style.labels.levels = shown,
            Setting::LabelPercent(percent) => style.labels.percent = percent,
            Setting::LabelPrices(shown) => style.labels.prices = shown,
            Setting::LabelAlign(align) => style.labels.align = align,
            Setting::LabelSide(side) => style.labels.side = side,
            Setting::LevelShown(index, shown) => {
                if let Some(level) = style.levels.get_mut(index) {
                    level.shown = shown;
                }
            }
            Setting::Typed(field, typed) => {
                read(drawing, field, &typed);
                self.drafts.insert(field, typed);
            }
            Setting::UnitShown(unit, shown) => {
                let range = shown.then(|| Visibility::limits(unit));
                style.visibility.set_range(unit, range);
                self.drafts.remove(&Field::From(unit));
                self.drafts.remove(&Field::To(unit));
            }
        }
    }
}

/// Puts what was typed in `field` into the drawing, if it can be read.
fn read(drawing: &mut Drawing, field: Field, typed: &str) {
    match field {
        Field::Level(index) => {
            if let (Some(value), Some(level)) = (
                studies::parse_number(typed),
                drawing.style.levels.get_mut(index),
            ) {
                level.value = value;
            }
        }
        Field::Price(index) => {
            if let (Some(price), Some(point)) =
                (studies::parse_number(typed), drawing.points.get_mut(index))
            {
                point.price = price;
            }
        }
        Field::Time(index) => {
            if let (Some(time), Some(point)) = (parse_time(typed), drawing.points.get_mut(index)) {
                point.time = time;
            }
        }
        Field::From(unit) | Field::To(unit) => {
            let visibility = &mut drawing.style.visibility;
            let (Ok(count), Some((from, to))) =
                (typed.trim().parse::<u32>(), visibility.range(unit))
            else {
                return;
            };
            // Each end stays on its side of the other.
            let range = match field {
                Field::From(_) => (count.min(to), to),
                _ => (from, count.max(from)),
            };
            visibility.set_range(unit, Some(range));
        }
    }
}

/// A moment as the window shows it: the local date and time, with the seconds if any.
pub fn time_text(time: i64) -> String {
    match Local.timestamp_millis_opt(time).single() {
        Some(local) if local.second() != 0 => local.format(TIME_FORMAT_SECONDS).to_string(),
        Some(local) => local.format(TIME_FORMAT).to_string(),
        None => String::new(),
    }
}

/// A local date and time as typed in the window, with or without the seconds, or a date
/// alone for its midnight.
pub fn parse_time(typed: &str) -> Option<i64> {
    let typed = typed.trim();
    let naive = NaiveDateTime::parse_from_str(typed, TIME_FORMAT_SECONDS)
        .or_else(|_| NaiveDateTime::parse_from_str(typed, TIME_FORMAT))
        .ok()
        .or_else(|| {
            NaiveDate::parse_from_str(typed, DATE_FORMAT)
                .ok()?
                .and_hms_opt(0, 0, 0)
        })?;
    Local
        .from_local_datetime(&naive)
        .earliest()
        .map(|time| time.timestamp_millis())
}

/// A price as the window shows it: to the symbol's decimals, without trailing zeros.
fn price_text(price: f64, decimals: u32) -> String {
    let text = format!("{price:.*}", decimals as usize);
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        text
    }
}

pub fn view<'a>(app: &'a Fitil, window: &'a DrawingWindow) -> Element<'a, Message> {
    let t = &app.translations;
    let Some((drawing, decimals)) = app.selection.as_ref().and_then(|selection| {
        let drawing = selection.chart.edited_drawing()?;
        Some((drawing, selection.symbol.price_decimals))
    }) else {
        return frame(String::new(), WIDTH, space().into());
    };

    let tabs = row(Tab::ALL.map(|tab| {
        let style = if window.tab == tab {
            button::primary
        } else {
            button::secondary
        };
        button(text(t.get(tab.key())).size(13))
            .style(style)
            .padding([4, 12])
            .on_press(Message::DrawingTab(tab))
            .into()
    }))
    .spacing(6);
    let page = match window.tab {
        Tab::Style => style_page(t, drawing, window),
        Tab::Coordinates => coordinates_page(t, drawing, window, decimals),
        Tab::Visibility => visibility_page(t, drawing, window),
    };
    let buttons = row![
        templates(app, drawing, window),
        space().width(Fill),
        button(text(t.get("drawing-cancel")))
            .style(button::secondary)
            .on_press(Message::CloseDialog),
        button(text(t.get("drawing-ok"))).on_press(Message::DrawingSettingsDone),
    ]
    .spacing(8)
    .align_y(iced::Center);
    let body = column![tabs, scrollable(page).spacing(6).height(Fill), buttons].spacing(14);
    frame(
        t.get(&format!("drawing-{}", drawing.kind.id())),
        WIDTH,
        body.into(),
    )
}

fn setting(setting: Setting) -> Message {
    Message::DrawingSetting(setting)
}

/// A row of the window: what is set, then how.
fn labeled<'a>(label: String, control: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    row![text(label).width(LABEL_WIDTH), control.into()]
        .spacing(12)
        .align_y(iced::Center)
        .into()
}

/// A list of `values` named by translation keys with `prefix` and their ids.
fn choices<T: Copy>(
    t: &Translations,
    values: impl IntoIterator<Item = T>,
    prefix: &str,
    id: impl Fn(T) -> &'static str,
) -> Vec<Choice<T>> {
    values
        .into_iter()
        .map(|value| Choice {
            value,
            name: t.get(&format!("{prefix}{}", id(value))),
        })
        .collect()
}

/// A pick list of `options`, with `current` chosen.
fn chooser<'a, T: Copy + PartialEq + 'static>(
    options: Vec<Choice<T>>,
    current: T,
    on_choose: impl Fn(T) -> Setting + 'a,
) -> Element<'a, Message> {
    let chosen = options
        .iter()
        .find(|choice| choice.value == current)
        .cloned();
    pick_list(options, chosen, move |choice: Choice<T>| {
        setting(on_choose(choice.value))
    })
    .width(150)
    .into()
}

fn line_chooser<'a>(
    t: &Translations,
    current: LineStyle,
    on_choose: impl Fn(LineStyle) -> Setting + 'a,
) -> Element<'a, Message> {
    let lines = choices(t, LineStyle::ALL, "drawing-line-", LineStyle::id);
    chooser(lines, current, on_choose)
}

fn width_chooser<'a>(width: u8) -> Element<'a, Message> {
    pick_list([1u8, 2, 3, 4], Some(width), |width| {
        setting(Setting::Width(width))
    })
    .width(60)
    .into()
}

/// A square of `rgb` that opens a palette beside it, to pick the color of `target`.
fn color_button<'a>(rgb: Rgb, target: ColorTarget, window: &DrawingWindow) -> Element<'a, Message> {
    let open = window.picker == Some(target);
    let anchor = swatch(rgb, open, Message::DrawingPicker((!open).then_some(target)));
    let palette = open.then(|| palette(rgb, target));
    flyout(anchor, palette, Message::DrawingPicker(None)).into()
}

/// The colors to pick from, with `current` first when the palette lacks it.
fn palette<'a>(current: Rgb, target: ColorTarget) -> Element<'a, Message> {
    let mut colors = PALETTE.to_vec();
    if !colors.contains(&current) {
        colors.insert(0, current);
    }
    let rows = colors.chunks(4).map(|chunk| {
        row(chunk
            .iter()
            .map(|&rgb| swatch(rgb, rgb == current, setting(Setting::Color(target, rgb)))))
        .spacing(4)
        .into()
    });
    container(column(rows).spacing(4))
        .padding(8)
        .style(menu_box)
        .into()
}

fn style_page<'a>(
    t: &Translations,
    drawing: &'a Drawing,
    window: &'a DrawingWindow,
) -> Element<'a, Message> {
    let style = &drawing.style;
    // A measure takes its colors from its direction.
    if drawing.kind == DrawingKind::Measure {
        return text(t.get("drawing-measure-colors"))
            .style(text::secondary)
            .into();
    }
    let levels = drawing.kind.has_levels();
    let mut page = Column::new().spacing(12);
    if levels {
        // A retracement's own color is the one its levels take with one color.
        page = page.push(labeled(
            t.get("drawing-levels-line"),
            row![
                width_chooser(style.width),
                line_chooser(t, style.line, Setting::Line)
            ]
            .spacing(8),
        ));
        page = page.push(labeled(
            t.get("drawing-trend-line"),
            row![
                checkbox(style.trend.shown).on_toggle(|shown| setting(Setting::TrendShown(shown))),
                color_button(style.trend.color, ColorTarget::Trend, window),
                line_chooser(t, style.trend.line, Setting::TrendLine),
            ]
            .spacing(8)
            .align_y(iced::Center),
        ));
        page = page.push(labeled(
            t.get("drawing-extend"),
            chooser(
                choices(t, Extend::ALL, "drawing-extend-", Extend::id),
                style.extend,
                Setting::Extend,
            ),
        ));
        page = page.push(level_table(drawing, window));
        page = page.push(labeled(
            t.get("drawing-one-color"),
            row![
                checkbox(style.one_color).on_toggle(|one| setting(Setting::OneColor(one))),
                color_button(style.color, ColorTarget::Line, window),
            ]
            .spacing(8)
            .align_y(iced::Center),
        ));
    } else {
        page = page.push(labeled(
            t.get("drawing-line"),
            row![
                color_button(style.color, ColorTarget::Line, window),
                width_chooser(style.width),
                line_chooser(t, style.line, Setting::Line),
            ]
            .spacing(8)
            .align_y(iced::Center),
        ));
    }
    if drawing.kind == DrawingKind::Text {
        page = page.push(labeled(
            t.get("drawing-words"),
            text_input(&t.get("drawing-text-placeholder"), &drawing.text)
                .on_input(|words| setting(Setting::Text(words)))
                .width(Fill),
        ));
    }
    if drawing.kind.has_fill() {
        let percent = t.get_with("drawing-opacity", &[("percent", style.opacity.to_string())]);
        page = page.push(labeled(
            t.get("drawing-background"),
            row![
                checkbox(style.fill).on_toggle(|fill| setting(Setting::Fill(fill))),
                slider(0..=100u8, style.opacity, |opacity| {
                    setting(Setting::Opacity(opacity))
                })
                .width(180),
                text(percent).size(13),
            ]
            .spacing(10)
            .align_y(iced::Center),
        ));
    }
    if levels {
        let toggle = |label: String, on: bool, change: fn(bool) -> Setting| {
            labeled(label, checkbox(on).on_toggle(move |on| setting(change(on))))
        };
        page = page
            .push(toggle(
                t.get("drawing-reverse"),
                style.reverse,
                Setting::Reverse,
            ))
            .push(toggle(
                t.get("drawing-prices"),
                style.labels.prices,
                Setting::LabelPrices,
            ));
        let percent = choices(t, [false, true], "drawing-levels-", |percent| {
            if percent { "percent" } else { "values" }
        });
        page = page.push(labeled(
            t.get("drawing-levels"),
            row![
                checkbox(style.labels.levels).on_toggle(|on| setting(Setting::LabelLevels(on))),
                chooser(percent, style.labels.percent, Setting::LabelPercent),
            ]
            .spacing(8)
            .align_y(iced::Center),
        ));
        page = page.push(labeled(
            t.get("drawing-labels"),
            row![
                chooser(
                    choices(t, LabelAlign::ALL, "drawing-label-", LabelAlign::id),
                    style.labels.align,
                    Setting::LabelAlign,
                ),
                chooser(
                    choices(t, LabelSide::ALL, "drawing-label-", LabelSide::id),
                    style.labels.side,
                    Setting::LabelSide,
                ),
            ]
            .spacing(8),
        ));
    }
    page.into()
}

/// The levels in two columns: each shown or not, its value and its color.
fn level_table<'a>(drawing: &'a Drawing, window: &'a DrawingWindow) -> Element<'a, Message> {
    let levels = &drawing.style.levels;
    let cell = |index: usize| -> Element<'a, Message> {
        let level = &levels[index];
        let typed = window
            .drafts
            .get(&Field::Level(index))
            .cloned()
            .unwrap_or_else(|| studies::number(level.value));
        row![
            checkbox(level.shown)
                .on_toggle(move |shown| setting(Setting::LevelShown(index, shown))),
            text_input("", &typed)
                .on_input(move |typed| setting(Setting::Typed(Field::Level(index), typed)))
                .width(90),
            color_button(level.color, ColorTarget::Level(index), window),
        ]
        .spacing(8)
        .align_y(iced::Center)
        .into()
    };
    let half = levels.len().div_ceil(2);
    let part = |range: Range<usize>| column(range.map(cell)).spacing(6);
    row![part(0..half), part(half..levels.len())]
        .spacing(32)
        .into()
}

fn coordinates_page<'a>(
    t: &Translations,
    drawing: &'a Drawing,
    window: &'a DrawingWindow,
    decimals: u32,
) -> Element<'a, Message> {
    let mut page = Column::new().spacing(14);
    for (index, point) in drawing.points.iter().enumerate() {
        let typed =
            |field: Field, shown: String| window.drafts.get(&field).cloned().unwrap_or(shown);
        let price = typed(Field::Price(index), price_text(point.price, decimals));
        let time = typed(Field::Time(index), time_text(point.time));
        page = page.push(
            column![
                text(t.get_with("drawing-point", &[("number", (index + 1).to_string())]))
                    .size(13)
                    .style(text::secondary),
                row![
                    text(t.get("drawing-price")).width(60),
                    text_input("", &price)
                        .on_input(move |typed| setting(Setting::Typed(Field::Price(index), typed)))
                        .width(160),
                    text(t.get("drawing-time")).width(60),
                    text_input(&t.get("drawing-time-placeholder"), &time)
                        .on_input(move |typed| setting(Setting::Typed(Field::Time(index), typed)))
                        .width(190),
                ]
                .spacing(10)
                .align_y(iced::Center),
            ]
            .spacing(6),
        );
    }
    page.into()
}

fn visibility_page<'a>(
    t: &Translations,
    drawing: &'a Drawing,
    window: &'a DrawingWindow,
) -> Element<'a, Message> {
    let mut page = Column::new().spacing(10);
    for unit in TimeUnit::ALL {
        let range = drawing.style.visibility.range(unit);
        let (low, high) = Visibility::limits(unit);
        let (from, to) = range.unwrap_or((low, high));
        let count = |field: Field, value: u32| {
            let typed = window
                .drafts
                .get(&field)
                .cloned()
                .unwrap_or_else(|| value.to_string());
            let input = text_input("", &typed).width(70);
            // A unit the drawing does not show on has no counts to type.
            if range.is_some() {
                input.on_input(move |typed| setting(Setting::Typed(field, typed)))
            } else {
                input
            }
        };
        let shown = checkbox(range.is_some())
            .label(t.get(&format!("drawing-visibility-{}", unit.id())))
            .on_toggle(move |shown| setting(Setting::UnitShown(unit, shown)));
        page = page.push(
            row![
                container(shown).width(LABEL_WIDTH),
                count(Field::From(unit), from),
                text("–"),
                count(Field::To(unit), to),
                text(t.get_with(
                    "drawing-visibility-limits",
                    &[("from", low.to_string()), ("to", high.to_string())],
                ))
                .size(12)
                .style(text::secondary),
            ]
            .spacing(10)
            .align_y(iced::Center),
        );
    }
    page.into()
}

/// The template button, with its menu above the window while open.
fn templates<'a>(
    app: &'a Fitil,
    drawing: &'a Drawing,
    window: &'a DrawingWindow,
) -> Element<'a, Message> {
    let t = &app.translations;
    let open = window.templates_open;
    let anchor = button(
        row![
            text(t.get("drawing-template")),
            icons::icon(icons::CHEVRON_DOWN, 14.0)
        ]
        .spacing(6)
        .align_y(iced::Center),
    )
    .style(button::secondary)
    .on_press(Message::DrawingLook(Look::Menu(!open)));
    let menu = open.then(|| template_menu(app, drawing, window));
    flyout(anchor, menu, Message::DrawingLook(Look::Menu(false))).into()
}

fn template_menu<'a>(
    app: &'a Fitil,
    drawing: &'a Drawing,
    window: &'a DrawingWindow,
) -> Element<'a, Message> {
    let t = &app.translations;
    let item = |label: String, look: Look| {
        button(text(label).size(13))
            .style(menu_row)
            .width(Fill)
            .padding([6, 10])
            .on_press(Message::DrawingLook(look))
    };
    let mut items = Column::new().width(MENU_WIDTH).spacing(2);
    items = match &window.template_name {
        Some(name) => {
            let save = Template::clean_name(name).map(|_| Message::DrawingLook(Look::Save));
            items.push(
                row![
                    text_input(&t.get("drawing-template-name"), name)
                        .id(TEMPLATE_NAME_ID)
                        .on_input(|name| Message::DrawingLook(Look::Naming(name)))
                        .on_submit_maybe(save.clone()),
                    button(text(t.get("drawing-template-save")).size(13)).on_press_maybe(save),
                ]
                .spacing(6)
                .padding(4)
                .align_y(iced::Center),
            )
        }
        None => items.push(item(
            t.get("drawing-template-save-as"),
            Look::Naming(String::new()),
        )),
    };
    items = items
        .push(item(t.get("drawing-default-save"), Look::SaveDefault))
        .push(item(t.get("drawing-default-apply"), Look::ApplyDefault))
        .push(item(t.get("drawing-fitil-look"), Look::Fitil));
    let mut saved = app
        .drawing_looks
        .templates
        .iter()
        .filter(|template| template.kind == drawing.kind)
        .peekable();
    if saved.peek().is_some() {
        items = items.push(rule::horizontal(1));
    }
    for template in saved {
        items = items.push(
            row![
                item(template.name.clone(), Look::Apply(template.name.clone())),
                button(icons::icon(icons::TRASH, 13.0))
                    .style(button::text)
                    .padding([4, 6])
                    .on_press(Message::DrawingLook(Look::Delete(template.name.clone()))),
            ]
            .align_y(iced::Center),
        );
    }
    container(items).padding(4).style(menu_box).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fitil_core::{Anchor, DrawingStyle};

    fn fib() -> Drawing {
        Drawing::new(
            1,
            DrawingKind::FibRetracement,
            vec![
                Anchor {
                    time: 0,
                    price: 100.0,
                },
                Anchor {
                    time: 3_600_000,
                    price: 110.0,
                },
            ],
        )
    }

    #[test]
    fn settings_change_the_drawing() {
        let mut window = DrawingWindow {
            picker: Some(ColorTarget::Level(2)),
            ..DrawingWindow::default()
        };
        let mut drawing = fib();
        window.apply(
            &mut drawing,
            Setting::Color(ColorTarget::Level(2), Rgb::hex(0xFFEB3B)),
        );
        assert_eq!(drawing.style.levels[2].color, Rgb::hex(0xFFEB3B));
        assert_eq!(window.picker, None, "picking a color closes the palette");
        window.apply(&mut drawing, Setting::Extend(Extend::Both));
        window.apply(&mut drawing, Setting::LevelShown(0, false));
        window.apply(&mut drawing, Setting::Opacity(250));
        assert_eq!(drawing.style.extend, Extend::Both);
        assert!(!drawing.style.levels[0].shown);
        assert_eq!(drawing.style.opacity, 100);
    }

    #[test]
    fn typed_settings_wait_until_they_can_be_read() {
        let mut window = DrawingWindow::default();
        let mut drawing = fib();
        let mut typed = |drawing: &mut Drawing, field, text: &str| {
            window.apply(drawing, Setting::Typed(field, text.into()));
        };
        typed(&mut drawing, Field::Level(1), "0,25");
        assert_eq!(drawing.style.levels[1].value, 0.25);
        typed(&mut drawing, Field::Level(1), "0,2x");
        assert_eq!(drawing.style.levels[1].value, 0.25);
        typed(&mut drawing, Field::Price(0), "95.5");
        assert_eq!(drawing.points[0].price, 95.5);

        // Times are local; a date alone is its midnight.
        let noon = parse_time("2026-11-02 12:30").unwrap();
        typed(&mut drawing, Field::Time(1), "2026-11-02 12:30");
        assert_eq!(drawing.points[1].time, noon);
        assert_eq!(time_text(noon), "2026-11-02 12:30");
        assert_eq!(parse_time("2026-11-02"), parse_time("2026-11-02 00:00:00"));
        assert_eq!(parse_time("02.11.2026"), None);

        // Counts of a unit stay in order and within its limits.
        typed(&mut drawing, Field::To(TimeUnit::Hour), "4");
        typed(&mut drawing, Field::From(TimeUnit::Hour), "9");
        assert_eq!(drawing.style.visibility.range(TimeUnit::Hour), Some((4, 4)));
        typed(&mut drawing, Field::To(TimeUnit::Hour), "99");
        assert_eq!(
            drawing.style.visibility.range(TimeUnit::Hour),
            Some((4, 24))
        );
        assert_eq!(window.drafts[&Field::To(TimeUnit::Hour)], "99");
    }

    #[test]
    fn units_are_turned_off_and_back_on_whole() {
        let mut window = DrawingWindow::default();
        let mut drawing = fib();
        window.apply(
            &mut drawing,
            Setting::Typed(Field::From(TimeUnit::Day), "3".into()),
        );
        window.apply(&mut drawing, Setting::UnitShown(TimeUnit::Day, false));
        assert_eq!(drawing.style.visibility.range(TimeUnit::Day), None);
        window.apply(&mut drawing, Setting::UnitShown(TimeUnit::Day, true));
        assert_eq!(
            drawing.style.visibility.range(TimeUnit::Day),
            Some(Visibility::limits(TimeUnit::Day))
        );
        assert!(window.drafts.is_empty());
        assert_eq!(drawing.style.levels, DrawingStyle::of(drawing.kind).levels);
    }

    #[test]
    fn prices_show_the_symbols_decimals() {
        assert_eq!(price_text(77908.38, 2), "77908.38");
        assert_eq!(price_text(77908.5, 2), "77908.5");
        assert_eq!(price_text(1.0, 2), "1");
        assert_eq!(price_text(0.00001234, 8), "0.00001234");
        assert_eq!(price_text(42.0, 0), "42");
    }
}
