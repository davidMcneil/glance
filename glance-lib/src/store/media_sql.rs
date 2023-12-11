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
