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

detail-empty = Pick a symbol on the left to see its details.
detail-timeframe = Timeframe
detail-source = Built from { $source } candles
detail-open-candle = Open candle
detail-ohlc = O { $open }   H { $high }   L { $low }   C { $close }
detail-closes-in = Closes in: { $time }
detail-waiting = Waiting for live data…
detail-cached = { $count } candles cached ({ $timeframe })
detail-built = { $count } candles ({ $timeframe }) built from them
detail-oldest = Oldest: { $time }
detail-newest = Newest: { $time }
detail-complete = The full history of this symbol is downloaded.
detail-last-load = Last load: { $cached } from the cache, { $downloaded } from Binance
detail-load-older = Load older candles
detail-loading = Loading…

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
