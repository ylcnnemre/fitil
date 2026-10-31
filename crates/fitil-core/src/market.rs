use std::fmt;

use crate::Timeframe;

/// Identifies an exchange, such as `binance`. Used as a key in the cache, so it never changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExchangeId(&'static str);

impl ExchangeId {
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }

    pub fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for ExchangeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// A kind of market an exchange can offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MarketKind {
    Spot,
    /// Perpetual futures margined and settled in a stablecoin such as USDT or USDC.
    LinearPerpetual,
}

impl MarketKind {
    /// Stable identifier used in cache keys.
    pub fn id(self) -> &'static str {
        match self {
            MarketKind::Spot => "spot",
            MarketKind::LinearPerpetual => "perp",
        }
    }
}

/// One market of one exchange, like "Binance spot". Each is a tab of the watchlist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Market {
    pub exchange: ExchangeId,
    pub kind: MarketKind,
}

/// A tradable instrument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub market: Market,
    /// The exchange's own name for it: `BTCUSDT` on Binance, `BTC-USDT-SWAP` on OKX.
    pub name: String,
    pub base_asset: String,
    pub quote_asset: String,
    /// Decimal places prices are shown with, taken from the exchange's tick size.
    pub price_decimals: u32,
}

/// Rolling 24-hour statistics for a symbol.
#[derive(Debug, Clone, PartialEq)]
pub struct Ticker {
    pub symbol: String,
    pub last_price: f64,
    /// Price 24 hours ago.
    pub open_price: f64,
    pub high_price: f64,
    pub low_price: f64,
    /// 24-hour traded amount in the quote asset.
    pub quote_volume: f64,
}

impl Ticker {
    /// Change over the last 24 hours, in percent.
    pub fn change_percent(&self) -> f64 {
        if self.open_price == 0.0 {
            0.0
        } else {
            (self.last_price - self.open_price) / self.open_price * 100.0
        }
    }
}

/// Identifies one stored candle series.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SeriesKey {
    pub market: Market,
    pub symbol: String,
    pub timeframe: Timeframe,
}

/// Decimal places in a step size written as a decimal string, such as `"0.01000000"` → 2.
pub fn decimals_from_step(step: &str) -> u32 {
    let trimmed = step.trim();
    match trimmed.split_once('.') {
        Some((_, fraction)) => fraction.trim_end_matches('0').len() as u32,
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimals_come_from_the_step() {
        assert_eq!(decimals_from_step("0.01000000"), 2);
        assert_eq!(decimals_from_step("0.10"), 1);
        assert_eq!(decimals_from_step("0.00000001"), 8);
        assert_eq!(decimals_from_step("1.00000000"), 0);
        assert_eq!(decimals_from_step("10"), 0);
    }

    #[test]
    fn change_percent_handles_zero_open() {
        let mut ticker = Ticker {
            symbol: "BTCUSDT".into(),
            last_price: 110.0,
            open_price: 100.0,
            high_price: 120.0,
            low_price: 90.0,
            quote_volume: 0.0,
        };
        assert!((ticker.change_percent() - 10.0).abs() < 1e-9);
        ticker.open_price = 0.0;
        assert_eq!(ticker.change_percent(), 0.0);
    }
}
