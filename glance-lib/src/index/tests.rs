use anyhow::{anyhow, Result};
use file_format::FileFormat;
use glance_util::function;
use insta::assert_debug_snapshot;
use walkdir::WalkDir;

use crate::index::{file_to_media_row, media::Device, Index};

#[test]
fn file_to_media_row_test() -> Result<()> {
    for entry in WalkDir::new("../test-media/exif-images/Canon_40D.jpg") {
        let entry = entry?;
        let media_row = file_to_media_row(&entry)?.ok_or_else(|| anyhow!("should be some"))?;
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
    index.add_directory("../test-media")?;
    assert_debug_snapshot!(index.get_media()?);
    Ok(())
}
