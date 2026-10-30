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
use fitil_core::{Candle, Market, SeriesKey, Symbol, Ticker, Timeframe};
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

/// The symbol whose details are shown.
struct Selection {
    symbol: Symbol,
    /// The timeframe the user picked.
    timeframe: Timeframe,
    /// The timeframe downloaded from Binance to build `timeframe`.
    source: Timeframe,
    loading: bool,
    loaded: Option<Loaded>,
    error: Option<AppError>,
    /// The open `source` candle, from the live feed.
    live: Option<Candle>,
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
    LoadOlder,
    SeriesLoaded(SeriesKey, Result<Loaded, AppError>),
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
            Message::LoadOlder => return self.load_older(),
            Message::SeriesLoaded(key, result) => {
                if let Some(selection) = &mut self.selection
                    && selection.key() == key
                {
                    selection.loading = false;
                    match result {
                        Ok(loaded) => {
                            // Keep the feed's candle unless the fetched one is newer.
                            if let Some(open) = loaded.open
                                && selection
                                    .live
                                    .is_none_or(|live| live.open_time < open.open_time)
                            {
                                selection.live = Some(open);
                            }
                            selection.loaded = Some(loaded);
                            selection.error = None;
                        }
                        Err(error) => selection.error = Some(error),
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
                    let bucket = selection.timeframe.bucket_start(now);
                    if let Some(loaded) = &mut selection.loaded {
                        loaded.recent.retain(|candle| candle.open_time >= bucket);
                    }
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
                    && let Some(selection) = &mut self.selection
                    && selection.symbol.market == market
                {
                    selection.loading = true;
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
            return Task::none();
        }
        selection.live = None;

        // While a load is running, it will pick this candle up itself.
        let (Some(loaded), false) = (&mut selection.loaded, selection.loading) else {
            return Task::none();
        };
        let Some(range) = &mut loaded.range else {
            return Task::none();
        };
        if candle.open_time <= range.newest {
            return Task::none();
        }
        if timeframe.next_bucket(range.newest) != candle.open_time {
            // Candles were missed; fill the gap from the exchange.
            selection.loading = true;
            return self.open_series();
        }

        range.newest = candle.open_time;
        range.count += 1;
        if selection.timeframe == timeframe
            || selection.timeframe.bucket_start(candle.open_time) == candle.open_time
        {
            loaded.target_count += 1;
        }
        loaded.recent.push(candle);

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

        let selection = Selection {
            symbol,
            timeframe,
            source,
            loading: true,
            loaded: None,
            error: None,
            live: None,
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

    /// Loads the selected series from the cache and brings it up to date.
    fn open_series(&mut self) -> Task<Message> {
        self.series_task(data::open_series)
    }

    fn load_older(&mut self) -> Task<Message> {
        match &mut self.selection {
            Some(selection) if !selection.loading => selection.loading = true,
            _ => return Task::none(),
        }
        self.series_task(data::load_older)
    }

    fn series_task<F>(
        &mut self,
        load: fn(Arc<BinanceClient>, SharedStore, SeriesKey, Timeframe) -> F,
    ) -> Task<Message>
    where
        F: Future<Output = Result<Loaded, AppError>> + Send + 'static,
    {
        let Some(selection) = &mut self.selection else {
            return Task::none();
        };
        let store = match &self.store {
            Ok(store) => store.clone(),
            Err(error) => {
                selection.loading = false;
                selection.error = Some(error.clone());
                return Task::none();
            }
        };
        let client = self.clients[&selection.symbol.market].clone();
        let key = selection.key();
        let future = load(client, store, key.clone(), selection.timeframe);
        Task::perform(future, move |result| {
            Message::SeriesLoaded(key.clone(), result)
        })
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
