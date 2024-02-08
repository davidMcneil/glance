use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use glance_lib::index::{AddDirectoryConfig, Index};
use glance_util::canonicalized_path_buf::CanonicalizedPathBuf;
use sloggers::{
    terminal::TerminalLoggerBuilder,
    types::{Severity, SourceLocation},
    Build,
};

/// The quick media viewer
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to save the media db index
    #[arg(long)]
    index_path: PathBuf,
    /// Enable hashing of files when storing in index
    #[arg(long)]
    calculate_hash: bool,
    /// Use the file modified time if the created time is not set in exif data
    #[arg(long)]
    use_modified_if_created_not_set: bool,
    /// Filter files that are not media
    #[arg(long)]
    filter_by_media_type: bool,
    /// Calculate the nearest city from exif data
    #[arg(long)]
    calculate_nearest_city: bool,
    /// How to name files
    #[arg(long, value_enum)]
    naming: Naming,
    /// Log level
    #[arg(long)]
    log_level: Option<Severity>,
    // Subcommands
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum Naming {
    /// Naming scheme is left unchanged
    #[default]
    Free,
    /// Standardize naming of files by moving them to folders of format `YY-mm-dd` within
    /// media path
    Time,
}

impl Naming {
    fn is_time(self) -> bool {
        matches!(self, Self::Time)
    }
}

#[derive(Debug, Parser)]
struct IndexMedia {
    /// Path to directory to import media files into
    #[arg(long)]
    media_path: CanonicalizedPathBuf,
}

#[derive(Debug, Parser)]
struct ImportMedia {
    /// Path to directory with media files to import
    #[arg(long)]
    import_media_path: CanonicalizedPathBuf,
    /// Path to directory to import media files into
    #[arg(long)]
    media_path: CanonicalizedPathBuf,
}

/// Doc comment
#[derive(Subcommand, Debug)]
#[command()]
enum Command {
    /// Add media files to the index
    #[command()]
    IndexMedia(IndexMedia),
    /// Copy media files from a directory to the `media-path`
    #[command()]
    ImportMedia(ImportMedia),
    /// Print stats on the media
    #[command()]
    Stats,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Build a terminal logger
    let logger = TerminalLoggerBuilder::new()
        .level(args.log_level.unwrap_or_default())
        .source_location(SourceLocation::None)
        .build()?;

    let mut index = Index::new(args.index_path)?.with_logger(logger);

    match args.command {
        Command::IndexMedia(sub_args) => {
            let media_path = PathBuf::from(sub_args.media_path);
            let config = AddDirectoryConfig {
                hash: args.calculate_hash,
                filter_by_media: args.filter_by_media_type,
                use_modified_if_created_not_set: args.use_modified_if_created_not_set,
                calculate_nearest_city: args.calculate_nearest_city,
            };
            index.add_directory(&media_path, &config)?;
            // TODO: should also validate every entry in index is in media_path

            if args.naming.is_time() {
                index.standardize_naming(&media_path)?;
            }
        }
        Command::ImportMedia(_) => todo!(),
        Command::Stats => {
            let stats = index.stats()?;
            println!("{}", serde_json::to_string_pretty(&stats)?);
        }
    }

    Ok(())
}
