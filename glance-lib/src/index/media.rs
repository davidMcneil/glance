use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use blake3::Hash;
use chrono::{DateTime, Utc};
use derive_more::{From, Into};
use serde::Serialize;

pub use crate::store::media_sql::MediaFilter;
use crate::store::media_sql::MediaSql;

use super::Stats;

#[derive(Debug, Into, From, PartialEq, Eq, Serialize)]
pub struct Device(pub String);

#[derive(Debug, Into, From, PartialEq, Eq, Serialize)]
pub struct Size(pub u64);

#[derive(Debug, Serialize)]
pub struct Media {
    pub filepath: PathBuf,
    pub size: Size,
    pub format: String,
    pub created: Option<DateTime<Utc>>,
    pub modified: DateTime<Utc>,
    pub location: Option<String>,
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
            modified: value.modified,
            location: value.location,
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
            modified: value.modified,
            location: value.location,
            device: value.device.map(|d| d.into()),
            hash: value.hash.map(|h| h.into()),
        }
    }
}

pub fn stats_from_media(media_vec: &Vec<Media>) -> Result<Stats, super::Error> {
    let mut count_by_format = HashMap::new();
    let mut count_by_device = HashMap::new();
    // TODO: populate count by year
    let count_by_year = HashMap::new();
    let mut hashes_seen = HashSet::new();
    let mut duplicates = 0;
    for media in media_vec {
        *count_by_format
            .entry(Some(media.format.to_string()))
            .or_default() += 1;
        if let Some(device) = &media.device {
            *count_by_device.entry(Some(device.0.clone())).or_default() += 1;
        }
        if let Some(hash) = media.hash {
            if hashes_seen.contains(&hash) {
                duplicates += 1;
            } else {
                hashes_seen.insert(hash);
            }
        }
    }
    Ok(Stats {
        count: media_vec.len() as i64,
        count_by_format,
        count_by_device,
        count_by_year,
        duplicates,
    })
}
