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

### Fixed

- Axis labels no longer show through the crosshair's and the last price's boxes.
