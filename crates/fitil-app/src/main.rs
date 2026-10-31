//! Fitil desktop application: the window, its screens, and the wiring between them and
//! the application layer (`fitil-market`). Decisions about market data are made there;
//! this crate shows them and passes user input on.

// Release builds on Windows should not open a console window next to the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod dialogs;
mod error;
mod feed;
mod format;
mod i18n;
mod storage;
mod studies;
mod view;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use error::AppError;
use fitil_chart::{Chart, ChartKind, ChartLabels, ChartMessage, PriceScale};
use fitil_core::{Candle, ExchangeId, Market, MarketKind, SeriesKey, Symbol, Timeframe};
use fitil_exchange::{Exchange, Features, FeedCommand, FeedEvent, Topic};
use fitil_indicators::{Rgb, Settings, Value};
use fitil_market::{
    Board, LiveUpdate, Loaded, MarketError, MarketSnapshot, Series, SharedStore, SharedUserData,
    Studies, StudyId, TEMPLATE_NAME_MAX, Template,
};
use futures::channel::mpsc;
use i18n::{Language, Translations};
use iced::keyboard::{self, key::Named};
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

/// The exchanges Fitil shows. Adding an exchange means adding its adapter here.
fn exchanges() -> Vec<Arc<dyn Exchange>> {
    vec![Arc::new(
        fitil_binance::Binance::new().expect("the HTTP client can be created"),
    )]
}

/// Timeframe shown when a symbol is first opened.
const DEFAULT_TIMEFRAME: &str = "1h";

/// Timeframes offered in the picker; ones an exchange lacks are built from shorter candles.
const TIMEFRAME_CHOICES: [&str; 21] = [
    "1s", "1m", "3m", "5m", "15m", "30m", "45m", "1h", "2h", "3h", "4h", "6h", "8h", "12h", "1d",
    "2d", "3d", "1w", "2w", "1M", "3M",
];

struct Fitil {
    translations: Translations,
    cache: Result<SharedStore, AppError>,
    user_data: Result<SharedUserData, AppError>,
    exchanges: Vec<Arc<dyn Exchange>>,
    /// Every market of every exchange, in exchange order.
    markets: BTreeMap<Market, MarketState>,
    /// The market the watchlist shows.
    market: Market,
    search: String,
    selection: Option<Selection>,
    /// Chart type, price scale, volume bars and indicators, kept when another symbol or
    /// timeframe is opened.
    chart_kind: ChartKind,
    price_scale: PriceScale,
    show_volume: bool,
    studies: Studies,
    templates: Vec<Template>,
    templates_error: Option<AppError>,
    dialog: Option<Dialog>,
    /// A chart to open once its market has loaded, from `--open` on the command line.
    pending_open: Option<OpenRequest>,
}

/// A window shown over the main screen.
enum Dialog {
    /// The indicators that can be added, and the templates.
    Indicators {
        search: String,
        template_name: String,
        /// The template whose delete button asks for a second click.
        confirm_delete: Option<String>,
    },
    /// One indicator's settings. Numbers are kept as typed until they can be read.
    Settings {
        study: StudyId,
        drafts: BTreeMap<&'static str, String>,
    },
}

struct MarketState {
    load: Load,
    board: Board,
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
    series: Series,
    chart: Chart,
    error: Option<AppError>,
}

impl Selection {
    fn topic(&self) -> Topic {
        Topic::Candles {
            symbol: self.symbol.name.clone(),
            timeframe: self.series.source(),
        }
    }
}

/// A chart named on the command line: `--open [exchange:]market:SYMBOL[:timeframe]`,
/// such as `--open spot:BTCUSDT` or `--open binance:futures:ETHUSDT:4h`.
#[derive(Debug, Clone, PartialEq)]
struct OpenRequest {
    /// The exchange's id; any exchange with the market when left out.
    exchange: Option<String>,
    kind: MarketKind,
    symbol: String,
    timeframe: Timeframe,
}

impl OpenRequest {
    fn from_args(mut args: impl Iterator<Item = String>) -> Option<Self> {
        args.find(|arg| arg == "--open")?;
        let value = args.next()?;
        let mut parts: Vec<&str> = value.split(':').collect();
        let exchange = match parts.first() {
            Some(first) if market_kind(first).is_none() => Some(parts.remove(0).to_lowercase()),
            _ => None,
        };
        let (kind, symbol, timeframe) = match parts.as_slice() {
            [kind, symbol] => (*kind, *symbol, DEFAULT_TIMEFRAME),
            [kind, symbol, timeframe] => (*kind, *symbol, *timeframe),
            _ => return None,
        };
        Some(Self {
            exchange,
            kind: market_kind(kind)?,
            symbol: symbol.to_uppercase(),
            timeframe: timeframe.parse().ok()?,
        })
    }

    fn matches(&self, market: Market) -> bool {
        market.kind == self.kind
            && self
                .exchange
                .as_deref()
                .is_none_or(|exchange| exchange == market.exchange.as_str())
    }
}

fn market_kind(name: &str) -> Option<MarketKind> {
    match name {
        "spot" => Some(MarketKind::Spot),
        "futures" | "perp" => Some(MarketKind::LinearPerpetual),
        _ => None,
    }
}

#[derive(Debug, Clone)]
enum Message {
    LanguageSelected(Language),
    ExchangeSelected(ExchangeId),
    MarketSelected(Market),
    SearchChanged(String),
    MarketLoaded(Market, Result<MarketSnapshot, MarketError>),
    RetryMarket(Market),
    FeedReady(Market, mpsc::UnboundedSender<FeedCommand>),
    Feed(Market, FeedEvent),
    SymbolSelected(Market, String),
    TimeframeSelected(Timeframe),
    ChartKindSelected(ChartKind),
    PriceScaleToggled(PriceScale),
    Chart(ChartMessage),
    SeriesLoaded(SeriesKey, Result<Loaded, MarketError>),
    OlderLoaded(SeriesKey, Result<Loaded, MarketError>),
    CandleSaved(SeriesKey, Result<(), MarketError>),
    Tick,
    VolumeToggled,
    OpenIndicators,
    CloseDialog,
    IndicatorSearchChanged(String),
    AddIndicator(&'static str),
    OpenStudySettings(StudyId),
    ToggleStudy(StudyId),
    RemoveStudy(StudyId),
    StudyNumberTyped {
        study: StudyId,
        param: &'static str,
        text: String,
    },
    StudyTextChosen {
        study: StudyId,
        param: &'static str,
        value: String,
    },
    StudyOutputColor {
        study: StudyId,
        output: usize,
        color: Rgb,
    },
    StudyOutputWidth {
        study: StudyId,
        output: usize,
        width: u8,
    },
    StudyOutputVisible {
        study: StudyId,
        output: usize,
        visible: bool,
    },
    StudyDefaults(StudyId),
    TemplateNameChanged(String),
    SaveTemplate,
    ApplyTemplate(String),
    ConfirmDeleteTemplate(String),
    DeleteTemplate(String),
    /// The saved templates, after loading, saving or deleting.
    Templates(Result<Vec<Template>, MarketError>),
}

impl Fitil {
    fn new() -> (Self, Task<Message>) {
        let exchanges = exchanges();
        let markets: BTreeMap<Market, MarketState> = exchanges
            .iter()
            .flat_map(|exchange| {
                exchange.markets().iter().map(|&kind| Market {
                    exchange: exchange.id(),
                    kind,
                })
            })
            .map(|market| {
                let state = MarketState {
                    load: Load::Loading,
                    board: Board::default(),
                    feed: None,
                    connection: Connection::Connecting,
                };
                (market, state)
            })
            .collect();
        let market = *markets.keys().next().expect("every exchange has a market");
        let user_data = storage::open_user_data();
        let app = Self {
            translations: Translations::new(Language::from_system()),
            cache: storage::open_cache(),
            templates_error: user_data.as_ref().err().cloned(),
            user_data,
            exchanges,
            markets,
            market,
            search: String::new(),
            selection: None,
            chart_kind: ChartKind::default(),
            price_scale: PriceScale::default(),
            show_volume: true,
            studies: Studies::default(),
            templates: Vec::new(),
            dialog: None,
            pending_open: OpenRequest::from_args(std::env::args()),
        };
        let mut tasks: Vec<_> = app
            .markets
            .keys()
            .map(|&market| app.load_market(market))
            .collect();
        if let Ok(user) = &app.user_data {
            tasks.push(Task::perform(
                fitil_market::load_templates(user.clone()),
                Message::Templates,
            ));
        }
        (app, Task::batch(tasks))
    }

    fn title(&self) -> String {
        self.translations.get("app-title")
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn view(&self) -> Element<'_, Message> {
        let screen = view::view(self);
        match &self.dialog {
            Some(dialog) => {
                dialogs::modal(screen, dialogs::view(self, dialog), Message::CloseDialog)
            }
            None => screen,
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let feeds = self
            .markets
            .iter()
            .filter(|(_, state)| matches!(state.load, Load::Ready))
            .map(|(&market, _)| {
                let source = feed::Source {
                    market,
                    exchange: self.exchange(market.exchange),
                };
                Subscription::run_with(source, feed::run)
            });
        let clock = iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick);
        let escape = keyboard::listen().filter_map(|event| match event {
            keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(Named::Escape),
                ..
            } => Some(Message::CloseDialog),
            _ => None,
        });
        Subscription::batch(feeds.chain([clock, escape]))
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::LanguageSelected(language) => {
                self.translations = Translations::new(language);
                if let Some(selection) = &mut self.selection {
                    let title = chart_title(
                        &self.translations,
                        self.exchanges.as_slice(),
                        &selection.symbol,
                        selection.series.target(),
                    );
                    selection.chart.set_title(title);
                    selection.chart.set_labels(chart_labels(&self.translations));
                }
                // Indicator labels are translated too.
                self.refresh_chart();
            }
            Message::ExchangeSelected(exchange) => {
                if let Some(&market) = self.markets.keys().find(|m| m.exchange == exchange) {
                    self.market = market;
                }
            }
            Message::MarketSelected(market) => self.market = market,
            Message::SearchChanged(search) => self.search = search,
            Message::MarketLoaded(market, result) => {
                let state = self.market_mut(market);
                match result {
                    Ok(snapshot) => {
                        state.board = Board::new(snapshot);
                        state.load = Load::Ready;
                        if let Some(request) = self.pending_open.take_if(|r| r.matches(market)) {
                            self.market = market;
                            return self.select(market, &request.symbol, request.timeframe);
                        }
                    }
                    Err(error) => state.load = Load::Failed(error.into()),
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
                    |selection| selection.series.target(),
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
                if let Some(selection) = self.selection_of(&key) {
                    match selection.series.finish_load(result) {
                        Ok(()) => selection.error = None,
                        Err(error) => selection.error = Some(error.into()),
                    }
                    self.refresh_chart();
                    return self.load_older_if_needed();
                }
            }
            Message::OlderLoaded(key, result) => {
                if let Some(selection) = self.selection_of(&key) {
                    match selection.series.finish_load_older(result) {
                        Ok(()) => selection.error = None,
                        Err(error) => selection.error = Some(error.into()),
                    }
                    self.refresh_chart();
                    return self.load_older_if_needed();
                }
            }
            Message::CandleSaved(key, result) => {
                if let Err(error) = result
                    && let Some(selection) = self.selection_of(&key)
                {
                    selection.error = Some(error.into());
                }
            }
            Message::Tick => {
                if let Some(selection) = &self.selection {
                    let now = self.exchange(selection.symbol.market.exchange).now();
                    if let Some(selection) = &mut self.selection {
                        selection.chart.set_now(now);
                    }
                }
            }
            Message::VolumeToggled => {
                self.show_volume = !self.show_volume;
                if let Some(selection) = &mut self.selection {
                    selection.chart.set_show_volume(self.show_volume);
                }
            }
            Message::OpenIndicators => {
                self.dialog = Some(Dialog::Indicators {
                    search: String::new(),
                    template_name: String::new(),
                    confirm_delete: None,
                });
                return iced::widget::operation::focus(dialogs::SEARCH_ID);
            }
            Message::CloseDialog => self.dialog = None,
            Message::IndicatorSearchChanged(text) => {
                if let Some(Dialog::Indicators { search, .. }) = &mut self.dialog {
                    *search = text;
                }
            }
            Message::AddIndicator(id) => {
                if let Some(definition) = fitil_indicators::definition(id) {
                    self.studies.add(definition);
                    self.refresh_chart();
                }
            }
            Message::OpenStudySettings(study) => {
                self.dialog = Some(Dialog::Settings {
                    study,
                    drafts: BTreeMap::new(),
                });
            }
            Message::ToggleStudy(id) => {
                if let Some(study) = self.studies.get(id) {
                    let visible = study.settings().visible;
                    self.studies.set_visible(id, !visible);
                    self.refresh_chart();
                }
            }
            Message::RemoveStudy(id) => {
                self.studies.remove(id);
                if matches!(self.dialog, Some(Dialog::Settings { study, .. }) if study == id) {
                    self.dialog = None;
                }
                self.refresh_chart();
            }
            Message::StudyNumberTyped { study, param, text } => {
                let number = studies::parse_number(&text);
                if let Some(Dialog::Settings { drafts, .. }) = &mut self.dialog {
                    drafts.insert(param, text);
                }
                if let Some(number) = number {
                    self.change_settings(study, |settings| {
                        settings.params.set(param, Value::Number(number));
                    });
                }
            }
            Message::StudyTextChosen {
                study,
                param,
                value,
            } => self.change_settings(study, |settings| {
                settings.params.set(param, Value::Text(value));
            }),
            Message::StudyOutputColor {
                study,
                output,
                color,
            } => self.change_settings(study, |settings| {
                if let Some(look) = settings.outputs.get_mut(output) {
                    look.color = color;
                }
            }),
            Message::StudyOutputWidth {
                study,
                output,
                width,
            } => self.change_settings(study, |settings| {
                if let Some(look) = settings.outputs.get_mut(output) {
                    look.width = width;
                }
            }),
            Message::StudyOutputVisible {
                study,
                output,
                visible,
            } => self.change_settings(study, |settings| {
                if let Some(look) = settings.outputs.get_mut(output) {
                    look.visible = visible;
                }
            }),
            Message::StudyDefaults(study) => {
                if let Some(Dialog::Settings { drafts, .. }) = &mut self.dialog {
                    drafts.clear();
                }
                self.change_settings(study, |settings| {
                    if let Some(definition) = settings.definition() {
                        *settings = Settings {
                            visible: settings.visible,
                            ..Settings::defaults(definition)
                        };
                    }
                });
            }
            Message::TemplateNameChanged(text) => {
                if let Some(Dialog::Indicators { template_name, .. }) = &mut self.dialog {
                    *template_name = text.chars().take(TEMPLATE_NAME_MAX).collect();
                }
            }
            Message::SaveTemplate => {
                let Some(Dialog::Indicators { template_name, .. }) = &mut self.dialog else {
                    return Task::none();
                };
                let (Some(name), Ok(user)) = (Template::clean_name(template_name), &self.user_data)
                else {
                    return Task::none();
                };
                if self.studies.is_empty() {
                    return Task::none();
                }
                template_name.clear();
                let template = Template {
                    name,
                    studies: self.studies.settings(),
                };
                return Task::perform(
                    fitil_market::save_template(user.clone(), template),
                    Message::Templates,
                );
            }
            Message::ApplyTemplate(name) => {
                let features = self.features();
                if let Some(template) = self.templates.iter().find(|t| t.name == name) {
                    self.studies.replace(&template.studies, features);
                    self.refresh_chart();
                }
            }
            Message::ConfirmDeleteTemplate(name) => {
                if let Some(Dialog::Indicators { confirm_delete, .. }) = &mut self.dialog {
                    *confirm_delete = Some(name);
                }
            }
            Message::DeleteTemplate(name) => {
                if let Some(Dialog::Indicators { confirm_delete, .. }) = &mut self.dialog {
                    *confirm_delete = None;
                }
                if let Ok(user) = &self.user_data {
                    return Task::perform(
                        fitil_market::delete_template(user.clone(), name),
                        Message::Templates,
                    );
                }
            }
            Message::Templates(result) => match result {
                Ok(templates) => {
                    self.templates = templates;
                    self.templates_error = None;
                }
                Err(error) => self.templates_error = Some(AppError::user_data(error)),
            },
        }
        Task::none()
    }

    /// Changes one indicator's settings with `change` and redraws the chart.
    fn change_settings(&mut self, id: StudyId, change: impl FnOnce(&mut Settings)) {
        let Some(study) = self.studies.get(id) else {
            return;
        };
        let mut settings = study.settings().clone();
        change(&mut settings);
        if self.studies.set_settings(id, settings) {
            self.refresh_chart();
        }
    }

    /// Shows the selected series' candles and indicators on its chart.
    fn refresh_chart(&mut self) {
        let Some(selection) = &mut self.selection else {
            return;
        };
        let candles = selection.series.shown();
        self.studies.update(&candles);
        selection.chart.set_candles(candles);
        selection
            .chart
            .set_studies(studies::chart_studies(&self.studies, &self.translations));
        selection
            .chart
            .set_history_complete(selection.series.complete());
        selection
            .chart
            .set_loading_older(selection.series.loading_older());
    }

    /// What the chart's exchange provides (the watchlist's, while no chart is open).
    fn features(&self) -> Features {
        let market = self
            .selection
            .as_ref()
            .map_or(self.market, |selection| selection.symbol.market);
        self.exchange(market.exchange).features()
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
            FeedEvent::Tickers(tickers) => self.market_mut(market).board.update(tickers),
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
        if selection.symbol.market != market || !selection.series.is_fed_by(symbol, timeframe) {
            return Task::none();
        }
        let update = selection.series.apply_live(candle, closed);
        let key = selection.series.key().clone();
        match update {
            LiveUpdate::Changed => {
                self.refresh_chart();
                Task::none()
            }
            LiveUpdate::Closed(candle) => {
                self.refresh_chart();
                let Ok(cache) = &self.cache else {
                    return Task::none();
                };
                Task::perform(
                    fitil_market::save_candle(cache.clone(), key.clone(), candle),
                    move |result| Message::CandleSaved(key.clone(), result),
                )
            }
            LiveUpdate::Reload => self.open_series(),
        }
    }

    /// Shows `name` from `market` in `timeframe`, or the default timeframe if the
    /// exchange cannot provide it.
    fn select(&mut self, market: Market, name: &str, timeframe: Timeframe) -> Task<Message> {
        let Some(symbol) = self.markets[&market].board.symbol(name).cloned() else {
            return Task::none();
        };
        let natives = self.exchange(market.exchange).timeframes(market.kind);
        let (timeframe, source) = match timeframe.best_source(natives) {
            Some(source) => (timeframe, source),
            None => {
                let default = DEFAULT_TIMEFRAME.parse().expect("valid default");
                (default, default)
            }
        };
        if let Some(current) = &self.selection
            && current.symbol == symbol
            && current.series.target() == timeframe
        {
            return Task::none();
        }

        let key = SeriesKey {
            market,
            symbol: symbol.name.clone(),
            timeframe: source,
        };
        let title = chart_title(&self.translations, &self.exchanges, &symbol, timeframe);
        // The chart stays, with its look and pane heights; only the series changes.
        let (chart, previous) = match self.selection.take() {
            Some(previous) => {
                let feed = (previous.symbol.market, previous.topic());
                let mut chart = previous.chart;
                chart.replace_series(title, timeframe, symbol.price_decimals);
                (chart, Some(feed))
            }
            None => {
                let mut chart = Chart::new(
                    title,
                    timeframe,
                    symbol.price_decimals,
                    self.chart_kind,
                    self.price_scale,
                    chart_labels(&self.translations),
                );
                chart.set_show_volume(self.show_volume);
                (chart, None)
            }
        };
        let selection = Selection {
            symbol,
            series: Series::new(key, timeframe),
            chart,
            error: None,
        };
        let topic = selection.topic();
        if let Some((previous_market, previous_topic)) = previous
            && (previous_market != market || previous_topic != topic)
        {
            self.send(previous_market, FeedCommand::Unsubscribe(previous_topic));
        }
        self.send(market, FeedCommand::Subscribe(topic));
        self.studies.reset();
        self.selection = Some(selection);
        self.open_series()
    }

    /// Loads the newest candles of the selected series, unless they are loading already.
    fn open_series(&mut self) -> Task<Message> {
        let Some(selection) = &mut self.selection else {
            return Task::none();
        };
        let cache = match &self.cache {
            Ok(cache) => cache.clone(),
            Err(error) => {
                selection.error = Some(error.clone());
                return Task::none();
            }
        };
        if !selection.series.begin_load() {
            return Task::none();
        }
        let exchange = find_exchange(&self.exchanges, selection.symbol.market.exchange);
        let key = selection.series.key().clone();
        Task::perform(
            fitil_market::open_series(exchange, cache, key.clone(), selection.series.target()),
            move |result| Message::SeriesLoaded(key.clone(), result),
        )
    }

    /// Loads older candles once the chart has been moved close to the oldest one.
    fn load_older_if_needed(&mut self) -> Task<Message> {
        let Some(selection) = &mut self.selection else {
            return Task::none();
        };
        let Ok(cache) = &self.cache else {
            return Task::none();
        };
        if !selection.chart.wants_older() {
            return Task::none();
        }
        let Some(before) = selection.series.begin_load_older() else {
            return Task::none();
        };
        selection.chart.set_loading_older(true);
        let exchange = find_exchange(&self.exchanges, selection.symbol.market.exchange);
        let key = selection.series.key().clone();
        Task::perform(
            fitil_market::load_older(
                exchange,
                cache.clone(),
                key.clone(),
                selection.series.target(),
                before,
            ),
            move |result| Message::OlderLoaded(key.clone(), result),
        )
    }

    fn load_market(&self, market: Market) -> Task<Message> {
        let exchange = self.exchange(market.exchange);
        Task::perform(
            fitil_market::load_market(exchange, market.kind),
            move |result| Message::MarketLoaded(market, result),
        )
    }

    /// The selection, if it still shows the series `key` (loads may finish after the
    /// user has moved on).
    fn selection_of(&mut self, key: &SeriesKey) -> Option<&mut Selection> {
        self.selection
            .as_mut()
            .filter(|selection| selection.series.key() == key)
    }

    fn exchange(&self, id: ExchangeId) -> Arc<dyn Exchange> {
        find_exchange(&self.exchanges, id)
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

fn find_exchange(exchanges: &[Arc<dyn Exchange>], id: ExchangeId) -> Arc<dyn Exchange> {
    exchanges
        .iter()
        .find(|exchange| exchange.id() == id)
        .cloned()
        .expect("markets only name registered exchanges")
}

/// The Fluent key naming a kind of market.
fn market_key(kind: MarketKind) -> &'static str {
    match kind {
        MarketKind::Spot => "market-spot",
        MarketKind::LinearPerpetual => "market-futures",
    }
}

fn chart_title(
    t: &Translations,
    exchanges: &[Arc<dyn Exchange>],
    symbol: &Symbol,
    timeframe: Timeframe,
) -> String {
    let exchange = find_exchange(exchanges, symbol.market.exchange);
    format!(
        "{} · {timeframe} · {} {}",
        symbol.name,
        exchange.name(),
        t.get(market_key(symbol.market.kind))
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use fitil_store::{Store, UserData};

    /// The app with one Binance spot symbol, its candle cache and user data in memory:
    /// no files and no network.
    fn app() -> (Fitil, Market) {
        let exchanges = exchanges();
        let market = Market {
            exchange: exchanges[0].id(),
            kind: MarketKind::Spot,
        };
        let symbol = Symbol {
            market,
            name: "BTCUSDT".into(),
            base_asset: "BTC".into(),
            quote_asset: "USDT".into(),
            price_decimals: 2,
        };
        let state = MarketState {
            load: Load::Ready,
            board: Board::new(MarketSnapshot {
                symbols: vec![symbol],
                tickers: Vec::new(),
            }),
            feed: None,
            connection: Connection::Connected,
        };
        let app = Fitil {
            translations: Translations::new(Language::English),
            cache: Ok(Arc::new(Mutex::new(Store::open_in_memory().unwrap()))),
            user_data: Ok(Arc::new(Mutex::new(UserData::open_in_memory().unwrap()))),
            exchanges,
            markets: BTreeMap::from([(market, state)]),
            market,
            search: String::new(),
            selection: None,
            chart_kind: ChartKind::default(),
            price_scale: PriceScale::default(),
            show_volume: true,
            studies: Studies::default(),
            templates: Vec::new(),
            templates_error: None,
            dialog: None,
            pending_open: None,
        };
        (app, market)
    }

    /// Opens BTCUSDT and hands it `count` hourly candles, as a load from the cache would.
    fn open_chart(app: &mut Fitil, market: Market, count: i64) {
        let _ = app.update(Message::SymbolSelected(market, "BTCUSDT".into()));
        let key = app.selection.as_ref().unwrap().series.key().clone();
        let candles = (0..count)
            .map(|hour| Candle {
                open_time: hour * 3_600_000,
                open: 100.0 + hour as f64,
                high: 102.0 + hour as f64,
                low: 99.0 + hour as f64,
                close: 101.0 + hour as f64,
                volume: 1.0,
                quote_volume: None,
                trades: None,
                taker_buy_volume: Some(0.5),
            })
            .collect();
        let loaded = Loaded {
            candles,
            open: None,
            complete: false,
            cached: count as u64,
        };
        let _ = app.update(Message::SeriesLoaded(key, Ok(loaded)));
    }

    fn first_study(app: &Fitil) -> &fitil_market::Study {
        app.studies.iter().next().expect("a study")
    }

    #[test]
    fn indicators_are_added_changed_and_removed() {
        let (mut app, market) = app();
        open_chart(&mut app, market, 100);
        let _ = app.update(Message::AddIndicator("rsi"));
        assert_eq!(first_study(&app).values()[0].len(), 100);
        let id = first_study(&app).id();

        let _ = app.update(Message::ToggleStudy(id));
        assert!(!first_study(&app).settings().visible);
        let _ = app.update(Message::ToggleStudy(id));

        let _ = app.update(Message::OpenStudySettings(id));
        let typed = |text: &str| Message::StudyNumberTyped {
            study: id,
            param: "length",
            text: text.into(),
        };
        let _ = app.update(typed("21"));
        assert_eq!(first_study(&app).settings().params.length("length"), 21);
        // Text that is not a number stays in the box but changes nothing.
        let _ = app.update(typed("2x"));
        assert_eq!(first_study(&app).settings().params.length("length"), 21);
        assert!(
            matches!(&app.dialog, Some(Dialog::Settings { drafts, .. }) if drafts["length"] == "2x")
        );

        let _ = app.update(Message::StudyOutputColor {
            study: id,
            output: 0,
            color: Rgb::hex(0x123456),
        });
        assert_eq!(
            first_study(&app).settings().outputs[0].color,
            Rgb::hex(0x123456)
        );
        let _ = app.update(Message::StudyDefaults(id));
        assert_eq!(first_study(&app).settings().params.length("length"), 14);
        assert!(matches!(&app.dialog, Some(Dialog::Settings { drafts, .. }) if drafts.is_empty()));

        let _ = app.update(Message::RemoveStudy(id));
        assert!(app.studies.is_empty());
        assert!(app.dialog.is_none(), "the removed study's settings close");
    }

    #[test]
    fn templates_are_applied_and_named() {
        let (mut app, market) = app();
        open_chart(&mut app, market, 50);
        let template = Template {
            name: "Flow".into(),
            studies: ["cvd", "ema"]
                .map(|id| Settings::defaults(fitil_indicators::definition(id).unwrap()))
                .to_vec(),
        };
        let _ = app.update(Message::Templates(Ok(vec![template])));
        let _ = app.update(Message::ApplyTemplate("Flow".into()));
        let ids: Vec<&str> = app.studies.iter().map(|s| s.definition().id).collect();
        assert_eq!(ids, ["cvd", "ema"]);
        assert_eq!(first_study(&app).values()[0].len(), 50);

        let _ = app.update(Message::OpenIndicators);
        let _ = app.update(Message::TemplateNameChanged("x".repeat(100)));
        let _ = app.update(Message::SaveTemplate);
        // Saving clears the name for the next template.
        assert!(
            matches!(&app.dialog, Some(Dialog::Indicators { template_name, .. }) if template_name.is_empty())
        );

        let _ = app.update(Message::Templates(Err(MarketError::Storage("disk".into()))));
        assert!(matches!(app.templates_error, Some(AppError::UserData(_))));
    }

    #[test]
    fn every_screen_can_be_built() {
        let (mut app, market) = app();
        let _ = app.view();
        open_chart(&mut app, market, 30);
        for id in ["ichimoku", "macd", "volume", "vwap"] {
            let _ = app.update(Message::AddIndicator(id));
        }
        let _ = app.view();
        let _ = app.update(Message::OpenIndicators);
        let _ = app.update(Message::IndicatorSearchChanged("nothing like this".into()));
        let _ = app.view();
        for study in app.studies.iter().map(|s| s.id()).collect::<Vec<_>>() {
            let _ = app.update(Message::OpenStudySettings(study));
            let _ = app.view();
        }
        let _ = app.update(Message::CloseDialog);
        assert!(app.dialog.is_none());
    }

    fn parse(args: &[&str]) -> Option<OpenRequest> {
        OpenRequest::from_args(args.iter().map(|arg| arg.to_string()))
    }

    #[test]
    fn open_requests_come_from_the_command_line() {
        assert_eq!(
            parse(&["fitil", "--open", "binance:futures:ethusdt:4h"]),
            Some(OpenRequest {
                exchange: Some("binance".into()),
                kind: MarketKind::LinearPerpetual,
                symbol: "ETHUSDT".into(),
                timeframe: "4h".parse().unwrap(),
            })
        );
        let spot = parse(&["fitil", "--open", "spot:BTCUSDT"]).unwrap();
        assert_eq!(spot.exchange, None);
        assert_eq!(spot.timeframe.to_string(), DEFAULT_TIMEFRAME);
        assert_eq!(parse(&["fitil"]), None);
        assert_eq!(parse(&["fitil", "--open", "binance:margin:BTCUSDT"]), None);
        assert_eq!(parse(&["fitil", "--open", "spot:BTCUSDT:7x"]), None);
    }

    #[test]
    fn open_requests_match_markets() {
        let binance_spot = Market {
            exchange: ExchangeId::new("binance"),
            kind: MarketKind::Spot,
        };
        let okx_spot = Market {
            exchange: ExchangeId::new("okx"),
            ..binance_spot
        };
        let any = parse(&["fitil", "--open", "spot:BTCUSDT"]).unwrap();
        assert!(any.matches(binance_spot) && any.matches(okx_spot));
        let okx = parse(&["fitil", "--open", "okx:spot:BTC-USDT"]).unwrap();
        assert!(okx.matches(okx_spot) && !okx.matches(binance_spot));
        assert_eq!(okx.symbol, "BTC-USDT");
    }
}
