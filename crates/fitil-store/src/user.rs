//! The user's own data: what they made and could not download again.

use std::path::Path;

use rusqlite::{Connection, params};

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
];

/// A saved indicator template, as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredTemplate {
    pub name: String,
    /// The template's content, as written by the application layer.
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
}
