use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use glance_lib::index::Index;

/// The quick media viewer
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to directory with media contents
    #[arg(short, long)]
    media_path: PathBuf,
    /// Path to save the media db index
    #[arg(short, long)]
    db_path: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut index = Index::new(args.db_path)?;
    index.add_directory(args.media_path, false)?;
    Ok(())
}
