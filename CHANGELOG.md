# Changelog

All notable changes to Fitil are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Project skeleton: a Cargo workspace and an empty application window with a Turkish and English UI.
- Design document ([docs/TASARIM.md](docs/TASARIM.md)).
- Continuous integration on Windows, macOS and Linux, with dependency license checks.
- Binance Spot and USDⓈ-M perpetual market data:
  - symbol lists, 24-hour tickers and candle history,
  - live tickers and candles over WebSocket, with automatic reconnects,
  - request-weight tracking and clock-offset correction.
- Local SQLite candle cache. Each candle is downloaded once; gaps after a reconnect are filled.
- Custom timeframes (such as 45m, 3h, 2d) built from Binance's own timeframes.
- Watchlist with live prices sorted by dollar volume and symbol search.
- Chart engine (`fitil-chart`), drawn from scratch on the GPU through iced's canvas:
  - candles, OHLC bars, line, area and Heikin Ashi charts with volume,
  - linear, logarithmic and percent price scales,
  - zoom with the mouse wheel, pan by dragging, scale by dragging either axis,
    double-click to reset,
  - crosshair with price and time labels, a legend with the hovered candle's values,
    and the last price with a countdown to the candle's close,
  - older candles load automatically when the chart is moved toward the past.
- `--open [exchange:]market:SYMBOL[:timeframe]`, such as `--open binance:futures:ETHUSDT:4h`,
  opens a chart at start.
- An exchange picker in the top bar, ready for more exchanges.
- `FITIL_DATA_DIR` moves the data folder (for tests and portable use).
- Indicators (`fitil-indicators`), with no limit on how many a chart shows:
  - SMA, EMA, WMA, VWMA, Bollinger Bands, Keltner and Donchian channels, SuperTrend,
    Parabolic SAR, ADX/DMI, Ichimoku Cloud, RSI, MACD, Stochastic, Stochastic RSI, CCI,
    ATR, Volume, Buy/Sell Volume, CVD, OBV and anchored VWAP;
  - checked against published reference values, and computed incrementally: a live
    update recomputes only the newest candle;
  - buy/sell volume and CVD are offered only on exchanges that report taker volume.
- Indicator panes under the price chart, resized by dragging their edges, with their own
  scales, guide lines (such as RSI's 70 and 30) and legends; a crosshair across all panes.
- Indicators over the price are kept in view when the price range is fitted, and can
  shade the area between two lines (Bollinger Bands, the Ichimoku cloud).
- An indicator list with search, and a settings window for each indicator: parameters,
  source, and each line's color, width and visibility.
- Indicator templates: the chart's indicators saved under a name and applied to another
  chart in one click. They live in a separate `user.sqlite`, so clearing the candle cache
  keeps them.
- A button that shows or hides the volume bars under the candles.
- A symbol search window, opened from the symbol's name in the toolbar or by typing on
  the chart: tabs per kind of market, the closest matches first and then the most
  traded, with each symbol's price, daily change and exchange; arrow keys and Enter
  pick a result.
- A chart opens at start on the most traded Bitcoin pair.
- Drawing tools in a bar left of the chart: trend line, ray, extended line, horizontal
  line and ray, vertical line, rectangle, parallel channel, Fibonacci retracement with
  editable levels, text notes and a measure of price change, candles and time.
  - Drawings are placed by clicking or dragging, picked and moved whole or by a point,
    copied, locked and deleted; a magnet sticks points to candles' prices.
  - Color, width and line style (solid, dashed, dotted) are set in a row above the chart;
    horizontal lines show their price on the axis.
  - Undo and redo (Ctrl+Z, Ctrl+Y) in the toolbar; Esc lets go of the tool or the
    drawing, Delete removes it.
  - Every drawing of a symbol can be hidden, locked or deleted at once.
  - Drawings are pinned to times and prices, so they show in every timeframe, and are
    saved per symbol in `user.sqlite`.
- Icons from the Lucide set (ISC license), built into the program.
- The drawing bar groups related tools as TradingView does: the lines button opens a menu
  beside it, in sections for lines, channels and pitchforks, with each tool's picture,
  name and shortcut. The button shows the tool picked last.
  - New lines: info line (price change, candles, time and angle), trend angle and cross
    line (its price and date on the axes; vertical lines show their date too).
  - New channels: regression trend (the closes' least-squares line with bands two
    standard deviations away, and Pearson's R), flat top/bottom and disjoint channel.
  - Pitchforks: Andrews', Schiff, modified Schiff and inside.
  - Alt+T, Alt+H, Alt+J, Alt+V and Alt+C pick the trend line, horizontal line, horizontal
    ray, vertical line and cross line.

### Changed

- Ports-and-adapters architecture, so more exchanges and modules can be added without
  touching the rest (see [docs/TASARIM.md](docs/TASARIM.md) §6):
  - `fitil-exchange` is now the exchange interface every adapter implements, with a
    shared reconnecting WebSocket loop, rate limiter and clock;
  - Binance moved into its own adapter crate, `fitil-binance`;
  - loading, live-update and watchlist logic moved out of the window into a new
    application layer, `fitil-market`, tested with a fake exchange.
- Candles' quote volume, trade count and taker volume are optional, since not every
  exchange reports them.
- The candle cache keys include the exchange. Existing caches are upgraded in place.
- The window is laid out like TradingView: the chart fills it, under a toolbar with the
  symbol, favorite timeframes as buttons, the chart type, scales, indicators and volume.
  The watchlist on the left is replaced by the symbol search window.

### Fixed

- Axis labels no longer show through the crosshair's and the last price's boxes.
- Dashed and dotted drawings with more than one line, such as Fibonacci levels and
  channels, no longer get stray lines joining their lines.
