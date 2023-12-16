use std::path::PathBuf;

use blake3::Hash;
use chrono::{DateTime, Utc};
use derive_more::{From, Into};
use file_format::FileFormat;

use crate::store::media_sql::MediaSql;

#[derive(Debug, Into, From, PartialEq, Eq)]
pub struct Device(String);

#[derive(Debug, Into, From, PartialEq, Eq)]
pub struct Size(u64);

#[derive(Debug)]
pub struct Media {
    pub filepath: PathBuf,
    pub size: Size,
    pub format: FileFormat,
    pub created: Option<DateTime<Utc>>,
    // pub location: (),
    pub device: Option<Device>,
    // pub iso: (),
    pub hash: Option<Hash>,
}

impl From<MediaSql> for Media {
    fn from(value: MediaSql) -> Self {
        Self {
            filepath: value.filepath.into(),
            size: value.size.into(),
            format: value.format.into(),
            created: value.created,
            device: value.device.map(|d| d.into()),
            hash: value.hash.map(|h| h.into()),
        }
    }
}

impl From<Media> for MediaSql {
    fn from(value: Media) -> Self {
        Self {
            filepath: value.filepath.into(),
            size: value.size.into(),
            format: value.format.into(),
            created: value.created,
            device: value.device.map(|d| d.into()),
            hash: value.hash.map(|h| h.into()),
        }
    }
}
