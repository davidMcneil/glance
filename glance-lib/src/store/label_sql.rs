use const_format::formatcp;
use rusqlite::{named_params, Connection, Error, Row, Statement, ToSql};

use super::converters::PathBufSql;

const COLUMNS: &str = "filepath, label";

/// Low level type for interacting with label rows
#[derive(Debug)]
pub(crate) struct LabelSql {
    pub filepath: PathBufSql,
    pub label: String,
}

#[derive(Default)]
pub struct LabelFilter {
    pub filepath: Option<PathBufSql>,
}

pub(crate) struct LabelSearch<'conn> {
    statement: Statement<'conn>,
    filter: LabelFilter,
}

impl LabelSql {
    pub fn create_table(conn: &mut Connection) -> Result<(), Error> {
        let transaction = conn.transaction()?;
        transaction.execute(
            "CREATE TABLE IF NOT EXISTS label (
                    filepath TEXT NOT NULL,
                    label TEXT NOT NULL,
                    FOREIGN KEY (filepath) REFERENCES media(filepath),
                    UNIQUE (filepath, label),
                    CHECK(label <> '')
                );",
            [],
        )?;
        transaction.execute(
            "CREATE INDEX IF NOT EXISTS filepath_index ON label (filepath);",
            [],
        )?;
        transaction.execute(
            "CREATE INDEX IF NOT EXISTS label_index ON label (label);",
            [],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn insert(&self, conn: &Connection) -> Result<i64, Error> {
        let mut stmt = conn.prepare(formatcp!(
            "INSERT INTO label ({COLUMNS}) \
            VALUES (:filepath, :label)"
        ))?;
        stmt.insert(named_params! {
            ":filepath": self.filepath,
            ":label": self.label,
        })
    }

    pub fn delete(&self, conn: &Connection) -> Result<usize, Error> {
        let mut stmt = conn.prepare(
            "DELETE FROM label \
                    WHERE filepath = :filepath \
                    AND label = :label",
        )?;
        stmt.execute(named_params! {
            ":filepath": self.filepath,
            ":label": self.label,
        })
    }

    pub fn get_all_labels(conn: &Connection) -> Result<Vec<String>, Error> {
        let mut stmt = conn.prepare("SELECT DISTINCT label FROM label ORDER BY label")?;
        let iter = stmt.query_map([], |row| row.get(0))?;
        iter.collect()
    }
}

impl LabelSearch<'_> {
    pub fn new(conn: &Connection, filter: LabelFilter) -> Result<LabelSearch, Error> {
        let statement = match &filter.filepath {
            Some(_) => conn.prepare(formatcp!(
                "SELECT {COLUMNS} FROM label \
                    WHERE filepath = :filepath \
                    ORDER BY filepath",
            ))?,

            None => conn.prepare(formatcp!(
                "SELECT {COLUMNS} FROM label \
                    ORDER BY filepath"
            ))?,
        };
        Ok(LabelSearch { statement, filter })
    }

    pub fn iter(&mut self) -> Result<impl Iterator<Item = Result<LabelSql, Error>> + '_, Error> {
        let params = self.filter.to_params();
        let iter = self
            .statement
            .query_map(params.as_slice(), |row| LabelSql::try_from(row))?;
        Ok(iter)
    }
}

impl LabelFilter {
    /// Convert the label filters into a type that can impl `Params`.
    ///
    /// We cannot impl `Params` directly because it is sealed and we cannot use `named_params`
    /// because we do not know at compile time which params will be set.
    fn to_params(&self) -> Vec<(&'static str, &dyn ToSql)> {
        let mut result = Vec::new();
        if let Some(filepath) = &self.filepath {
            result.push((":filepath", filepath as &dyn ToSql))
        }
        result
    }
}

impl TryFrom<&Row<'_>> for LabelSql {
    type Error = Error;

    fn try_from(row: &Row<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            filepath: row.get(0)?,
            label: row.get(1)?,
        })
    }
}
