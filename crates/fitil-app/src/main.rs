//! Fitil desktop application entry point.

// Release builds on Windows should not open a console window next to the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod data;
mod error;
mod format;
mod i18n;
mod view;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use data::{Loaded, MarketSnapshot, SharedStore};
use error::AppError;
use fitil_chart::{Chart, ChartKind, ChartLabels, ChartMessage, PriceScale};
use fitil_core::{Candle, Market, SeriesKey, Symbol, Ticker, Timeframe, aggregate_whole};
use fitil_exchange::binance::{self, BinanceClient, FeedCommand, FeedEvent, Topic};
use futures::channel::mpsc;
use futures::{SinkExt, Stream, StreamExt};
use i18n::{Language, Translations};
use iced::{Element, Subscription, Task, Theme};

fn main() -> iced::Result {
    iced::application(Fitil::new, Fitil::update, Fitil::view)
        .title(Fitil::title)
        .theme(Fitil::theme)
        .subscription(Fitil::subscription)
        .window_size((1280.0, 800.0))
        .centered()
        .run()
}

/// Timeframe shown when a symbol is first opened.
const DEFAULT_TIMEFRAME: &str = "1h";

/// Timeframes offered in the picker; ones Binance lacks are built from shorter candles.
const TIMEFRAME_CHOICES: [&str; 21] = [
    "1s", "1m", "3m", "5m", "15m", "30m", "45m", "1h", "2h", "3h", "4h", "6h", "8h", "12h", "1d",
    "2d", "3d", "1w", "2w", "1M", "3M",
];

struct Fitil {
    translations: Translations,
    store: Result<SharedStore, AppError>,
    clients: HashMap<Market, Arc<BinanceClient>>,
    markets: HashMap<Market, MarketState>,
    /// The market whose symbols the watchlist shows.
    market: Market,
    search: String,
    selection: Option<Selection>,
    /// Chart type and price scale, kept when another symbol is opened.
    chart_kind: ChartKind,
    price_scale: PriceScale,
    /// A chart to open once its market has loaded, from `--open` on the command line.
    pending_open: Option<OpenRequest>,
}

/// A chart named on the command line: `--open spot:BTCUSDT:1h` or `--open futures:ETHUSDT:4h`.
#[derive(Debug, Clone, PartialEq)]
struct OpenRequest {
    market: Market,
    symbol: String,
    timeframe: Timeframe,
}

impl OpenRequest {
    fn from_args(mut args: impl Iterator<Item = String>) -> Option<Self> {
        args.find(|arg| arg == "--open")?;
        let value = args.next()?;
        let mut parts = value.split(':');
        let market = match parts.next()? {
            "spot" => Market::Spot,
            "futures" => Market::UsdMFutures,
            _ => return None,
        };
        let symbol = parts.next()?.to_uppercase();
        let timeframe = match parts.next() {
            Some(timeframe) => timeframe.parse().ok()?,
            None => DEFAULT_TIMEFRAME.parse().ok()?,
        };
        Some(Self {
            market,
            symbol,
            timeframe,
        })
    }
}

struct MarketState {
    load: Load,
    symbols: HashMap<String, Symbol>,
    tickers: HashMap<String, Ticker>,
    /// Sends subscription changes to the live feed once it is running.
    feed: Option<mpsc::UnboundedSender<FeedCommand>>,
    connection: Connection,
}

enum Load {
    Loading,
    Ready,
    Failed(AppError),
}

#[derive(Debug, Clone, Copy)]
enum Connection {
    Connecting,
    Connected,
    Reconnecting { attempt: u32, delay: Duration },
}

/// The symbol on the chart.
struct Selection {
    symbol: Symbol,
    /// The timeframe the user picked.
    timeframe: Timeframe,
    /// The timeframe downloaded from Binance to build `timeframe`.
    source: Timeframe,
    /// Loading the newest candles: on open, after a reconnect or after missed candles.
    loading: bool,
    loading_older: bool,
    /// Closed `source` candles in memory, oldest first, without gaps.
    candles: Vec<Candle>,
    /// The open `source` candle.
    live: Option<Candle>,
    /// Whether `candles` starts at the symbol's first candle.
    complete: bool,
    /// Candles of this series in the cache.
    cached: u64,
    error: Option<AppError>,
    chart: Chart,
}

impl Selection {
    fn key(&self) -> SeriesKey {
        SeriesKey {
            market: self.symbol.market,
            symbol: self.symbol.name.clone(),
            timeframe: self.source,
        }
    }

    fn topic(&self) -> Topic {
        Topic::Candles {
            symbol: self.symbol.name.clone(),
            timeframe: self.source,
        }
    }

    /// Rebuilds the chart's candles from the closed ones and the open one.
    fn refresh_chart(&mut self) {
        let mut source = self.candles.clone();
        if let Some(live) = self.live
            && source
                .last()
                .is_none_or(|last| last.open_time < live.open_time)
        {
            source.push(live);
        }
        let shown = if self.timeframe == self.source {
            source
        } else {
            aggregate_whole(&source, self.timeframe)
        };
        self.chart.set_candles(shown);
        self.chart.set_history_complete(self.complete);
        self.chart.set_loading_older(self.loading_older);
    }
}

#[derive(Debug, Clone)]
enum Message {
    LanguageSelected(Language),
    MarketSelected(Market),
    SearchChanged(String),
    MarketLoaded(Market, Result<MarketSnapshot, AppError>),
    RetryMarket(Market),
    FeedReady(Market, mpsc::UnboundedSender<FeedCommand>),
    Feed(Market, FeedEvent),
    SymbolSelected(Market, String),
    TimeframeSelected(Timeframe),
    ChartKindSelected(ChartKind),
    PriceScaleToggled(PriceScale),
    Chart(ChartMessage),
    SeriesLoaded(SeriesKey, Result<Loaded, AppError>),
    OlderLoaded(SeriesKey, Result<Loaded, AppError>),
    CandleSaved(SeriesKey, Result<(), AppError>),
    Tick,
}

impl Fitil {
    fn new() -> (Self, Task<Message>) {
        let clients: HashMap<_, _> = Market::ALL
            .into_iter()
            .map(|market| {
                let client = BinanceClient::new(market).expect("the HTTP client can be created");
                (market, Arc::new(client))
            })
            .collect();
        let markets = Market::ALL
            .into_iter()
            .map(|market| {
                let state = MarketState {
                    load: Load::Loading,
                    symbols: HashMap::new(),
                    tickers: HashMap::new(),
                    feed: None,
                    connection: Connection::Connecting,
                };
                (market, state)
            })
            .collect();
        let app = Self {
            translations: Translations::new(Language::from_system()),
            store: data::open_store(),
            clients,
            markets,
            market: Market::Spot,
            search: String::new(),
            selection: None,
            chart_kind: ChartKind::default(),
            price_scale: PriceScale::default(),
            pending_open: OpenRequest::from_args(std::env::args()),
        };
        let loads = Market::ALL.map(|market| app.load_market(market));
        (app, Task::batch(loads))
    }

    fn title(&self) -> String {
        self.translations.get("app-title")
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn view(&self) -> Element<'_, Message> {
        view::view(self)
    }

    fn subscription(&self) -> Subscription<Message> {
        let feeds = Market::ALL
            .into_iter()
            .filter(|market| matches!(self.markets[market].load, Load::Ready))
            .map(|market| Subscription::run_with(market, live_feed));
        let clock = iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick);
        Subscription::batch(feeds.chain([clock]))
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::LanguageSelected(language) => {
                self.translations = Translations::new(language);
                if let Some(selection) = &mut self.selection {
                    let title =
                        chart_title(&self.translations, &selection.symbol, selection.timeframe);
                    selection.chart.set_title(title);
                    selection.chart.set_labels(chart_labels(&self.translations));
                }
            }
            Message::MarketSelected(market) => self.market = market,
            Message::SearchChanged(search) => self.search = search,
            Message::MarketLoaded(market, result) => {
                let state = self.market_mut(market);
                match result {
                    Ok(snapshot) => {
                        state.symbols = snapshot
                            .symbols
                            .into_iter()
                            .map(|symbol| (symbol.name.clone(), symbol))
                            .collect();
                        state.tickers = snapshot
                            .tickers
                            .into_iter()
                            .filter(|ticker| state.symbols.contains_key(&ticker.symbol))
                            .map(|ticker| (ticker.symbol.clone(), ticker))
                            .collect();
                        state.load = Load::Ready;
                        if let Some(request) = self.pending_open.take_if(|r| r.market == market) {
                            self.market = market;
                            return self.select(market, &request.symbol, request.timeframe);
                        }
                    }
                    Err(error) => state.load = Load::Failed(error),
                }
            }
            Message::RetryMarket(market) => {
                self.market_mut(market).load = Load::Loading;
                return self.load_market(market);
            }
            Message::FeedReady(market, feed) => {
                let _ = feed.unbounded_send(FeedCommand::Subscribe(Topic::AllTickers));
                if let Some(selection) = &self.selection
                    && selection.symbol.market == market
                {
                    let _ = feed.unbounded_send(FeedCommand::Subscribe(selection.topic()));
                }
                self.market_mut(market).feed = Some(feed);
            }
            Message::Feed(market, event) => return self.on_feed(market, event),
            Message::SymbolSelected(market, name) => {
                let timeframe = self.selection.as_ref().map_or_else(
                    || DEFAULT_TIMEFRAME.parse().expect("valid default"),
                    |selection| selection.timeframe,
                );
                return self.select(market, &name, timeframe);
            }
            Message::TimeframeSelected(timeframe) => {
                if let Some(selection) = &self.selection {
                    let (market, name) = (selection.symbol.market, selection.symbol.name.clone());
                    return self.select(market, &name, timeframe);
                }
            }
            Message::ChartKindSelected(kind) => {
                self.chart_kind = kind;
                if let Some(selection) = &mut self.selection {
                    selection.chart.set_kind(kind);
                }
            }
            Message::PriceScaleToggled(scale) => {
                // Pressing the active scale's button goes back to the linear scale.
                self.price_scale = if self.price_scale == scale {
                    PriceScale::Linear
                } else {
                    scale
                };
                if let Some(selection) = &mut self.selection {
                    selection.chart.set_scale(self.price_scale);
                }
            }
            Message::Chart(message) => {
                if let Some(selection) = &mut self.selection {
                    selection.chart.update(message);
                }
                return self.load_older_if_needed();
            }
            Message::SeriesLoaded(key, result) => {
                if let Some(selection) = &mut self.selection
                    && selection.key() == key
                {
                    selection.loading = false;
                    match result {
                        Ok(loaded) => {
                            merge_newest(selection, loaded);
                            selection.error = None;
                            selection.refresh_chart();
                            return self.load_older_if_needed();
                        }
                        Err(error) => selection.error = Some(error),
                    }
                }
            }
            Message::OlderLoaded(key, result) => {
                if let Some(selection) = &mut self.selection
                    && selection.key() == key
                {
                    selection.loading_older = false;
                    match result {
                        Ok(older) => {
                            let connects = match (older.candles.last(), selection.candles.first()) {
                                (Some(last), Some(first)) => {
                                    selection.source.next_bucket(last.open_time) == first.open_time
                                }
                                _ => false,
                            };
                            if connects {
                                let mut candles = older.candles;
                                candles.append(&mut selection.candles);
                                selection.candles = candles;
                            }
                            selection.complete = older.complete;
                            selection.cached = older.cached;
                            selection.error = None;
                            selection.refresh_chart();
                            return self.load_older_if_needed();
                        }
                        Err(error) => {
                            selection.error = Some(error);
                            selection.chart.set_loading_older(false);
                        }
                    }
                }
            }
            Message::CandleSaved(key, result) => {
                if let Err(error) = result
                    && let Some(selection) = &mut self.selection
                    && selection.key() == key
                {
                    selection.error = Some(error);
                }
            }
            Message::Tick => {
                if let Some(selection) = &mut self.selection {
                    let now = self.clients[&selection.symbol.market].now();
                    selection.chart.set_now(now);
                }
            }
        }
        Task::none()
    }

    fn on_feed(&mut self, market: Market, event: FeedEvent) -> Task<Message> {
        match event {
            FeedEvent::Connected => {
                let state = self.market_mut(market);
                let reconnected = matches!(state.connection, Connection::Reconnecting { .. });
                state.connection = Connection::Connected;
                // Candles may have closed while the connection was down; fetch them.
                if reconnected
                    && self
                        .selection
                        .as_ref()
                        .is_some_and(|selection| selection.symbol.market == market)
                {
                    return self.open_series();
                }
            }
            FeedEvent::Reconnecting { attempt, delay } => {
                self.market_mut(market).connection = Connection::Reconnecting { attempt, delay };
            }
            FeedEvent::Tickers(tickers) => {
                let state = self.market_mut(market);
                for ticker in tickers {
                    if state.symbols.contains_key(&ticker.symbol) {
                        state.tickers.insert(ticker.symbol.clone(), ticker);
                    }
                }
            }
            FeedEvent::Candle {
                symbol,
                timeframe,
                candle,
                closed,
            } => return self.on_candle(market, &symbol, timeframe, candle, closed),
        }
        Task::none()
    }

    fn on_candle(
        &mut self,
        market: Market,
        symbol: &str,
        timeframe: Timeframe,
        candle: Candle,
        closed: bool,
    ) -> Task<Message> {
        let Some(selection) = &mut self.selection else {
            return Task::none();
        };
        if selection.symbol.market != market
            || selection.symbol.name != symbol
            || selection.source != timeframe
        {
            return Task::none();
        }
        if !closed {
            selection.live = Some(candle);
            selection.refresh_chart();
            return Task::none();
        }
        selection.live = None;

        // While a load is running, it will pick this candle up itself.
        let Some(last) = selection.candles.last().filter(|_| !selection.loading) else {
            return Task::none();
        };
        if candle.open_time <= last.open_time {
            return Task::none();
        }
        if timeframe.next_bucket(last.open_time) != candle.open_time {
            // Candles were missed; fill the gap from the exchange.
            return self.open_series();
        }

        selection.candles.push(candle);
        selection.cached += 1;
        selection.refresh_chart();
        let Ok(store) = &self.store else {
            return Task::none();
        };
        let key = selection.key();
        Task::perform(
            data::save_candle(store.clone(), key.clone(), candle),
            move |result| Message::CandleSaved(key.clone(), result),
        )
    }

    /// Shows `name` from `market` in `timeframe`, or the default timeframe if `market`
    /// cannot provide it.
    fn select(&mut self, market: Market, name: &str, timeframe: Timeframe) -> Task<Message> {
        let Some(symbol) = self.markets[&market].symbols.get(name).cloned() else {
            return Task::none();
        };
        let natives = binance::native_timeframes(market);
        let (timeframe, source) = match timeframe.best_source(natives) {
            Some(source) => (timeframe, source),
            None => {
                let default = DEFAULT_TIMEFRAME.parse().expect("valid default");
                (default, default)
            }
        };
        if let Some(current) = &self.selection
            && current.symbol == symbol
            && current.timeframe == timeframe
        {
            return Task::none();
        }

        let chart = Chart::new(
            chart_title(&self.translations, &symbol, timeframe),
            timeframe,
            symbol.price_decimals,
            self.chart_kind,
            self.price_scale,
            chart_labels(&self.translations),
        );
        let selection = Selection {
            symbol,
            timeframe,
            source,
            loading: false,
            loading_older: false,
            candles: Vec::new(),
            live: None,
            complete: false,
            cached: 0,
            error: None,
            chart,
        };
        let topic = selection.topic();
        if let Some(previous) = self.selection.take()
            && previous.topic() != topic
        {
            self.send(
                previous.symbol.market,
                FeedCommand::Unsubscribe(previous.topic()),
            );
        }
        self.send(market, FeedCommand::Subscribe(topic));
        self.selection = Some(selection);
        self.open_series()
    }

    /// Loads the newest candles of the selected series, from the cache and the exchange.
    fn open_series(&mut self) -> Task<Message> {
        let Some(selection) = &mut self.selection else {
            return Task::none();
        };
        let store = match &self.store {
            Ok(store) => store.clone(),
            Err(error) => {
                selection.error = Some(error.clone());
                return Task::none();
            }
        };
        selection.loading = true;
        let client = self.clients[&selection.symbol.market].clone();
        let key = selection.key();
        Task::perform(
            data::open_series(client, store, key.clone(), selection.timeframe),
            move |result| Message::SeriesLoaded(key.clone(), result),
        )
    }

    /// Loads older candles when the chart has been moved close to the oldest one.
    fn load_older_if_needed(&mut self) -> Task<Message> {
        let Some(selection) = &mut self.selection else {
            return Task::none();
        };
        if selection.loading || selection.loading_older || !selection.chart.wants_older() {
            return Task::none();
        }
        let (Some(first), Ok(store)) = (selection.candles.first(), &self.store) else {
            return Task::none();
        };
        let before = first.open_time;
        selection.loading_older = true;
        selection.chart.set_loading_older(true);
        let client = self.clients[&selection.symbol.market].clone();
        let key = selection.key();
        Task::perform(
            data::load_older(
                client,
                store.clone(),
                key.clone(),
                selection.timeframe,
                before,
            ),
            move |result| Message::OlderLoaded(key.clone(), result),
        )
    }

    fn load_market(&self, market: Market) -> Task<Message> {
        let client = self.clients[&market].clone();
        Task::perform(data::load_market(client), move |result| {
            Message::MarketLoaded(market, result)
        })
    }

    fn send(&self, market: Market, command: FeedCommand) {
        // Without a running feed there is nothing to tell; it subscribes when it starts.
        if let Some(feed) = &self.markets[&market].feed {
            let _ = feed.unbounded_send(command);
        }
    }

    fn market_mut(&mut self, market: Market) -> &mut MarketState {
        self.markets
            .get_mut(&market)
            .expect("every market has a state")
    }
}

/// Puts freshly loaded newest candles into the selection, keeping older candles already
/// in memory when they connect.
fn merge_newest(selection: &mut Selection, loaded: Loaded) {
    // Keep the feed's candle unless the fetched one is newer.
    if let Some(open) = loaded.open
        && selection
            .live
            .is_none_or(|live| live.open_time < open.open_time)
    {
        selection.live = Some(open);
    }
    selection.cached = loaded.cached;
    let Some(first_new) = loaded.candles.first().map(|candle| candle.open_time) else {
        selection.candles = loaded.candles;
        selection.complete = loaded.complete;
        return;
    };
    let older: Vec<Candle> = selection
        .candles
        .iter()
        .take_while(|candle| candle.open_time < first_new)
        .copied()
        .collect();
    let connects = older
        .last()
        .is_some_and(|last| selection.source.next_bucket(last.open_time) == first_new);
    if connects {
        selection.candles = older;
        selection.candles.extend(loaded.candles);
    } else {
        selection.candles = loaded.candles;
        selection.complete = loaded.complete;
    }
}

fn chart_title(t: &Translations, symbol: &Symbol, timeframe: Timeframe) -> String {
    let market = match symbol.market {
        Market::Spot => t.get("market-spot"),
        Market::UsdMFutures => t.get("market-futures"),
    };
    format!("{} · {timeframe} · Binance {market}", symbol.name)
}

fn chart_labels(t: &Translations) -> ChartLabels {
    let list = |key: &str| -> Vec<String> {
        t.get(key)
            .split(',')
            .map(|item| item.trim().to_owned())
            .collect()
    };
    let months = list("chart-months");
    let magnitudes = list("chart-magnitudes");
    ChartLabels {
        open: t.get("chart-open"),
        high: t.get("chart-high"),
        low: t.get("chart-low"),
        close: t.get("chart-close"),
        volume: t.get("chart-volume"),
        months: std::array::from_fn(|i| months.get(i).cloned().unwrap_or_default()),
        magnitudes: std::array::from_fn(|i| magnitudes.get(i).cloned().unwrap_or_default()),
        day_suffix: t.get("chart-day-suffix"),
        loading: t.get("chart-loading-older"),
    }
}

/// Runs the live feed for `market`, turning its events into messages.
fn live_feed(market: &Market) -> impl Stream<Item = Message> + use<> {
    let market = *market;
    iced::stream::channel(100, async move |mut output: mpsc::Sender<Message>| {
        let (commands, command_receiver) = mpsc::unbounded();
        if output
            .send(Message::FeedReady(market, commands))
            .await
            .is_err()
        {
            return;
        }
        let (event_sender, mut events) = mpsc::channel(100);
        let forward = async move {
            while let Some(event) = events.next().await {
                if output.send(Message::Feed(market, event)).await.is_err() {
                    break;
                }
            }
        };
        futures::join!(
            binance::run_feed(market, command_receiver, event_sender),
            forward
        );
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Option<OpenRequest> {
        OpenRequest::from_args(args.iter().map(|arg| arg.to_string()))
    }

    #[test]
    fn open_requests_come_from_the_command_line() {
        assert_eq!(
            parse(&["fitil", "--open", "futures:ethusdt:4h"]),
            Some(OpenRequest {
                market: Market::UsdMFutures,
                symbol: "ETHUSDT".into(),
                timeframe: "4h".parse().unwrap(),
            })
        );
        assert_eq!(
            parse(&["fitil", "--open", "spot:BTCUSDT"]).map(|r| r.timeframe.to_string()),
            Some(DEFAULT_TIMEFRAME.into())
        );
        assert_eq!(parse(&["fitil"]), None);
        assert_eq!(parse(&["fitil", "--open", "margin:BTCUSDT"]), None);
        assert_eq!(parse(&["fitil", "--open", "spot:BTCUSDT:7x"]), None);
    }
}
