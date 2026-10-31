//! Finding symbols: each market's symbols and their latest 24-hour statistics.

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

/// A symbol found by a search, with its latest statistics.
#[derive(Debug, Clone, Copy)]
pub struct Match<'a> {
    pub symbol: &'a Symbol,
    pub ticker: &'a Ticker,
    /// How closely the symbol matches: 0 for its very name, 1 for its coin, 2 for a name
    /// that starts with the search, 3 for one that only contains it.
    closeness: u8,
    /// The 24-hour volume in dollars.
    dollar_volume: f64,
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

    /// The symbols matching `text`, closest first and then most traded.
    ///
    /// Case, spaces and separators are ignored, so `eth/usdt` finds `ETHUSDT` (and would
    /// find `ETH-USDT`). Pairs are quoted in many currencies (USDT, TRY, IDR, BTC…), so
    /// volumes are compared in dollars, using the market's own prices.
    pub fn search(&self, text: &str) -> Vec<Match<'_>> {
        let wanted = simplified(text);
        let mut rates: HashMap<&str, f64> = HashMap::new();
        let mut matches: Vec<Match> = self
            .tickers
            .values()
            .filter_map(|ticker| {
                let symbol = self.symbols.get(&ticker.symbol)?;
                let closeness = closeness(symbol, &wanted)?;
                let quote = symbol.quote_asset.as_str();
                let rate = *rates
                    .entry(quote)
                    .or_insert_with(|| usd_rate(quote, &self.tickers).unwrap_or(0.0));
                Some(Match {
                    symbol,
                    ticker,
                    closeness,
                    dollar_volume: ticker.quote_volume * rate,
                })
            })
            .collect();
        sort(&mut matches);
        matches
    }

    /// The symbol a chart opens on when nothing else was asked for: the most traded
    /// Bitcoin pair, or the most traded symbol of all.
    pub fn default_symbol(&self) -> Option<&Symbol> {
        let all = self.search("");
        all.iter()
            .find(|found| found.symbol.base_asset == "BTC")
            .or(all.first())
            .map(|found| found.symbol)
    }
}

/// Searches several markets at once: their matches together, closest first and then
/// most traded.
pub fn search_markets<'a>(
    boards: impl IntoIterator<Item = &'a Board>,
    text: &str,
) -> Vec<Match<'a>> {
    let mut matches: Vec<Match<'a>> = boards
        .into_iter()
        .flat_map(|board| board.search(text))
        .collect();
    sort(&mut matches);
    matches
}

fn sort(matches: &mut [Match]) {
    matches.sort_by(|a, b| {
        a.closeness
            .cmp(&b.closeness)
            .then(b.dollar_volume.total_cmp(&a.dollar_volume))
    });
}

/// How closely `symbol` matches `wanted` (simplified), if at all.
fn closeness(symbol: &Symbol, wanted: &str) -> Option<u8> {
    let name = simplified(&symbol.name);
    if wanted.is_empty() || name == wanted {
        Some(0)
    } else if simplified(&symbol.base_asset) == wanted {
        Some(1)
    } else if name.starts_with(wanted) {
        Some(2)
    } else if name.contains(wanted) {
        Some(3)
    } else {
        None
    }
}

/// Only the letters and digits of `text`, in upper case.
fn simplified(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_uppercase)
        .collect()
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

    fn symbol(kind: MarketKind, name: &str, base: &str, quote: &str) -> Symbol {
        Symbol {
            market: Market {
                exchange: FAKE,
                kind,
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
        let spot = |name, base, quote| symbol(MarketKind::Spot, name, base, quote);
        Board::new(MarketSnapshot {
            symbols: vec![
                spot("BTCUSDT", "BTC", "USDT"),
                spot("ETHBTC", "ETH", "BTC"),
                spot("USDTIDR", "USDT", "IDR"),
                spot("BTCIDR", "BTC", "IDR"),
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

    fn names(matches: &[Match]) -> Vec<String> {
        matches
            .iter()
            .map(|found| found.symbol.name.clone())
            .collect()
    }

    #[test]
    fn volumes_are_compared_in_dollars() {
        assert_eq!(
            names(&board().search("")),
            ["BTCUSDT", "ETHBTC", "USDTIDR", "BTCIDR"]
        );
    }

    #[test]
    fn closer_matches_come_first() {
        // The coin itself before pairs that only contain its name.
        assert_eq!(
            names(&board().search("btc")),
            ["BTCUSDT", "BTCIDR", "ETHBTC"]
        );
        // The very name first, whatever the case and separators.
        assert_eq!(names(&board().search(" Eth/BTC ")), ["ETHBTC"]);
        assert_eq!(names(&board().search("idr")), ["USDTIDR", "BTCIDR"]);
        assert!(board().search("XYZ").is_empty());
    }

    #[test]
    fn markets_are_searched_together() {
        let futures = Board::new(MarketSnapshot {
            symbols: vec![symbol(
                MarketKind::LinearPerpetual,
                "BTCUSDT",
                "BTC",
                "USDT",
            )],
            tickers: vec![ticker("BTCUSDT", 77_000.0, 9_000_000_000.0)],
        });
        let spot = board();
        let found = search_markets([&spot, &futures], "btc");
        assert_eq!(names(&found), ["BTCUSDT", "BTCUSDT", "BTCIDR", "ETHBTC"]);
        // The futures pair trades more, so it comes first among the Bitcoin pairs.
        assert_eq!(found[0].symbol.market.kind, MarketKind::LinearPerpetual);
        assert_eq!(found[1].symbol.market.kind, MarketKind::Spot);
    }

    #[test]
    fn charts_open_on_the_most_traded_bitcoin_pair() {
        assert_eq!(board().default_symbol().unwrap().name, "BTCUSDT");
        let no_bitcoin = Board::new(MarketSnapshot {
            symbols: vec![symbol(MarketKind::Spot, "ETHUSDT", "ETH", "USDT")],
            tickers: vec![ticker("ETHUSDT", 2_000.0, 1.0)],
        });
        assert_eq!(no_bitcoin.default_symbol().unwrap().name, "ETHUSDT");
        assert!(Board::default().default_symbol().is_none());
    }

    #[test]
    fn updates_only_touch_known_symbols() {
        let mut board = board();
        board.update(vec![
            ticker("BTCUSDT", 78_000.0, 1.0),
            ticker("NEWCOIN", 1.0, 1.0),
        ]);
        assert_eq!(board.search("").len(), 4);
        assert_eq!(board.search("BTCUSDT")[0].ticker.last_price, 78_000.0);
        assert!(board.symbol("NEWCOIN").is_none());
    }
}
