use chrono::{DateTime, Utc};
use const_format::formatcp;
use rusqlite::{named_params, Connection, Error, Row, Statement, ToSql};

use super::converters::{FileFormatSql, HashSql, PathBufSql};

const COLUMNS: &str = "filepath, size, format, created, device, hash";

/// Low level type for interacting with media rows
#[derive(Debug)]
pub(crate) struct MediaSql {
    pub filepath: PathBufSql,
    pub size: u64,
    pub format: FileFormatSql,
    pub created: Option<DateTime<Utc>>,
    // pub location: (),
    pub device: Option<String>,
    // pub iso: (),
    pub hash: HashSql,
}

#[derive(Debug, Default)]
pub(crate) struct MediaFilter {
    pub created_start: Option<DateTime<Utc>>,
    pub created_end: Option<DateTime<Utc>>,
}

pub(crate) struct MediaSearch<'conn> {
    statement: Statement<'conn>,
    filter: MediaFilter,
}

impl MediaSql {
    pub fn create_table(conn: &mut Connection) -> Result<(), Error> {
        let transaction = conn.transaction()?;
        transaction.execute(
            "CREATE TABLE IF NOT EXISTS media (
                    filepath TEXT NOT NULL,
                    size INTEGER NOT NULL,
                    format TEXT NOT NULL,
                    created TEXT,
                    device TEXT,
                    hash BLOB NOT NULL,
                    UNIQUE (hash),
                    UNIQUE (filepath)
                );",
            [],
        )?;
        transaction.execute("CREATE INDEX IF NOT EXISTS hash_index ON media (hash);", [])?;
        transaction.commit()?;
        Ok(())
    }

    pub fn get_rows(conn: &Connection) -> Result<Vec<MediaSql>, Error> {
        let mut media_search = Self::open_search(conn)?;
        let iter = media_search.iter()?;
        iter.collect()
    }

    pub fn open_search(conn: &Connection) -> Result<MediaSearch, Error> {
        Self::search(conn, MediaFilter::default())
    }

    pub fn search(conn: &Connection, filter: MediaFilter) -> Result<MediaSearch, Error> {
        let statement = match (&filter.created_start, &filter.created_end) {
            (Some(_), Some(_)) => conn.prepare(formatcp!(
                "SELECT {COLUMNS} FROM media \
                    WHERE created >= :created_start \
                        AND created <= :created_end \
                    ORDER BY created",
            ))?,
            (Some(_), None) => conn.prepare(formatcp!(
                "SELECT {COLUMNS} FROM media \
                    WHERE created >= :created_start \
                    ORDER BY created",
            ))?,
            (None, Some(_)) => conn.prepare(formatcp!(
                "SELECT {COLUMNS} FROM media \
                    WHERE created <= :created_end \
                    ORDER BY created",
            ))?,
            (None, None) => conn.prepare(formatcp!(
                "SELECT {COLUMNS} FROM media \
                    ORDER BY created"
            ))?,
        };
        Ok(MediaSearch { statement, filter })
    }

    pub fn insert(&self, conn: &Connection) -> Result<i64, Error> {
        let mut stmt = conn.prepare(formatcp!(
            "INSERT INTO media ({COLUMNS}) \
            VALUES (:filepath, :size, :format, :created, :device, :hash)"
        ))?;
        stmt.insert(named_params! {
            ":filepath": self.filepath,
            ":size": self.size,
            ":format": self.format,
            ":created": &self.created,
            ":device": &self.device,
            ":hash": self.hash,
        })
    }

    pub fn exists(&self, conn: &Connection, hash: HashSql) -> Result<bool, Error> {
        let mut stmt = conn.prepare("SELECT 1 FROM media WHERE hash = :hash")?;
        stmt.exists(named_params! {
            ":hash": hash,
        })
    }
}

impl<'conn> MediaSearch<'conn> {
    pub fn iter(&mut self) -> Result<impl Iterator<Item = Result<MediaSql, Error>> + '_, Error> {
        let params = self.filter.to_params();
        let iter = self
            .statement
            .query_map(params.as_slice(), |row| MediaSql::try_from(row))?;
        Ok(iter)
    }
}

impl MediaFilter {
    /// Convert the media filters into a type that can impl `Params`.
    ///
    /// We cannot impl `Params` directly because it is sealed and we cannot use `named_params`
    /// because we do not know at compile time which params will be set.
    fn to_params(&self) -> Vec<(&'static str, &dyn ToSql)> {
        let mut result = Vec::new();
        if let Some(created_start) = &self.created_start {
            result.push((":created_start", created_start as &dyn ToSql))
        }
        if let Some(created_end) = &self.created_end {
            result.push((":created_end", created_end as &dyn ToSql))
        }
        result
    }
}

impl TryFrom<&Row<'_>> for MediaSql {
    type Error = Error;

    fn try_from(row: &Row<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            filepath: row.get(0)?,
            size: row.get(1)?,
            format: row.get(2)?,
            created: row.get(3)?,
            device: row.get(4)?,
            hash: row.get(5)?,
        })
    }
}
