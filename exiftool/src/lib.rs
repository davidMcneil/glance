use chrono::{DateTime, NaiveDateTime, Utc};
use displaydoc::Display;
use serde::{Deserialize, Serialize};
use slog::{warn, Logger};
use std::{
    collections::VecDeque,
    path::Path,
    process::{Command, ExitStatus},
};
use thiserror::Error;

#[derive(Error, Debug, Display)]
pub enum Error {
    /// exiftool returned failed status code: {0}
    ExiftoolCommandFailed(ExitStatus),
    /// io: {0}"
    Io(#[from] std::io::Error),
    /// no exif data found
    MissingExifData,
    /// serde_json: {0}"
    JsonParseError(#[from] serde_json::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExiftoolData {
    #[serde(
        alias = "CreateDate",
        deserialize_with = "deserialize_datetime",
        default
    )]
    pub created: Option<DateTime<Utc>>,
}

impl ExiftoolData {
    pub fn get(path: &Path, logger: &Logger) -> Result<ExiftoolData, Error> {
        let output = Command::new("exiftool").arg("-json").arg(path).output()?;
        if !output.status.success() {
            return Err(Error::ExiftoolCommandFailed(output.status));
        }
        let mut exiftool_data_list =
            serde_json::from_slice::<VecDeque<ExiftoolData>>(&output.stdout)?;
        let first = exiftool_data_list.pop_front();
        if !exiftool_data_list.is_empty() {
            warn!(logger, "multiple exif data returned, ignoring all but first"; "path" => path.display());
        }
        first.ok_or_else(|| Error::MissingExifData)
    }
}

fn deserialize_datetime<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = <Option<String>>::deserialize(deserializer)?;
    Ok(if let Some(s) = s {
        // TODO: this is a datetime not supported by dateparser, open pull request to add it
        if let Ok(t) = NaiveDateTime::parse_from_str(&s, "%Y:%m:%d %H:%M:%S") {
            let t = t
                .and_local_timezone(Utc)
                .single()
                .ok_or_else(|| serde::de::Error::custom("ambiguous datetime"))?;
            return Ok(Some(t));
        }
        let t = dateparser::parse_with_timezone(&s, &Utc).map_err(serde::de::Error::custom)?;
        Some(t)
    } else {
        None
    })
}
