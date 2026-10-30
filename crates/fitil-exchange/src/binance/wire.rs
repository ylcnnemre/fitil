//! Binance's JSON formats and their conversion into Fitil's types.

use fitil_core::{Candle, Market, Symbol, Ticker, Timeframe, decimals_from_step};
use serde::Deserialize;
use serde::de::IgnoredAny;

use super::{ExchangeInfo as Info, endpoints};
use crate::ExchangeError;

fn number(field: &str, value: &str) -> Result<f64, ExchangeError> {
    value
        .parse()
        .map_err(|_| ExchangeError::Parse(format!("{field} is not a number: {value:?}")))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ServerTime {
    pub server_time: i64,
}

#[derive(Deserialize)]
pub(super) struct ApiError {
    pub msg: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ExchangeInfo {
    rate_limits: Vec<RateLimit>,
    symbols: Vec<SymbolInfo>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimit {
    rate_limit_type: String,
    interval: String,
    interval_num: u32,
    limit: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SymbolInfo {
    symbol: String,
    status: String,
    base_asset: String,
    quote_asset: String,
    /// Futures only: `PERPETUAL`, `TRADIFI_PERPETUAL`, `CURRENT_QUARTER`, ...
    contract_type: Option<String>,
    filters: Vec<Filter>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Filter {
    filter_type: String,
    tick_size: Option<String>,
}

impl ExchangeInfo {
    pub fn into_exchange_info(self, market: Market) -> Info {
        let weight_limit = self
            .rate_limits
            .iter()
            .find(|limit| {
                limit.rate_limit_type == "REQUEST_WEIGHT"
                    && limit.interval == "MINUTE"
                    && limit.interval_num == 1
            })
            .map_or(endpoints(market).weight_limit, |limit| limit.limit);

        let symbols = self
            .symbols
            .into_iter()
            .filter(|symbol| symbol.status == "TRADING")
            .filter(|symbol| match market {
                Market::Spot => true,
                Market::UsdMFutures => symbol
                    .contract_type
                    .as_deref()
                    .is_some_and(|kind| kind.ends_with("PERPETUAL")),
            })
            .map(|symbol| {
                let price_decimals = symbol
                    .filters
                    .iter()
                    .find(|filter| filter.filter_type == "PRICE_FILTER")
                    .and_then(|filter| filter.tick_size.as_deref())
                    .map_or(2, decimals_from_step);
                Symbol {
                    market,
                    name: symbol.symbol,
                    base_asset: symbol.base_asset,
                    quote_asset: symbol.quote_asset,
                    price_decimals,
                }
            })
            .collect();

        Info {
            symbols,
            weight_limit,
        }
    }
}

/// `[open time, open, high, low, close, volume, close time, quote volume, trades,
/// taker buy base volume, taker buy quote volume, ignore]`
#[derive(Deserialize)]
pub(super) struct Kline(
    i64,
    String,
    String,
    String,
    String,
    String,
    IgnoredAny,
    String,
    u64,
    String,
    IgnoredAny,
    IgnoredAny,
);

impl Kline {
    pub fn into_candle(self) -> Result<Candle, ExchangeError> {
        Ok(Candle {
            open_time: self.0,
            open: number("open", &self.1)?,
            high: number("high", &self.2)?,
            low: number("low", &self.3)?,
            close: number("close", &self.4)?,
            volume: number("volume", &self.5)?,
            quote_volume: number("quote volume", &self.7)?,
            trades: self.8,
            taker_buy_volume: number("taker buy volume", &self.9)?,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Ticker24h {
    symbol: String,
    last_price: String,
    open_price: String,
    high_price: String,
    low_price: String,
    quote_volume: String,
}

impl Ticker24h {
    pub fn into_ticker(self) -> Result<Ticker, ExchangeError> {
        Ok(Ticker {
            last_price: number("last price", &self.last_price)?,
            open_price: number("open price", &self.open_price)?,
            high_price: number("high price", &self.high_price)?,
            low_price: number("low price", &self.low_price)?,
            quote_volume: number("quote volume", &self.quote_volume)?,
            symbol: self.symbol,
        })
    }
}

/// A message on a combined stream connection.
#[derive(Deserialize)]
pub(super) struct Envelope {
    pub stream: String,
    pub data: serde_json::Value,
}

#[derive(Deserialize)]
pub(super) struct MiniTicker {
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "c")]
    close: String,
    #[serde(rename = "o")]
    open: String,
    #[serde(rename = "h")]
    high: String,
    #[serde(rename = "l")]
    low: String,
    #[serde(rename = "q")]
    quote_volume: String,
}

impl MiniTicker {
    pub fn into_ticker(self) -> Result<Ticker, ExchangeError> {
        Ok(Ticker {
            last_price: number("close", &self.close)?,
            open_price: number("open", &self.open)?,
            high_price: number("high", &self.high)?,
            low_price: number("low", &self.low)?,
            quote_volume: number("quote volume", &self.quote_volume)?,
            symbol: self.symbol,
        })
    }
}

#[derive(Deserialize)]
pub(super) struct KlineEvent {
    #[serde(rename = "k")]
    kline: StreamKline,
}

#[derive(Deserialize)]
struct StreamKline {
    #[serde(rename = "t")]
    open_time: i64,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "i")]
    interval: String,
    #[serde(rename = "o")]
    open: String,
    #[serde(rename = "h")]
    high: String,
    #[serde(rename = "l")]
    low: String,
    #[serde(rename = "c")]
    close: String,
    #[serde(rename = "v")]
    volume: String,
    #[serde(rename = "q")]
    quote_volume: String,
    #[serde(rename = "n")]
    trades: u64,
    #[serde(rename = "V")]
    taker_buy_volume: String,
    #[serde(rename = "x")]
    closed: bool,
}

/// A candle update from the stream.
pub(super) struct StreamCandle {
    pub symbol: String,
    pub timeframe: Timeframe,
    pub candle: Candle,
    pub closed: bool,
}

impl KlineEvent {
    pub fn into_stream_candle(self) -> Result<StreamCandle, ExchangeError> {
        let k = self.kline;
        let timeframe = k
            .interval
            .parse()
            .map_err(|_| ExchangeError::Parse(format!("unknown interval {:?}", k.interval)))?;
        Ok(StreamCandle {
            candle: Candle {
                open_time: k.open_time,
                open: number("open", &k.open)?,
                high: number("high", &k.high)?,
                low: number("low", &k.low)?,
                close: number("close", &k.close)?,
                volume: number("volume", &k.volume)?,
                quote_volume: number("quote volume", &k.quote_volume)?,
                trades: k.trades,
                taker_buy_volume: number("taker buy volume", &k.taker_buy_volume)?,
            },
            symbol: k.symbol,
            timeframe,
            closed: k.closed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPOT_INFO: &str = include_str!("../../tests/fixtures/binance/spot_exchange_info.json");
    const FUTURES_INFO: &str =
        include_str!("../../tests/fixtures/binance/futures_exchange_info.json");
    const SPOT_KLINES: &str = include_str!("../../tests/fixtures/binance/spot_klines.json");
    const FUTURES_KLINES: &str = include_str!("../../tests/fixtures/binance/futures_klines.json");
    const SPOT_TICKERS: &str = include_str!("../../tests/fixtures/binance/spot_ticker_24hr.json");
    const FUTURES_TICKERS: &str =
        include_str!("../../tests/fixtures/binance/futures_ticker_24hr.json");

    fn names(info: &Info) -> Vec<&str> {
        info.symbols.iter().map(|s| s.name.as_str()).collect()
    }

    #[test]
    fn spot_symbols_skip_halted_ones() {
        let raw: ExchangeInfo = serde_json::from_str(SPOT_INFO).unwrap();
        let info = raw.into_exchange_info(Market::Spot);

        assert_eq!(info.weight_limit, 6000);
        assert_eq!(names(&info), ["BTCUSDT", "ETHBTC"]); // NEOBTC is in BREAK status
        let btc = &info.symbols[0];
        assert_eq!(
            (btc.base_asset.as_str(), btc.quote_asset.as_str()),
            ("BTC", "USDT")
        );
        assert_eq!(btc.price_decimals, 2);
        assert_eq!(info.symbols[1].price_decimals, 5);
    }

    #[test]
    fn futures_symbols_are_perpetuals() {
        let raw: ExchangeInfo = serde_json::from_str(FUTURES_INFO).unwrap();
        let info = raw.into_exchange_info(Market::UsdMFutures);

        assert_eq!(info.weight_limit, 2400);
        assert_eq!(names(&info), ["BTCUSDT", "ETHUSDT", "XAUUSDT"]);
        assert_eq!(info.symbols[0].price_decimals, 1);
        assert!(info.symbols.iter().all(|s| s.market == Market::UsdMFutures));
    }

    #[test]
    fn klines_become_candles() {
        for json in [SPOT_KLINES, FUTURES_KLINES] {
            let klines: Vec<Kline> = serde_json::from_str(json).unwrap();
            let candles: Vec<Candle> = klines
                .into_iter()
                .map(|k| k.into_candle().unwrap())
                .collect();
            assert_eq!(candles.len(), 3);
            assert!(
                candles
                    .windows(2)
                    .all(|w| w[1].open_time - w[0].open_time == 3_600_000)
            );
            for c in &candles {
                assert!(c.low <= c.open && c.open <= c.high);
                assert!(c.low <= c.close && c.close <= c.high);
                assert!(c.taker_buy_volume <= c.volume);
                assert!(c.trades > 0);
            }
        }

        let klines: Vec<Kline> = serde_json::from_str(SPOT_KLINES).unwrap();
        let first = klines.into_iter().next().unwrap().into_candle().unwrap();
        assert_eq!(first.open_time, 1_793_383_200_000);
        assert_eq!(first.open, 77192.46);
        assert_eq!(first.trades, 227_428);
    }

    #[test]
    fn rest_tickers_parse() {
        for json in [SPOT_TICKERS, FUTURES_TICKERS] {
            let raw: Vec<Ticker24h> = serde_json::from_str(json).unwrap();
            let tickers: Vec<Ticker> = raw.into_iter().map(|t| t.into_ticker().unwrap()).collect();
            assert_eq!(tickers.len(), 2);
            assert!(
                tickers
                    .iter()
                    .all(|t| t.last_price > 0.0 && t.quote_volume > 0.0)
            );
        }
    }

    #[test]
    fn bad_numbers_are_reported() {
        let kline: Kline =
            serde_json::from_str(r#"[1, "x", "1", "1", "1", "1", 2, "1", 3, "1", "1", "0"]"#)
                .unwrap();
        match kline.into_candle() {
            Err(ExchangeError::Parse(message)) => assert!(message.contains("open")),
            other => panic!("expected a parse error, got {other:?}"),
        }
    }
}
