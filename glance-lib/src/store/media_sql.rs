use std::{collections::HashMap, path::Path};

use chrono::{DateTime, Utc};
use const_format::formatcp;
use rusqlite::{named_params, Connection, Error, ErrorCode, Row, Statement, ToSql};

use super::converters::{FileFormatSql, HashSql, PathBufSql};

const IMPORT_DB: &str = "import";
const COLUMNS: &str = "filepath, size, format, created, location, device, hash";
const COLUMNS_WITH_PREFIX: &str = "media.filepath, size, format, created, location, device, hash";

/// Low level type for interacting with media rows
#[derive(Debug)]
pub(crate) struct MediaSql {
    pub filepath: PathBufSql,
    pub size: u64,
    pub format: FileFormatSql,
    pub created: Option<DateTime<Utc>>,
    pub location: Option<String>,
    pub device: Option<String>,
    // pub iso: (),
    pub hash: Option<HashSql>,
}

#[derive(Debug, Default)]
pub struct MediaFilter {
    pub created_start: Option<DateTime<Utc>>,
    pub created_end: Option<DateTime<Utc>>,
    pub label: Option<String>,
    pub device: Option<String>,
    pub format: Option<String>,
}

pub(crate) struct MediaSearch<'conn> {
    statement: Statement<'conn>,
    filter: MediaFilter,
}

pub(crate) struct MediaDuplicates<'conn> {
    statement: Statement<'conn>,
}

pub(crate) struct MediaNewFromImport<'conn> {
    statement: Statement<'conn>,
}

impl MediaSql {
    pub fn create_table(conn: &mut Connection) -> Result<(), Error> {
        let transaction = conn.transaction()?;
        transaction.execute(
            "CREATE TABLE IF NOT EXISTS media (
                    filepath TEXT NOT NULL PRIMARY KEY,
                    size INTEGER NOT NULL,
                    format TEXT NOT NULL,
                    created TEXT,
                    location TEXT,
                    device TEXT,
                    hash BLOB
                );",
            [],
        )?;
        transaction.execute("CREATE INDEX IF NOT EXISTS hash_index ON media (hash);", [])?;
        transaction.commit()?;
        Ok(())
    }

    pub fn attach_for_import(import_path: &Path, conn: &mut Connection) -> Result<(), Error> {
        conn.execute(
            &format!("ATTACH DATABASE '{}' AS {IMPORT_DB}", import_path.display()),
            [],
        )?;
        Ok(())
    }

    pub fn insert(&self, conn: &Connection) -> Result<bool, Error> {
        let mut stmt = conn.prepare(formatcp!(
            "INSERT INTO media ({COLUMNS}) \
            VALUES (:filepath, :size, :format, :created, :location, :device, :hash)"
        ))?;
        let res = stmt.insert(named_params! {
            ":filepath": self.filepath,
            ":size": self.size,
            ":format": self.format,
            ":created": &self.created,
            ":location": &self.location,
            ":device": &self.device,
            ":hash": self.hash,
        });
        Ok(if duplicate_row(&res) {
            false
        } else {
            res?;
            true
        })
    }

    pub fn delete(&self, conn: &Connection) -> Result<usize, Error> {
        let mut stmt = conn.prepare("DELETE FROM media WHERE filepath = :filepath")?;
        stmt.execute(named_params! {
            ":filepath": self.filepath,
        })
    }

    pub fn rename(
        conn: &Connection,
        old_filepath: &PathBufSql,
        new_filepath: &PathBufSql,
    ) -> Result<usize, Error> {
        let mut stmt = conn.prepare(formatcp!(
            "UPDATE media
            SET filepath = :new_filepath
            WHERE filepath = :old_filepath"
        ))?;
        stmt.execute(named_params! {
            ":new_filepath": new_filepath,
            ":old_filepath": old_filepath,
        })
    }

    pub fn count(conn: &Connection) -> Result<i64, Error> {
        let mut stmt = conn.prepare("SELECT count(*) from media")?;
        stmt.query_row([], |row| row.get(0))
    }

    pub fn count_by_format(conn: &Connection) -> Result<HashMap<Option<String>, i64>, Error> {
        let mut stmt = conn.prepare("SELECT format, COUNT(*) from media GROUP BY format")?;
        let iter = stmt.query_and_then([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        iter.collect()
    }

    pub fn count_by_device(conn: &Connection) -> Result<HashMap<Option<String>, i64>, Error> {
        let mut stmt = conn.prepare("SELECT device, COUNT(*) from media GROUP BY device")?;
        let iter = stmt.query_and_then([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        iter.collect()
    }

    pub fn exists_by_filepath(conn: &Connection, filepath: &PathBufSql) -> Result<bool, Error> {
        let mut stmt = conn.prepare("SELECT 1 FROM media WHERE filepath = :filepath")?;
        stmt.exists(named_params! {
            ":filepath": filepath,
        })
    }

    #[allow(dead_code)]
    pub fn exists_by_hash(conn: &Connection, hash: HashSql) -> Result<bool, Error> {
        let mut stmt = conn.prepare("SELECT 1 FROM media WHERE hash = :hash")?;
        stmt.exists(named_params! {
            ":hash": hash,
        })
    }
}

impl MediaSearch<'_> {
    pub fn new(conn: &Connection, filter: MediaFilter) -> Result<MediaSearch, Error> {
        let mut sql = match filter.label {
            Some(_) => formatcp!(
                "SELECT {COLUMNS_WITH_PREFIX} FROM label \
                    JOIN media ON media.filepath = label.filepath \
                    WHERE label = :label",
            ),
            None => formatcp!(
                "SELECT {COLUMNS} FROM media \
                    WHERE true",
            ),
        }
        .to_string();
        if filter.device.is_some() {
            sql.push_str("\nAND device = :device");
        }
        if filter.format.is_some() {
            sql.push_str("\nAND format = :format");
        }
        if filter.created_start.is_some() {
            sql.push_str("\nAND created >= :created_start");
        }
        if filter.created_end.is_some() {
            sql.push_str("\nAND created <= :created_end");
        }
        sql.push_str("\nORDER BY created");
        let statement = conn.prepare(&sql)?;
        Ok(MediaSearch { statement, filter })
    }

    pub fn new_with_filter_defaults(conn: &Connection) -> Result<MediaSearch, Error> {
        Self::new(conn, MediaFilter::default())
    }

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
        if let Some(label) = &self.label {
            result.push((":label", label as &dyn ToSql))
        }
        if let Some(device) = &self.device {
            result.push((":device", device as &dyn ToSql))
        }
        if let Some(format) = &self.format {
            result.push((":format", format as &dyn ToSql))
        }
        result
    }
}

impl<'conn> MediaDuplicates<'conn> {
    pub fn new(conn: &'conn Connection) -> Result<Self, Error> {
        let statement = conn.prepare(
            "SELECT m.* FROM media m
                    JOIN (
                        SELECT hash
                        FROM media
                        GROUP BY hash
                        HAVING COUNT(*) > 1
                    ) AS duplicates ON m.hash = duplicates.hash;",
        )?;
        Ok(Self { statement })
    }

    pub fn iter(&mut self) -> Result<impl Iterator<Item = Result<MediaSql, Error>> + '_, Error> {
        let iter = self
            .statement
            .query_map([], |row| MediaSql::try_from(row))?;
        Ok(iter)
    }
}

impl<'conn> MediaNewFromImport<'conn> {
    pub fn new(conn: &'conn Connection) -> Result<Self, Error> {
        let statement = conn.prepare(
            "SELECT import.*
                 FROM import.media AS import
                 LEFT JOIN media ON import.hash = media.hash
                 WHERE media.hash IS NULL",
        )?;
        Ok(Self { statement })
    }

    pub fn iter(&mut self) -> Result<impl Iterator<Item = Result<MediaSql, Error>> + '_, Error> {
        let iter = self
            .statement
            .query_map([], |row| MediaSql::try_from(row))?;
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
            location: row.get(4)?,
            device: row.get(5)?,
            hash: row.get(6)?,
        })
    }
}

fn duplicate_row(res: &Result<i64, rusqlite::Error>) -> bool {
    matches!(res.as_ref().err().and_then(|e| e.sqlite_error_code()), Some(e) if e == ErrorCode::ConstraintViolation)
}
