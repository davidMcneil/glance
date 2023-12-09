use file_format::FileFormat;
use walkdir::WalkDir;

use crate::index::Index;

#[test]
fn file_to_media_row_test() {
    for entry in WalkDir::new("../test-media/ferris.jpg") {
        let entry = entry.unwrap();
        let media_row = Index::file_to_media_row(&entry).unwrap();
        assert_eq!(media_row.filepath, entry.path());
        assert_eq!(media_row.size, 14737);
        assert_eq!(media_row.format, FileFormat::JointPhotographicExpertsGroup);
    }
}
