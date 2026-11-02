//! Local SQLite storage for Fitil, in two files:
//!
//! - [`Store`], the candle cache. It only holds what can be downloaded again, so it may be
//!   deleted at any time. Callers keep every candle series contiguous: they only add
//!   candles right before the oldest or right after the newest stored one, so a series
//!   has no holes.
//! - [`UserData`], what the user made: indicator templates, and later drawings, layouts
//!   and settings.

mod user;

pub use user::{StoredDrawingTemplate, StoredTemplate, UserData};

use std::path::Path;

use fitil_core::{Candle, SeriesKey};
use rusqlite::{Connection, OptionalExtension, params};

/// Changes to the cache's schema, applied in order (see [`migrate`]).
const MIGRATIONS: &[&str] = &[
    // 1: candles and series notes, for Binance only.
    "
    CREATE TABLE candles (
        market TEXT NOT NULL,
        symbol TEXT NOT NULL,
        timeframe TEXT NOT NULL,
        open_time INTEGER NOT NULL,
        open REAL NOT NULL,
        high REAL NOT NULL,
        low REAL NOT NULL,
        close REAL NOT NULL,
        volume REAL NOT NULL,
        quote_volume REAL NOT NULL,
        trades INTEGER NOT NULL,
        taker_buy_volume REAL NOT NULL,
        PRIMARY KEY (market, symbol, timeframe, open_time)
    ) WITHOUT ROWID;

    CREATE TABLE series (
        market TEXT NOT NULL,
        symbol TEXT NOT NULL,
        timeframe TEXT NOT NULL,
        history_start INTEGER,
        PRIMARY KEY (market, symbol, timeframe)
    ) WITHOUT ROWID;
    ",
    // 2: the exchange becomes part of every key, and quote volume, trade count and taker
    // volume become optional because not every exchange reports them. Existing rows all
    // came from Binance, whose futures market `usdm` is now called `perp`.
    "
    CREATE TABLE candles_v2 (
        exchange TEXT NOT NULL,
        market TEXT NOT NULL,
        symbol TEXT NOT NULL,
        timeframe TEXT NOT NULL,
        open_time INTEGER NOT NULL,
        open REAL NOT NULL,
        high REAL NOT NULL,
        low REAL NOT NULL,
        close REAL NOT NULL,
        volume REAL NOT NULL,
        quote_volume REAL,
        trades INTEGER,
        taker_buy_volume REAL,
        PRIMARY KEY (exchange, market, symbol, timeframe, open_time)
    ) WITHOUT ROWID;
    INSERT INTO candles_v2
        SELECT 'binance', CASE market WHEN 'usdm' THEN 'perp' ELSE market END,
               symbol, timeframe, open_time, open, high, low, close, volume,
               quote_volume, trades, taker_buy_volume
        FROM candles;
    DROP TABLE candles;
    ALTER TABLE candles_v2 RENAME TO candles;

    CREATE TABLE series_v2 (
        exchange TEXT NOT NULL,
        market TEXT NOT NULL,
        symbol TEXT NOT NULL,
        timeframe TEXT NOT NULL,
        history_start INTEGER,
        PRIMARY KEY (exchange, market, symbol, timeframe)
    ) WITHOUT ROWID;
    INSERT INTO series_v2
        SELECT 'binance', CASE market WHEN 'usdm' THEN 'perp' ELSE market END,
               symbol, timeframe, history_start
        FROM series;
    DROP TABLE series;
    ALTER TABLE series_v2 RENAME TO series;
    ",
];

/// Matches one series in the `candles` and `series` tables; parameters ?1 to ?4.
const SERIES: &str = "exchange = ?1 AND market = ?2 AND symbol = ?3 AND timeframe = ?4";

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("could not create the data folder: {0}")]
    Io(#[from] std::io::Error),
    #[error("the file was created by a newer Fitil (schema {found}, this version knows {known})")]
    NewerSchema { found: u32, known: u32 },
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// What is stored for one series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeriesRange {
    pub oldest: i64,
    pub newest: i64,
    pub count: u64,
}

pub struct Store {
    conn: Connection,
}

impl Store {
    /// Opens the cache at `path`, creating it and its folder if needed.
    pub fn open(path: &Path) -> Result<Self> {
        let mut conn = open_file(path)?;
        migrate(&mut conn, MIGRATIONS)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        migrate(&mut conn, MIGRATIONS)?;
        Ok(Self { conn })
    }

    /// Stores candles, replacing any with the same open time.
    pub fn save_candles(&mut self, key: &SeriesKey, candles: &[Candle]) -> Result<()> {
        let [exchange, market, symbol, timeframe] = columns(key);
        let tx = self.conn.transaction()?;
        {
            let mut insert = tx.prepare_cached(
                "INSERT OR REPLACE INTO candles
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            )?;
            for c in candles {
                insert.execute(params![
                    exchange,
                    market,
                    symbol,
                    timeframe,
                    c.open_time,
                    c.open,
                    c.high,
                    c.low,
                    c.close,
                    c.volume,
                    c.quote_volume,
                    c.trades.map(|trades| trades as i64),
                    c.taker_buy_volume,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Candles opening between `from` and `to` (both inclusive), oldest first.
    pub fn candles(&self, key: &SeriesKey, from: i64, to: i64) -> Result<Vec<Candle>> {
        let [exchange, market, symbol, timeframe] = columns(key);
        let mut query = self.conn.prepare_cached(&format!(
            "SELECT open_time, open, high, low, close, volume, quote_volume, trades, taker_buy_volume
             FROM candles
             WHERE {SERIES} AND open_time BETWEEN ?5 AND ?6
             ORDER BY open_time"
        ))?;
        let rows = query.query_map(
            params![exchange, market, symbol, timeframe, from, to],
            read_candle,
        )?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// The newest `limit` candles, oldest first.
    pub fn latest_candles(&self, key: &SeriesKey, limit: usize) -> Result<Vec<Candle>> {
        let [exchange, market, symbol, timeframe] = columns(key);
        let mut query = self.conn.prepare_cached(&format!(
            "SELECT open_time, open, high, low, close, volume, quote_volume, trades, taker_buy_volume
             FROM candles
             WHERE {SERIES}
             ORDER BY open_time DESC
             LIMIT ?5"
        ))?;
        let rows = query.query_map(
            params![exchange, market, symbol, timeframe, limit as i64],
            read_candle,
        )?;
        let mut candles = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        candles.reverse();
        Ok(candles)
    }

    /// The oldest and newest stored open times and the number of candles, if any are stored.
    pub fn range(&self, key: &SeriesKey) -> Result<Option<SeriesRange>> {
        let [exchange, market, symbol, timeframe] = columns(key);
        let range = self.conn.query_row(
            &format!("SELECT MIN(open_time), MAX(open_time), COUNT(*) FROM candles WHERE {SERIES}"),
            params![exchange, market, symbol, timeframe],
            |row| {
                let oldest: Option<i64> = row.get(0)?;
                let newest: Option<i64> = row.get(1)?;
                let count: i64 = row.get(2)?;
                Ok(oldest.zip(newest).map(|(oldest, newest)| SeriesRange {
                    oldest,
                    newest,
                    count: count as u64,
                }))
            },
        )?;
        Ok(range)
    }

    /// Open time of the exchange's first candle for this series, once the cache reaches it.
    pub fn history_start(&self, key: &SeriesKey) -> Result<Option<i64>> {
        let [exchange, market, symbol, timeframe] = columns(key);
        let start = self
            .conn
            .query_row(
                &format!("SELECT history_start FROM series WHERE {SERIES}"),
                params![exchange, market, symbol, timeframe],
                |row| row.get(0),
            )
            .optional()?;
        Ok(start.flatten())
    }

    /// Records that the exchange has no candles before `open_time` for this series.
    pub fn set_history_start(&mut self, key: &SeriesKey, open_time: i64) -> Result<()> {
        let [exchange, market, symbol, timeframe] = columns(key);
        self.conn.execute(
            "INSERT INTO series (exchange, market, symbol, timeframe, history_start)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (exchange, market, symbol, timeframe)
             DO UPDATE SET history_start = excluded.history_start",
            params![exchange, market, symbol, timeframe, open_time],
        )?;
        Ok(())
    }

    /// Removes every stored candle and note about this series.
    pub fn clear_series(&mut self, key: &SeriesKey) -> Result<()> {
        let [exchange, market, symbol, timeframe] = columns(key);
        let tx = self.conn.transaction()?;
        for table in ["candles", "series"] {
            tx.execute(
                &format!("DELETE FROM {table} WHERE {SERIES}"),
                params![exchange, market, symbol, timeframe],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}

/// Opens the database at `path`, creating it and its folder if needed.
fn open_file(path: &Path) -> Result<Connection> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let conn = Connection::open(path)?;
    conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    Ok(conn)
}

/// Runs the `migrations` not applied yet. `PRAGMA user_version` records how many have
/// run, so a file written by an older Fitil is upgraded in place instead of being lost.
fn migrate(conn: &mut Connection, migrations: &[&str]) -> Result<()> {
    let applied: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let known = migrations.len() as u32;
    if applied > known {
        return Err(StoreError::NewerSchema {
            found: applied,
            known,
        });
    }
    let tx = conn.transaction()?;
    for migration in &migrations[applied as usize..] {
        tx.execute_batch(migration)?;
    }
    tx.pragma_update(None, "user_version", known)?;
    tx.commit()?;
    Ok(())
}

/// Exchange, market, symbol and timeframe of a series, as stored.
fn columns(key: &SeriesKey) -> [String; 4] {
    [
        key.market.exchange.as_str().to_owned(),
        key.market.kind.id().to_owned(),
        key.symbol.clone(),
        key.timeframe.to_string(),
    ]
}

fn read_candle(row: &rusqlite::Row) -> rusqlite::Result<Candle> {
    Ok(Candle {
        open_time: row.get(0)?,
        open: row.get(1)?,
        high: row.get(2)?,
        low: row.get(3)?,
        close: row.get(4)?,
        volume: row.get(5)?,
        quote_volume: row.get(6)?,
        trades: row.get::<_, Option<i64>>(7)?.map(|trades| trades as u64),
        taker_buy_volume: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fitil_core::{ExchangeId, Market, MarketKind};
    use std::path::PathBuf;

    const BINANCE: ExchangeId = ExchangeId::new("binance");

    fn key_on(exchange: ExchangeId, kind: MarketKind, timeframe: &str) -> SeriesKey {
        SeriesKey {
            market: Market { exchange, kind },
            symbol: "BTCUSDT".into(),
            timeframe: timeframe.parse().unwrap(),
        }
    }

    fn key(timeframe: &str) -> SeriesKey {
        key_on(BINANCE, MarketKind::Spot, timeframe)
    }

    fn candle(hour: i64, close: f64) -> Candle {
        Candle {
            open_time: hour * 3_600_000,
            open: close - 1.0,
            high: close + 1.0,
            low: close - 2.0,
            close,
            volume: 1.5,
            quote_volume: Some(close * 1.5),
            trades: Some(42),
            taker_buy_volume: Some(0.5),
        }
    }

    /// A database file in the temp folder, removed when dropped.
    pub(crate) struct TempDb(pub(crate) PathBuf);

    impl TempDb {
        pub(crate) fn new(name: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("fitil-store-{}-{name}", std::process::id()))
                .join("cache.sqlite");
            Self(path)
        }
    }

    impl Drop for TempDb {
        fn drop(&mut self) {
            if let Some(folder) = self.0.parent() {
                let _ = std::fs::remove_dir_all(folder);
            }
        }
    }

    #[test]
    fn saves_and_reads_candles_in_order() {
        let mut store = Store::open_in_memory().unwrap();
        let candles = [candle(3, 30.0), candle(1, 10.0), candle(2, 20.0)];
        store.save_candles(&key("1h"), &candles).unwrap();

        let all = store.candles(&key("1h"), 0, i64::MAX).unwrap();
        assert_eq!(all, vec![candles[1], candles[2], candles[0]]);
        assert_eq!(
            store
                .candles(&key("1h"), 2 * 3_600_000, 2 * 3_600_000)
                .unwrap(),
            vec![candles[2]]
        );
        assert_eq!(
            store.latest_candles(&key("1h"), 2).unwrap(),
            vec![candles[2], candles[0]]
        );
        assert!(store.candles(&key("4h"), 0, i64::MAX).unwrap().is_empty());
    }

    #[test]
    fn unknown_fields_are_kept_unknown() {
        let mut store = Store::open_in_memory().unwrap();
        let bare = Candle {
            quote_volume: None,
            trades: None,
            taker_buy_volume: None,
            ..candle(1, 10.0)
        };
        store.save_candles(&key("1h"), &[bare]).unwrap();
        assert_eq!(store.candles(&key("1h"), 0, i64::MAX).unwrap(), vec![bare]);
    }

    #[test]
    fn exchanges_and_markets_are_kept_apart() {
        let mut store = Store::open_in_memory().unwrap();
        let okx = key_on(ExchangeId::new("okx"), MarketKind::Spot, "1h");
        let futures = key_on(BINANCE, MarketKind::LinearPerpetual, "1h");
        store.save_candles(&key("1h"), &[candle(1, 10.0)]).unwrap();
        store.save_candles(&okx, &[candle(1, 20.0)]).unwrap();

        assert_eq!(
            store.candles(&okx, 0, i64::MAX).unwrap(),
            vec![candle(1, 20.0)]
        );
        assert_eq!(
            store.candles(&key("1h"), 0, i64::MAX).unwrap(),
            vec![candle(1, 10.0)]
        );
        assert_eq!(store.range(&futures).unwrap(), None);
    }

    #[test]
    fn saving_the_same_open_time_replaces_the_candle() {
        let mut store = Store::open_in_memory().unwrap();
        store.save_candles(&key("1h"), &[candle(1, 10.0)]).unwrap();
        store.save_candles(&key("1h"), &[candle(1, 11.0)]).unwrap();

        assert_eq!(
            store.candles(&key("1h"), 0, i64::MAX).unwrap(),
            vec![candle(1, 11.0)]
        );
    }

    #[test]
    fn reports_the_stored_range() {
        let mut store = Store::open_in_memory().unwrap();
        assert_eq!(store.range(&key("1h")).unwrap(), None);

        store
            .save_candles(
                &key("1h"),
                &[candle(5, 1.0), candle(6, 1.0), candle(7, 1.0)],
            )
            .unwrap();
        let range = store.range(&key("1h")).unwrap().unwrap();
        assert_eq!(
            (range.oldest, range.newest, range.count),
            (5 * 3_600_000, 7 * 3_600_000, 3)
        );
    }

    #[test]
    fn remembers_where_history_starts() {
        let mut store = Store::open_in_memory().unwrap();
        assert_eq!(store.history_start(&key("1d")).unwrap(), None);

        store
            .set_history_start(&key("1d"), 1_502_928_000_000)
            .unwrap();
        store
            .set_history_start(&key("1d"), 1_502_928_000_001)
            .unwrap();
        assert_eq!(
            store.history_start(&key("1d")).unwrap(),
            Some(1_502_928_000_001)
        );
        assert_eq!(store.history_start(&key("1h")).unwrap(), None);
    }

    #[test]
    fn clearing_a_series_leaves_others_alone() {
        let mut store = Store::open_in_memory().unwrap();
        store.save_candles(&key("1h"), &[candle(1, 1.0)]).unwrap();
        store.save_candles(&key("4h"), &[candle(4, 1.0)]).unwrap();
        store.set_history_start(&key("1h"), 0).unwrap();

        store.clear_series(&key("1h")).unwrap();
        assert_eq!(store.range(&key("1h")).unwrap(), None);
        assert_eq!(store.history_start(&key("1h")).unwrap(), None);
        assert!(store.range(&key("4h")).unwrap().is_some());
    }

    #[test]
    fn reopening_keeps_the_data() {
        let db = TempDb::new("reopen");
        {
            let mut store = Store::open(&db.0).unwrap();
            store.save_candles(&key("1h"), &[candle(1, 10.0)]).unwrap();
        }
        let store = Store::open(&db.0).unwrap();
        assert_eq!(
            store.candles(&key("1h"), 0, i64::MAX).unwrap(),
            vec![candle(1, 10.0)]
        );
    }

    #[test]
    fn upgrades_a_cache_from_the_first_version() {
        let db = TempDb::new("upgrade");
        {
            // A cache as the first version of Fitil left it.
            std::fs::create_dir_all(db.0.parent().unwrap()).unwrap();
            let conn = Connection::open(&db.0).unwrap();
            conn.execute_batch(MIGRATIONS[0]).unwrap();
            conn.pragma_update(None, "user_version", 1).unwrap();
            conn.execute(
                "INSERT INTO candles VALUES ('usdm', 'ETHUSDT', '1h', 3600000, 1, 2, 0.5, 1.5, 10, 15, 7, 4)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO series VALUES ('usdm', 'ETHUSDT', '1h', 3600000)",
                [],
            )
            .unwrap();
        }

        let store = Store::open(&db.0).unwrap();
        let key = SeriesKey {
            symbol: "ETHUSDT".into(),
            ..key_on(BINANCE, MarketKind::LinearPerpetual, "1h")
        };
        let candles = store.candles(&key, 0, i64::MAX).unwrap();
        assert_eq!(candles.len(), 1);
        assert_eq!(candles[0].close, 1.5);
        assert_eq!(candles[0].trades, Some(7));
        assert_eq!(candles[0].taker_buy_volume, Some(4.0));
        assert_eq!(store.history_start(&key).unwrap(), Some(3_600_000));
    }

    #[test]
    fn refuses_a_cache_from_a_newer_version() {
        let db = TempDb::new("newer");
        drop(Store::open(&db.0).unwrap());
        Connection::open(&db.0)
            .unwrap()
            .pragma_update(None, "user_version", 99)
            .unwrap();

        match Store::open(&db.0) {
            Err(StoreError::NewerSchema { found: 99, .. }) => {}
            other => panic!("expected NewerSchema, got {:?}", other.err()),
        }
    }
}
