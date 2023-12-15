use std::{fs, path::Path};

use dateparser::parse;
use derive_more::Display;
use exif::{In, Tag};
use file_format::{FileFormat, Kind};
use rusqlite::{Connection, MAIN_DB};
use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

use crate::store::media_sql::MediaSql;

use self::media::{Device, Media};

pub mod media;
#[cfg(test)]
mod tests;

#[derive(Debug, Error, Display)]
pub enum Error {
    /// rusqlite: {:0}
    Rusqlite(#[from] rusqlite::Error),
    /// walkdir: {:0}
    Walkdir(#[from] walkdir::Error),
}

pub struct Index {
    connection: Connection,
}

impl Index {
    pub fn new(path: &Path) -> Result<Self, Error> {
        let connection = Connection::open(path)?;
        Self::new_impl(connection)
    }

    pub fn new_in_memory() -> Result<Self, Error> {
        let connection = Connection::open_in_memory()?;
        Self::new_impl(connection)
    }

    fn new_impl(connection: Connection) -> Result<Self, Error> {
        MediaSql::create_table(&connection)?;
        Ok(Self { connection })
    }

    pub fn add_directory(&mut self, root: &str) -> Result<(), Error> {
        let transaction = self.connection.transaction()?;
        for entry in WalkDir::new(root) {
            let entry = entry?;
            if entry.file_type().is_file() {
                match file_to_media_row(&entry) {
                    Ok(Some(new_row)) => {
                        MediaSql::from(new_row).insert(&transaction)?;
                    }
                    Ok(None) => (),
                    Err(e) => {
                        eprintln!(
                            "failed to process file {}: {}",
                            entry.path().display(),
                            e.to_string()
                        )
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

    fn backup(&self, dst_path: &str) -> Result<(), Error> {
        self.connection.backup(MAIN_DB, dst_path, None)?;
        Ok(())
    }
}

fn file_to_media_row(entry: &DirEntry) -> Result<Option<Media>, std::io::Error> {
    let path = entry.path().to_path_buf();
    let format = FileFormat::from_file(&path)?;
    match format.kind() {
        Kind::Image | Kind::Video => {
            let metadata = entry.metadata()?;
            let bytes = fs::read(&path)?;

            let mut row = Media {
                filepath: path.clone(),
                size: metadata.len().into(),
                format: format.into(),
                created: None,
                device: None,
                hash: blake3::hash(&bytes),
            };

            let file = std::fs::File::open(&path)?;
            let mut bufreader = std::io::BufReader::new(&file);
            let exifreader = exif::Reader::new();
            let exif = exifreader.read_from_container(&mut bufreader);
            if let Ok(exif) = exif {
                match exif.get_field(Tag::DateTime, In::PRIMARY) {
                    Some(date_taken) => {
                        let date_taken_string = format!("{}", date_taken.display_value());
                        if let Ok(date_taken) = parse(&date_taken_string) {
                            row.created = Some(date_taken);
                        }
                    }
                    None => (),
                }
                match exif.get_field(Tag::Model, In::PRIMARY) {
                    Some(model) => {
                        let model_string = format!("{}", model.display_value());
                        row.device = Some(Device::from(model_string));
                    }
                    None => (),
                }
            }
            Ok(Some(row))
        }
        _ => Ok(None),
    }
}
