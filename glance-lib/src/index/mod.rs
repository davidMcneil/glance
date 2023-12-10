use exif::{In, Tag};
use file_format::{FileFormat, Kind};
use std::fs;
use time::{macros::format_description, OffsetDateTime};
use walkdir::{DirEntry, WalkDir};

use crate::store::{self, media_sql::MediaSql};

#[cfg(test)]
mod tests;

struct Index;

impl Index {
    fn add_directory(root: &str) {
        for entry in WalkDir::new(root) {
            let entry = entry.unwrap();
            if entry.file_type().is_file() {
                if let Some(new_row) = file_to_media_row(&entry) {
                    new_row.insert(todo!());
                }
            }
        }
    }
}

fn file_to_media_row(entry: &DirEntry) -> Option<MediaSql> {
    let path = entry.path().to_path_buf();
    let format = FileFormat::from_file(&path).unwrap();
    match format.kind() {
        Kind::Image | Kind::Video => {
            let metadata = entry.metadata().unwrap();
            let bytes = fs::read(&path).unwrap();

            let mut row = MediaSql {
                filepath: path.clone().into(),
                size: metadata.len(),
                format: format.into(),
                created: None,
                device: None,
                hash: blake3::hash(&bytes).into(),
            };

            let file = std::fs::File::open(&path).unwrap();
            let mut bufreader = std::io::BufReader::new(&file);
            let exifreader = exif::Reader::new();
            let exif = exifreader.read_from_container(&mut bufreader).unwrap();
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
                    let date_taken_string = format!("{} \"+00:00\"", date_taken.display_value());
                    let format =
                    format_description!("[year]-[month]-[day] [hour]:[minute]:[second] \"[offset_hour]:[offset_minute]\"");
                    let date = OffsetDateTime::parse(&date_taken_string, &format).unwrap();
                    row.created = Some(date);
                }
                None => (),
            }
            Some(row)
        }
        _ => None,
    }
}
