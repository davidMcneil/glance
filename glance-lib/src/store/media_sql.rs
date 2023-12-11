use rusqlite::{named_params, Connection, Error};
use time::OffsetDateTime;

use super::converters::{FileFormatSql, HashSql, PathBufSql};

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

impl MediaSql {
    pub fn create_table(conn: &Connection) -> Result<(), Error> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS media (
                    filepath TEXT NOT NULL,
                    size INTEGER NOT NULL,
                    format TEXT NOT NULL,
                    created TEXT NOT NULL,
                    device TEXT NOT NULL,
                    hash BLOB NOT NULL,
                    INDEX hash_index (hash)
                    UNIQUE (hash)
                    UNIQUE (filepath)
            )",
            [],
        )?;
        Ok(())
    }

    pub fn insert(&self, conn: &Connection) -> Result<(), Error> {
        let mut stmt = conn.prepare(
            "
        INSERT INTO media (filepath, size, format, created, device, hash)
        VALUES (:filepath, :size, :format, :created, :device, :hash)
    ",
        )?;
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
