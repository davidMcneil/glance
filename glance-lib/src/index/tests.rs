use anyhow::{anyhow, Result};
use file_format::FileFormat;
use glance_util::function;
use insta::assert_debug_snapshot;
use walkdir::WalkDir;

use crate::index::{file_to_media_row, media::Device, AddDirectoryConfig, Index};

#[test]
fn file_to_media_row_test() -> Result<()> {
    for entry in WalkDir::new("../test-media/exif-images/Canon_40D.jpg") {
        let entry = entry?;
        let config = AddDirectoryConfig {
            hash: true,
            filter_by_media: false,
            use_modified_if_created_not_set: false,
        };
        let media_row =
            file_to_media_row(&entry, &config)?.ok_or_else(|| anyhow!("should be some"))?;
        assert_eq!(media_row.filepath, entry.path());
        assert_eq!(media_row.size, 7958.into());
        assert_eq!(media_row.format, FileFormat::JointPhotographicExpertsGroup);
        assert!(media_row.created.is_some());
        assert_eq!(
            media_row.device.ok_or_else(|| anyhow!("missing device"))?,
            Device::from("\"Canon EOS 40D\"".to_string())
        );
    }
    Ok(())
}

#[test]
fn add_directory_test() -> Result<()> {
    let mut index = Index::new_for_test(function!())?;
    let config = AddDirectoryConfig {
        hash: true,
        filter_by_media: false,
        use_modified_if_created_not_set: false,
    };
    index.add_directory("../test-media", &config)?;
    let mut data = index.get_media()?;
    data.sort_by(|a, b| a.filepath.cmp(&b.filepath));
    assert_debug_snapshot!(data);
    Ok(())
}
