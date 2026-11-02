//! Windows shown over the main screen: the symbol search, the indicator list with the
//! templates, and one indicator's settings.

use std::collections::BTreeMap;

use fitil_core::MarketKind;
use fitil_indicators::{Group, PALETTE, ParamKind, Rgb, Source, catalog};
use fitil_market::{Match, StudyId, Template};
use iced::alignment::Horizontal;
use iced::widget::{
    Column, button, center, checkbox, column, container, mouse_area, opaque, pick_list, row, rule,
    scrollable, space, stack, text, text_input,
};
use iced::{Border, Color, Element, Fill, Theme};

use crate::view::Choice;
use crate::{Dialog, Fitil, Load, Message, SEARCH_RESULTS, format, market_key, studies};

/// Text boxes focused when their window opens, and the list of found symbols.
pub const SYMBOL_SEARCH_ID: &str = "symbol-search";
pub const SYMBOL_RESULTS_ID: &str = "symbol-results";
pub const INDICATOR_SEARCH_ID: &str = "indicator-search";

const DIALOG_WIDTH: f32 = 580.0;
const SEARCH_WIDTH: f32 = 760.0;
const DIALOG_HEIGHT: f32 = 640.0;

/// `content` over `base`, which is dimmed; a click beside the content sends `on_blur`.
pub fn modal<'a>(
    base: Element<'a, Message>,
    content: Element<'a, Message>,
    on_blur: Message,
) -> Element<'a, Message> {
    let backdrop = center(opaque(content)).style(|_theme| container::Style {
        background: Some(
            Color {
                a: 0.6,
                ..Color::BLACK
            }
            .into(),
        ),
        ..container::Style::default()
    });
    stack![base, opaque(mouse_area(backdrop).on_press(on_blur))].into()
}

pub fn view<'a>(app: &'a Fitil, dialog: &'a Dialog) -> Element<'a, Message> {
    match dialog {
        Dialog::Symbols {
            search,
            kind,
            highlight,
        } => symbols(app, search, *kind, *highlight),
        Dialog::Indicators {
            search,
            template_name,
            confirm_delete,
        } => indicators(app, search, template_name, confirm_delete.as_deref()),
        Dialog::Settings { study, drafts } => settings(app, *study, drafts),
        Dialog::Drawing(window) => crate::drawing_dialog::view(app, window),
    }
}

/// A dialog's box: its title with a close button, then `body`.
pub(crate) fn frame<'a>(
    title: String,
    width: f32,
    body: Element<'a, Message>,
) -> Element<'a, Message> {
    let header = row![
        text(title).size(18),
        space().width(Fill),
        button(text("×").size(18))
            .style(button::text)
            .padding([0, 8])
            .on_press(Message::CloseDialog),
    ]
    .align_y(iced::Center);
    container(column![header, rule::horizontal(1), body].spacing(12))
        .width(width)
        .max_height(DIALOG_HEIGHT)
        .padding(16)
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(palette.background.base.color.into()),
                border: Border {
                    color: palette.background.strong.color,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..container::Style::default()
            }
        })
        .into()
}

/// The symbols of every loaded market, found by name, with a tab per kind of market.
fn symbols<'a>(
    app: &'a Fitil,
    search: &'a str,
    kind: Option<MarketKind>,
    highlight: usize,
) -> Element<'a, Message> {
    let t = &app.translations;
    let search_box = text_input(&t.get("search-placeholder"), search)
        .id(SYMBOL_SEARCH_ID)
        .on_input(Message::SymbolSearchChanged)
        .on_submit(Message::SymbolSearchSubmit)
        .size(16)
        .padding(10);

    let mut kinds: Vec<MarketKind> = app.markets.keys().map(|market| market.kind).collect();
    kinds.sort();
    kinds.dedup();
    let tab = |name: String, value: Option<MarketKind>| {
        let style = if kind == value {
            button::primary
        } else {
            button::secondary
        };
        button(text(name).size(13))
            .style(style)
            .padding([4, 12])
            .on_press(Message::SymbolSearchKind(value))
    };
    let mut tabs = row![tab(t.get("search-all"), None)].spacing(6);
    for market_kind in kinds {
        tabs = tabs.push(tab(t.get(market_key(market_kind)), Some(market_kind)));
    }

    // Markets whose symbols are not here yet.
    let mut notes = Column::new().spacing(4);
    for (&market, state) in &app.markets {
        if kind.is_some_and(|kind| kind != market.kind) {
            continue;
        }
        let name = format!(
            "{} {}",
            app.exchange(market.exchange).name(),
            t.get(market_key(market.kind))
        );
        match &state.load {
            Load::Loading => {
                let loading = t.get_with("search-market-loading", &[("market", name)]);
                notes = notes.push(text(loading).size(12).style(text::secondary));
            }
            Load::Failed(error) => {
                notes = notes.push(
                    row![
                        text(format!("{name}: {}", error.message(t)))
                            .size(12)
                            .style(text::danger),
                        button(text(t.get("symbols-retry")).size(12))
                            .padding([2, 8])
                            .on_press(Message::RetryMarket(market)),
                    ]
                    .spacing(8)
                    .align_y(iced::Center),
                );
            }
            Load::Ready => {}
        }
    }

    let results = app.symbol_results(search, kind);
    let shown = results.len().min(SEARCH_RESULTS);
    let mut list = Column::new().spacing(2);
    for (index, found) in results.iter().take(SEARCH_RESULTS).enumerate() {
        list = list.push(symbol_row(app, *found, index == highlight));
    }
    if results.is_empty() {
        list = list.push(text(t.get("search-empty")).style(text::secondary));
    }

    let mut body = column![
        search_box,
        tabs,
        notes,
        scrollable(list)
            .id(SYMBOL_RESULTS_ID)
            .spacing(6)
            .height(Fill),
    ]
    .spacing(12);
    if results.len() > shown {
        let language = t.language();
        let more = t.get_with(
            "search-more",
            &[
                ("shown", format::count(shown as u64, language)),
                ("total", format::count(results.len() as u64, language)),
            ],
        );
        body = body.push(text(more).size(12).style(text::secondary));
    }
    frame(t.get("search-title"), SEARCH_WIDTH, body.into())
}

/// One found symbol: its name, coins and market on the left, price and daily change,
/// then exchange on the right.
fn symbol_row<'a>(app: &'a Fitil, found: Match<'a>, picked: bool) -> Element<'a, Message> {
    let t = &app.translations;
    let symbol = found.symbol;
    let change = found.ticker.change_percent();
    let change_style = if change >= 0.0 {
        text::success
    } else {
        text::danger
    };
    let tag = container(text(t.get(market_tag_key(symbol.market.kind))).size(10))
        .padding([1, 5])
        .style(container::bordered_box);
    let content = row![
        column![
            text(&symbol.name).size(15),
            row![
                text(format!("{} / {}", symbol.base_asset, symbol.quote_asset))
                    .size(12)
                    .style(text::secondary),
                tag,
            ]
            .spacing(8)
            .align_y(iced::Center),
        ]
        .spacing(2)
        .width(Fill),
        column![
            text(format::price(
                found.ticker.last_price,
                symbol.price_decimals
            ))
            .size(14),
            text(format::percent(change)).size(12).style(change_style),
        ]
        .align_x(Horizontal::Right)
        .width(130),
        text(app.exchange(symbol.market.exchange).name())
            .size(12)
            .style(text::secondary)
            .width(80)
            .align_x(Horizontal::Right),
    ]
    .spacing(16)
    .align_y(iced::Center);
    button(content)
        .style(move |theme: &Theme, status| {
            if picked {
                picked_row(theme)
            } else {
                button::text(theme, status)
            }
        })
        .width(Fill)
        .padding([6, 10])
        .on_press(Message::SymbolSelected(symbol.market, symbol.name.clone()))
        .into()
}

/// The result the arrow keys picked, which Enter opens.
fn picked_row(theme: &Theme) -> button::Style {
    let palette = theme.extended_palette();
    button::Style {
        background: Some(palette.background.strong.color.into()),
        text_color: palette.background.base.text,
        border: Border {
            radius: 4.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// The Fluent key of a market's short tag in the results, like `SPOT`.
fn market_tag_key(kind: MarketKind) -> &'static str {
    match kind {
        MarketKind::Spot => "market-tag-spot",
        MarketKind::LinearPerpetual => "market-tag-futures",
    }
}

/// Every indicator the exchange can feed, by group, and the saved templates.
fn indicators<'a>(
    app: &'a Fitil,
    search: &'a str,
    template_name: &'a str,
    confirm_delete: Option<&'a str>,
) -> Element<'a, Message> {
    let t = &app.translations;
    let features = app.features();
    let search_box = text_input(&t.get("indicators-search"), search)
        .id(INDICATOR_SEARCH_ID)
        .on_input(Message::IndicatorSearchChanged)
        .padding(8);

    let mut list = Column::new().spacing(2);
    let mut found = false;
    for group in Group::ALL {
        let mut items = catalog()
            .iter()
            .filter(|definition| definition.group == group)
            .filter(|definition| fitil_market::is_available(definition, features))
            .filter(|definition| studies::matches(definition, search, t))
            .peekable();
        if items.peek().is_none() {
            continue;
        }
        found = true;
        list = list.push(
            container(
                text(studies::group_name(group, t))
                    .size(13)
                    .style(text::secondary),
            )
            .padding(iced::padding::top(8).left(8)),
        );
        for definition in items {
            // How many are on the chart already, so a click visibly adds one.
            let on_chart = app
                .studies
                .iter()
                .filter(|study| study.definition().id == definition.id)
                .count();
            let mut item = row![text(studies::name(definition, t)).width(Fill)];
            if on_chart > 0 {
                item = item.push(text(on_chart.to_string()).style(text::success));
            }
            list = list.push(
                button(item)
                    .style(button::text)
                    .width(Fill)
                    .padding([4, 8])
                    .on_press(Message::AddIndicator(definition.id)),
            );
        }
    }
    if !found {
        list = list.push(text(t.get("indicators-empty")).style(text::secondary));
    }

    let can_save = Template::clean_name(template_name).is_some() && !app.studies.is_empty();
    let mut templates = column![
        text(t.get("templates-title")).size(15),
        text(t.get("templates-hint"))
            .size(12)
            .style(text::secondary),
        row![
            text_input(&t.get("templates-name"), template_name)
                .on_input(Message::TemplateNameChanged)
                .on_submit(Message::SaveTemplate)
                .padding(6),
            button(text(t.get("templates-save")))
                .on_press_maybe(can_save.then_some(Message::SaveTemplate)),
        ]
        .spacing(8),
    ]
    .spacing(8);
    if app.templates.is_empty() {
        templates = templates.push(
            text(t.get("templates-empty"))
                .size(13)
                .style(text::secondary),
        );
    }
    let mut saved = Column::new().spacing(6);
    for template in &app.templates {
        let contents: Vec<String> = template
            .studies
            .iter()
            .map(|settings| studies::label(settings, t))
            .collect();
        let delete = if confirm_delete == Some(template.name.as_str()) {
            button(text(t.get("templates-confirm-delete")))
                .style(button::danger)
                .on_press(Message::DeleteTemplate(template.name.clone()))
        } else {
            button(text(t.get("templates-delete")))
                .style(button::secondary)
                .on_press(Message::ConfirmDeleteTemplate(template.name.clone()))
        };
        saved = saved.push(
            row![
                column![
                    text(&template.name),
                    text(contents.join(", ")).size(12).style(text::secondary),
                ]
                .width(Fill),
                button(text(t.get("templates-apply")))
                    .on_press(Message::ApplyTemplate(template.name.clone())),
                delete,
            ]
            .spacing(8)
            .align_y(iced::Center),
        );
    }
    templates = templates.push(scrollable(saved).spacing(6).height(iced::Length::Shrink));
    if let Some(error) = &app.templates_error {
        templates = templates.push(text(error.message(t)).style(text::danger));
    }

    let body = column![
        search_box,
        scrollable(list).spacing(6).height(Fill),
        rule::horizontal(1),
        templates,
    ]
    .spacing(12);
    frame(t.get("indicators-title"), DIALOG_WIDTH, body.into())
}

/// One indicator's parameters and the look of each of its lines.
fn settings<'a>(
    app: &'a Fitil,
    id: StudyId,
    drafts: &'a BTreeMap<&'static str, String>,
) -> Element<'a, Message> {
    let t = &app.translations;
    let Some(study) = app.studies.get(id) else {
        return frame(String::new(), DIALOG_WIDTH, space().into());
    };
    let definition = study.definition();
    let current = study.settings();

    let mut inputs = column![text(t.get("study-inputs")).size(15)].spacing(8);
    for param in definition.params {
        let control: Element<'a, Message> = match param.kind {
            ParamKind::Integer { .. } | ParamKind::Float { .. } => {
                let shown = drafts
                    .get(param.id)
                    .cloned()
                    .unwrap_or_else(|| studies::number(current.params.number(param.id)));
                text_input("", &shown)
                    .on_input(move |typed| Message::StudyNumberTyped {
                        study: id,
                        param: param.id,
                        text: typed,
                    })
                    .width(120)
                    .into()
            }
            ParamKind::Source { .. } => {
                let choices: Vec<Choice<Source>> = Source::ALL
                    .into_iter()
                    .map(|source| Choice {
                        value: source,
                        name: studies::source_name(source, t),
                    })
                    .collect();
                let chosen = current.params.source(param.id);
                let selected = choices.iter().find(|c| c.value == chosen).cloned();
                pick_list(choices, selected, move |choice: Choice<Source>| {
                    Message::StudyTextChosen {
                        study: id,
                        param: param.id,
                        value: choice.value.id().to_owned(),
                    }
                })
                .width(200)
                .into()
            }
            ParamKind::Choice { options, .. } => {
                let choices: Vec<Choice<&'static str>> = options
                    .iter()
                    .map(|&option| Choice {
                        value: option,
                        name: studies::choice_name(option, t),
                    })
                    .collect();
                let chosen = current.params.text(param.id);
                let selected = choices.iter().find(|c| c.value == chosen).cloned();
                pick_list(choices, selected, move |choice: Choice<&'static str>| {
                    Message::StudyTextChosen {
                        study: id,
                        param: param.id,
                        value: choice.value.to_owned(),
                    }
                })
                .width(200)
                .into()
            }
        };
        inputs = inputs.push(
            row![text(studies::param_name(param.id, t)).width(220), control]
                .spacing(12)
                .align_y(iced::Center),
        );
    }

    let mut looks = column![text(t.get("study-style")).size(15)].spacing(10);
    for (index, (output, look)) in definition.outputs.iter().zip(&current.outputs).enumerate() {
        // A color of the indicator's own that the palette lacks is offered first.
        let mut colors = PALETTE.to_vec();
        if !colors.contains(&look.color) {
            colors.insert(0, look.color);
        }
        let swatches = row(colors.into_iter().map(|rgb| {
            swatch(
                rgb,
                rgb == look.color,
                Message::StudyOutputColor {
                    study: id,
                    output: index,
                    color: rgb,
                },
            )
        }))
        .spacing(4);
        let widths = pick_list([1u8, 2, 3, 4], Some(look.width), move |width| {
            Message::StudyOutputWidth {
                study: id,
                output: index,
                width,
            }
        })
        .width(60);
        let shown = checkbox(look.visible)
            .label(t.get("study-show"))
            .on_toggle(move |visible| Message::StudyOutputVisible {
                study: id,
                output: index,
                visible,
            });
        looks = looks.push(
            column![
                text(studies::output_name(output.id, t)).size(13),
                row![swatches, text(t.get("study-width")).size(13), widths, shown]
                    .spacing(10)
                    .align_y(iced::Center),
            ]
            .spacing(4),
        );
    }

    let buttons = row![
        button(text(t.get("study-defaults")))
            .style(button::secondary)
            .on_press(Message::StudyDefaults(id)),
        space().width(Fill),
        button(text(t.get("study-done"))).on_press(Message::CloseDialog),
    ];
    let title = t.get_with(
        "study-settings-title",
        &[("name", studies::label(current, t))],
    );
    let body = column![
        scrollable(column![inputs, looks].spacing(20))
            .spacing(6)
            .height(Fill),
        buttons,
    ]
    .spacing(16);
    frame(title, DIALOG_WIDTH, body.into())
}

/// A small square of `rgb` to click; outlined when it is the chosen color.
pub fn swatch<'a>(rgb: Rgb, chosen: bool, message: Message) -> Element<'a, Message> {
    let fill = studies::color(rgb);
    button(space())
        .width(20)
        .height(20)
        .padding(0)
        .style(move |theme: &Theme, status| {
            let text = theme.extended_palette().background.base.text;
            let outline = if chosen {
                text
            } else if status == button::Status::Hovered {
                text.scale_alpha(0.5)
            } else {
                Color::TRANSPARENT
            };
            button::Style {
                background: Some(fill.into()),
                border: Border {
                    color: outline,
                    width: 2.0,
                    radius: 4.0.into(),
                },
                ..button::Style::default()
            }
        })
        .on_press(message)
        .into()
}
