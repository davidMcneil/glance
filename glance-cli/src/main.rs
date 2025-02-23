use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, ValueEnum};
use directories::ProjectDirs;
use glance_lib::index::{AddDirectoryConfig, Index as GlanceIndex};
use glance_util::canonicalized_path_buf::CanonicalizedPathBuf;
use sloggers::{
    terminal::TerminalLoggerBuilder,
    types::{Severity, SourceLocation},
    Build,
};

const QUALIFIER: &str = "";
const ORGANIZATION: &str = "";
const APPLICATION: &str = "glance";

/// The quick media viewer
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to save the media db index
    #[arg(long, default_value = default_main_index_path())]
    index: PathBuf,
    /// Disable hashing of files when storing in index
    ///
    /// This drasticly speeds up the initial indexing
    #[arg(long)]
    disable_hash: bool,
    /// Use the file created time if the created time is not set in exif data
    #[arg(long)]
    metadata_fallback_for_created: bool,
    /// Filter files that are not media
    // TODO: allow filtering by any type
    #[arg(long)]
    filter_by_media_type: bool,
    /// Calculate the nearest city from exif data
    #[arg(long)]
    calculate_nearest_city: bool,
    /// Disable using the exiftool cli fallback when the library fails
    #[arg(long)]
    disable_exiftool: bool,
    /// Log level
    #[arg(long)]
    log_level: Option<Severity>,
    // Subcommands
    #[command(subcommand)]
    command: Command,
}

/// Doc comment
#[derive(Subcommand, Debug)]
#[command()]
enum Command {
    /// Add media files to the index
    #[command()]
    Index(Index),
    /// Copy media files from a directory to another directory and index
    #[command()]
    Import(Import),
    /// Remove media from just the index
    ///
    /// This does not remove any files from the filesystem
    #[command()]
    Deindex(Deindex),
    /// Rename files in `media-paths`
    #[command()]
    StandardizeNaming(StandardizeNaming),
    /// Print stats on the media
    #[command()]
    Stats,
}

#[derive(Debug, Parser)]
struct Index {
    /// Directories with media to index
    #[arg(long)]
    paths: Vec<CanonicalizedPathBuf>,
}

#[derive(Debug, Parser)]
struct Import {
    /// Directory to import media files into
    #[arg(long)]
    to_path: CanonicalizedPathBuf,
    /// Path to save the import media db index
    #[arg(long, default_value = default_import_index_path())]
    import_index: PathBuf,
    /// Directory with media files to import
    #[arg(long)]
    from_path: CanonicalizedPathBuf,
    /// Dry run; dont actually move any files
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Parser)]
struct Deindex {
    /// Paths to remove from the index
    #[arg(long)]
    paths: Vec<CanonicalizedPathBuf>,
}

#[derive(Debug, Parser)]
struct StandardizeNaming {
    /// Directories with media to standardize paths
    #[arg(long)]
    paths: Vec<CanonicalizedPathBuf>,
    /// How to name files
    #[arg(long, value_enum)]
    naming: Standardization,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Standardization {
    /// Standardize naming of files by moving them to folders of format `YY-mm` within
    /// media path
    YearMonth,
}

fn data_directory() -> PathBuf {
    ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
        .map(|proj_dirs| proj_dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| ".".into())
}

fn default_main_index_path() -> clap::builder::OsStr {
    let mut path = data_directory();
    path.push("main.db");
    path.into_os_string().into()
}

fn default_import_index_path() -> clap::builder::OsStr {
    let mut path = data_directory();
    path.push("import.db");
    path.into_os_string().into()
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Build a terminal logger
    let logger = TerminalLoggerBuilder::new()
        .level(args.log_level.unwrap_or_default())
        .source_location(SourceLocation::None)
        .build()?;

    std::fs::create_dir_all(data_directory())?;
    let mut index = GlanceIndex::new(args.index)?.with_logger(logger.clone());

    let config = AddDirectoryConfig {
        hash: !args.disable_hash,
        filter_by_media: args.filter_by_media_type,
        metadata_fallback_for_created: args.metadata_fallback_for_created,
        calculate_nearest_city: args.calculate_nearest_city,
        use_exiftool: !args.disable_exiftool,
    };

    match args.command {
        Command::Index(sub_args) => {
            index.index_many(sub_args.paths.iter(), &config)?;
            index.deindex_missing()?;
        }
        Command::Import(sub_args) => {
            if !args.disable_hash {
                // TODO: we could recompute the hashes
                return Err(anyhow!("Cannot import media without calculating the hash"));
            }

            let from_index_path = &sub_args.import_index;

            // Build up the import index
            let mut import_index = GlanceIndex::new(from_index_path)?.with_logger(logger);
            import_index.index(&sub_args.from_path, &config)?;
            import_index.deindex_missing()?;

            // Build up the main index
            index.index(&sub_args.to_path, &config)?;
            index.deindex_missing()?;

            index.import(from_index_path, sub_args.to_path.as_ref(), sub_args.dry_run)?;
        }
        Command::Deindex(sub_args) => {
            index.deindex(sub_args.paths)?;
        }
        Command::StandardizeNaming(sub_args) => match sub_args.naming {
            Standardization::YearMonth => {
                index.standardize_year_month_naming_many(sub_args.paths)?
            }
        },
        Command::Stats => {
            let stats = index.stats()?;
            println!("{}", serde_json::to_string_pretty(&stats)?);
        }
    }

    Ok(())
}
