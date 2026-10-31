# Fitil — English UI strings.
# Every key here must also exist in the other languages' fitil.ftl files.

app-title = Fitil
app-version = Version { $version }

language-tr = Türkçe
language-en = English

market-spot = Spot
market-futures = Futures (USDⓈ-M)

search-placeholder = Search symbols…

watchlist-symbol = Symbol
watchlist-price = Price
watchlist-change = 24h change
watchlist-loading = Loading symbols…
watchlist-empty = No matching symbols.
watchlist-retry = Try again
watchlist-count = { $shown } of { $total } symbols, sorted by volume

detail-empty = Pick a symbol on the left to see its chart.
detail-source = built from { $source } candles
detail-cached = { $count } candles cached ({ $timeframe })
detail-complete = full history loaded
detail-loading = Loading…

chart-kind-candles = Candles
chart-kind-bars = Bars (OHLC)
chart-kind-line = Line
chart-kind-area = Area
chart-kind-heikin-ashi = Heikin Ashi
scale-log = Log
scale-percent = %
chart-hint = Wheel: zoom · Drag: pan · Drag the price axis: vertical scale · Drag the time axis: horizontal scale · Double-click: reset
chart-open = O
chart-high = H
chart-low = L
chart-close = C
chart-volume = Vol
chart-months = Jan, Feb, Mar, Apr, May, Jun, Jul, Aug, Sep, Oct, Nov, Dec
chart-magnitudes = K, M, B
chart-day-suffix = d
chart-loading-older = Loading older candles…

status-connecting = Connecting to live data…
status-connected = Live data connected
status-reconnecting = Connection lost, retrying in { $seconds } s (attempt { $attempt })
status-weight = Request weight: { $used } / { $limit }
status-clock = Clock offset: { $seconds } s
status-cache = Cache: { $path }

error-restricted = Binance is not available from your location.
error-rate-limited = Binance reported too many requests. Wait { $seconds } s and try again.
error-network = Could not reach Binance. Check your internet connection.
error-exchange = Binance returned an error: { $detail }
error-storage = The local cache is unavailable: { $detail }
