use std::{fs, path::Path};

use chrono::Utc;
use dateparser::parse_with_timezone;
use derive_more::Display;
use exif::{In, Tag};
use file_format::{FileFormat, Kind};
use rusqlite::Connection;
use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

use crate::store::media_sql::MediaSql;

use self::media::{Device, Media};

pub mod media;
#[cfg(test)]
mod tests;

#[derive(Debug, Error, Display)]
pub enum Error {
    /// io: {:0}
    Io(#[from] std::io::Error),
    /// rusqlite: {:0}
    Rusqlite(#[from] rusqlite::Error),
    /// walkdir: {:0}
    Walkdir(#[from] walkdir::Error),
}

pub struct Index {
    connection: Connection,
}

impl Index {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let connection = Connection::open(path)?;
        Self::new_impl(connection)
    }

    pub fn new_in_memory() -> Result<Self, Error> {
        let connection = Connection::open_in_memory()?;
        Self::new_impl(connection)
    }

    /// Create a new index and store its db at `test-dbs/<test>.db`
    #[cfg(test)]
    pub fn new_for_test(test: &str) -> Result<Self, Error> {
        let crate_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let path = crate_root.join("test-dbs");
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }

        let mut path = path.join(test);
        path.set_extension("db");
        // Start with a clean database
        if path.exists() {
            fs::remove_file(&path)?;
        }

        Self::new(path)
    }

    fn new_impl(mut connection: Connection) -> Result<Self, Error> {
        MediaSql::create_table(&mut connection)?;
        Ok(Self { connection })
    }

    pub fn add_directory<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Error> {
        let transaction = self.connection.transaction()?;
        for entry in WalkDir::new(path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                match file_to_media_row(&entry) {
                    Ok(Some(new_row)) => {
                        MediaSql::from(new_row).insert(&transaction)?;
                    }
                    Ok(None) => (),
                    Err(e) => {
                        eprintln!("failed to process file {}: {}", entry.path().display(), e)
                    }
                }
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn get_media(&self) -> Result<Vec<Media>, Error> {
        Ok(MediaSql::get_rows(&self.connection)?
            .into_iter()
            .map(|row| row.into())
            .collect())
    }
}

fn file_to_media_row(entry: &DirEntry) -> Result<Option<Media>, Error> {
    let path = entry.path().to_path_buf();
    let format = FileFormat::from_file(&path)?;
    match format.kind() {
        Kind::Image | Kind::Video => {
            let metadata = entry.metadata()?;
            let bytes = fs::read(&path)?;

            let mut row = Media {
                filepath: path.clone(),
                size: metadata.len().into(),
                format,
                created: None,
                device: None,
                hash: blake3::hash(&bytes),
            };

            let file = std::fs::File::open(&path)?;
            let mut bufreader = std::io::BufReader::new(&file);
            let exifreader = exif::Reader::new();
            let exif = exifreader.read_from_container(&mut bufreader);
            if let Ok(exif) = exif {
                if let Some(date_taken) = exif.get_field(Tag::DateTime, In::PRIMARY) {
                    let date_taken_string = format!("{}", date_taken.display_value());
                    if let Ok(date_taken) = parse_with_timezone(&date_taken_string, &Utc) {
                        row.created = Some(date_taken);
                    }
                }
                if let Some(model) = exif.get_field(Tag::Model, In::PRIMARY) {
                    let model_string = format!("{}", model.display_value());
                    row.device = Some(Device::from(model_string));
                }
            }
            Ok(Some(row))
        }
        _ => Ok(None),
    }
}
