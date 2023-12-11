use core::prelude::rust_2015;
use derive_more::Display;
use exif::{In, Tag};
use file_format::{FileFormat, Kind};
use rusqlite::Connection;
use std::fs;
use thiserror::Error;
use time::{macros::format_description, OffsetDateTime};
use walkdir::{DirEntry, WalkDir};

use crate::store::{self, media_sql::MediaSql};

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
        let connection = Connection::open_in_memory().unwrap();
        MediaSql::create_table(&connection).unwrap();
        Self { connection }
    }

    pub fn add_directory(&mut self, root: &str) -> Result<(), Error> {
        let transaction = self.connection.transaction()?;
        for entry in WalkDir::new(root) {
            let entry = entry.expect("todo");
            if entry.file_type().is_file() {
                if let Some(new_row) = file_to_media_row(&entry) {
                    MediaSql::from(new_row).insert(&transaction)?;
                }
            }
        }
        transaction.commit()?;
        Ok(())
    }

    fn get_media(&self) -> Vec<MediaSql> {
        MediaSql::get_rows(&self.connection).unwrap()
    }
}

fn file_to_media_row(entry: &DirEntry) -> Option<Media> {
    let path = entry.path().to_path_buf();
    let format = FileFormat::from_file(&path).unwrap();
    match format.kind() {
        Kind::Image | Kind::Video => {
            let metadata = entry.metadata().unwrap();
            let bytes = fs::read(&path).unwrap();

            let mut row = Media {
                filepath: path.clone(),
                size: metadata.len().into(),
                format: format.into(),
                created: None,
                device: None,
                hash: blake3::hash(&bytes),
            };

            let file = std::fs::File::open(&path).unwrap();
            let mut bufreader = std::io::BufReader::new(&file);
            let exifreader = exif::Reader::new();
            let exif = exifreader.read_from_container(&mut bufreader);
            if let Ok(exif) = exif {
                // for f in exif.fields() {
                //     println!(
                //         "{} {} {}",
                //         f.tag,
                //         f.ifd_num,
                //         f.display_value().with_unit(&exif)
                //     );
                // }
                match exif.get_field(Tag::DateTime, In::PRIMARY) {
                    Some(date_taken) => {
                        let date_taken_string =
                            format!("{} \"+00:00\"", date_taken.display_value());
                        let format =
                    format_description!("[year]-[month]-[day] [hour]:[minute]:[second] \"[offset_hour]:[offset_minute]\"");
                        let date = OffsetDateTime::parse(&date_taken_string, &format).unwrap();
                        row.created = Some(date);
                    }
                    None => (),
                }
            }
            Some(row)
        }
        _ => None,
    }
}
