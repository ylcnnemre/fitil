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
- `--open spot:BTCUSDT:1h` (or `futures:…`) opens a chart at start.
