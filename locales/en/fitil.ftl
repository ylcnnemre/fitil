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
chart-hint = Wheel: zoom · Drag: pan · Drag the price axis: vertical scale · Drag the time axis: horizontal scale · Drag a pane's edge: pane height · Double-click: reset
chart-indicators = Indicators
chart-volume-toggle = Volume
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

error-restricted = This exchange is not available from your location.
error-rate-limited = The exchange reported too many requests. Wait { $seconds } s and try again.
error-network = Could not reach the exchange. Check your internet connection.
error-exchange = The exchange returned an error: { $detail }
error-storage = The local cache is unavailable: { $detail }
error-user-data = Your saved data (templates) is unavailable: { $detail }

## Indicator list and templates

indicators-title = Indicators
indicators-search = Search indicators…
indicators-empty = No matching indicators.
templates-title = Templates
templates-hint = Save the chart's indicators with their settings under a name, then apply them to another chart in one click.
templates-name = Template name
templates-save = Save
templates-apply = Apply
templates-delete = Delete
templates-confirm-delete = Sure?
templates-empty = No saved templates yet.

## One indicator's settings

study-settings-title = { $name } settings
study-inputs = Inputs
study-style = Style
study-width = Width
study-show = Show
study-hide = Hide
study-remove = Remove
study-defaults = Defaults
study-done = Done

## Indicators: long name (in the list) and short name (on the chart)

indicator-sma = Simple Moving Average (SMA)
indicator-sma-short = SMA
indicator-ema = Exponential Moving Average (EMA)
indicator-ema-short = EMA
indicator-wma = Weighted Moving Average (WMA)
indicator-wma-short = WMA
indicator-vwma = Volume-Weighted Moving Average (VWMA)
indicator-vwma-short = VWMA
indicator-bollinger = Bollinger Bands
indicator-bollinger-short = BB
indicator-keltner = Keltner Channels
indicator-keltner-short = KC
indicator-donchian = Donchian Channels
indicator-donchian-short = DC
indicator-supertrend = SuperTrend
indicator-supertrend-short = SuperTrend
indicator-parabolic-sar = Parabolic SAR
indicator-parabolic-sar-short = SAR
indicator-dmi = ADX / DMI (Directional Movement)
indicator-dmi-short = DMI
indicator-ichimoku = Ichimoku Cloud
indicator-ichimoku-short = Ichimoku
indicator-rsi = Relative Strength Index (RSI)
indicator-rsi-short = RSI
indicator-macd = MACD
indicator-macd-short = MACD
indicator-stochastic = Stochastic
indicator-stochastic-short = Stoch
indicator-stoch-rsi = Stochastic RSI
indicator-stoch-rsi-short = Stoch RSI
indicator-cci = Commodity Channel Index (CCI)
indicator-cci-short = CCI
indicator-atr = Average True Range (ATR)
indicator-atr-short = ATR
indicator-volume = Volume
indicator-volume-short = Volume
indicator-buy-sell-volume = Buy/Sell Volume
indicator-buy-sell-volume-short = Buy/Sell
indicator-cvd = Cumulative Volume Delta (CVD)
indicator-cvd-short = CVD
indicator-obv = On-Balance Volume (OBV)
indicator-obv-short = OBV
indicator-vwap = Volume-Weighted Average Price (VWAP)
indicator-vwap-short = VWAP

indicator-group-moving-averages = Moving averages
indicator-group-bands = Bands and channels
indicator-group-trend = Trend
indicator-group-momentum = Momentum
indicator-group-volatility = Volatility
indicator-group-volume = Volume

indicator-param-length = Length
indicator-param-source = Source
indicator-param-multiplier = Multiplier
indicator-param-atr-length = ATR length
indicator-param-factor = Factor
indicator-param-start = Start
indicator-param-increment = Increment
indicator-param-maximum = Maximum
indicator-param-di-length = DI length
indicator-param-adx-smoothing = ADX smoothing
indicator-param-conversion = Conversion line length
indicator-param-base = Base line length
indicator-param-span-b = Leading span B length
indicator-param-displacement = Displacement
indicator-param-fast = Fast length
indicator-param-slow = Slow length
indicator-param-signal = Signal length
indicator-param-k-length = %K length
indicator-param-k-smoothing = %K smoothing
indicator-param-d-smoothing = %D smoothing
indicator-param-rsi-length = RSI length
indicator-param-stoch-length = Stochastic length
indicator-param-anchor = Resets every

indicator-output-value = Value
indicator-output-basis = Basis
indicator-output-upper = Upper band
indicator-output-lower = Lower band
indicator-output-up = Up trend
indicator-output-down = Down trend
indicator-output-adx = ADX
indicator-output-plus-di = +DI
indicator-output-minus-di = −DI
indicator-output-conversion = Conversion line
indicator-output-base = Base line
indicator-output-lagging = Lagging span
indicator-output-lead-a = Leading span A
indicator-output-lead-b = Leading span B
indicator-output-histogram = Histogram
indicator-output-macd = MACD
indicator-output-signal = Signal
indicator-output-k = %K
indicator-output-d = %D
indicator-output-buy = Buy
indicator-output-sell = Sell

indicator-source-open = Open
indicator-source-high = High
indicator-source-low = Low
indicator-source-close = Close
indicator-source-hl2 = hl2
indicator-source-hlc3 = hlc3
indicator-source-ohlc4 = ohlc4

indicator-choice-day = Day
indicator-choice-week = Week
indicator-choice-month = Month
indicator-choice-none = Never
