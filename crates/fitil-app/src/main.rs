//! Fitil desktop application: the window, its screens, and the wiring between them and
//! the application layer (`fitil-market`). Decisions about market data are made there;
//! this crate shows them and passes user input on.

// Release builds on Windows should not open a console window next to the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod dialogs;
mod drawing_dialog;
mod error;
mod feed;
mod flyout;
mod format;
mod i18n;
mod icons;
mod storage;
mod studies;
mod tools;
mod view;

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;

use drawing_dialog::{ColorTarget, DrawingWindow, Look, Setting, Tab};
use error::AppError;
use fitil_chart::{Chart, ChartKind, ChartLabels, ChartMessage, PriceScale};
use fitil_core::{
    Candle, Drawing, DrawingKind, DrawingStyle, ExchangeId, LineStyle, Market, MarketKind,
    SeriesKey, Symbol, Timeframe,
};
use fitil_exchange::{Exchange, Features, FeedCommand, FeedEvent, Topic};
use fitil_indicators::{Rgb, Settings, Value};
use fitil_market::{
    Board, DrawingLooks, DrawingTemplate, LiveUpdate, Loaded, MarketError, MarketSnapshot, Match,
    Series, SharedStore, SharedUserData, Studies, StudyId, TEMPLATE_NAME_MAX, Template,
};
use futures::channel::mpsc;
use i18n::{Language, Translations};
use iced::keyboard::{self, key::Named};
use iced::widget::operation;
use iced::{Element, Subscription, Task, Theme};
use tools::ToolGroup;

fn main() -> iced::Result {
    iced::application(Fitil::new, Fitil::update, Fitil::view)
        .title(Fitil::title)
        .theme(Fitil::theme)
        .subscription(Fitil::subscription)
        .font(icons::FONT_BYTES)
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

/// Results the symbol search lists; typing more finds the rest.
const SEARCH_RESULTS: usize = 100;

struct Fitil {
    translations: Translations,
    cache: Result<SharedStore, AppError>,
    user_data: Result<SharedUserData, AppError>,
    exchanges: Vec<Arc<dyn Exchange>>,
    /// Every market of every exchange, in exchange order.
    markets: BTreeMap<Market, MarketState>,
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
    drawings: DrawingSaves,
    /// Deleting every drawing waits for a second click.
    confirm_clear: bool,
    /// The looks saved for drawings: what new ones start with, and named templates.
    drawing_looks: DrawingLooks,
    /// The drawing tool group whose menu is open.
    tool_menu: Option<ToolGroup>,
    /// The tool of each group picked last, which the group's button shows.
    group_tools: HashMap<ToolGroup, DrawingKind>,
}

/// The drawings of the symbols opened, kept in step with the user's data.
#[derive(Default)]
struct DrawingSaves {
    /// The newest drawings of each symbol opened, so that going back to one needs no read.
    known: HashMap<(Market, String), Vec<Drawing>>,
    /// A save is running. The next ones wait their turn, so that none can overtake it.
    saving: bool,
    waiting: Vec<(Market, String, Vec<Drawing>)>,
}

/// A window shown over the main screen.
enum Dialog {
    /// Finding a symbol to open.
    Symbols {
        search: String,
        /// Only this kind of market, or all of them.
        kind: Option<MarketKind>,
        /// The result picked with the arrow keys.
        highlight: usize,
    },
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
    /// The settings of the drawing the chart has open for them.
    Drawing(DrawingWindow),
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
    /// Opens the symbol search with some text already typed.
    OpenSymbolSearch(String),
    /// A letter or digit typed while nothing else took it.
    KeyTyped(String),
    SymbolSearchChanged(String),
    SymbolSearchKind(Option<MarketKind>),
    /// Moves the picked result up (negative) or down.
    SymbolSearchMove(isize),
    SymbolSearchSubmit,
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
    /// Esc: closes a menu or a window, or else drops what the chart is in the middle of.
    Escape,
    /// Opens a tool group's menu, or closes the one open.
    ToolMenu(Option<ToolGroup>),
    ToolSelected(Option<DrawingKind>),
    MagnetToggled,
    DrawingsHiddenToggled,
    DrawingsLockToggled,
    /// Deletes every drawing of the symbol, on the second click.
    ClearDrawings,
    DrawingColor(Rgb),
    DrawingWidth(u8),
    DrawingLine(LineStyle),
    DrawingText(String),
    /// Opens the picked drawing's settings.
    OpenDrawingSettings,
    DrawingTab(Tab),
    DrawingSetting(Setting),
    /// Opens the palette of one of the drawing's colors, or closes it.
    DrawingPicker(Option<ColorTarget>),
    /// Keeps the changes made in the drawing's settings and closes them.
    DrawingSettingsDone,
    DrawingLook(Look),
    /// The saved looks of drawings, once loaded.
    DrawingLooks(Result<DrawingLooks, MarketError>),
    DrawingLookSaved(Result<(), MarketError>),
    /// Locks or unlocks the picked drawing.
    DrawingLockToggled,
    DuplicateDrawing,
    DeleteDrawing,
    Undo,
    Redo,
    DrawingsLoaded(Market, String, Result<Vec<Drawing>, MarketError>),
    DrawingsSaved(Result<(), MarketError>),
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
        let user_data = storage::open_user_data();
        let app = Self {
            translations: Translations::new(Language::from_system()),
            cache: storage::open_cache(),
            templates_error: user_data.as_ref().err().cloned(),
            user_data,
            exchanges,
            markets,
            selection: None,
            chart_kind: ChartKind::default(),
            price_scale: PriceScale::default(),
            show_volume: true,
            studies: Studies::default(),
            templates: Vec::new(),
            dialog: None,
            pending_open: OpenRequest::from_args(std::env::args()),
            drawings: DrawingSaves::default(),
            confirm_clear: false,
            drawing_looks: DrawingLooks::default(),
            tool_menu: None,
            group_tools: HashMap::new(),
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
            tasks.push(Task::perform(
                fitil_market::load_drawing_looks(user.clone()),
                Message::DrawingLooks,
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
        let keys = iced::event::listen_with(key_message);
        Subscription::batch(feeds.chain([clock, keys]))
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
            Message::MarketLoaded(market, result) => {
                let state = self.market_mut(market);
                match result {
                    Ok(snapshot) => {
                        state.board = Board::new(snapshot);
                        state.load = Load::Ready;
                        if let Some(request) = self.pending_open.take_if(|r| r.matches(market)) {
                            return self.select(market, &request.symbol, request.timeframe);
                        }
                        return self.open_default_chart(market);
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
                if matches!(self.dialog, Some(Dialog::Symbols { .. })) {
                    self.dialog = None;
                }
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
                self.confirm_clear = false;
                let edit = match message {
                    ChartMessage::EditDrawing(id) => Some(id),
                    _ => None,
                };
                if let Some(selection) = &mut self.selection {
                    selection.chart.update(message);
                }
                if let Some(id) = edit {
                    self.open_drawing_settings(id);
                }
                return Task::batch([self.load_older_if_needed(), self.save_drawings()]);
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
            Message::OpenSymbolSearch(text) => return self.open_symbol_search(text),
            Message::KeyTyped(text) => {
                // Typing on the chart starts a search, as in TradingView.
                if self.dialog.is_none() {
                    return self.open_symbol_search(text);
                }
            }
            Message::SymbolSearchChanged(text) => {
                if let Some(Dialog::Symbols {
                    search, highlight, ..
                }) = &mut self.dialog
                {
                    *search = text;
                    *highlight = 0;
                }
                return operation::snap_to(dialogs::SYMBOL_RESULTS_ID, top());
            }
            Message::SymbolSearchKind(chosen) => {
                if let Some(Dialog::Symbols {
                    kind, highlight, ..
                }) = &mut self.dialog
                {
                    *kind = chosen;
                    *highlight = 0;
                }
                // Typing can go on after a click on a tab.
                return Task::batch([
                    operation::snap_to(dialogs::SYMBOL_RESULTS_ID, top()),
                    operation::focus(dialogs::SYMBOL_SEARCH_ID),
                ]);
            }
            Message::SymbolSearchMove(step) => {
                let Some((search, kind, _)) = self.symbol_search() else {
                    return Task::none();
                };
                let count = self.symbol_results(&search, kind).len().min(SEARCH_RESULTS);
                if let Some(Dialog::Symbols { highlight, .. }) = &mut self.dialog {
                    *highlight = highlight
                        .saturating_add_signed(step)
                        .min(count.saturating_sub(1));
                    // Keeps the picked result in view, about where it is in the list.
                    let share = *highlight as f32 / count.saturating_sub(1).max(1) as f32;
                    let offset = operation::RelativeOffset {
                        x: None,
                        y: Some(share),
                    };
                    return operation::snap_to(dialogs::SYMBOL_RESULTS_ID, offset);
                }
            }
            Message::SymbolSearchSubmit => {
                let picked = self.symbol_search().and_then(|(search, kind, highlight)| {
                    self.symbol_results(&search, kind)
                        .get(highlight)
                        .map(|found| (found.symbol.market, found.symbol.name.clone()))
                });
                if let Some((market, name)) = picked {
                    return self.update(Message::SymbolSelected(market, name));
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
                return operation::focus(dialogs::INDICATOR_SEARCH_ID);
            }
            Message::CloseDialog => self.close_dialog(),
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
            Message::Escape => {
                self.confirm_clear = false;
                if self.tool_menu.take().is_some() {
                    return Task::none();
                }
                // A palette or the template menu of a drawing's settings closes first.
                if let Some(Dialog::Drawing(window)) = &mut self.dialog
                    && (window.picker.is_some() || window.templates_open)
                {
                    window.picker = None;
                    window.templates_open = false;
                    window.template_name = None;
                    return Task::none();
                }
                if self.dialog.is_some() {
                    self.close_dialog();
                } else if let Some(selection) = &mut self.selection {
                    selection.chart.update(ChartMessage::Cancel);
                }
            }
            Message::ToolMenu(group) => {
                self.confirm_clear = false;
                self.tool_menu = group;
            }
            Message::ToolSelected(tool) => {
                // A shortcut typed in a window is not for the chart behind it.
                if self.dialog.is_some() {
                    return Task::none();
                }
                self.confirm_clear = false;
                self.tool_menu = None;
                if let Some(kind) = tool
                    && let Some(group) = ToolGroup::of(kind)
                {
                    self.group_tools.insert(group, kind);
                }
                if let Some(selection) = &mut self.selection {
                    selection.chart.set_tool(tool);
                }
            }
            Message::MagnetToggled => {
                if let Some(selection) = &mut self.selection {
                    let on = !selection.chart.magnet();
                    selection.chart.set_magnet(on);
                }
            }
            Message::DrawingsHiddenToggled => {
                if let Some(selection) = &mut self.selection {
                    let hidden = !selection.chart.drawings_hidden();
                    selection.chart.set_drawings_hidden(hidden);
                }
            }
            Message::DrawingsLockToggled => {
                if let Some(selection) = &mut self.selection {
                    let locked = !selection.chart.all_locked();
                    selection.chart.lock_all(locked);
                }
                return self.save_drawings();
            }
            Message::ClearDrawings => {
                if !std::mem::replace(&mut self.confirm_clear, true) {
                    return Task::none();
                }
                self.confirm_clear = false;
                if let Some(selection) = &mut self.selection {
                    selection.chart.clear_drawings();
                }
                return self.save_drawings();
            }
            Message::DrawingColor(color) => {
                return self.edit_drawing(|drawing| drawing.style.color = color);
            }
            Message::DrawingWidth(width) => {
                return self.edit_drawing(|drawing| drawing.style.width = width);
            }
            Message::DrawingLine(line) => {
                return self.edit_drawing(|drawing| drawing.style.line = line);
            }
            Message::DrawingText(text) => return self.edit_drawing(|drawing| drawing.text = text),
            Message::OpenDrawingSettings => {
                if let Some(id) = self.selected_drawing().map(|drawing| drawing.id) {
                    self.open_drawing_settings(id);
                }
            }
            Message::DrawingTab(tab) => {
                if let Some(Dialog::Drawing(window)) = &mut self.dialog {
                    window.tab = tab;
                    window.picker = None;
                    window.templates_open = false;
                }
            }
            Message::DrawingSetting(setting) => {
                if let (Some(Dialog::Drawing(window)), Some(selection)) =
                    (&mut self.dialog, &mut self.selection)
                {
                    selection
                        .chart
                        .edit_open(|drawing| window.apply(drawing, setting));
                }
            }
            Message::DrawingPicker(target) => {
                if let Some(Dialog::Drawing(window)) = &mut self.dialog {
                    window.picker = target;
                    window.templates_open = false;
                }
            }
            Message::DrawingSettingsDone => {
                if matches!(self.dialog, Some(Dialog::Drawing(_))) {
                    self.dialog = None;
                    if let Some(selection) = &mut self.selection {
                        selection.chart.finish_edit(true);
                    }
                    return self.save_drawings();
                }
            }
            Message::DrawingLook(look) => return self.drawing_look(look),
            Message::DrawingLooks(result) => match result {
                Ok(looks) => {
                    if let Some(selection) = &mut self.selection {
                        selection.chart.set_drawing_defaults(looks.defaults.clone());
                    }
                    self.drawing_looks = looks;
                }
                Err(error) => self.show_drawing_error(error),
            },
            Message::DrawingLookSaved(result) => {
                if let Err(error) = result {
                    self.show_drawing_error(error);
                }
            }
            Message::DrawingLockToggled => {
                return self.edit_drawing(|drawing| drawing.locked = !drawing.locked);
            }
            Message::DuplicateDrawing => {
                if let Some(selection) = &mut self.selection {
                    selection.chart.duplicate_selected();
                }
                return self.save_drawings();
            }
            Message::DeleteDrawing => {
                // Delete in a window is not for the chart behind it.
                if self.dialog.is_none()
                    && let Some(selection) = &mut self.selection
                {
                    selection.chart.delete_selected();
                }
                return self.save_drawings();
            }
            Message::Undo | Message::Redo => {
                if self.dialog.is_none()
                    && let Some(selection) = &mut self.selection
                {
                    if matches!(message, Message::Undo) {
                        selection.chart.undo();
                    } else {
                        selection.chart.redo();
                    }
                }
                return self.save_drawings();
            }
            Message::DrawingsLoaded(market, name, result) => {
                let showing = self.selection.as_mut().filter(|selection| {
                    selection.symbol.market == market && selection.symbol.name == name
                });
                match result {
                    Ok(drawings) => {
                        // Changes made meanwhile are newer than what was read.
                        if let Entry::Vacant(unknown) = self.drawings.known.entry((market, name)) {
                            if let Some(selection) = showing {
                                selection.chart.set_drawings(drawings.clone());
                            }
                            unknown.insert(drawings);
                        }
                    }
                    Err(error) => {
                        if let Some(selection) = showing {
                            selection.error = Some(AppError::user_data(error));
                        }
                    }
                }
            }
            Message::DrawingsSaved(result) => {
                self.drawings.saving = false;
                if let Err(error) = result
                    && let Some(selection) = &mut self.selection
                {
                    selection.error = Some(AppError::user_data(error));
                }
                return self.next_drawing_save();
            }
        }
        Task::none()
    }

    fn selected_drawing(&self) -> Option<&Drawing> {
        self.selection.as_ref()?.chart.selected_drawing()
    }

    /// Changes the picked drawing and saves the change.
    fn edit_drawing(&mut self, change: impl FnOnce(&mut Drawing)) -> Task<Message> {
        if let Some(selection) = &mut self.selection {
            selection.chart.edit_selected(change);
        }
        self.save_drawings()
    }

    /// Opens the settings of drawing `id`, if the chart has it and no window is open.
    fn open_drawing_settings(&mut self, id: u64) {
        if self.dialog.is_some() {
            return;
        }
        if let Some(selection) = &mut self.selection
            && selection.chart.begin_edit(id)
        {
            self.tool_menu = None;
            self.confirm_clear = false;
            self.dialog = Some(Dialog::Drawing(DrawingWindow::default()));
        }
    }

    /// Closes the window open. A drawing's settings put the drawing back as it was.
    fn close_dialog(&mut self) {
        if let Some(Dialog::Drawing(_)) = self.dialog.take()
            && let Some(selection) = &mut self.selection
        {
            selection.chart.finish_edit(false);
        }
    }

    /// Does what was picked in the template menu of a drawing's settings.
    fn drawing_look(&mut self, look: Look) -> Task<Message> {
        let (Some(Dialog::Drawing(window)), Some(selection)) =
            (&mut self.dialog, &mut self.selection)
        else {
            return Task::none();
        };
        let Some(drawing) = selection.chart.edited_drawing() else {
            return Task::none();
        };
        let (kind, style) = (drawing.kind, drawing.style.clone());
        match look {
            Look::Menu(open) => {
                window.templates_open = open;
                window.template_name = None;
                window.picker = None;
                return Task::none();
            }
            Look::Naming(name) => {
                let first = window.template_name.replace(name).is_none();
                return if first {
                    operation::focus(drawing_dialog::TEMPLATE_NAME_ID)
                } else {
                    Task::none()
                };
            }
            _ => {}
        }
        // Anything else picked closes the menu.
        window.templates_open = false;
        let name = window.template_name.take();
        let looks = &mut self.drawing_looks;
        let restyle = match &look {
            Look::Apply(name) => looks
                .templates
                .iter()
                .find(|template| template.kind == kind && template.name == *name)
                .map(|template| template.style.clone()),
            Look::ApplyDefault => Some(selection.chart.default_style(kind)),
            Look::Fitil => Some(DrawingStyle::of(kind)),
            _ => None,
        };
        if let Some(style) = restyle {
            selection.chart.edit_open(|drawing| drawing.style = style);
            window.drafts.clear();
            return Task::none();
        }
        // Looks are kept for this run even when the user's data cannot be written.
        let user = self.user_data.as_ref().ok().cloned();
        match look {
            Look::Save => {
                let Some(name) = name.as_deref().and_then(Template::clean_name) else {
                    return Task::none();
                };
                let template = DrawingTemplate { kind, name, style };
                looks
                    .templates
                    .retain(|saved| saved.kind != kind || saved.name != template.name);
                looks.templates.push(template.clone());
                looks.templates.sort_by_cached_key(|template| {
                    (template.kind.id(), template.name.to_lowercase())
                });
                user.map_or_else(Task::none, |user| {
                    Task::perform(
                        fitil_market::save_drawing_template(user, template),
                        Message::DrawingLookSaved,
                    )
                })
            }
            Look::Delete(name) => {
                looks
                    .templates
                    .retain(|saved| saved.kind != kind || saved.name != name);
                user.map_or_else(Task::none, |user| {
                    Task::perform(
                        fitil_market::delete_drawing_template(user, kind, name),
                        Message::DrawingLookSaved,
                    )
                })
            }
            Look::SaveDefault => {
                looks.defaults.insert(kind, style.clone());
                selection.chart.set_drawing_defaults(looks.defaults.clone());
                user.map_or_else(Task::none, |user| {
                    Task::perform(
                        fitil_market::save_drawing_default(user, kind, Some(style)),
                        Message::DrawingLookSaved,
                    )
                })
            }
            _ => Task::none(),
        }
    }

    /// Shows on the chart that drawings or their looks could not be read or saved.
    fn show_drawing_error(&mut self, error: MarketError) {
        if let Some(selection) = &mut self.selection {
            selection.error = Some(AppError::user_data(error));
        }
    }

    /// Saves the chart's drawings if they changed. Saves run one at a time and in order,
    /// so that an older one never overwrites a newer one.
    fn save_drawings(&mut self) -> Task<Message> {
        let Some(selection) = &mut self.selection else {
            return Task::none();
        };
        let Some(drawings) = selection.chart.take_changed_drawings() else {
            return Task::none();
        };
        let (market, name) = (selection.symbol.market, selection.symbol.name.clone());
        self.drawings
            .known
            .insert((market, name.clone()), drawings.clone());
        // A save of the same symbol still waiting takes the newer drawings instead.
        let saves = &mut self.drawings.waiting;
        match saves.iter_mut().find(|(waiting_market, waiting_name, _)| {
            *waiting_market == market && *waiting_name == name
        }) {
            Some(waiting) => waiting.2 = drawings,
            None => saves.push((market, name, drawings)),
        }
        self.next_drawing_save()
    }

    fn next_drawing_save(&mut self) -> Task<Message> {
        if self.drawings.saving || self.drawings.waiting.is_empty() {
            return Task::none();
        }
        let Ok(user) = &self.user_data else {
            self.drawings.waiting.clear();
            return Task::none();
        };
        let (market, name, drawings) = self.drawings.waiting.remove(0);
        self.drawings.saving = true;
        Task::perform(
            fitil_market::save_drawings(user.clone(), market, name, drawings),
            Message::DrawingsSaved,
        )
    }

    /// Shows the drawings of the chart's symbol: from memory if it was opened before,
    /// otherwise once read from the user's data.
    fn show_drawings(&mut self) -> Task<Message> {
        let Some(selection) = &mut self.selection else {
            return Task::none();
        };
        let (market, name) = (selection.symbol.market, selection.symbol.name.clone());
        if let Some(drawings) = self.drawings.known.get(&(market, name.clone())) {
            selection.chart.set_drawings(drawings.clone());
            return Task::none();
        }
        selection.chart.set_drawings(Vec::new());
        let Ok(user) = &self.user_data else {
            return Task::none();
        };
        Task::perform(
            fitil_market::load_drawings(user.clone(), market, name.clone()),
            move |result| Message::DrawingsLoaded(market, name.clone(), result),
        )
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

    /// What the chart's exchange provides.
    fn features(&self) -> Features {
        self.exchange(self.chart_market().exchange).features()
    }

    /// The market of the chart, or the first market while no chart is open.
    fn chart_market(&self) -> Market {
        self.selection.as_ref().map_or_else(
            || {
                *self
                    .markets
                    .keys()
                    .next()
                    .expect("every exchange has a market")
            },
            |selection| selection.symbol.market,
        )
    }

    /// Opens a chart once the first market has loaded, so the window does not start
    /// empty, unless one is open or asked for already.
    fn open_default_chart(&mut self, market: Market) -> Task<Message> {
        let first = self.markets.keys().next() == Some(&market);
        if !first || self.selection.is_some() || self.pending_open.is_some() {
            return Task::none();
        }
        let Some(name) = self.markets[&market]
            .board
            .default_symbol()
            .map(|symbol| symbol.name.clone())
        else {
            return Task::none();
        };
        let timeframe = DEFAULT_TIMEFRAME.parse().expect("valid default");
        self.select(market, &name, timeframe)
    }

    fn open_symbol_search(&mut self, search: String) -> Task<Message> {
        // Typing with a tool menu open starts a search too; the menu gives way.
        self.tool_menu = None;
        self.dialog = Some(Dialog::Symbols {
            search,
            kind: None,
            highlight: 0,
        });
        operation::focus(dialogs::SYMBOL_SEARCH_ID)
    }

    /// What the symbol search holds: the text, the kind of market and the picked result.
    fn symbol_search(&self) -> Option<(String, Option<MarketKind>, usize)> {
        match &self.dialog {
            Some(Dialog::Symbols {
                search,
                kind,
                highlight,
            }) => Some((search.clone(), *kind, *highlight)),
            _ => None,
        }
    }

    /// The symbols matching `search` in the loaded markets of `kind` (all when `None`),
    /// closest first and then most traded.
    fn symbol_results(&self, search: &str, kind: Option<MarketKind>) -> Vec<Match<'_>> {
        let boards = self
            .markets
            .iter()
            .filter(|(market, state)| {
                kind.is_none_or(|kind| market.kind == kind) && matches!(state.load, Load::Ready)
            })
            .map(|(_, state)| &state.board);
        fitil_market::search_markets(boards, search)
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
        // Drawings belong to the symbol: a timeframe of the same symbol keeps them.
        let same_symbol = self
            .selection
            .as_ref()
            .is_some_and(|current| current.symbol == symbol);
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
                chart.set_drawing_defaults(self.drawing_looks.defaults.clone());
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
        let drawings = if same_symbol {
            Task::none()
        } else {
            self.show_drawings()
        };
        Task::batch([self.open_series(), drawings])
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

/// Keys that act anywhere in the window: Esc closes a dialog, the arrows move through
/// search results, and a letter or digit typed outside any text box starts a symbol
/// search.
fn key_message(
    event: iced::Event,
    status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    let iced::Event::Keyboard(keyboard::Event::KeyPressed {
        key,
        text,
        modifiers,
        ..
    }) = event
    else {
        return None;
    };
    match key.as_ref() {
        keyboard::Key::Named(Named::Escape) => Some(Message::Escape),
        keyboard::Key::Named(Named::ArrowUp) => Some(Message::SymbolSearchMove(-1)),
        keyboard::Key::Named(Named::ArrowDown) => Some(Message::SymbolSearchMove(1)),
        // Keys a text box took are not for the chart.
        _ if status == iced::event::Status::Captured => None,
        keyboard::Key::Named(Named::Delete | Named::Backspace) => Some(Message::DeleteDrawing),
        keyboard::Key::Character(c) if modifiers.command() && c.eq_ignore_ascii_case("z") => {
            Some(if modifiers.shift() {
                Message::Redo
            } else {
                Message::Undo
            })
        }
        keyboard::Key::Character(c) if modifiers.command() && c.eq_ignore_ascii_case("y") => {
            Some(Message::Redo)
        }
        // Drawing tools, as in TradingView: Alt+T for a trend line and so on.
        keyboard::Key::Character(c) if modifiers.alt() && !modifiers.command() => {
            tools::by_shortcut(c).map(|kind| Message::ToolSelected(Some(kind)))
        }
        _ if modifiers.command() || modifiers.alt() => None,
        _ => text
            .filter(|text| text.chars().all(char::is_alphanumeric))
            .map(|text| Message::KeyTyped(text.to_string())),
    }
}

/// The top of a list.
fn top() -> operation::RelativeOffset<Option<f32>> {
    operation::RelativeOffset {
        x: None,
        y: Some(0.0),
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
        hour_suffix: t.get("chart-hour-suffix"),
        minute_suffix: t.get("chart-minute-suffix"),
        loading: t.get("chart-loading-older"),
        note: t.get("chart-note"),
        bars: t.get("chart-bars"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use fitil_store::{Store, UserData};

    /// The app with one Binance spot symbol, its candle cache and user data in memory:
    /// no files and no network.
    /// Three spot pairs: BTCUSDT, then ETHUSDT and ETHBTC, the first trading the most.
    fn snapshot(market: Market) -> MarketSnapshot {
        let pairs = [
            ("BTCUSDT", "BTC", "USDT", 77_000.0, 2e9),
            ("ETHUSDT", "ETH", "USDT", 2_200.0, 1e9),
            ("ETHBTC", "ETH", "BTC", 0.03, 1_000.0), // 1,000 BTC: $77 million
        ];
        MarketSnapshot {
            symbols: pairs
                .iter()
                .map(|&(name, base, quote, _, _)| Symbol {
                    market,
                    name: name.into(),
                    base_asset: base.into(),
                    quote_asset: quote.into(),
                    price_decimals: 2,
                })
                .collect(),
            tickers: pairs
                .iter()
                .map(|&(name, _, _, price, volume)| fitil_core::Ticker {
                    symbol: name.into(),
                    last_price: price,
                    open_price: price,
                    high_price: price,
                    low_price: price,
                    quote_volume: volume,
                })
                .collect(),
        }
    }

    fn app() -> (Fitil, Market) {
        let exchanges = exchanges();
        let market = Market {
            exchange: exchanges[0].id(),
            kind: MarketKind::Spot,
        };
        let state = MarketState {
            load: Load::Ready,
            board: Board::new(snapshot(market)),
            feed: None,
            connection: Connection::Connected,
        };
        let app = Fitil {
            translations: Translations::new(Language::English),
            cache: Ok(Arc::new(Mutex::new(Store::open_in_memory().unwrap()))),
            user_data: Ok(Arc::new(Mutex::new(UserData::open_in_memory().unwrap()))),
            exchanges,
            markets: BTreeMap::from([(market, state)]),
            selection: None,
            chart_kind: ChartKind::default(),
            price_scale: PriceScale::default(),
            show_volume: true,
            studies: Studies::default(),
            templates: Vec::new(),
            templates_error: None,
            dialog: None,
            pending_open: None,
            drawings: DrawingSaves::default(),
            confirm_clear: false,
            drawing_looks: DrawingLooks::default(),
            tool_menu: None,
            group_tools: HashMap::new(),
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
        let _ = app.update(Message::OpenSymbolSearch(String::new()));
        let _ = app.view();
        let _ = app.update(Message::SymbolSearchChanged("nothing like this".into()));
        let _ = app.view();
        let _ = app.update(Message::CloseDialog);
        assert!(app.dialog.is_none());
    }

    #[test]
    fn the_first_market_opens_a_bitcoin_chart() {
        let (mut plain, market) = app();
        assert!(plain.selection.is_none());
        let _ = plain.update(Message::MarketLoaded(market, Ok(snapshot(market))));
        assert_eq!(plain.selection.as_ref().unwrap().symbol.name, "BTCUSDT");

        // A chart asked for on the command line wins.
        let (mut asked, market) = app();
        asked.pending_open = OpenRequest::from_args(
            ["fitil", "--open", "spot:ETHBTC:4h"]
                .map(String::from)
                .into_iter(),
        );
        let _ = asked.update(Message::MarketLoaded(market, Ok(snapshot(market))));
        let selection = asked.selection.as_ref().unwrap();
        assert_eq!(selection.symbol.name, "ETHBTC");
        assert_eq!(selection.series.target().to_string(), "4h");
    }

    #[test]
    fn typing_on_the_chart_searches_symbols() {
        let (mut app, market) = app();
        open_chart(&mut app, market, 10);
        let _ = app.update(Message::KeyTyped("e".into()));
        assert!(matches!(&app.dialog, Some(Dialog::Symbols { search, .. }) if search == "e"));

        let _ = app.update(Message::SymbolSearchChanged("eth".into()));
        let (search, kind, _) = app.symbol_search().unwrap();
        let found: Vec<&str> = app
            .symbol_results(&search, kind)
            .iter()
            .map(|found| found.symbol.name.as_str())
            .collect();
        // Both Ether pairs, the more traded first.
        assert_eq!(found, ["ETHUSDT", "ETHBTC"]);

        // The arrows stop at the last result.
        let _ = app.update(Message::SymbolSearchMove(1));
        let _ = app.update(Message::SymbolSearchMove(5));
        assert!(matches!(
            app.dialog,
            Some(Dialog::Symbols { highlight: 1, .. })
        ));
        let _ = app.update(Message::SymbolSearchMove(-3));
        assert!(matches!(
            app.dialog,
            Some(Dialog::Symbols { highlight: 0, .. })
        ));
        let _ = app.update(Message::SymbolSearchMove(1));
        let _ = app.update(Message::SymbolSearchSubmit);
        assert!(app.dialog.is_none());
        let selection = app.selection.as_ref().unwrap();
        assert_eq!(selection.symbol.name, "ETHBTC");
        // The timeframe stays.
        assert_eq!(selection.series.target().to_string(), DEFAULT_TIMEFRAME);

        // Typing in another window types there and starts no search.
        let _ = app.update(Message::OpenIndicators);
        let _ = app.update(Message::KeyTyped("x".into()));
        assert!(matches!(app.dialog, Some(Dialog::Indicators { .. })));
    }

    #[test]
    fn tabs_narrow_the_search_to_a_kind_of_market() {
        let (mut app, _) = app();
        let _ = app.update(Message::OpenSymbolSearch("btc".into()));
        let _ = app.update(Message::SymbolSearchKind(Some(MarketKind::LinearPerpetual)));
        let (search, kind, _) = app.symbol_search().unwrap();
        assert_eq!(kind, Some(MarketKind::LinearPerpetual));
        // This app has only a spot market.
        assert!(app.symbol_results(&search, kind).is_empty());
        let _ = app.update(Message::SymbolSearchKind(None));
        let (search, kind, _) = app.symbol_search().unwrap();
        assert_eq!(app.symbol_results(&search, kind).len(), 2);
    }

    fn chart(app: &Fitil) -> &Chart {
        &app.selection.as_ref().unwrap().chart
    }

    /// Draws a horizontal line at `price` through the chart's messages.
    fn draw_line(app: &mut Fitil, price: f64) {
        let _ = app.update(Message::ToolSelected(Some(DrawingKind::HorizontalLine)));
        let anchor = fitil_core::Anchor {
            time: 3 * 3_600_000,
            price,
        };
        let _ = app.update(Message::Chart(ChartMessage::PlacePoint(anchor)));
    }

    #[test]
    fn drawings_are_drawn_changed_and_saved_in_order() {
        let (mut app, market) = app();
        open_chart(&mut app, market, 10);
        draw_line(&mut app, 105.0);
        assert_eq!(chart(&app).drawings().len(), 1);
        // The first save runs; the next change waits for it.
        assert!(app.drawings.saving);
        let _ = app.update(Message::DrawingColor(Rgb::hex(0xFF6D00)));
        let _ = app.update(Message::DrawingWidth(4));
        assert_eq!(
            app.drawings.waiting.len(),
            1,
            "waiting saves of a symbol merge"
        );
        assert_eq!(app.drawings.waiting[0].2[0].style.width, 4);
        let _ = app.update(Message::DrawingsSaved(Ok(())));
        assert!(app.drawings.saving && app.drawings.waiting.is_empty());

        let _ = app.update(Message::Undo);
        assert_eq!(chart(&app).drawings()[0].style.width, 2);
        let _ = app.update(Message::Redo);
        assert_eq!(chart(&app).drawings()[0].style.width, 4);

        // Deleting all takes a second click.
        let _ = app.update(Message::ClearDrawings);
        assert_eq!(chart(&app).drawings().len(), 1);
        let _ = app.update(Message::ClearDrawings);
        assert!(chart(&app).drawings().is_empty());

        // Esc puts the tool away.
        let _ = app.update(Message::ToolSelected(Some(DrawingKind::Ray)));
        let _ = app.update(Message::Escape);
        assert_eq!(chart(&app).tool(), None);
    }

    #[test]
    fn drawings_belong_to_their_symbol() {
        let (mut app, market) = app();
        open_chart(&mut app, market, 10);
        let _ = app.update(Message::DrawingsLoaded(
            market,
            "BTCUSDT".into(),
            Ok(Vec::new()),
        ));
        draw_line(&mut app, 105.0);

        // Another timeframe of the same symbol keeps them.
        let four_hours = "4h".parse().unwrap();
        let _ = app.update(Message::TimeframeSelected(four_hours));
        assert_eq!(chart(&app).drawings().len(), 1);

        // Another symbol has its own; going back shows the first one's again.
        let _ = app.update(Message::SymbolSelected(market, "ETHUSDT".into()));
        assert!(chart(&app).drawings().is_empty());
        let _ = app.update(Message::SymbolSelected(market, "BTCUSDT".into()));
        assert_eq!(chart(&app).drawings().len(), 1);

        // Drawings read late do not replace newer ones.
        let _ = app.update(Message::DrawingsLoaded(
            market,
            "BTCUSDT".into(),
            Ok(Vec::new()),
        ));
        assert_eq!(chart(&app).drawings().len(), 1);
    }

    #[test]
    fn notes_are_typed_in() {
        let (mut app, market) = app();
        open_chart(&mut app, market, 10);
        let _ = app.update(Message::ToolSelected(Some(DrawingKind::Text)));
        let anchor = fitil_core::Anchor {
            time: 2 * 3_600_000,
            price: 104.0,
        };
        let _ = app.update(Message::Chart(ChartMessage::PlacePoint(anchor)));
        let _ = app.update(Message::DrawingText("Destek".into()));
        assert_eq!(chart(&app).selected_drawing().unwrap().text, "Destek");
        let _ = app.view();
    }

    /// Draws a Fibonacci retracement from 100 at the first hour to 110 at the fifth.
    fn draw_fib(app: &mut Fitil) {
        let _ = app.update(Message::ToolSelected(Some(DrawingKind::FibRetracement)));
        for (hour, price) in [(1, 100.0), (5, 110.0)] {
            let anchor = fitil_core::Anchor {
                time: hour * 3_600_000,
                price,
            };
            let _ = app.update(Message::Chart(ChartMessage::PlacePoint(anchor)));
        }
    }

    fn drawing_window(app: &Fitil) -> &DrawingWindow {
        match &app.dialog {
            Some(Dialog::Drawing(window)) => window,
            _ => panic!("no drawing settings are open"),
        }
    }

    #[test]
    fn drawing_settings_show_changes_and_cancel_them() {
        use drawing_dialog::Field;
        let (mut app, market) = app();
        open_chart(&mut app, market, 10);
        draw_fib(&mut app);
        // A double click on the drawing opens its settings.
        let id = chart(&app).drawings()[0].id;
        let _ = app.update(Message::Chart(ChartMessage::EditDrawing(id)));
        assert_eq!(drawing_window(&app).tab, Tab::Style);
        let _ = app.view();

        let level = |app: &Fitil| chart(app).drawings()[0].style.levels[1];
        let _ = app.update(Message::DrawingPicker(Some(ColorTarget::Level(1))));
        let _ = app.view();
        let yellow = Rgb::hex(0xFFEB3B);
        let _ = app.update(Message::DrawingSetting(Setting::Color(
            ColorTarget::Level(1),
            yellow,
        )));
        assert_eq!(level(&app).color, yellow);
        assert_eq!(drawing_window(&app).picker, None);
        // With a comma before the decimals; what cannot be read changes nothing.
        let typed =
            |text: &str| Message::DrawingSetting(Setting::Typed(Field::Level(1), text.into()));
        let _ = app.update(typed("0,25"));
        assert_eq!(level(&app).value, 0.25);
        let _ = app.update(typed("0,2x"));
        assert_eq!(level(&app).value, 0.25);
        for tab in [Tab::Coordinates, Tab::Visibility] {
            let _ = app.update(Message::DrawingTab(tab));
            let _ = app.view();
        }
        // A tool's shortcut typed in the window is not for the chart.
        let _ = app.update(Message::ToolSelected(Some(DrawingKind::Ray)));
        assert_eq!(chart(&app).tool(), None);

        // Esc drops every change; nothing new is saved.
        let _ = app.update(Message::Escape);
        assert!(app.dialog.is_none());
        assert_eq!(
            chart(&app).drawings()[0].style,
            DrawingStyle::of(DrawingKind::FibRetracement)
        );
        assert!(app.drawings.waiting.is_empty());
    }

    #[test]
    fn drawing_settings_keep_their_changes_in_one_step() {
        use drawing_dialog::Field;
        let (mut app, market) = app();
        open_chart(&mut app, market, 10);
        draw_line(&mut app, 105.0);
        // The settings button of the picked drawing's row opens them too.
        let _ = app.update(Message::OpenDrawingSettings);
        let _ = app.update(Message::DrawingSetting(Setting::Width(4)));
        let _ = app.update(Message::DrawingSetting(Setting::Typed(
            Field::Price(0),
            "107,5".into(),
        )));
        let _ = app.update(Message::DrawingSettingsDone);
        assert!(app.dialog.is_none());
        let line = |app: &Fitil| {
            let drawing = &chart(app).drawings()[0];
            (drawing.style.width, drawing.points[0].price)
        };
        assert_eq!(line(&app), (4, 107.5));
        assert_eq!(app.drawings.waiting.len(), 1, "the change is saved");
        let _ = app.update(Message::Undo);
        assert_eq!(line(&app), (2, 105.0));
    }

    #[test]
    fn drawing_looks_are_saved_applied_and_given_to_new_drawings() {
        let (mut app, market) = app();
        open_chart(&mut app, market, 10);
        draw_line(&mut app, 105.0);
        let width = |app: &Fitil, index: usize| chart(app).drawings()[index].style.width;
        let _ = app.update(Message::OpenDrawingSettings);
        let _ = app.update(Message::DrawingSetting(Setting::Width(3)));

        // A look saved under a name, trimmed.
        let _ = app.update(Message::DrawingLook(Look::Menu(true)));
        let _ = app.update(Message::DrawingLook(Look::Naming(String::new())));
        let _ = app.update(Message::DrawingLook(Look::Naming("  Kalın ".into())));
        let _ = app.view();
        let _ = app.update(Message::DrawingLook(Look::Save));
        let templates = &app.drawing_looks.templates;
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].kind, DrawingKind::HorizontalLine);
        assert_eq!(templates[0].name, "Kalın");
        assert!(!drawing_window(&app).templates_open);

        // Applied back after a change; Fitil's look applied.
        let _ = app.update(Message::DrawingSetting(Setting::Width(1)));
        let _ = app.update(Message::DrawingLook(Look::Apply("Kalın".into())));
        assert_eq!(width(&app, 0), 3);
        let _ = app.update(Message::DrawingLook(Look::Fitil));
        assert_eq!(width(&app, 0), 2);

        // Saved as the default: going back to it gives it, and new lines start with it.
        let _ = app.update(Message::DrawingLook(Look::Apply("Kalın".into())));
        let _ = app.update(Message::DrawingLook(Look::SaveDefault));
        let _ = app.update(Message::DrawingLook(Look::Fitil));
        let _ = app.update(Message::DrawingLook(Look::ApplyDefault));
        assert_eq!(width(&app, 0), 3);
        let _ = app.update(Message::DrawingSettingsDone);
        draw_line(&mut app, 106.0);
        assert_eq!(width(&app, 1), 3);

        // Deleting the template keeps the default.
        let _ = app.update(Message::OpenDrawingSettings);
        let _ = app.update(Message::DrawingLook(Look::Menu(true)));
        let _ = app.view();
        let _ = app.update(Message::DrawingLook(Look::Delete("Kalın".into())));
        assert!(app.drawing_looks.templates.is_empty());
        assert!(
            app.drawing_looks
                .defaults
                .contains_key(&DrawingKind::HorizontalLine)
        );
    }

    #[test]
    fn drawing_settings_are_translated() {
        use fitil_core::{Extend, LabelAlign, LabelSide, TimeUnit};
        for language in Language::ALL {
            let t = Translations::new(language);
            let keys = [
                "drawing-settings",
                "drawing-tab-style",
                "drawing-tab-coordinates",
                "drawing-tab-visibility",
                "drawing-line",
                "drawing-levels-line",
                "drawing-words",
                "drawing-background",
                "drawing-opacity",
                "drawing-extend",
                "drawing-one-color",
                "drawing-reverse",
                "drawing-prices",
                "drawing-levels",
                "drawing-levels-values",
                "drawing-levels-percent",
                "drawing-labels",
                "drawing-measure-colors",
                "drawing-point",
                "drawing-price",
                "drawing-time",
                "drawing-time-placeholder",
                "drawing-visibility-limits",
                "drawing-cancel",
                "drawing-ok",
                "drawing-template",
                "drawing-template-save-as",
                "drawing-template-name",
                "drawing-template-save",
                "drawing-default-save",
                "drawing-default-apply",
                "drawing-fitil-look",
            ]
            .map(String::from)
            .into_iter()
            .chain(Extend::ALL.map(|extend| format!("drawing-extend-{}", extend.id())))
            .chain(LabelAlign::ALL.map(|align| format!("drawing-label-{}", align.id())))
            .chain(LabelSide::ALL.map(|side| format!("drawing-label-{}", side.id())))
            .chain(TimeUnit::ALL.map(|unit| format!("drawing-visibility-{}", unit.id())));
            for key in keys {
                assert_ne!(t.get(&key), key, "{} has no {key}", language.code());
            }
        }
    }

    #[test]
    fn every_drawing_tool_is_translated() {
        for language in Language::ALL {
            let t = Translations::new(language);
            let groups = ToolGroup::ALL.iter().flat_map(|group| {
                std::iter::once(group.key())
                    .chain(group.sections().iter().map(|section| section.key))
                    .map(String::from)
            });
            let keys = DrawingKind::ALL
                .map(tools::name_key)
                .into_iter()
                .chain(LineStyle::ALL.map(|style| format!("drawing-line-{}", style.id())))
                .chain(groups)
                .chain(["drawing-shortcut".to_owned()]);
            for key in keys {
                assert_ne!(t.get(&key), key, "{} has no {key}", language.code());
            }
            let shortcut = t.get_with("drawing-shortcut", &[("key", "T".into())]);
            assert_eq!(shortcut, "Alt + T");
        }
    }

    #[test]
    fn tool_menus_open_pick_and_close() {
        let (mut app, market) = app();
        open_chart(&mut app, market, 10);
        let _ = app.update(Message::ToolMenu(Some(ToolGroup::Lines)));
        assert_eq!(app.tool_menu, Some(ToolGroup::Lines));
        let _ = app.view();

        // Picking a tool closes the menu; the group's button shows that tool from then on.
        let _ = app.update(Message::ToolSelected(Some(DrawingKind::Pitchfork)));
        assert_eq!(app.tool_menu, None);
        assert_eq!(chart(&app).tool(), Some(DrawingKind::Pitchfork));
        assert_eq!(
            app.group_tools.get(&ToolGroup::Lines),
            Some(&DrawingKind::Pitchfork)
        );
        let _ = app.view();

        // Esc closes an open menu before it puts the tool away.
        let _ = app.update(Message::ToolMenu(Some(ToolGroup::Lines)));
        let _ = app.update(Message::Escape);
        assert_eq!(app.tool_menu, None);
        assert_eq!(chart(&app).tool(), Some(DrawingKind::Pitchfork));

        // Typing starts a search, and an open menu gives way to it.
        let _ = app.update(Message::ToolMenu(Some(ToolGroup::Lines)));
        let _ = app.update(Message::KeyTyped("e".into()));
        assert_eq!(app.tool_menu, None);
        assert!(app.dialog.is_some());
    }

    #[test]
    fn alt_and_a_letter_pick_a_tool() {
        let press = |letter: &str, modifiers| {
            let event = keyboard::Event::KeyPressed {
                key: keyboard::Key::Character(letter.into()),
                modified_key: keyboard::Key::Character(letter.into()),
                physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyT),
                location: keyboard::Location::Standard,
                modifiers,
                text: None,
                repeat: false,
            };
            key_message(
                iced::Event::Keyboard(event),
                iced::event::Status::Ignored,
                iced::window::Id::unique(),
            )
        };
        let alt = keyboard::Modifiers::ALT;
        assert!(matches!(
            press("t", alt),
            Some(Message::ToolSelected(Some(DrawingKind::TrendLine)))
        ));
        assert!(matches!(
            press("c", alt),
            Some(Message::ToolSelected(Some(DrawingKind::CrossLine)))
        ));
        assert!(press("q", alt).is_none());
        assert!(press("t", alt | keyboard::Modifiers::COMMAND).is_none());
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
