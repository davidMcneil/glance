#[cfg(target_os = "linux")]
use std::os::unix::fs::symlink;
#[cfg(target_os = "macos")]
use std::os::unix::fs::symlink;
#[cfg(target_os = "windows")]
use std::os::windows::fs::symlink_file as symlink;
use std::{collections::HashMap, fs, path::Path};

use chrono::{DateTime, Utc};
use dateparser::parse_with_timezone;
use displaydoc::Display;
use exif::{Exif, In, Rational, Tag, Value};
use exiftool::ExiftoolData;
use file_format::{FileFormat, Kind};
use glance_util::hash_map_with_unknown::HashMapWithUnknown;
use reverse_geocoder::ReverseGeocoder;
use rusqlite::Connection;
use serde::Serialize;
use serde_with::{serde_as, FromInto};
use slog::{error, info, o, trace, Logger};
use sloggers::{null::NullLoggerBuilder, Build};
use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

use crate::index::media::{Device, Media};
use crate::store::label_sql::{LabelFilter, LabelSearch, LabelSql};
use crate::store::media_sql::{
    MediaDuplicates, MediaFilter, MediaNewFromImport, MediaSearch, MediaSql,
};

use self::label::Label;

mod label;
pub mod media;
#[cfg(test)]
mod tests;

#[derive(Debug, Error, Display)]
pub enum Error {
    /// exiftool: {0}
    Exiftool(#[from] exiftool::Error),
    /// io: {0}
    Io(#[from] std::io::Error),
    /// rusqlite: {0}
    Rusqlite(#[from] rusqlite::Error),
    /// sloggers: {0}
    Sloggers(#[from] sloggers::Error),
    /// walkdir: {0}
    Walkdir(#[from] walkdir::Error),
}

pub struct Index {
    connection: Connection,
    logger: Logger,
}

#[serde_as]
#[derive(Debug, Serialize)]
pub struct Stats {
    pub count: i64,
    #[serde_as(as = "FromInto<HashMapWithUnknown<String, i64>>")]
    pub count_by_format: HashMap<Option<String>, i64>,
    #[serde_as(as = "FromInto<HashMapWithUnknown<String, i64>>")]
    pub count_by_device: HashMap<Option<String>, i64>,
    pub duplicates: usize,
}

#[derive(Debug)]
pub struct AddDirectoryConfig {
    /// Compute the hash of the files
    pub hash: bool,
    /// Filter contents to only include images and videos
    pub filter_by_media: bool,
    /// Use the modified time of the file if created is not set in exif data
    pub use_modified_if_created_not_set: bool,
    /// Calculate the nearest city based on the exif GPS data
    pub calculate_nearest_city: bool,
    /// Try to use exiftool cli program
    pub use_exiftool: bool,
}

impl Default for AddDirectoryConfig {
    fn default() -> Self {
        Self {
            hash: false,
            filter_by_media: true,
            use_modified_if_created_not_set: true,
            calculate_nearest_city: false,
            use_exiftool: false,
        }
    }
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
        LabelSql::create_table(&mut connection)?;
        Ok(Self {
            connection,
            logger: NullLoggerBuilder.build()?,
        })
    }

    pub fn with_logger(mut self, logger: Logger) -> Self {
        self.logger = logger;
        self
    }

    pub fn add_directory<P: AsRef<Path>>(
        &mut self,
        path: P,
        config: &AddDirectoryConfig,
    ) -> Result<(), Error> {
        let logger = self
            .logger
            .new(o!("path" => path.as_ref().display().to_string()));
        info!(logger, "adding directory");
        let mut files = 0u64;
        let mut dirs = 0u64;
        let mut added = 0u64;
        let mut duplicates = 0u64;
        let mut filtered = 0u64;
        let mut failed = 0u64;
        let transaction = self.connection.transaction()?;
        for entry in WalkDir::new(path) {
            let entry = entry?;
            if entry
                .path()
                .display()
                .to_string()
                .contains("glance-exports")
            {
                continue;
            }

            if entry.file_type().is_dir() {
                dirs += 1;
            }

            if entry.file_type().is_file() {
                let filename = entry.file_name();
                if filename == "glance.db" {
                    continue;
                }

                files += 1;
                let logger = self
                    .logger
                    .new(o!("path" => entry.path().display().to_string()));

                let filepath = entry.path().to_path_buf().into();
                if MediaSql::exists_by_filepath(&transaction, &filepath)? {
                    trace!(logger, "duplicate filepath");
                    duplicates += 1;
                    continue;
                }

                match file_to_media_row(&entry, config, &logger) {
                    Ok(Some(new_row)) => {
                        trace!(logger, "adding file");
                        let inserted = MediaSql::from(new_row).insert(&transaction)?;
                        if !inserted {
                            trace!(logger, "duplicate file");
                            duplicates += 1;
                            continue;
                        }
                        added += 1;
                    }
                    Ok(None) => {
                        trace!(logger, "filtered file");
                        filtered += 1;
                    }
                    Err(e) => {
                        error!(logger, "failed to process file";
                            "error" => e.to_string(),
                        );
                        failed += 1;
                    }
                }
            }
        }
        transaction.commit()?;
        info!(logger, "added directory";
            "files" => files,
            "dirs" => dirs,
            "added" => added,
            "duplicates" => duplicates,
            "filtered" => filtered,
            "failed" => failed,
        );
        Ok(())
    }

    pub fn remove_not_in_directory<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Error> {
        let logger = self
            .logger
            .new(o!("path" => path.as_ref().display().to_string()));
        info!(logger, "removing files not in directory");
        let mut removed = 0u64;
        let transaction = self.connection.transaction()?;
        for media in MediaSearch::new_with_filter_defaults(&transaction)?
            .iter()?
            .map(from_media_sql_result)
        {
            let media = media?;
            if !media.filepath.exists() {
                trace!(self.logger, "removing from index"; "path" => media.filepath.display());
                MediaSql::from(media).delete(&transaction)?;
                removed += 1;
            }
        }
        transaction.commit()?;
        info!(logger, "removed files not in directory";
            "removed" => removed,
        );
        Ok(())
    }

    pub fn get_media(&self) -> Result<Vec<Media>, Error> {
        MediaSearch::new_with_filter_defaults(&self.connection)?
            .iter()?
            .map(from_media_sql_result)
            .collect()
    }

    pub fn get_media_with_filter(&self, media_filter: MediaFilter) -> Result<Vec<Media>, Error> {
        MediaSearch::new(&self.connection, media_filter)?
            .iter()?
            .map(from_media_sql_result)
            .collect()
    }

    pub fn stats(&self) -> Result<Stats, Error> {
        Ok(Stats {
            count: MediaSql::count(&self.connection)?,
            count_by_format: MediaSql::count_by_format(&self.connection)?,
            count_by_device: MediaSql::count_by_device(&self.connection)?,
            duplicates: self.duplicates()?.len(),
        })
    }

    pub fn duplicates(&self) -> Result<Vec<Media>, Error> {
        MediaDuplicates::new(&self.connection)?
            .iter()?
            .map(from_media_sql_result)
            .collect()
    }

    pub fn import(
        &mut self,
        import_index_path: &Path,
        media_path: &Path,
        dry_run: bool,
    ) -> Result<(), Error> {
        MediaSql::attach_for_import(import_index_path, &mut self.connection)?;
        info!(self.logger, "importing media files"; "path" => import_index_path.display());
        let mut imported = 0u64;
        let mut duplicates = 0u64;
        let transaction = self.connection.transaction()?;
        for media in MediaNewFromImport::new(&transaction)?
            .iter()?
            .map(from_media_sql_result)
        {
            let mut media = media?;
            trace!(self.logger, "importing media"; "path" => media.filepath.display());

            let Some(destination_file_name) = media.filepath.file_name() else {
                continue;
            };
            let destination_path: std::path::PathBuf = media_path.join(destination_file_name);
            if !dry_run {
                fs::copy(&media.filepath, &destination_path)?;
                media.filepath = destination_path;
                let inserted = MediaSql::from(media).insert(&transaction)?;
                if !inserted {
                    duplicates += 1;
                    continue;
                }
            }

            imported += 1;
        }
        transaction.commit()?;
        info!(self.logger, "imported media files";
            "imported" => imported,
            "duplicates" => duplicates,
        );
        Ok(())
    }

    pub fn standardize_naming<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let logger = self
            .logger
            .new(o!("path" => path.as_ref().display().to_string()));
        info!(logger, "standardizing naming");
        let path = path.as_ref();
        let mut total = 0u64;
        let mut renamed = 0u64;
        for media in MediaSearch::new_with_filter_defaults(&self.connection)?
            .iter()?
            .map(from_media_sql_result)
        {
            total += 1;
            let media = media?;
            if let Some(created) = media.created {
                let folder_name = created.format("%Y-%m").to_string();
                let destination_folder = path.join(&folder_name);
                let Some(destination_file_name) = media.filepath.file_name() else {
                    continue;
                };
                fs::create_dir_all(&destination_folder)?;
                let destination_path = destination_folder.join(destination_file_name);
                if media.filepath == destination_path {
                    continue;
                }

                if destination_path.exists() {
                    error!(self.logger, "standardized destination name already exists";
                        "old_path" => media.filepath.display(),
                        "new_path" => destination_path.display(),
                    );
                    continue;
                }
                fs::rename(&media.filepath, &destination_path)?;
                MediaSql::rename(
                    &self.connection,
                    // TODO: cleanup clones
                    &media.filepath.clone().into(),
                    &destination_path.clone().into(),
                )?;

                trace!(self.logger, "standardized naming";
                    "old_path" => media.filepath.display(),
                    "new_path" => destination_path.display(),
                );
                renamed += 1;
            }
        }
        info!(logger, "standardized naming";
            "total" => total,
            "renamed" => renamed,
        );
        Ok(())
    }

    pub fn add_label<P: AsRef<Path>>(&self, path: P, label: String) -> Result<(), Error> {
        let label = Label {
            filepath: path.as_ref().to_path_buf(),
            label,
        };
        LabelSql::from(label).insert(&self.connection)?;
        Ok(())
    }

    pub fn delete_label<P: AsRef<Path>>(&self, path: P, label: String) -> Result<(), Error> {
        let label = Label {
            filepath: path.as_ref().to_path_buf(),
            label,
        };
        LabelSql::from(label).delete(&self.connection)?;
        Ok(())
    }

    pub fn get_labels<P: AsRef<Path>>(&self, path: P) -> Result<Vec<String>, Error> {
        LabelSearch::new(
            &self.connection,
            LabelFilter {
                filepath: Some(path.as_ref().to_path_buf().into()),
            },
        )?
        .iter()?
        .map(from_label_sql_result)
        .map(|label| label.map(|l| l.label))
        .collect()
    }

    pub fn get_all_labels(&self) -> Result<Vec<String>, Error> {
        LabelSql::get_all_labels(&self.connection).map_err(|e| e.into())
    }

    pub fn export_images_with_label(
        &self,
        path_to_index: String,
        label: String,
    ) -> Result<(), Error> {
        let labeled_media = self.get_media_with_filter(MediaFilter {
            label: Some(label.clone()),
            ..Default::default()
        })?;
        let label_folder = format!("{path_to_index}/glance-exports/{label}");
        fs::create_dir_all(label_folder.clone())?;
        for media in labeled_media {
            if let Some(filename) = media.filepath.file_name() {
                if let Some(filename) = filename.to_str() {
                    symlink(&media.filepath, format!("{label_folder}/{filename}"))?;
                }
            }
        }
        info!(self.logger, "exported all images with label";
            "label" => label,
            "label_folder" => label_folder,
        );
        Ok(())
    }
}

fn file_to_media_row(
    entry: &DirEntry,
    config: &AddDirectoryConfig,
    logger: &Logger,
) -> Result<Option<Media>, Error> {
    let path = entry.path().to_path_buf();
    let format = FileFormat::from_file(&path)?;
    if config.filter_by_media && !matches!(format.kind(), Kind::Image) {
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
        format: format.name().to_string(),
        created,
        location: None,
        device: None,
        hash,
    };

    let file = std::fs::File::open(&path)?;
    let mut bufreader = std::io::BufReader::new(&file);
    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut bufreader);
    match exif {
        Ok(exif) => {
            if let Some(date_taken) = exif.get_field(Tag::DateTime, In::PRIMARY) {
                let date_taken_string = format!("{}", date_taken.display_value());
                if let Ok(date_taken) = parse_with_timezone(&date_taken_string, &Utc) {
                    row.created = Some(date_taken);
                }
            }
            if let Some(model) = exif.get_field(Tag::Model, In::PRIMARY) {
                let model_string = exif_field_to_string(model);
                row.device = Some(Device::from(model_string));
            }
            if config.calculate_nearest_city {
                row.location = get_location_from_exif(&exif);
            }
        }
        Err(e) => {
            trace!(logger, "failed reading exif"; "error" => e.to_string());
            if config.use_exiftool {
                let exif = ExiftoolData::get(&path, logger)?;
                row.created = exif.created;
            }
        }
    }
    if row.created.is_none() {
        error!(logger, "failed to get created");
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

fn from_label_sql_result(label_sql: Result<LabelSql, rusqlite::Error>) -> Result<Label, Error> {
    label_sql.map(|l| l.into()).map_err(|e| e.into())
}

fn exif_field_to_string(field: &exif::Field) -> String {
    use exif::Value::*;

    match &field.value {
        Ascii(ascii) => ascii
            .iter()
            .map(|bytes| String::from_utf8_lossy(bytes).trim().to_string())
            .filter(|s| !String::is_empty(s))
            .collect::<Vec<_>>()
            .join(" "),
        _ => field.display_value().to_string(),
    }
}
