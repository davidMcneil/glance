use file_format::{FileFormat, Kind};
use std::fs::{self};
use walkdir::{DirEntry, WalkDir};

use crate::store::{self, MediaRow};

#[cfg(test)]
mod tests;

struct Index;

impl Index {
    fn add_directory(root: &str) {
        for entry in WalkDir::new(root) {
            let entry = entry.unwrap();
            if entry.file_type().is_file() {
                if let Some(new_row) = Index::file_to_media_row(&entry) {
                    store::insert_row(new_row);
                }
            }
        }
    }

    fn file_to_media_row(entry: &DirEntry) -> Option<MediaRow> {
        let path = entry.path().to_path_buf();
        let format = FileFormat::from_file(&path).unwrap();
        match format.kind() {
            Kind::Image | Kind::Video => {
                let metadata = entry.metadata().unwrap();
                let bytes = fs::read(&path).unwrap();
                Some(store::MediaRow {
                    filepath: path,
                    size: metadata.len(),
                    format,
                    created: metadata.created().unwrap().into(), // I think this is wrong, this is when the file was created not the image was taken
                    device: String::new(),
                    hash: blake3::hash(&bytes),
                })
            }
            _ => None,
        }
    }
}
