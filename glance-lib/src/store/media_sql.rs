use const_format::formatcp;
use rusqlite::{named_params, Connection, Error, Row, Statement};
use time::OffsetDateTime;

use super::converters::{FileFormatSql, HashSql, PathBufSql};

const COLUMNS: &str = "(filepath, size, format, created, device, hash)";

/// Low level type for interacting with media rows
#[derive(Debug)]
pub(crate) struct MediaSql {
    pub filepath: PathBufSql,
    pub size: u64,
    pub format: FileFormatSql,
    pub created: Option<OffsetDateTime>,
    // pub location: (),
    pub device: Option<String>,
    // pub iso: (),
    pub hash: HashSql,
}

#[derive(Debug, Default)]
pub(crate) struct MediaFilter {
    pub crated_start: Option<OffsetDateTime>,
    pub created_end: Option<OffsetDateTime>,
}

pub(crate) struct MediaSearch<'conn> {
    statement: Statement<'conn>,
    filter: MediaFilter,
}

impl MediaSql {
    pub fn create_table(conn: &Connection) -> Result<(), Error> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS media (
                    filepath TEXT NOT NULL,
                    size INTEGER NOT NULL,
                    format TEXT NOT NULL,
                    created TEXT,
                    device TEXT,
                    hash BLOB NOT NULL,
                    UNIQUE (hash),
                    UNIQUE (filepath)
            );
            CREATE INDEX hash_index ON media (hash);",
            [],
        )?;
        Ok(())
    }

    pub fn get_rows(conn: &Connection) -> Result<Vec<MediaSql>, Error> {
        let mut stmt = conn.prepare("SELECT * FROM media")?;

        let rows = stmt.query_map([], |row| {
            Ok(MediaSql {
                filepath: row.get(0)?,
                size: row.get(1)?,
                format: row.get(2)?,
                created: row.get(3)?,
                device: row.get(4)?,
                hash: row.get(5)?,
            })
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }

        Ok(result)
    }

    pub fn open_search<'conn>(conn: &'conn Connection) -> Result<MediaSearch<'conn>, Error> {
        Self::search(conn, MediaFilter::default())
    }

    pub fn search<'conn>(
        conn: &'conn Connection,
        filter: MediaFilter,
    ) -> Result<MediaSearch<'conn>, Error> {
        let statement = match (filter.crated_start, filter.created_end) {
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
            (_, _) => conn.prepare(formatcp!("SELECT {COLUMNS} FROM media ORDER By created",))?,
        };
        Ok(MediaSearch { statement, filter })
    }

    pub fn insert(&self, conn: &Connection) -> Result<(), Error> {
        let mut stmt = conn.prepare(formatcp!(
            "INSERT INTO media {COLUMNS} \
            VALUES (:filepath, :size, :format, :created, :device, :hash)"
        ))?;
        stmt.insert(named_params! {
            ":filepath": self.filepath,
            ":size": self.size,
            ":format": self.format,
            ":created": &self.created,
            ":device": &self.device,
            ":hash": self.hash,
        })?;

        Ok(())
    }
}

impl<'conn> MediaSearch<'conn> {
    fn new(statement: Statement<'conn>, filter: MediaFilter) -> Self {
        Self { statement, filter }
    }

    pub fn iter(
        &'conn mut self,
    ) -> Result<impl Iterator<Item = Result<MediaSql, Error>> + 'conn, Error> {
        let params = named_params! {
            ":created_start": self.filter.crated_start,
            ":created_end": self.filter.created_end,
        };
        let iter = self
            .statement
            .query_map(params, |row| MediaSql::try_from(row))?;
        Ok(iter)
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
