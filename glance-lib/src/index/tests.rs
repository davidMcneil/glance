use anyhow::{anyhow, Result};
use file_format::FileFormat;
use glance_util::function;
use insta::assert_debug_snapshot;
use walkdir::WalkDir;

use crate::{
    index::{file_to_media_row, media::Device, AddDirectoryConfig, Index},
    store::media_sql::MediaFilter,
};

#[test]
fn file_to_media_row_test() -> Result<()> {
    for entry in WalkDir::new("../test-media/exif-images/Canon_40D.jpg") {
        let entry = entry?;
        let config = AddDirectoryConfig {
            hash: true,
            filter_by_media: false,
            use_modified_if_created_not_set: false,
            calculate_nearest_city: false,
        };
        let media_row =
            file_to_media_row(&entry, &config)?.ok_or_else(|| anyhow!("should be some"))?;
        assert_eq!(media_row.filepath, entry.path());
        assert_eq!(media_row.size, 7958.into());
        assert_eq!(
            media_row.format,
            FileFormat::JointPhotographicExpertsGroup.name()
        );
        assert!(media_row.created.is_some());
        assert_eq!(
            media_row.device.ok_or_else(|| anyhow!("missing device"))?,
            Device::from("Canon EOS 40D".to_string())
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
        calculate_nearest_city: true,
    };
    index.add_directory("../test-media", &config)?;
    let mut data = index.get_media()?;
    data.sort_by(|a, b| a.filepath.cmp(&b.filepath));
    assert_debug_snapshot!(data);
    Ok(())
}

#[test]
fn add_label_test() -> Result<()> {
    let mut index = Index::new_for_test(function!())?;
    let config = AddDirectoryConfig::default();
    index.add_directory("../test-media", &config)?;
    let mut data = index.get_media()?;
    data.sort_by(|a, b| a.filepath.cmp(&b.filepath));
    for media in &data {
        index.add_label(media.filepath.clone(), "all".to_string())?;
    }
    let first = data
        .get(0)
        .ok_or_else(|| anyhow!("should have first element"))?;
    index.add_label(first.filepath.clone(), "test".to_string())?;
    let mut expected = Vec::new();
    expected.push("all");
    expected.push("test");
    assert_eq!(index.get_labels(first.filepath.clone())?, expected);
    Ok(())
}

#[test]
fn get_media_with_label_filter_test() -> Result<()> {
    let mut index = Index::new_for_test(function!())?;
    let config = AddDirectoryConfig::default();
    index.add_directory("../test-media", &config)?;
    let mut data = index.get_media()?;
    data.sort_by(|a, b| a.filepath.cmp(&b.filepath));
    for media in &data {
        index.add_label(media.filepath.clone(), "all".to_string())?;
    }
    let first = data
        .get(0)
        .ok_or_else(|| anyhow!("should have first element"))?;
    index.add_label(first.filepath.clone(), "test".to_string())?;

    let labeled_first_media = index.get_media_with_filter(MediaFilter {
        label: Some("test".to_string()),
        ..Default::default()
    })?;
    assert_eq!(labeled_first_media.len(), 1);

    let labeled_all_media = index.get_media_with_filter(MediaFilter {
        label: Some("all".to_string()),
        ..Default::default()
    })?;
    assert_eq!(labeled_all_media.len(), data.len());

    let labeled_invalid_media = index.get_media_with_filter(MediaFilter {
        label: Some("invalid".to_string()),
        ..Default::default()
    })?;
    assert_eq!(labeled_invalid_media.len(), 0);
    Ok(())
}

#[test]
fn get_all_labels_test() -> Result<()> {
    let mut index = Index::new_for_test(function!())?;
    let config = AddDirectoryConfig::default();
    index.add_directory("../test-media", &config)?;
    let mut data = index.get_media()?;
    data.sort_by(|a, b| a.filepath.cmp(&b.filepath));
    for media in &data {
        index.add_label(media.filepath.clone(), "all".to_string())?;
    }
    let first = data
        .get(0)
        .ok_or_else(|| anyhow!("should have first element"))?;
    index.add_label(first.filepath.clone(), "test".to_string())?;

    let all_labels = index.get_all_labels()?;
    let mut expected = Vec::new();
    expected.push("all");
    expected.push("test");
    assert_eq!(all_labels, expected);
    Ok(())
}
