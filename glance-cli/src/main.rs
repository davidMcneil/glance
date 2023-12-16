use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use glance_lib::index::Index;

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
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut index = Index::new(args.db_path)?;
    index.add_directory(args.media_path, args.enable_hash)?;

    let duplicates = index.duplicates();
    println!("{:?}", duplicates);

    Ok(())
}
