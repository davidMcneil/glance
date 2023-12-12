use file_format::FileFormat;
use walkdir::WalkDir;

use crate::index::{file_to_media_row, Index};

#[test]
fn file_to_media_row_test() {
    for entry in WalkDir::new("../test-media/Canon_40D.jpg") {
        let entry = entry.unwrap();
        let media_row = file_to_media_row(&entry)
            .expect("should be ok")
            .expect("should be some");
        assert_eq!(media_row.filepath, entry.path());
        assert_eq!(media_row.size, 7958.into());
        assert_eq!(media_row.format, FileFormat::JointPhotographicExpertsGroup);
        assert!(media_row.created.is_some());
    }
}

#[test]
fn add_directory_test() {
    let mut index = Index::new();
    index.add_directory("../test-media").expect("should be ok");
    let rows = index.get_media();
    assert_eq!(rows.len(), 5);
}
