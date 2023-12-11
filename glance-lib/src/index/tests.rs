use file_format::FileFormat;
use walkdir::WalkDir;

use crate::index::file_to_media_row;

#[test]
fn file_to_media_row_test() {
    for entry in WalkDir::new("../test-media/Canon_40D.jpg") {
        let entry = entry.unwrap();
        let media_row = file_to_media_row(&entry).unwrap();
        assert_eq!(media_row.filepath, entry.path());
        assert_eq!(media_row.size, 7958.into());
        assert_eq!(media_row.format, FileFormat::JointPhotographicExpertsGroup);
        assert!(media_row.created.is_some());
    }
}
