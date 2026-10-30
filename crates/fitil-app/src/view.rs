//! What the window shows: a watchlist on the left, the selected symbol on the right
//! and a status bar at the bottom.

use std::collections::HashMap;

use fitil_core::{Market, Ticker, Timeframe, aggregate};
use iced::alignment::Horizontal;
use iced::widget::{
    Column, button, column, container, pick_list, row, rule, scrollable, space, text, text_input,
};
use iced::{Element, Fill, FillPortion, Length};

use crate::i18n::Language;
use crate::{Connection, Fitil, Load, Message, Selection, TIMEFRAME_CHOICES, data, format};

/// Rows shown in the watchlist; the rest are reachable by searching.
const WATCHLIST_ROWS: usize = 100;

const PRICE_WIDTH: f32 = 130.0;
const CHANGE_WIDTH: f32 = 100.0;

pub fn view(app: &Fitil) -> Element<'_, Message> {
    column![
        top_bar(app),
        rule::horizontal(1),
        row![
            container(watchlist(app)).width(FillPortion(2)).height(Fill),
            rule::vertical(1),
            container(details(app))
                .width(FillPortion(3))
                .height(Fill)
                .padding(20),
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
        button(row![
            text(&ticker.symbol).width(Fill),
            text(format::price(ticker.last_price, decimals))
                .width(PRICE_WIDTH)
                .align_x(Horizontal::Right),
            text(format::percent(change))
                .width(CHANGE_WIDTH)
                .align_x(Horizontal::Right)
                .style(change_style),
        ])
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

fn details(app: &Fitil) -> Element<'_, Message> {
    let t = &app.translations;
    let Some(selection) = &app.selection else {
        return text(t.get("detail-empty")).style(text::secondary).into();
    };
    let language = t.language();
    let market_key = match selection.symbol.market {
        Market::Spot => "market-spot",
        Market::UsdMFutures => "market-futures",
    };

    let natives = fitil_exchange::binance::native_timeframes(selection.symbol.market);
    let choices: Vec<Timeframe> = TIMEFRAME_CHOICES
        .iter()
        .filter_map(|choice| choice.parse::<Timeframe>().ok())
        .filter(|timeframe| timeframe.best_source(natives).is_some())
        .collect();

    let mut timeframe_row = row![
        text(t.get("detail-timeframe")),
        pick_list(
            choices,
            Some(selection.timeframe),
            Message::TimeframeSelected
        )
        .width(110),
    ]
    .spacing(10)
    .align_y(iced::Center);
    if selection.source != selection.timeframe {
        timeframe_row = timeframe_row.push(
            text(t.get_with("detail-source", &[("source", selection.source.to_string())]))
                .style(text::secondary),
        );
    }

    let mut content = column![
        row![
            text(&selection.symbol.name).size(28),
            text(t.get(market_key)).size(16).style(text::secondary),
        ]
        .spacing(12)
        .align_y(iced::Center),
        timeframe_row,
        rule::horizontal(1),
        open_candle(app, selection),
        rule::horizontal(1),
    ]
    .spacing(16);

    if let Some(loaded) = &selection.loaded {
        let mut cache = column![text(t.get_with(
            "detail-cached",
            &[
                (
                    "count",
                    format::count(loaded.range.map_or(0, |r| r.count), language)
                ),
                ("timeframe", selection.source.to_string()),
            ],
        ))]
        .spacing(6);
        if selection.source != selection.timeframe {
            cache = cache.push(text(t.get_with(
                "detail-built",
                &[
                    ("count", format::count(loaded.target_count, language)),
                    ("timeframe", selection.timeframe.to_string()),
                ],
            )));
        }
        if let Some(range) = loaded.range {
            cache = cache
                .push(text(t.get_with(
                    "detail-oldest",
                    &[("time", format::date_time(range.oldest))],
                )))
                .push(text(t.get_with(
                    "detail-newest",
                    &[("time", format::date_time(range.newest))],
                )));
        }
        cache = cache.push(
            text(t.get_with(
                "detail-last-load",
                &[
                    ("cached", format::count(loaded.from_cache, language)),
                    ("downloaded", format::count(loaded.downloaded, language)),
                ],
            ))
            .style(text::secondary),
        );
        if loaded.complete {
            cache = cache.push(text(t.get("detail-complete")).style(text::success));
        }
        content = content.push(cache);
    }

    let older = if selection.loading {
        button(text(t.get("detail-loading")))
    } else if selection
        .loaded
        .as_ref()
        .is_some_and(|loaded| loaded.complete)
    {
        button(text(t.get("detail-load-older")))
    } else {
        button(text(t.get("detail-load-older"))).on_press(Message::LoadOlder)
    };
    content = content.push(older);

    if let Some(error) = &selection.error {
        content = content.push(text(error.message(t)).style(text::danger));
    }

    scrollable(content).height(Fill).into()
}

/// The displayed timeframe's current candle, built from the cached closed candles
/// inside it plus the live one.
fn open_candle<'a>(app: &'a Fitil, selection: &'a Selection) -> Element<'a, Message> {
    let t = &app.translations;
    let now = app.clients[&selection.symbol.market].now();
    let bucket = selection.timeframe.bucket_start(now);

    let mut parts: Vec<_> = selection
        .loaded
        .iter()
        .flat_map(|loaded| loaded.recent.iter().copied())
        .filter(|candle| candle.open_time >= bucket)
        .collect();
    if let Some(live) = selection.live
        && live.open_time >= bucket
        && parts
            .last()
            .is_none_or(|last| last.open_time < live.open_time)
    {
        parts.push(live);
    }
    let current = aggregate(&parts, selection.timeframe).pop();

    let closes_in = t.get_with(
        "detail-closes-in",
        &[(
            "time",
            format::countdown(selection.timeframe.next_bucket(now) - now, t.language()),
        )],
    );
    let candle_text: Element<'_, Message> = match current {
        Some(candle) => {
            let decimals = selection.symbol.price_decimals;
            let style = if candle.close >= candle.open {
                text::success
            } else {
                text::danger
            };
            text(t.get_with(
                "detail-ohlc",
                &[
                    ("open", format::price(candle.open, decimals)),
                    ("high", format::price(candle.high, decimals)),
                    ("low", format::price(candle.low, decimals)),
                    ("close", format::price(candle.close, decimals)),
                ],
            ))
            .size(18)
            .style(style)
            .into()
        }
        _ => text(t.get("detail-waiting")).style(text::secondary).into(),
    };

    column![
        text(t.get("detail-open-candle"))
            .size(14)
            .style(text::secondary),
        candle_text,
        text(closes_in),
    ]
    .spacing(6)
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
