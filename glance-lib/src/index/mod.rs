use std::{fs, path::Path};

use chrono::{DateTime, Utc};
use dateparser::parse_with_timezone;
use derive_more::Display;
use exif::{Exif, In, Rational, Tag, Value};
use file_format::{FileFormat, Kind};
use reverse_geocoder::ReverseGeocoder;
use rusqlite::{Connection, ErrorCode};
use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

use crate::store::media_sql::{MediaDuplicates, MediaSearch, MediaSql};

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

#[derive(Debug, Default)]
pub struct AddDirectoryConfig {
    /// Compute the hash of the files
    pub hash: bool,
    /// Filter contents to only include images and videos
    pub filter_by_media: bool,
    /// Use the modified time of the file if created is not set in exif data
    pub use_modified_if_created_not_set: bool,
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
        fs::create_dir_all(&path)?;

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

    pub fn add_directory<P: AsRef<Path>>(
        &mut self,
        path: P,
        config: &AddDirectoryConfig,
    ) -> Result<(), Error> {
        let transaction = self.connection.transaction()?;
        for entry in WalkDir::new(path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                match file_to_media_row(&entry, config) {
                    Ok(Some(new_row)) => {
                        let res = MediaSql::from(new_row).insert(&transaction);
                        if let Some(e) = res.as_ref().err().and_then(|e| e.sqlite_error_code()) {
                            if e == ErrorCode::ConstraintViolation {
                                // eprintln!("duplicate file '{}'", entry.path().display());
                                continue;
                            }
                        }
                        res?;
                    }
                    Ok(None) => (),
                    Err(e) => {
                        eprintln!("failed to process file '{}': {}", entry.path().display(), e)
                    }
                }
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn get_media(&self) -> Result<Vec<Media>, Error> {
        MediaSearch::new_with_filter_defaults(&self.connection)?
            .iter()?
            .map(from_media_sql_result)
            .collect()
    }

    pub fn duplicates(&self) -> Result<Vec<Media>, Error> {
        MediaDuplicates::new(&self.connection)?
            .iter()?
            .map(from_media_sql_result)
            .collect()
    }

    pub fn standardize_naming<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let path = path.as_ref();
        for media in MediaSearch::new_with_filter_defaults(&self.connection)?
            .iter()?
            .map(from_media_sql_result)
        {
            let media = media?;
            if let Some(created) = media.created {
                let folder_name = created.format("%Y-%m").to_string();
                let destination_folder = path.join(&folder_name);
                let Some(destination_file_name) = media.filepath.file_name() else {
                    continue;
                };
                fs::create_dir_all(&destination_folder)?;
                let destination_path = destination_folder.join(destination_file_name);
                fs::rename(&media.filepath, &destination_path)?;

                println!(
                    "Moved {} to {}",
                    media.filepath.display(),
                    destination_path.display()
                );
            }
        }
        Ok(())
    }
}

fn file_to_media_row(
    entry: &DirEntry,
    config: &AddDirectoryConfig,
) -> Result<Option<Media>, Error> {
    let path = entry.path().to_path_buf();
    let format = FileFormat::from_file(&path)?;
    if config.filter_by_media && !matches!(format.kind(), Kind::Image | Kind::Video) {
        return Ok(None);
    }
    let metadata = entry.metadata()?;
    let hash = if config.hash {
        let bytes = fs::read(&path)?;
        Some(blake3::hash(&bytes))
    } else {
        None
    };
    let created = if config.use_modified_if_created_not_set {
        metadata.created().ok().map(DateTime::<Utc>::from)
    } else {
        None
    };

    let mut row = Media {
        filepath: path.clone(),
        size: metadata.len().into(),
        format,
        created,
        location: None,
        device: None,
        hash,
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
        row.location = get_location_from_exif(&exif);
    }
    Ok(Some(row))
}

fn get_location_from_exif(exif: &Exif) -> Option<String> {
    fn to_decimal_degrees(degree_minute_second: &[Rational], bearing: &str) -> Option<f64> {
        let degrees = degree_minute_second.get(0)?.num as i32;
        let minutes = degree_minute_second.get(1)?.num as i32;
        let seconds =
            (degree_minute_second.get(2)?.num as f64) / (degree_minute_second.get(2)?.denom as f64);
        let ddeg: f64 = degrees as f64 + minutes as f64 / 60.0_f64 + seconds / 3600.0_f64;
        match bearing {
            "N" | "E" => Some(ddeg),
            "S" | "W" => Some(-ddeg),
            _ => None,
        }
    }
    let latitude = exif.get_field(Tag::GPSLatitude, In::PRIMARY)?;
    let latitude_ref = exif.get_field(Tag::GPSLatitudeRef, In::PRIMARY)?;
    let longitude = exif.get_field(Tag::GPSLongitude, In::PRIMARY)?;
    let longitude_ref = exif.get_field(Tag::GPSLongitudeRef, In::PRIMARY)?;
    match (
        &latitude.value,
        &longitude.value,
        format!("{}", latitude_ref.display_value()),
        format!("{}", longitude_ref.display_value()),
    ) {
        (Value::Rational(lat_dms), Value::Rational(long_dms), lat_bearing, long_bearing) => {
            let lat_degrees = to_decimal_degrees(lat_dms, &lat_bearing)?;
            let long_degrees = to_decimal_degrees(long_dms, &long_bearing)?;

            let geocoder = ReverseGeocoder::new();
            let search_result = geocoder.search((lat_degrees, long_degrees));
            Some(format!(
                "{}, {}",
                search_result.record.name, search_result.record.admin1
            ))
        }
        _ => None,
    }
}

fn from_media_sql_result(media_sql: Result<MediaSql, rusqlite::Error>) -> Result<Media, Error> {
    media_sql.map(|m| m.into()).map_err(|e| e.into())
}
