//! What the window shows: a watchlist on the left, the selected symbol on the right
//! and a status bar at the bottom.

use std::collections::HashMap;

use fitil_chart::{ChartKind, PriceScale};
use fitil_core::{Market, Ticker, Timeframe};
use iced::alignment::Horizontal;
use iced::widget::{
    Column, button, column, container, pick_list, row, rule, scrollable, space, text, text_input,
};
use iced::{Element, Fill, Length};

use crate::i18n::Language;
use crate::{Connection, Fitil, Load, Message, TIMEFRAME_CHOICES, data, format};

/// Rows shown in the watchlist; the rest are reachable by searching.
const WATCHLIST_ROWS: usize = 100;

const WATCHLIST_WIDTH: f32 = 400.0;
const PRICE_WIDTH: f32 = 110.0;
const CHANGE_WIDTH: f32 = 90.0;
const COLUMN_GAP: f32 = 12.0;

pub fn view(app: &Fitil) -> Element<'_, Message> {
    column![
        top_bar(app),
        rule::horizontal(1),
        row![
            container(watchlist(app))
                .width(WATCHLIST_WIDTH)
                .height(Fill),
            rule::vertical(1),
            container(details(app)).width(Fill).height(Fill).padding(10),
        ]
        .height(Fill),
        rule::horizontal(1),
        status_bar(app),
    ]
    .into()
}

fn top_bar(app: &Fitil) -> Element<'_, Message> {
    let t = &app.translations;
    let market_button = |market: Market, key: &str| {
        let style = if app.market == market {
            button::primary
        } else {
            button::secondary
        };
        button(text(t.get(key)))
            .style(style)
            .on_press(Message::MarketSelected(market))
    };
    let language_button = |language: Language| {
        let style = if t.language() == language {
            button::primary
        } else {
            button::secondary
        };
        button(text(t.get(language.name_key())).size(14))
            .style(style)
            .on_press(Message::LanguageSelected(language))
    };

    row![
        text(t.get("app-title")).size(22),
        space().width(12),
        market_button(Market::Spot, "market-spot"),
        market_button(Market::UsdMFutures, "market-futures"),
        text_input(&t.get("search-placeholder"), &app.search)
            .on_input(Message::SearchChanged)
            .width(260),
        space().width(Fill),
        row(Language::ALL.map(|language| language_button(language).into())).spacing(6),
    ]
    .spacing(10)
    .padding(12)
    .align_y(iced::Center)
    .into()
}

fn watchlist(app: &Fitil) -> Element<'_, Message> {
    let t = &app.translations;
    let state = &app.markets[&app.market];
    match &state.load {
        Load::Loading => return padded(text(t.get("watchlist-loading"))),
        Load::Failed(error) => {
            return padded(
                column![
                    text(error.message(t)).style(text::danger),
                    button(text(t.get("watchlist-retry")))
                        .on_press(Message::RetryMarket(app.market)),
                ]
                .spacing(12),
            );
        }
        Load::Ready => {}
    }

    let search = app.search.trim().to_uppercase();
    let tickers: Vec<_> = state
        .tickers
        .values()
        .filter(|ticker| ticker.symbol.contains(&search))
        .collect();
    // Pairs are quoted in many currencies (USDT, TRY, IDR, BTC...), so volumes are
    // compared in dollars.
    let mut rates: HashMap<&str, f64> = HashMap::new();
    let mut usd_volume = |ticker: &Ticker| {
        let quote = state
            .symbols
            .get(&ticker.symbol)
            .map_or("", |symbol| symbol.quote_asset.as_str());
        let rate = *rates
            .entry(quote)
            .or_insert_with(|| usd_rate(quote, &state.tickers).unwrap_or(0.0));
        ticker.quote_volume * rate
    };
    let mut ranked: Vec<_> = tickers
        .into_iter()
        .map(|ticker| (usd_volume(ticker), ticker))
        .collect();
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut tickers: Vec<_> = ranked.into_iter().map(|(_, ticker)| ticker).collect();
    let total = tickers.len();
    tickers.truncate(WATCHLIST_ROWS);

    let selected = app
        .selection
        .as_ref()
        .filter(|selection| selection.symbol.market == app.market)
        .map(|selection| selection.symbol.name.as_str());

    let header = row![
        text(t.get("watchlist-symbol")).width(Fill),
        text(t.get("watchlist-price"))
            .width(PRICE_WIDTH)
            .align_x(Horizontal::Right),
        text(t.get("watchlist-change"))
            .width(CHANGE_WIDTH)
            .align_x(Horizontal::Right),
    ]
    .spacing(COLUMN_GAP)
    .padding([8, 16]);

    let rows = Column::with_children(tickers.iter().map(|ticker| {
        let decimals = state
            .symbols
            .get(&ticker.symbol)
            .map_or(2, |symbol| symbol.price_decimals);
        let change = ticker.change_percent();
        let change_style = if change >= 0.0 {
            text::success
        } else {
            text::danger
        };
        let style = if selected == Some(ticker.symbol.as_str()) {
            button::primary
        } else {
            button::text
        };
        button(
            row![
                text(&ticker.symbol).width(Fill),
                text(format::price(ticker.last_price, decimals))
                    .width(PRICE_WIDTH)
                    .align_x(Horizontal::Right),
                text(format::percent(change))
                    .width(CHANGE_WIDTH)
                    .align_x(Horizontal::Right)
                    .style(change_style),
            ]
            .spacing(COLUMN_GAP),
        )
        .style(style)
        .width(Fill)
        .padding([4, 8])
        .on_press(Message::SymbolSelected(app.market, ticker.symbol.clone()))
        .into()
    }))
    .padding([0, 8]);

    let footer = if total == 0 {
        t.get("watchlist-empty")
    } else {
        let language = t.language();
        t.get_with(
            "watchlist-count",
            &[
                (
                    "shown",
                    format::count(total.min(WATCHLIST_ROWS) as u64, language),
                ),
                ("total", format::count(total as u64, language)),
            ],
        )
    };

    column![
        header,
        scrollable(rows).height(Fill),
        container(text(footer).size(13).style(text::secondary)).padding([0, 16]),
    ]
    .spacing(6)
    .padding(iced::padding::bottom(10))
    .into()
}

/// A chart type in the picker, shown with its translated name.
#[derive(Debug, Clone, PartialEq)]
struct KindChoice {
    kind: ChartKind,
    name: String,
}

impl std::fmt::Display for KindChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
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

/// The chart of the selected symbol, with its toolbar.
fn details(app: &Fitil) -> Element<'_, Message> {
    let t = &app.translations;
    let Some(selection) = &app.selection else {
        return padded(text(t.get("detail-empty")).style(text::secondary));
    };

    let natives = fitil_exchange::binance::native_timeframes(selection.symbol.market);
    let timeframes: Vec<Timeframe> = TIMEFRAME_CHOICES
        .iter()
        .filter_map(|choice| choice.parse::<Timeframe>().ok())
        .filter(|timeframe| timeframe.best_source(natives).is_some())
        .collect();
    let kinds: Vec<KindChoice> = ChartKind::ALL
        .into_iter()
        .map(|kind| KindChoice {
            kind,
            name: t.get(kind_key(kind)),
        })
        .collect();
    let current_kind = kinds
        .iter()
        .find(|choice| choice.kind == app.chart_kind)
        .cloned();
    let scale_button = |scale: PriceScale, key: &str| {
        let style = if app.price_scale == scale {
            button::primary
        } else {
            button::secondary
        };
        button(text(t.get(key)).size(14))
            .style(style)
            .on_press(Message::PriceScaleToggled(scale))
    };

    let mut info = t.get_with(
        "detail-cached",
        &[
            ("count", format::count(selection.cached, t.language())),
            ("timeframe", selection.source.to_string()),
        ],
    );
    if selection.source != selection.timeframe {
        info.push_str(" · ");
        info.push_str(&t.get_with("detail-source", &[("source", selection.source.to_string())]));
    }
    if selection.complete {
        info.push_str(" · ");
        info.push_str(&t.get("detail-complete"));
    }

    let toolbar = row![
        pick_list(
            timeframes,
            Some(selection.timeframe),
            Message::TimeframeSelected
        )
        .width(90),
        pick_list(kinds, current_kind, |choice| Message::ChartKindSelected(
            choice.kind
        ))
        .width(150),
        scale_button(PriceScale::Logarithmic, "scale-log"),
        scale_button(PriceScale::Percent, "scale-percent"),
        space().width(Fill),
        text(info).size(13).style(text::secondary),
    ]
    .spacing(8)
    .align_y(iced::Center);

    let mut content = column![toolbar].spacing(8);
    if let Some(error) = &selection.error {
        content = content.push(text(error.message(t)).style(text::danger));
    }
    let chart: Element<'_, Message> = if selection.candles.is_empty() && selection.loading {
        container(text(t.get("detail-loading")).style(text::secondary))
            .center(Fill)
            .into()
    } else {
        selection.chart.view().map(Message::Chart)
    };
    content
        .push(chart)
        .push(text(t.get("chart-hint")).size(12).style(text::secondary))
        .into()
}

fn status_bar(app: &Fitil) -> Element<'_, Message> {
    let t = &app.translations;
    let language = t.language();
    let state = &app.markets[&app.market];
    let client = &app.clients[&app.market];

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
    let (used, limit) = client.weight_usage();
    let weight = t.get_with(
        "status-weight",
        &[
            ("used", format::count(u64::from(used), language)),
            ("limit", format::count(u64::from(limit), language)),
        ],
    );
    let clock = t.get_with(
        "status-clock",
        &[("seconds", format::seconds(client.clock_offset(), language))],
    );
    let cache = data::cache_path()
        .map(|path| t.get_with("status-cache", &[("path", path.display().to_string())]))
        .unwrap_or_default();
    let version = t.get_with(
        "app-version",
        &[("version", env!("CARGO_PKG_VERSION").to_owned())],
    );

    row![
        connection,
        text(weight),
        text(clock),
        space().width(Fill),
        text(cache).style(text::secondary),
        text(version).style(text::secondary),
    ]
    .spacing(24)
    .padding([6, 12])
    .into()
}

/// Dollars per unit of `asset`, from the market's own `ASSETUSDT` or `USDTASSET` price.
fn usd_rate(asset: &str, tickers: &HashMap<String, Ticker>) -> Option<f64> {
    const DOLLARS: [&str; 6] = ["USDT", "USDC", "FDUSD", "TUSD", "USD1", "DAI"];
    if DOLLARS.contains(&asset) {
        return Some(1.0);
    }
    let price = |name: String| {
        tickers
            .get(&name)
            .map(|ticker| ticker.last_price)
            .filter(|price| *price > 0.0)
    };
    price(format!("{asset}USDT")).or_else(|| price(format!("USDT{asset}")).map(|p| 1.0 / p))
}

fn padded<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content).padding(20).width(Length::Fill).into()
}
