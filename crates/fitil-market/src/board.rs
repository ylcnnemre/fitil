//! The watchlist's data: a market's symbols and their latest 24-hour statistics.

use std::collections::HashMap;
use std::sync::Arc;

use fitil_core::{MarketKind, Symbol, Ticker};
use fitil_exchange::Exchange;

use crate::MarketError;

/// A market's symbols and their 24-hour statistics, as first loaded.
#[derive(Debug, Clone)]
pub struct MarketSnapshot {
    pub symbols: Vec<Symbol>,
    pub tickers: Vec<Ticker>,
}

/// Measures the exchange's clock and fetches the symbols and statistics of a market.
pub async fn load_market(
    exchange: Arc<dyn Exchange>,
    kind: MarketKind,
) -> Result<MarketSnapshot, MarketError> {
    exchange.sync_clock().await?;
    let symbols = exchange.symbols(kind).await?;
    let tickers = exchange.tickers(kind).await?;
    Ok(MarketSnapshot { symbols, tickers })
}

/// One market's symbols with their latest statistics.
#[derive(Debug, Default)]
pub struct Board {
    symbols: HashMap<String, Symbol>,
    tickers: HashMap<String, Ticker>,
}

impl Board {
    pub fn new(snapshot: MarketSnapshot) -> Self {
        let mut board = Self {
            symbols: snapshot
                .symbols
                .into_iter()
                .map(|symbol| (symbol.name.clone(), symbol))
                .collect(),
            tickers: HashMap::new(),
        };
        board.update(snapshot.tickers);
        board
    }

    pub fn symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    /// Takes in new statistics; ones for symbols not trading here are ignored.
    pub fn update(&mut self, tickers: Vec<Ticker>) {
        for ticker in tickers {
            if self.symbols.contains_key(&ticker.symbol) {
                self.tickers.insert(ticker.symbol.clone(), ticker);
            }
        }
    }

    /// Statistics of the symbols containing `search` (in any case), most traded first.
    ///
    /// Pairs are quoted in many currencies (USDT, TRY, IDR, BTC…), so volumes are compared
    /// in dollars, using the market's own prices.
    pub fn ranked(&self, search: &str) -> Vec<&Ticker> {
        let search = search.trim().to_uppercase();
        let mut rates: HashMap<&str, f64> = HashMap::new();
        let mut ranked: Vec<(f64, &Ticker)> = self
            .tickers
            .values()
            .filter(|ticker| ticker.symbol.to_uppercase().contains(&search))
            .map(|ticker| {
                let quote = self
                    .symbols
                    .get(&ticker.symbol)
                    .map_or("", |symbol| symbol.quote_asset.as_str());
                let rate = *rates
                    .entry(quote)
                    .or_insert_with(|| usd_rate(quote, &self.tickers).unwrap_or(0.0));
                (ticker.quote_volume * rate, ticker)
            })
            .collect();
        ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
        ranked.into_iter().map(|(_, ticker)| ticker).collect()
    }
}

/// Dollars per unit of `asset`, from the market's own `ASSETUSDT` or `USDTASSET` price.
fn usd_rate(asset: &str, tickers: &HashMap<String, Ticker>) -> Option<f64> {
    const DOLLARS: [&str; 6] = ["USDT", "USDC", "FDUSD", "TUSD", "USD1", "DAI"];
    if DOLLARS.contains(&asset) {
        return Some(1.0);
    }
    let price = |name: String| {
        tickers
            .get(&name)
            .map(|ticker| ticker.last_price)
            .filter(|price| *price > 0.0)
    };
    price(format!("{asset}USDT")).or_else(|| price(format!("USDT{asset}")).map(|p| 1.0 / p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FAKE;
    use fitil_core::Market;

    fn symbol(name: &str, base: &str, quote: &str) -> Symbol {
        Symbol {
            market: Market {
                exchange: FAKE,
                kind: MarketKind::Spot,
            },
            name: name.into(),
            base_asset: base.into(),
            quote_asset: quote.into(),
            price_decimals: 2,
        }
    }

    fn ticker(symbol: &str, last_price: f64, quote_volume: f64) -> Ticker {
        Ticker {
            symbol: symbol.into(),
            last_price,
            open_price: last_price,
            high_price: last_price,
            low_price: last_price,
            quote_volume,
        }
    }

    fn board() -> Board {
        Board::new(MarketSnapshot {
            symbols: vec![
                symbol("BTCUSDT", "BTC", "USDT"),
                symbol("ETHBTC", "ETH", "BTC"),
                symbol("USDTIDR", "USDT", "IDR"),
                symbol("BTCIDR", "BTC", "IDR"),
            ],
            tickers: vec![
                ticker("BTCUSDT", 77_000.0, 2_000_000_000.0), // $2 billion
                ticker("ETHBTC", 0.03, 20_000.0),             // 20k BTC: $1.54 billion
                ticker("USDTIDR", 18_000.0, 1_000_000_000_000.0), // 1 trillion IDR: $55 million
                ticker("BTCIDR", 1_380_000_000.0, 100_000_000_000.0), // $5.6 million
                ticker("DELISTED", 1.0, 1e12),
            ],
        })
    }

    fn names(tickers: &[&Ticker]) -> Vec<String> {
        tickers.iter().map(|t| t.symbol.clone()).collect()
    }

    #[test]
    fn volumes_are_compared_in_dollars() {
        assert_eq!(
            names(&board().ranked("")),
            ["BTCUSDT", "ETHBTC", "USDTIDR", "BTCIDR"]
        );
    }

    #[test]
    fn search_matches_any_part_in_any_case() {
        assert_eq!(names(&board().ranked(" idr ")), ["USDTIDR", "BTCIDR"]);
        assert!(board().ranked("XYZ").is_empty());
    }

    #[test]
    fn updates_only_touch_known_symbols() {
        let mut board = board();
        board.update(vec![
            ticker("BTCUSDT", 78_000.0, 1.0),
            ticker("NEWCOIN", 1.0, 1.0),
        ]);
        let all = board.ranked("");
        assert_eq!(all.len(), 4);
        assert_eq!(board.ranked("BTCUSDT")[0].last_price, 78_000.0);
        assert!(board.symbol("NEWCOIN").is_none());
    }
}
