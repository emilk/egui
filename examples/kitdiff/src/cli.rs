use clap::{Parser, Subcommand};
use kitdiff::DiffSource;
use kitdiff::github_auth::parse_github_artifact_url;

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
    /// Load and compare snapshot files from a GitHub artifact
    GhArtifact { url: String },
}

impl Commands {
    pub fn to_source(&self) -> DiffSource {
        match self {
            Commands::Files => DiffSource::Files,
            Commands::Git => DiffSource::Git,
            Commands::Pr { url } => {
                // Check if the PR URL is actually a GitHub artifact URL
                if let Some((owner, repo, artifact_id)) = parse_github_artifact_url(url) {
                    DiffSource::GHArtifact { owner, repo, artifact_id }
                } else {
                    DiffSource::Pr(url.clone())
                }
            },
            Commands::Zip { source } => {
                // Check if it's a GitHub artifact URL first
                if let Some((owner, repo, artifact_id)) = parse_github_artifact_url(source) {
                    DiffSource::GHArtifact { owner, repo, artifact_id }
                } else if source.starts_with("http://") || source.starts_with("https://") {
                    #[cfg(target_arch = "wasm32")]
                    {
                        if source.ends_with(".tar.gz") || source.ends_with(".tgz") {
                            DiffSource::TarGz(kitdiff::PathOrBlob::Url(source.clone(), None))
                        } else {
                            DiffSource::Zip(kitdiff::PathOrBlob::Url(source.clone(), None))
                        }
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        panic!("URL sources not supported on native platforms. Use 'gh-artifact' command for GitHub artifacts or download and provide a local file path.");
                    }
                } else {
                    if source.ends_with(".tar.gz") || source.ends_with(".tgz") {
                        DiffSource::TarGz(kitdiff::PathOrBlob::Path(source.clone().into()))
                    } else {
                        DiffSource::Zip(kitdiff::PathOrBlob::Path(source.clone().into()))
                    }
                }
            },
            Commands::GhArtifact { url } => {
                if let Some((owner, repo, artifact_id)) = parse_github_artifact_url(url) {
                    DiffSource::GHArtifact { owner, repo, artifact_id }
                } else {
                    panic!("Invalid GitHub artifact URL: {}", url);
                }
            }
        }
    }
}
