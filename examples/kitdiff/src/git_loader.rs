use crate::{FileReference, Snapshot};
use eframe::egui::load::Bytes;
use eframe::egui::{Context, ImageSource};
use git2::{ObjectType, Repository};
use ignore::{WalkBuilder, types::TypesBuilder};
use std::borrow::Cow;
use std::path::Path;
use std::sync::mpsc;
use std::str;

#[derive(Debug)]
pub enum GitError {
    RepoNotFound,
    BranchNotFound,
    FileNotFound,
    GitError(git2::Error),
    IoError(std::io::Error),
}

impl From<git2::Error> for GitError {
    fn from(err: git2::Error) -> Self {
        GitError::GitError(err)
    }
}

impl From<std::io::Error> for GitError {
    fn from(err: std::io::Error) -> Self {
        GitError::IoError(err)
    }
}

pub fn git_discovery(sender: mpsc::Sender<Snapshot>, ctx: Context) -> Result<(), GitError> {
    std::thread::spawn(move || {
        if let Err(e) = run_git_discovery(sender, ctx) {
            eprintln!("Git discovery error: {:?}", e);
        }
    });
    Ok(())
}

fn run_git_discovery(sender: mpsc::Sender<Snapshot>, ctx: Context) -> Result<(), GitError> {
    // Open git repository in current directory
    let repo = Repository::open(".").map_err(|_| GitError::RepoNotFound)?;

    // Get current branch
    let head = repo.head()?;
    let current_branch = head.shorthand().unwrap_or("HEAD").to_string();

    // Find default branch (try main, then master, then first branch)
    let default_branch = find_default_branch(&repo)?;

    // Don't compare branch with itself
    if current_branch == default_branch {
        eprintln!(
            "Current branch is the same as default branch ({})",
            current_branch
        );
        return Ok(());
    }

    // Get the commit from default branch
    let default_commit = repo
        .resolve_reference_from_short_name(&default_branch)?
        .peel_to_commit()?;

    // Get GitHub repository info for LFS support
    let github_repo_info = get_github_repo_info(&repo);
    let commit_sha = default_commit.id().to_string();

    // Create type matcher for .png files
    let mut types_builder = TypesBuilder::new();
    types_builder.add("png", "*.png").unwrap();
    types_builder.select("png");
    let types = types_builder.build().unwrap();

    // Walk current working tree for .png files
    for result in WalkBuilder::new(".").types(types).build() {
        if let Ok(entry) = result {
            if entry.file_type().map_or(false, |ft| ft.is_file()) {
                if let Some(snapshot) = create_git_snapshot(
                    &repo,
                    &default_commit.tree()?,
                    entry.path(),
                    &github_repo_info,
                    &commit_sha,
                )? {
                    if sender.send(snapshot).is_ok() {
                        ctx.request_repaint();
                    }
                }
            }
        }
    }

    Ok(())
}

fn find_default_branch(repo: &Repository) -> Result<String, GitError> {
    // Try common default branch names
    for branch_name in ["main", "master"] {
        if repo.resolve_reference_from_short_name(branch_name).is_ok() {
            return Ok(branch_name.to_string());
        }
    }

    // Fall back to first branch found
    let branches = repo.branches(Some(git2::BranchType::Local))?;
    for branch in branches {
        let (branch, _) = branch?;
        if let Some(name) = branch.name()? {
            return Ok(name.to_string());
        }
    }

    Err(GitError::BranchNotFound)
}

fn create_git_snapshot(
    repo: &Repository,
    default_tree: &git2::Tree,
    current_path: &Path,
    github_repo_info: &Option<(String, String)>,
    commit_sha: &str,
) -> Result<Option<Snapshot>, GitError> {
    // Skip files that are variants
    let file_name = current_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or(GitError::FileNotFound)?;

    if file_name.ends_with(".old.png")
        || file_name.ends_with(".new.png")
        || file_name.ends_with(".diff.png")
    {
        return Ok(None);
    }

    // Try to get the file from default branch
    let relative_path = current_path.strip_prefix(".").unwrap_or(current_path);

    let default_file_content = match get_file_from_tree(repo, default_tree, relative_path) {
        Ok(content) => content,
        Err(_) => {
            // File doesn't exist in default branch, skip
            return Ok(None);
        }
    };

    // Get the current file from the current branch's tree to compare git objects properly
    let head_commit = repo.head()?.peel_to_commit()?;
    let head_tree = head_commit.tree()?;

    // Compare git object content (both should be LFS pointers if using LFS)
    if let Ok(current_content) = get_file_from_tree(repo, &head_tree, relative_path) {
        if default_file_content == current_content {
            return Ok(None);
        }
    }

    // Check if this is an LFS pointer file
    let default_image_source = if is_lfs_pointer(&default_file_content) {
        // If we have GitHub repo info, create media URL
        if let Some((org, repo_name)) = github_repo_info {
            let media_url = create_lfs_media_url(org, repo_name, commit_sha, relative_path);
            ImageSource::Uri(Cow::Owned(media_url))
        } else {
            // Fallback to bytes (will likely fail to load but better than nothing)
            ImageSource::Bytes {
                uri: Cow::Owned(format!("bytes://{}", relative_path.display())),
                bytes: Bytes::Shared(default_file_content.into()),
            }
        }
    } else {
        // Regular file content
        ImageSource::Bytes {
            uri: Cow::Owned(format!("bytes://{}", relative_path.display())),
            bytes: Bytes::Shared(default_file_content.into()),
        }
    };

    Ok(Some(Snapshot {
        path: relative_path.to_path_buf(),
        old: FileReference::Source(default_image_source), // Default branch version as ImageSource
        new: FileReference::Path(current_path.to_path_buf()), // Current working tree version
        diff: None,                                       // Always None for git mode
    }))
}

fn get_file_from_tree(
    repo: &Repository,
    tree: &git2::Tree,
    path: &Path,
) -> Result<Vec<u8>, GitError> {
    let entry = tree.get_path(path)?;
    let object = entry.to_object(repo)?;

    match object.kind() {
        Some(ObjectType::Blob) => {
            let blob = object.as_blob().unwrap();
            Ok(blob.content().to_vec())
        }
        _ => Err(GitError::FileNotFound),
    }
}

fn is_lfs_pointer(content: &[u8]) -> bool {
    // LFS pointer files must be < 1024 bytes and UTF-8
    if content.len() >= 1024 {
        return false;
    }

    // Try to parse as UTF-8
    let text = match str::from_utf8(content) {
        Ok(text) => text,
        Err(_) => return false,
    };

    // Check for LFS pointer format
    // Must start with "version https://git-lfs.github.com/spec/v1"
    let lines: Vec<&str> = text.trim().split('\n').collect();
    if lines.is_empty() {
        return false;
    }

    // First line must be version
    if !lines[0].starts_with("version https://git-lfs.github.com/spec/v1") {
        return false;
    }

    // Look for required oid and size lines
    let mut has_oid = false;
    let mut has_size = false;

    for line in &lines[1..] {
        if line.starts_with("oid sha256:") {
            has_oid = true;
        } else if line.starts_with("size ") {
            has_size = true;
        }
    }

    has_oid && has_size
}

fn get_github_repo_info(repo: &Repository) -> Option<(String, String)> {
    // Try to get the origin remote
    let remote = repo.find_remote("origin").ok()?;
    let url = remote.url()?;

    // Parse GitHub URLs (both HTTPS and SSH)
    if let Some(caps) = parse_github_https_url(url) {
        return Some(caps);
    }

    if let Some(caps) = parse_github_ssh_url(url) {
        return Some(caps);
    }

    None
}

fn parse_github_https_url(url: &str) -> Option<(String, String)> {
    // Match: https://github.com/org/repo.git or https://github.com/org/repo
    if url.starts_with("https://github.com/") {
        let path = url.strip_prefix("https://github.com/")?;
        let path = path.strip_suffix(".git").unwrap_or(path);

        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }
    None
}

fn parse_github_ssh_url(url: &str) -> Option<(String, String)> {
    // Match: git@github.com:org/repo.git
    if url.starts_with("git@github.com:") {
        let path = url.strip_prefix("git@github.com:")?;
        let path = path.strip_suffix(".git").unwrap_or(path);

        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }
    None
}

fn create_lfs_media_url(org: &str, repo: &str, commit_sha: &str, file_path: &Path) -> String {
    format!(
        "https://media.githubusercontent.com/media/{}/{}/{}/{}",
        org,
        repo,
        commit_sha,
        file_path.display()
    )
}
