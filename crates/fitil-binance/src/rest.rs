//! Binance's REST API for one market.

use std::sync::Arc;
use std::time::Duration;

use fitil_core::{Candle, Market, MarketKind, Symbol, Ticker, Timeframe};
use fitil_exchange::{CandleQuery, Clock, ExchangeError, RateLimiter, RequestUsage, local_now};
use serde::de::DeserializeOwned;

use crate::{ID, MAX_CANDLES, wire};

struct Endpoint {
    base: &'static str,
    api: &'static str,
    weight_limit: u32,
    exchange_info_weight: u32,
    tickers_weight: u32,
    candles_weight: u32,
}

/// Addresses and request weights from Binance's API documentation.
fn endpoint(market: MarketKind) -> Endpoint {
    match market {
        MarketKind::Spot => Endpoint {
            base: "https://api.binance.com",
            api: "/api/v3",
            weight_limit: 6000,
            exchange_info_weight: 20,
            tickers_weight: 80,
            candles_weight: 2,
        },
        MarketKind::LinearPerpetual => Endpoint {
            base: "https://fapi.binance.com",
            api: "/fapi/v1",
            weight_limit: 2400,
            exchange_info_weight: 1,
            tickers_weight: 40,
            candles_weight: 5,
        },
    }
}

/// REST client for one Binance market. Spot and futures have separate servers and
/// separate request-weight limits.
#[derive(Debug)]
pub struct RestClient {
    market: MarketKind,
    http: reqwest::Client,
    clock: Arc<Clock>,
    limiter: RateLimiter,
}

impl RestClient {
    pub fn new(market: MarketKind, http: reqwest::Client, clock: Arc<Clock>) -> Self {
        Self {
            market,
            http,
            clock,
            limiter: RateLimiter::new(endpoint(market).weight_limit, Duration::from_secs(60)),
        }
    }

    pub fn usage(&self) -> RequestUsage {
        self.limiter.usage(self.clock.now())
    }

    pub async fn sync_clock(&self) -> Result<(), ExchangeError> {
        let sent = local_now();
        let time: wire::ServerTime = self.get("/time", &[], 1).await?;
        self.clock.record(sent, time.server_time, local_now());
        Ok(())
    }

    /// Symbols that are currently trading (for futures: perpetual contracts only).
    pub async fn symbols(&self) -> Result<Vec<Symbol>, ExchangeError> {
        let weight = endpoint(self.market).exchange_info_weight;
        let info: wire::ExchangeInfo = self.get("/exchangeInfo", &[], weight).await?;
        let market = Market {
            exchange: ID,
            kind: self.market,
        };
        let (symbols, weight_limit) = info.into_symbols(market);
        if let Some(limit) = weight_limit {
            self.limiter.set_limit(limit);
        }
        Ok(symbols)
    }

    pub async fn tickers(&self) -> Result<Vec<Ticker>, ExchangeError> {
        let weight = endpoint(self.market).tickers_weight;
        let tickers: Vec<wire::Ticker24h> = self.get("/ticker/24hr", &[], weight).await?;
        tickers
            .into_iter()
            .map(wire::Ticker24h::into_ticker)
            .collect()
    }

    pub async fn candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        query: CandleQuery,
    ) -> Result<Vec<Candle>, ExchangeError> {
        let (start, end, limit) = match query {
            CandleQuery::Latest { limit } => (None, None, limit),
            CandleQuery::From { start, limit } => (Some(start), None, limit),
            CandleQuery::Until { end, limit } => (None, Some(end), limit),
        };
        let mut params = vec![
            ("symbol", symbol.to_owned()),
            ("interval", timeframe.to_string()),
            ("limit", limit.clamp(1, MAX_CANDLES).to_string()),
        ];
        if let Some(start) = start {
            params.push(("startTime", start.to_string()));
        }
        if let Some(end) = end {
            params.push(("endTime", end.to_string()));
        }
        let weight = endpoint(self.market).candles_weight;
        let klines: Vec<wire::Kline> = self.get("/klines", &params, weight).await?;
        klines.into_iter().map(wire::Kline::into_candle).collect()
    }

    async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, String)],
        weight: u32,
    ) -> Result<T, ExchangeError> {
        let wait = self.limiter.delay(self.clock.now(), weight);
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }

        let endpoint = endpoint(self.market);
        let mut url = format!("{}{}{path}", endpoint.base, endpoint.api);
        for (i, (name, value)) in params.iter().enumerate() {
            url.push(if i == 0 { '?' } else { '&' });
            url.push_str(name);
            url.push('=');
            url.push_str(value);
        }

        let response = self.http.get(url).send().await?;
        let now = self.clock.now();
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
