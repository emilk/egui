use clap::{Parser, Subcommand};
use kitdiff::DiffSource;

#[derive(Parser)]
#[command(name = "kitdiff")]
#[command(about = "A viewer for egui kittest snapshot test files")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Compare snapshot test files (.png with .old/.new/.diff variants) (default)
    Files,
    /// Compare images between current branch and default branch
    Git,
    /// Compare images between PR branches from GitHub PR URL (needs to be run from within the repo)
    Pr { url: String },
    /// Load and compare snapshot files from a zip archive (URL or local file)
    Zip { source: String },
}

impl Commands {
    pub fn to_source(&self) -> DiffSource {
        match self {
            Commands::Files => DiffSource::Files,
            Commands::Git => DiffSource::Git,
            Commands::Pr { url } => DiffSource::Pr(url.clone()),
            Commands::Zip { source } => {
                if source.starts_with("http://") || source.starts_with("https://") {
                    // DiffSource::Zip(kitdiff::PathOrBlob::Url(source.clone()))
                    todo!()
                } else {
                    DiffSource::Zip(kitdiff::PathOrBlob::Path(source.clone().into()))
                }
            }
        }
    }
}
