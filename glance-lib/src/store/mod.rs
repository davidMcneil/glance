use std::path::PathBuf;

use blake3::Hash;
use file_format::FileFormat;
use time::OffsetDateTime;

#[cfg(test)]
mod tests;

pub(crate) struct MediaRow {
    pub filepath: PathBuf,
    pub size: u64,
    pub format: FileFormat,
    pub created: OffsetDateTime,
    // pub location: (),
    pub device: String,
    // pub iso: (),
    pub hash: Hash,
}
