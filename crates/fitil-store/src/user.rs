//! The user's own data: what they made and could not download again.

use std::path::Path;

use fitil_core::Market;
use rusqlite::{Connection, OptionalExtension, params};

use crate::{Result, migrate, open_file};

/// Changes to the user data's schema, applied in order (see [`migrate`]).
const MIGRATIONS: &[&str] = &[
    // 1: indicator templates. The body is written by the application layer, so templates
    // can gain fields without a schema change.
    "
    CREATE TABLE indicator_templates (
        name TEXT PRIMARY KEY,
        body TEXT NOT NULL
    ) WITHOUT ROWID;
    ",
    // 2: the drawings of each symbol, all of a symbol's in one body written by the
    // application layer.
    "
    CREATE TABLE drawings (
        exchange TEXT NOT NULL,
        market TEXT NOT NULL,
        symbol TEXT NOT NULL,
        body TEXT NOT NULL,
        PRIMARY KEY (exchange, market, symbol)
    ) WITHOUT ROWID;
    ",
    // 3: named looks for each kind of drawing, and the look each kind's new drawings
    // start with. Bodies are written by the application layer.
    "
    CREATE TABLE drawing_templates (
        kind TEXT NOT NULL,
        name TEXT NOT NULL,
        body TEXT NOT NULL,
        PRIMARY KEY (kind, name)
    ) WITHOUT ROWID;
    CREATE TABLE drawing_defaults (
        kind TEXT PRIMARY KEY,
        body TEXT NOT NULL
    ) WITHOUT ROWID;
    ",
];

/// A saved indicator template, as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredTemplate {
    pub name: String,
    /// The template's content, as written by the application layer.
    pub body: String,
}

/// A saved look for one kind of drawing, as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredDrawingTemplate {
    /// The drawing kind's identifier.
    pub kind: String,
    pub name: String,
    /// The look, as written by the application layer.
    pub body: String,
}

pub struct UserData {
    conn: Connection,
}

impl UserData {
    /// Opens the user data at `path`, creating it and its folder if needed.
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

    /// Every saved indicator template, by name.
    pub fn indicator_templates(&self) -> Result<Vec<StoredTemplate>> {
        let mut query = self.conn.prepare_cached(
            "SELECT name, body FROM indicator_templates ORDER BY name COLLATE NOCASE, name",
        )?;
        let rows = query.query_map([], |row| {
            Ok(StoredTemplate {
                name: row.get(0)?,
                body: row.get(1)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Saves a template, replacing any with the same name.
    pub fn save_indicator_template(&mut self, template: &StoredTemplate) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO indicator_templates (name, body) VALUES (?1, ?2)",
            params![template.name, template.body],
        )?;
        Ok(())
    }

    pub fn delete_indicator_template(&mut self, name: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM indicator_templates WHERE name = ?1",
            params![name],
        )?;
        Ok(())
    }

    /// Every saved drawing template, by kind and then by name.
    pub fn drawing_templates(&self) -> Result<Vec<StoredDrawingTemplate>> {
        let mut query = self.conn.prepare_cached(
            "SELECT kind, name, body FROM drawing_templates
             ORDER BY kind, name COLLATE NOCASE, name",
        )?;
        let rows = query.query_map([], |row| {
            Ok(StoredDrawingTemplate {
                kind: row.get(0)?,
                name: row.get(1)?,
                body: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Saves a drawing template, replacing any of the same kind with the same name.
    pub fn save_drawing_template(&mut self, template: &StoredDrawingTemplate) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO drawing_templates (kind, name, body) VALUES (?1, ?2, ?3)",
            params![template.kind, template.name, template.body],
        )?;
        Ok(())
    }

    pub fn delete_drawing_template(&mut self, kind: &str, name: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM drawing_templates WHERE kind = ?1 AND name = ?2",
            params![kind, name],
        )?;
        Ok(())
    }

    /// The looks new drawings start with, as `(kind, body)` pairs, for the kinds that have
    /// one saved.
    pub fn drawing_defaults(&self) -> Result<Vec<(String, String)>> {
        let mut query = self
            .conn
            .prepare_cached("SELECT kind, body FROM drawing_defaults ORDER BY kind")?;
        let rows = query.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Saves the look new drawings of `kind` start with; `None` goes back to Fitil's.
    pub fn save_drawing_default(&mut self, kind: &str, body: Option<&str>) -> Result<()> {
        match body {
            Some(body) => self.conn.execute(
                "INSERT OR REPLACE INTO drawing_defaults (kind, body) VALUES (?1, ?2)",
                params![kind, body],
            )?,
            None => self.conn.execute(
                "DELETE FROM drawing_defaults WHERE kind = ?1",
                params![kind],
            )?,
        };
        Ok(())
    }

    /// The drawings of `symbol` in `market`, as written by the application layer.
    pub fn drawings(&self, market: Market, symbol: &str) -> Result<Option<String>> {
        let body = self
            .conn
            .query_row(
                "SELECT body FROM drawings WHERE exchange = ?1 AND market = ?2 AND symbol = ?3",
                params![market.exchange.as_str(), market.kind.id(), symbol],
                |row| row.get(0),
            )
            .optional()?;
        Ok(body)
    }

    /// Saves the drawings of `symbol` in `market`, replacing what was there; `None`
    /// removes them.
    pub fn save_drawings(
        &mut self,
        market: Market,
        symbol: &str,
        body: Option<&str>,
    ) -> Result<()> {
        let keys = params![market.exchange.as_str(), market.kind.id(), symbol];
        match body {
            Some(body) => self.conn.execute(
                "INSERT OR REPLACE INTO drawings (exchange, market, symbol, body)
                 VALUES (?1, ?2, ?3, ?4)",
                params![market.exchange.as_str(), market.kind.id(), symbol, body],
            )?,
            None => self.conn.execute(
                "DELETE FROM drawings WHERE exchange = ?1 AND market = ?2 AND symbol = ?3",
                keys,
            )?,
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TempDb;

    fn template(name: &str, body: &str) -> StoredTemplate {
        StoredTemplate {
            name: name.into(),
            body: body.into(),
        }
    }

    #[test]
    fn saves_replaces_and_deletes_templates() {
        let mut user = UserData::open_in_memory().unwrap();
        assert!(user.indicator_templates().unwrap().is_empty());

        user.save_indicator_template(&template("Trend", "1"))
            .unwrap();
        user.save_indicator_template(&template("Trend", "2"))
            .unwrap();
        assert_eq!(
            user.indicator_templates().unwrap(),
            [template("Trend", "2")]
        );

        user.delete_indicator_template("Trend").unwrap();
        user.delete_indicator_template("never saved").unwrap();
        assert!(user.indicator_templates().unwrap().is_empty());
    }

    #[test]
    fn templates_are_listed_by_name() {
        let mut user = UserData::open_in_memory().unwrap();
        for name in ["scalp", "Averages", "volume"] {
            user.save_indicator_template(&template(name, "{}")).unwrap();
        }
        let names: Vec<String> = user
            .indicator_templates()
            .unwrap()
            .into_iter()
            .map(|template| template.name)
            .collect();
        assert_eq!(names, ["Averages", "scalp", "volume"]);
    }

    #[test]
    fn templates_survive_reopening() {
        let db = TempDb::new("user");
        {
            let mut user = UserData::open(&db.0).unwrap();
            user.save_indicator_template(&template("RSI", "{}"))
                .unwrap();
        }
        let user = UserData::open(&db.0).unwrap();
        assert_eq!(user.indicator_templates().unwrap(), [template("RSI", "{}")]);
    }

    #[test]
    fn drawings_are_kept_per_symbol_and_market() {
        use fitil_core::{ExchangeId, MarketKind};
        let spot = Market {
            exchange: ExchangeId::new("binance"),
            kind: MarketKind::Spot,
        };
        let futures = Market {
            kind: MarketKind::LinearPerpetual,
            ..spot
        };
        let mut user = UserData::open_in_memory().unwrap();
        assert_eq!(user.drawings(spot, "BTCUSDT").unwrap(), None);

        user.save_drawings(spot, "BTCUSDT", Some("a")).unwrap();
        user.save_drawings(spot, "BTCUSDT", Some("b")).unwrap();
        user.save_drawings(futures, "BTCUSDT", Some("c")).unwrap();
        assert_eq!(
            user.drawings(spot, "BTCUSDT").unwrap().as_deref(),
            Some("b")
        );
        assert_eq!(
            user.drawings(futures, "BTCUSDT").unwrap().as_deref(),
            Some("c")
        );
        assert_eq!(user.drawings(spot, "ETHUSDT").unwrap(), None);

        user.save_drawings(spot, "BTCUSDT", None).unwrap();
        assert_eq!(user.drawings(spot, "BTCUSDT").unwrap(), None);
    }

    #[test]
    fn drawing_templates_are_kept_per_kind() {
        let template = |kind: &str, name: &str, body: &str| StoredDrawingTemplate {
            kind: kind.into(),
            name: name.into(),
            body: body.into(),
        };
        let mut user = UserData::open_in_memory().unwrap();
        assert!(user.drawing_templates().unwrap().is_empty());

        user.save_drawing_template(&template("ray", "Kalın", "1"))
            .unwrap();
        user.save_drawing_template(&template("fib-retracement", "Altın", "2"))
            .unwrap();
        user.save_drawing_template(&template("fib-retracement", "Altın", "3"))
            .unwrap();
        user.save_drawing_template(&template("fib-retracement", "kısa", "4"))
            .unwrap();
        assert_eq!(
            user.drawing_templates().unwrap(),
            [
                template("fib-retracement", "Altın", "3"),
                template("fib-retracement", "kısa", "4"),
                template("ray", "Kalın", "1"),
            ]
        );

        user.delete_drawing_template("fib-retracement", "Altın")
            .unwrap();
        user.delete_drawing_template("ray", "never saved").unwrap();
        assert_eq!(user.drawing_templates().unwrap().len(), 2);
    }

    #[test]
    fn each_kind_has_at_most_one_default_look() {
        let mut user = UserData::open_in_memory().unwrap();
        user.save_drawing_default("ray", Some("1")).unwrap();
        user.save_drawing_default("ray", Some("2")).unwrap();
        user.save_drawing_default("text", Some("3")).unwrap();
        assert_eq!(
            user.drawing_defaults().unwrap(),
            [
                ("ray".to_owned(), "2".to_owned()),
                ("text".into(), "3".into())
            ]
        );

        user.save_drawing_default("ray", None).unwrap();
        assert_eq!(
            user.drawing_defaults().unwrap(),
            [("text".to_owned(), "3".to_owned())]
        );
    }
}
