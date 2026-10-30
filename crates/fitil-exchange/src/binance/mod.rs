//! Binance Spot and USDⓈ-M perpetual futures market data.

mod feed;
mod wire;

pub use feed::{FeedCommand, FeedEvent, Topic, run_feed};

use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fitil_core::{Candle, Market, Symbol, Ticker, TimeUnit, Timeframe};
use serde::de::DeserializeOwned;

use crate::{ExchangeError, RateLimiter};

/// Most candles requested at once. Futures allow 1500, but 1000 costs less weight per candle.
pub const MAX_CANDLES_PER_REQUEST: usize = 1000;

struct Endpoints {
    rest: &'static str,
    api: &'static str,
    stream: &'static str,
    weight_limit: u32,
    exchange_info_weight: u32,
    tickers_weight: u32,
    candles_weight: u32,
}

/// Addresses and request weights from Binance's API documentation.
fn endpoints(market: Market) -> Endpoints {
    match market {
        Market::Spot => Endpoints {
            rest: "https://api.binance.com",
            api: "/api/v3",
            stream: "wss://stream.binance.com:9443/stream",
            weight_limit: 6000,
            exchange_info_weight: 20,
            tickers_weight: 80,
            candles_weight: 2,
        },
        Market::UsdMFutures => Endpoints {
            rest: "https://fapi.binance.com",
            api: "/fapi/v1",
            // Since 2026-04-23, futures market streams are only served under `/market`.
            stream: "wss://fstream.binance.com/market/stream",
            weight_limit: 2400,
            exchange_info_weight: 1,
            tickers_weight: 40,
            candles_weight: 5,
        },
    }
}

const fn tf(count: u32, unit: TimeUnit) -> Timeframe {
    Timeframe::new(count, unit).unwrap()
}

static TIMEFRAMES: [Timeframe; 16] = [
    tf(1, TimeUnit::Second),
    tf(1, TimeUnit::Minute),
    tf(3, TimeUnit::Minute),
    tf(5, TimeUnit::Minute),
    tf(15, TimeUnit::Minute),
    tf(30, TimeUnit::Minute),
    tf(1, TimeUnit::Hour),
    tf(2, TimeUnit::Hour),
    tf(4, TimeUnit::Hour),
    tf(6, TimeUnit::Hour),
    tf(8, TimeUnit::Hour),
    tf(12, TimeUnit::Hour),
    tf(1, TimeUnit::Day),
    tf(3, TimeUnit::Day),
    tf(1, TimeUnit::Week),
    tf(1, TimeUnit::Month),
];

/// Timeframes Binance serves directly; other ones are built from these.
pub fn native_timeframes(market: Market) -> &'static [Timeframe] {
    match market {
        Market::Spot => &TIMEFRAMES,
        // Futures have no 1-second candles.
        Market::UsdMFutures => &TIMEFRAMES[1..],
    }
}

/// The tradable symbols of a market.
#[derive(Debug, Clone)]
pub struct ExchangeInfo {
    pub symbols: Vec<Symbol>,
    /// Request weight allowed per minute.
    pub weight_limit: u32,
}

/// REST client for one Binance market.
#[derive(Debug)]
pub struct BinanceClient {
    market: Market,
    http: reqwest::Client,
    limiter: RateLimiter,
    /// Exchange clock minus local clock, in milliseconds.
    clock_offset: AtomicI64,
}

impl BinanceClient {
    pub fn new(market: Market) -> Result<Self, ExchangeError> {
        crate::install_crypto_provider();
        let http = reqwest::Client::builder()
            .user_agent(concat!("Fitil/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self {
            market,
            http,
            limiter: RateLimiter::new(endpoints(market).weight_limit),
            clock_offset: AtomicI64::new(0),
        })
    }

    pub fn market(&self) -> Market {
        self.market
    }

    /// Current time on the exchange's clock, in Unix milliseconds.
    pub fn now(&self) -> i64 {
        local_now() + self.clock_offset()
    }

    /// Exchange clock minus local clock, in milliseconds, as of the last [`Self::sync_clock`].
    pub fn clock_offset(&self) -> i64 {
        self.clock_offset.load(Ordering::Relaxed)
    }

    /// Request weight used in the current minute, and the limit.
    pub fn weight_usage(&self) -> (u32, u32) {
        self.limiter.usage(self.now())
    }

    /// Measures how far the local clock is from the exchange's and returns the offset.
    pub async fn sync_clock(&self) -> Result<i64, ExchangeError> {
        let before = local_now();
        let time: wire::ServerTime = self.get("/time", &[], 1).await?;
        let after = local_now();
        let offset = time.server_time - (before + after) / 2;
        self.clock_offset.store(offset, Ordering::Relaxed);
        Ok(offset)
    }

    /// Symbols that are currently trading (for futures: perpetual contracts only).
    pub async fn exchange_info(&self) -> Result<ExchangeInfo, ExchangeError> {
        let weight = endpoints(self.market).exchange_info_weight;
        let info: wire::ExchangeInfo = self.get("/exchangeInfo", &[], weight).await?;
        let info = info.into_exchange_info(self.market);
        self.limiter.set_limit(info.weight_limit);
        Ok(info)
    }

    /// 24-hour statistics for every symbol.
    pub async fn tickers(&self) -> Result<Vec<Ticker>, ExchangeError> {
        let weight = endpoints(self.market).tickers_weight;
        let tickers: Vec<wire::Ticker24h> = self.get("/ticker/24hr", &[], weight).await?;
        tickers
            .into_iter()
            .map(wire::Ticker24h::into_ticker)
            .collect()
    }

    /// Up to `limit` candles opening between `start` and `end` (both optional, inclusive),
    /// oldest first. With only `end`, these are the newest candles opening at or before it.
    ///
    /// `timeframe` must be one of [`native_timeframes`]. The newest candle may still be open.
    pub async fn candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        start: Option<i64>,
        end: Option<i64>,
        limit: usize,
    ) -> Result<Vec<Candle>, ExchangeError> {
        let mut query = vec![
            ("symbol", symbol.to_owned()),
            ("interval", timeframe.to_string()),
            ("limit", limit.clamp(1, MAX_CANDLES_PER_REQUEST).to_string()),
        ];
        if let Some(start) = start {
            query.push(("startTime", start.to_string()));
        }
        if let Some(end) = end {
            query.push(("endTime", end.to_string()));
        }
        let weight = endpoints(self.market).candles_weight;
        let klines: Vec<wire::Kline> = self.get("/klines", &query, weight).await?;
        klines.into_iter().map(wire::Kline::into_candle).collect()
    }

    async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
        weight: u32,
    ) -> Result<T, ExchangeError> {
        let wait = self.limiter.delay(self.now(), weight);
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }

        let endpoints = endpoints(self.market);
        let mut url = format!("{}{}{path}", endpoints.rest, endpoints.api);
        for (i, (name, value)) in query.iter().enumerate() {
            url.push(if i == 0 { '?' } else { '&' });
            url.push_str(name);
            url.push('=');
            url.push_str(value);
        }

        let response = self.http.get(url).send().await?;
        let now = self.now();
        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
        };
        if let Some(used) = header("x-mbx-used-weight-1m") {
            self.limiter.record(now, used as u32);
        }

        let status = response.status();
        if status.is_success() {
            let body = response.bytes().await?;
            return serde_json::from_slice(&body).map_err(|e| ExchangeError::Parse(e.to_string()));
        }
        match status.as_u16() {
            451 => Err(ExchangeError::RestrictedLocation),
            // 429: slow down. 418: the IP is temporarily banned for not slowing down.
            418 | 429 => {
                let retry_after = Duration::from_secs(header("retry-after").unwrap_or(60));
                self.limiter
                    .block_until(now + retry_after.as_millis() as i64);
                Err(ExchangeError::RateLimited { retry_after })
            }
            status => {
                let body = response.text().await.unwrap_or_default();
                let message = serde_json::from_str::<wire::ApiError>(&body)
                    .map(|error| error.msg)
                    .unwrap_or(body);
                Err(ExchangeError::Api { status, message })
            }
        }
    }
}

fn local_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn futures_have_no_one_second_candles() {
        let spot: Vec<String> = native_timeframes(Market::Spot)
            .iter()
            .map(|t| t.to_string())
            .collect();
        let futures: Vec<String> = native_timeframes(Market::UsdMFutures)
            .iter()
            .map(|t| t.to_string())
            .collect();
        assert_eq!(spot.first().map(String::as_str), Some("1s"));
        assert_eq!(futures.first().map(String::as_str), Some("1m"));
        assert_eq!(spot.len(), futures.len() + 1);
        assert_eq!(spot.last().map(String::as_str), Some("1M"));
    }
}
