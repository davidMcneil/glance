use std::path::PathBuf;

use blake3::Hash;
use chrono::{DateTime, Utc};
use derive_more::{From, Into};
use file_format::FileFormat;

use crate::store::media_sql::MediaSql;

#[derive(Debug, Into, From)]
pub struct Device(String);

#[derive(Debug, Into, From, PartialEq, Eq)]
pub struct Size(u64);

#[derive(Debug)]
pub(crate) struct Media {
    pub filepath: PathBuf,
    pub size: Size,
    pub format: FileFormat,
    pub created: Option<DateTime<Utc>>,
    // pub location: (),
    pub device: Option<Device>,
    // pub iso: (),
    pub hash: Hash,
}

impl From<MediaSql> for Media {
    fn from(value: MediaSql) -> Self {
        Self {
            filepath: value.filepath.into(),
            size: value.size.into(),
            format: value.format.into(),
            created: value.created.map(|c| c.into()),
            device: value.device.map(|d| d.into()),
            hash: value.hash.into(),
        }
    }
}

impl From<Media> for MediaSql {
    fn from(value: Media) -> Self {
        Self {
            filepath: value.filepath.into(),
            size: value.size.into(),
            format: value.format.into(),
            created: value.created.map(|c| c.into()),
            device: value.device.map(|d| d.into()),
            hash: value.hash.into(),
        }
    }
}
