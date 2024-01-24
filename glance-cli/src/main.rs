use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use glance_lib::index::{AddDirectoryConfig, Index};
use glance_util::canonicalized_path_buf::CanonicalizedPathBuf;
use sloggers::{
    terminal::TerminalLoggerBuilder,
    types::{Severity, SourceLocation},
    Build,
};

/// The quick media viewer
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to directory with media contents
    #[arg(long)]
    media_path: CanonicalizedPathBuf,
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
    /// Log level
    #[arg(long)]
    log_level: Option<Severity>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Build a terminal logger
    let logger = TerminalLoggerBuilder::new()
        .level(args.log_level.unwrap_or_default())
        .source_location(SourceLocation::None)
        .build()?;

    let mut index = Index::new(args.db_path)?.with_logger(logger);

    let media_path = PathBuf::from(args.media_path);
    let config = AddDirectoryConfig {
        hash: false,
        filter_by_media: false,
        use_modified_if_created_not_set: true,
        calculate_nearest_city: false,
    };
    index.add_directory(&media_path, &config)?;

    let stats = index.stats()?;
    println!("{}", serde_json::to_string_pretty(&stats)?);

    if args.standardize_naming {
        index.standardize_naming(&media_path)?;
    }

    Ok(())
}
