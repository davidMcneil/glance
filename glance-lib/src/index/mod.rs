use std::fs;

use dateparser::parse;
use derive_more::Display;
use exif::{In, Tag};
use file_format::{FileFormat, Kind};
use rusqlite::Connection;
use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

use crate::store::media_sql::MediaSql;

use self::media::Media;

mod media;
#[cfg(test)]
mod tests;

#[derive(Debug, Error, Display)]
pub enum Error {
    /// sql: {:0}
    Sql(#[from] rusqlite::Error),
}

pub struct Index {
    connection: Connection,
}

impl Index {
    fn new() -> Self {
        let connection = Connection::open_in_memory().expect("able to open in memory connection");
        MediaSql::create_table(&connection).expect("able to create table");
        Self { connection }
    }

    pub fn add_directory(&mut self, root: &str) -> Result<(), Error> {
        let transaction = self.connection.transaction()?;
        for entry in WalkDir::new(root) {
            let entry = entry.expect("todo");
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

    fn get_media(&self) -> Vec<Media> {
        MediaSql::get_rows(&self.connection)
            .unwrap()
            .into_iter()
            .map(|row| row.into())
            .collect()
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
            }
            Ok(Some(row))
        }
        _ => Ok(None),
    }
}
