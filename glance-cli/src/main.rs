use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use glance_lib::index::{AddDirectoryConfig, Index};

/// The quick media viewer
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to directory with media contents
    #[arg(long)]
    media_path: PathBuf,
    /// Path to save the media db index
    #[arg(long)]
    db_path: PathBuf,
    /// Enable hashing of every file
    #[arg(long)]
    enable_hash: bool,
    /// Standardize naming of files by moving them to folders of format `YY-mm-dd` within
    /// media path
    #[arg(long)]
    standardize_naming: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut index = Index::new(args.db_path)?;
    let config = AddDirectoryConfig {
        hash: false,
        filter_by_media: false,
        use_modified_if_created_not_set: true,
    };
    index.add_directory(&args.media_path, &config)?;

    let duplicates = index.duplicates();
    println!("{:?}", duplicates);

    if args.standardize_naming {
        index.standardize_naming(&args.media_path)?;
    }

    Ok(())
}
