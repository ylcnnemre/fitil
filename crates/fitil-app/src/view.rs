//! What the window shows, as in TradingView: a toolbar, the chart filling the window and
//! a status bar. Symbols are picked in a search window (see `dialogs`), opened from the
//! symbol's name in the toolbar or by typing on the chart.

use fitil_chart::{ChartKind, PriceScale};
use fitil_core::{Drawing, DrawingKind, LineStyle, Timeframe};
use fitil_indicators::PALETTE;
use fitil_market::Study;
use iced::widget::{
    Column, button, column, container, pick_list, row, rule, scrollable, space, text, text_input,
    tooltip,
};
use iced::{Element, Fill, Shrink};

use crate::i18n::{Language, Translations};
use crate::{
    Connection, Fitil, Load, Message, Selection, TIMEFRAME_CHOICES, dialogs, format, icons,
    storage, studies,
};

/// Timeframes with a button of their own; the others are in a list beside them.
const FAVORITE_TIMEFRAMES: [&str; 7] = ["1m", "5m", "15m", "1h", "4h", "1d", "1w"];

pub fn view(app: &Fitil) -> Element<'_, Message> {
    column![
        toolbar(app),
        rule::horizontal(1),
        container(chart_area(app))
            .width(Fill)
            .height(Fill)
            .padding([6, 10]),
        rule::horizontal(1),
        status_bar(app),
    ]
    .into()
}

/// An item of a picker: a value shown with a translated or proper name.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice<T> {
    pub value: T,
    pub name: String,
}

impl<T> std::fmt::Display for Choice<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

/// The symbol, timeframes, chart type, scales and indicators, then the languages.
fn toolbar(app: &Fitil) -> Element<'_, Message> {
    let t = &app.translations;
    let symbol = app.selection.as_ref().map_or_else(
        || t.get("chart-symbol-search"),
        |selection| selection.symbol.name.clone(),
    );
    let symbol_button = tooltip(
        button(
            row![icons::icon(icons::SEARCH, 15.0), text(symbol).size(16)]
                .spacing(8)
                .align_y(iced::Center),
        )
        .style(button::text)
        .padding([4, 10])
        .on_press(Message::OpenSymbolSearch(String::new())),
        hint(t.get("chart-symbol-hint")),
        tooltip::Position::Bottom,
    );

    let mut bar = row![symbol_button].spacing(8).align_y(iced::Center);
    if let Some(selection) = &app.selection {
        let kinds: Vec<Choice<ChartKind>> = ChartKind::ALL
            .into_iter()
            .map(|kind| Choice {
                value: kind,
                name: t.get(kind_key(kind)),
            })
            .collect();
        let current_kind = kinds
            .iter()
            .find(|choice| choice.value == app.chart_kind)
            .cloned();
        let switch = |on: bool| {
            if on { button::primary } else { button::text }
        };
        let scale_button = |scale: PriceScale, key: &str| {
            button(text(t.get(key)).size(14))
                .style(switch(app.price_scale == scale))
                .padding([4, 8])
                .on_press(Message::PriceScaleToggled(scale))
        };
        bar = bar
            .push(divider())
            .push(timeframes(app, selection))
            .push(divider())
            .push(
                pick_list(kinds, current_kind, |choice| {
                    Message::ChartKindSelected(choice.value)
                })
                .width(150),
            )
            .push(scale_button(PriceScale::Logarithmic, "scale-log"))
            .push(scale_button(PriceScale::Percent, "scale-percent"))
            .push(divider())
            .push(
                button(
                    row![
                        icons::icon(icons::INDICATORS, 15.0),
                        text(t.get("chart-indicators")).size(14),
                    ]
                    .spacing(6)
                    .align_y(iced::Center),
                )
                .style(button::text)
                .padding([4, 8])
                .on_press(Message::OpenIndicators),
            )
            .push(
                button(text(t.get("chart-volume-toggle")).size(14))
                    .style(switch(app.show_volume))
                    .padding([4, 8])
                    .on_press(Message::VolumeToggled),
            )
            .push(divider());
        let chart = &selection.chart;
        let history = |glyph: char, key: &str, can: bool, message: Message| {
            tooltip(
                button(icons::icon(glyph, 16.0))
                    .style(button::text)
                    .padding([4, 8])
                    .on_press_maybe(can.then_some(message)),
                hint(t.get(key)),
                tooltip::Position::Bottom,
            )
        };
        bar = bar
            .push(history(
                icons::UNDO,
                "chart-undo",
                chart.can_undo(),
                Message::Undo,
            ))
            .push(history(
                icons::REDO,
                "chart-redo",
                chart.can_redo(),
                Message::Redo,
            ));
    }

    let language_button = |language: Language| {
        let style = if t.language() == language {
            button::primary
        } else {
            button::secondary
        };
        button(text(t.get(language.name_key())).size(13))
            .style(style)
            .padding([3, 8])
            .on_press(Message::LanguageSelected(language))
    };
    bar.push(space().width(Fill))
        .push(row(Language::ALL.map(|language| language_button(language).into())).spacing(6))
        .padding([6, 10])
        .into()
}

/// A button for each favorite timeframe the exchange can provide, and a list of the
/// others.
fn timeframes<'a>(app: &'a Fitil, selection: &Selection) -> Element<'a, Message> {
    let t = &app.translations;
    let market = selection.symbol.market;
    let natives = app.exchange(market.exchange).timeframes(market.kind);
    let current = selection.series.target();
    let parse = |choices: &[&str]| -> Vec<Timeframe> {
        choices
            .iter()
            .filter_map(|choice| choice.parse::<Timeframe>().ok())
            .filter(|timeframe| timeframe.best_source(natives).is_some())
            .collect()
    };
    let favorites = parse(&FAVORITE_TIMEFRAMES);
    let others: Vec<Timeframe> = parse(&TIMEFRAME_CHOICES)
        .into_iter()
        .filter(|timeframe| !favorites.contains(timeframe))
        .collect();

    let mut buttons = row![].spacing(2).align_y(iced::Center);
    for timeframe in favorites {
        let style = if timeframe == current {
            button::primary
        } else {
            button::text
        };
        buttons = buttons.push(
            button(text(timeframe.to_string()).size(14))
                .style(style)
                .padding([4, 8])
                .on_press(Message::TimeframeSelected(timeframe)),
        );
    }
    let chosen = others.contains(&current).then_some(current);
    buttons
        .push(
            pick_list(others, chosen, Message::TimeframeSelected)
                .placeholder(t.get("chart-more-timeframes"))
                .width(90),
        )
        .into()
}

/// A thin line between groups of the toolbar.
fn divider<'a>() -> Element<'a, Message> {
    container(rule::vertical(1)).height(24).into()
}

/// A small box of text shown when hovering something.
fn hint<'a>(content: String) -> Element<'a, Message> {
    container(text(content).size(12))
        .padding(6)
        .style(container::rounded_box)
        .into()
}

fn kind_key(kind: ChartKind) -> &'static str {
    match kind {
        ChartKind::Candles => "chart-kind-candles",
        ChartKind::Bars => "chart-kind-bars",
        ChartKind::Line => "chart-kind-line",
        ChartKind::Area => "chart-kind-area",
        ChartKind::HeikinAshi => "chart-kind-heikin-ashi",
    }
}

/// The chart with its indicators, or what stands in for it while there is none.
fn chart_area(app: &Fitil) -> Element<'_, Message> {
    let t = &app.translations;
    let Some(selection) = &app.selection else {
        return no_chart(app);
    };
    let series = &selection.series;

    let mut content = column![].spacing(6);
    if !app.studies.is_empty() {
        content = content.push(
            row(app.studies.iter().map(|study| chip(study, t)))
                .spacing(6)
                .wrap(),
        );
    }
    if let Some(drawing) = selection.chart.selected_drawing() {
        content = content.push(drawing_bar(app, drawing));
    }
    if let Some(error) = &selection.error {
        content = content.push(text(error.message(t)).style(text::danger));
    }
    let chart: Element<'_, Message> = if series.candles().is_empty() && series.loading() {
        container(text(t.get("detail-loading")).style(text::secondary))
            .center(Fill)
            .into()
    } else {
        selection.chart.view().map(Message::Chart)
    };
    let content = content
        .push(chart)
        .push(text(t.get("chart-hint")).size(12).style(text::secondary));
    row![drawing_tools(app, selection), rule::vertical(1), content]
        .spacing(6)
        .into()
}

/// The drawing tools down the left side, as in TradingView, then switches for every
/// drawing of the symbol.
fn drawing_tools<'a>(app: &'a Fitil, selection: &'a Selection) -> Element<'a, Message> {
    let t = &app.translations;
    let chart = &selection.chart;
    let tool = |glyph: char, tip: String, on: bool, message: Message| -> Element<'a, Message> {
        let style = if on { button::primary } else { button::text };
        tooltip(
            button(icons::icon(glyph, 18.0))
                .style(style)
                .padding([5, 8])
                .on_press(message),
            hint(tip),
            tooltip::Position::Right,
        )
        .into()
    };

    let mut tools = Column::new().spacing(2).push(tool(
        icons::CURSOR,
        t.get("drawing-cursor"),
        chart.tool().is_none(),
        Message::ToolSelected(None),
    ));
    for kind in DrawingKind::ALL {
        // Lines, shapes and levels, and notes and measures, in groups.
        if matches!(kind, DrawingKind::Rectangle | DrawingKind::Text) {
            tools = tools.push(rule::horizontal(1));
        }
        tools = tools.push(tool(
            icons::for_tool(kind),
            t.get(&format!("drawing-{}", kind.id())),
            chart.tool() == Some(kind),
            Message::ToolSelected(Some(kind)),
        ));
    }

    let (lock_icon, lock_tip) = if chart.all_locked() {
        (icons::LOCK, "drawing-unlock-all")
    } else {
        (icons::UNLOCK, "drawing-lock-all")
    };
    let (eye_icon, eye_tip) = if chart.drawings_hidden() {
        (icons::EYE_OFF, "drawing-show-all")
    } else {
        (icons::EYE, "drawing-hide-all")
    };
    let clear_tip = if app.confirm_clear {
        "drawing-clear-confirm"
    } else {
        "drawing-clear-all"
    };
    let clear_style = if app.confirm_clear {
        button::danger
    } else {
        button::text
    };
    tools = tools
        .push(rule::horizontal(1))
        .push(tool(
            icons::MAGNET,
            t.get("drawing-magnet"),
            chart.magnet(),
            Message::MagnetToggled,
        ))
        .push(tool(
            lock_icon,
            t.get(lock_tip),
            chart.all_locked(),
            Message::DrawingsLockToggled,
        ))
        .push(tool(
            eye_icon,
            t.get(eye_tip),
            chart.drawings_hidden(),
            Message::DrawingsHiddenToggled,
        ))
        .push(tooltip(
            button(icons::icon(icons::TRASH, 18.0))
                .style(clear_style)
                .padding([5, 8])
                .on_press(Message::ClearDrawings),
            hint(t.get(clear_tip)),
            tooltip::Position::Right,
        ));
    // As narrow as its buttons; it scrolls only in a window too short for them.
    scrollable(tools.width(Shrink)).width(Shrink).into()
}

/// The look of the picked drawing, with buttons to lock, copy and delete it.
fn drawing_bar<'a>(app: &'a Fitil, drawing: &'a Drawing) -> Element<'a, Message> {
    let t = &app.translations;
    let mut bar = row![text(t.get(&format!("drawing-{}", drawing.kind.id()))).size(13)]
        .spacing(10)
        .align_y(iced::Center);
    // A measure takes its colors from its direction.
    if drawing.kind != DrawingKind::Measure {
        let lines: Vec<Choice<LineStyle>> = LineStyle::ALL
            .into_iter()
            .map(|style| Choice {
                value: style,
                name: t.get(&format!("drawing-line-{}", style.id())),
            })
            .collect();
        let current = lines
            .iter()
            .find(|choice| choice.value == drawing.line)
            .cloned();
        bar = bar
            .push(
                row(PALETTE.into_iter().map(|rgb| {
                    dialogs::swatch(rgb, rgb == drawing.color, Message::DrawingColor(rgb))
                }))
                .spacing(3),
            )
            .push(pick_list([1u8, 2, 3, 4], Some(drawing.width), Message::DrawingWidth).width(60))
            .push(
                pick_list(lines, current, |choice: Choice<LineStyle>| {
                    Message::DrawingLine(choice.value)
                })
                .width(110),
            );
    }
    match drawing.kind {
        DrawingKind::Text => {
            bar = bar.push(
                text_input(&t.get("drawing-text-placeholder"), &drawing.text)
                    .on_input(Message::DrawingText)
                    .width(220),
            );
        }
        DrawingKind::FibRetracement => {
            // The levels as typed, until they can be read.
            let typed = app
                .levels_text
                .as_ref()
                .filter(|(id, _)| *id == drawing.id)
                .map(|(_, text)| text.clone());
            let levels = typed.unwrap_or_else(|| {
                let levels: Vec<String> =
                    drawing.levels.iter().map(|l| studies::number(*l)).collect();
                levels.join(" ")
            });
            bar = bar.push(text(t.get("drawing-levels")).size(13)).push(
                text_input("0 0.5 1", &levels)
                    .on_input(Message::DrawingLevels)
                    .width(240),
            );
        }
        _ => {}
    }
    let action = |glyph: char, key: &str, message: Message| {
        tooltip(
            button(icons::icon(glyph, 15.0))
                .style(button::text)
                .padding([3, 6])
                .on_press(message),
            hint(t.get(key)),
            tooltip::Position::Bottom,
        )
    };
    let (lock_icon, lock_tip) = if drawing.locked {
        (icons::LOCK, "drawing-unlock")
    } else {
        (icons::UNLOCK, "drawing-lock")
    };
    bar = bar
        .push(action(lock_icon, lock_tip, Message::DrawingLockToggled))
        .push(action(
            icons::COPY,
            "drawing-copy",
            Message::DuplicateDrawing,
        ))
        .push(action(
            icons::TRASH,
            "drawing-delete",
            Message::DeleteDrawing,
        ));
    container(bar)
        .padding([4, 8])
        .style(container::bordered_box)
        .into()
}

/// Before the first chart opens: the first market loading or failing, or a way to search.
fn no_chart(app: &Fitil) -> Element<'_, Message> {
    let t = &app.translations;
    let (&market, state) = app
        .markets
        .iter()
        .next()
        .expect("every exchange has a market");
    let search = button(text(t.get("chart-symbol-search")))
        .on_press(Message::OpenSymbolSearch(String::new()));
    let content: Element<'_, Message> = match &state.load {
        Load::Loading => text(t.get("symbols-loading")).style(text::secondary).into(),
        Load::Failed(error) => column![
            text(error.message(t)).style(text::danger),
            row![
                button(text(t.get("symbols-retry"))).on_press(Message::RetryMarket(market)),
                search,
            ]
            .spacing(8),
        ]
        .spacing(12)
        .align_x(iced::Center)
        .into(),
        Load::Ready => column![
            text(t.get("chart-pick-symbol")).style(text::secondary),
            search,
        ]
        .spacing(12)
        .align_x(iced::Center)
        .into(),
    };
    container(content).center(Fill).into()
}

/// An indicator on the chart: its label opens its settings, then buttons to hide and
/// remove it.
fn chip<'a>(study: &'a Study, t: &Translations) -> Element<'a, Message> {
    let id = study.id();
    let settings = study.settings();
    let visible = settings.visible;
    let label_style = if visible {
        text::default
    } else {
        text::secondary
    };
    let small = |content: &'static str, message: Message, tip: String| {
        tooltip(
            button(text(content).size(13))
                .style(button::text)
                .padding([2, 6])
                .on_press(message),
            hint(tip),
            tooltip::Position::Bottom,
        )
    };
    let swatch = text("■")
        .size(13)
        .color(studies::color(settings.outputs[0].color));
    let name = button(
        row![
            swatch,
            text(studies::label(settings, t))
                .size(13)
                .style(label_style)
        ]
        .spacing(6)
        .align_y(iced::Center),
    )
    .style(button::text)
    .padding([2, 6])
    .on_press(Message::OpenStudySettings(id));
    let (symbol, tip) = if visible {
        ("●", t.get("study-hide"))
    } else {
        ("○", t.get("study-show"))
    };
    container(
        row![
            name,
            small(symbol, Message::ToggleStudy(id), tip),
            small("×", Message::RemoveStudy(id), t.get("study-remove")),
        ]
        .align_y(iced::Center),
    )
    .style(container::bordered_box)
    .into()
}

/// The chart market's connection and request budget, what the chart holds, and the
/// version.
fn status_bar(app: &Fitil) -> Element<'_, Message> {
    let t = &app.translations;
    let language = t.language();
    let market = app.chart_market();
    let state = &app.markets[&market];
    let exchange = app.exchange(market.exchange);

    let connection: Element<'_, Message> = match state.connection {
        Connection::Connecting => text(t.get("status-connecting")).into(),
        Connection::Connected => text(t.get("status-connected")).style(text::success).into(),
        Connection::Reconnecting { attempt, delay } => text(t.get_with(
            "status-reconnecting",
            &[
                ("seconds", delay.as_secs().to_string()),
                ("attempt", attempt.to_string()),
            ],
        ))
        .style(text::danger)
        .into(),
    };
    // Shown only for exchanges that report them.
    let usage = exchange.usage(market.kind).map(|usage| {
        t.get_with(
            "status-weight",
            &[
                ("used", format::count(u64::from(usage.used), language)),
                ("limit", format::count(u64::from(usage.limit), language)),
            ],
        )
    });
    let clock = exchange.clock_offset().map(|offset| {
        t.get_with(
            "status-clock",
            &[("seconds", format::seconds(offset, language))],
        )
    });
    let version = t.get_with(
        "app-version",
        &[("version", env!("CARGO_PKG_VERSION").to_owned())],
    );

    let mut bar = row![connection].spacing(24).padding([6, 12]);
    for item in [usage, clock].into_iter().flatten() {
        bar = bar.push(text(item));
    }
    bar = bar.push(space().width(Fill));
    if let Some(selection) = &app.selection {
        // How much of the chart is cached; where the cache is shows when hovered.
        let series = &selection.series;
        let mut held = t.get_with(
            "detail-cached",
            &[
                ("count", format::count(series.cached(), language)),
                ("timeframe", series.source().to_string()),
            ],
        );
        if series.source() != series.target() {
            held.push_str(" · ");
            held.push_str(&t.get_with("detail-source", &[("source", series.source().to_string())]));
        }
        if series.complete() {
            held.push_str(" · ");
            held.push_str(&t.get("detail-complete"));
        }
        let path = storage::cache_path()
            .map(|path| t.get_with("status-cache", &[("path", path.display().to_string())]))
            .unwrap_or_default();
        bar = bar.push(tooltip(
            text(held).style(text::secondary),
            hint(path),
            tooltip::Position::Top,
        ));
    }
    bar.push(text(version).style(text::secondary)).into()
}
