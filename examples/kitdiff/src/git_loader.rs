use crate::{FileReference, Snapshot};
use eframe::egui::load::Bytes;
use eframe::egui::{Context, ImageSource};
use git2::{ObjectType, Repository};
use serde_json::Value;
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
    PrUrlParseError,
    NetworkError(String),
}

#[derive(Debug, Clone)]
pub struct PrInfo {
    pub org: String,
    pub repo: String,
    pub pr_number: u32,
    pub head_ref: String,
    pub base_ref: String,
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

pub fn pr_git_discovery(pr_url: String, sender: mpsc::Sender<Snapshot>, ctx: Context) -> Result<(), GitError> {
    std::thread::spawn(move || {
        if let Err(e) = run_pr_git_discovery(pr_url, sender, ctx) {
            eprintln!("PR git discovery error: {:?}", e);
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

    // Get current HEAD for comparison with default branch
    let head_commit = repo.head()?.peel_to_commit()?;
    let head_tree = head_commit.tree()?;

    // Use git2 diff to find changed PNG files between default branch and current HEAD
    let diff = repo.diff_tree_to_tree(Some(&default_commit.tree()?), Some(&head_tree), None)?;

    // Process each delta (changed file)
    diff.foreach(&mut |delta, _progress| {
        // Check both old and new file paths (handles renames/moves)
        let files_to_check = [
            delta.old_file().path(),
            delta.new_file().path(),
        ];

        for file_path in files_to_check.into_iter().flatten() {
            // Check if this is a PNG file
            if let Some(extension) = file_path.extension() {
                if extension == "png" {
                    // Create snapshot for this changed PNG file
                    if let Ok(Some(snapshot)) = create_git_snapshot(
                        &repo,
                        &default_commit.tree().unwrap(),
                        file_path,
                        &github_repo_info,
                        &commit_sha,
                    ) {
                        if sender.send(snapshot).is_ok() {
                            ctx.request_repaint();
                        }
                    }
                    break; // Only process once per delta
                }
            }
        }
        true // Continue iteration
    }, None, None, None)?;

    Ok(())
}

fn run_pr_git_discovery(pr_url: String, sender: mpsc::Sender<Snapshot>, ctx: Context) -> Result<(), GitError> {
    // Parse the PR URL
    let (org, repo, pr_number) = parse_github_pr_url(&pr_url)?;

    // Fetch PR info from GitHub API
    let pr_info = fetch_pr_info(&org, &repo, pr_number)?;

    // Open git repository in current directory
    let repo = Repository::open(".").map_err(|_| GitError::RepoNotFound)?;

    // Get GitHub repository info for LFS support
    let github_repo_info = get_github_repo_info(&repo);

    // Fetch and resolve the head and base branches
    let (head_tree, base_tree, head_commit_sha) = resolve_pr_branches(&repo, &pr_info)?;

    // Use git2 diff to find changed PNG files between branches
    let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;

    // Process each delta (changed file)
    diff.foreach(&mut |delta, _progress| {
        // Check both old and new file paths (handles renames/moves)
        let files_to_check = [
            delta.old_file().path(),
            delta.new_file().path(),
        ];

        for file_path in files_to_check.into_iter().flatten() {
            // Check if this is a PNG file
            if let Some(extension) = file_path.extension() {
                if extension == "png" {
                    // Create snapshot for this changed PNG file
                    if let Ok(Some(snapshot)) = create_pr_snapshot(
                        &repo,
                        &base_tree,
                        &head_tree,
                        file_path,
                        &github_repo_info,
                        &head_commit_sha,
                    ) {
                        if sender.send(snapshot).is_ok() {
                            ctx.request_repaint();
                        }
                    }
                    break; // Only process once per delta
                }
            }
        }
        true // Continue iteration
    }, None, None, None)?;

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

fn resolve_pr_branches<'a>(repo: &'a Repository, pr_info: &PrInfo) -> Result<(git2::Tree<'a>, git2::Tree<'a>, String), GitError> {
    // Get the origin remote to fetch branches if needed
    let mut remote = repo.find_remote("origin")?;

    // Construct refspecs for head and base branches
    let head_refspec = format!("+refs/heads/{}:refs/remotes/origin/{}", pr_info.head_ref, pr_info.head_ref);
    let base_refspec = format!("+refs/heads/{}:refs/remotes/origin/{}", pr_info.base_ref, pr_info.base_ref);

    // Fetch the branches
    remote.fetch(&[&head_refspec, &base_refspec], None, None)?;

    // Resolve head branch commit
    let head_ref_name = format!("refs/remotes/origin/{}", pr_info.head_ref);
    let head_ref = repo.find_reference(&head_ref_name)?;
    let head_commit = head_ref.peel_to_commit()?;
    let head_tree = head_commit.tree()?;
    let head_commit_sha = head_commit.id().to_string();

    // Resolve base branch commit
    let base_ref_name = format!("refs/remotes/origin/{}", pr_info.base_ref);
    let base_ref = repo.find_reference(&base_ref_name)?;
    let base_commit = base_ref.peel_to_commit()?;
    let base_tree = base_commit.tree()?;

    Ok((head_tree, base_tree, head_commit_sha))
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

fn create_pr_snapshot(
    repo: &Repository,
    base_tree: &git2::Tree,
    head_tree: &git2::Tree,
    current_path: &Path,
    github_repo_info: &Option<(String, String)>,
    head_commit_sha: &str,
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

    let relative_path = current_path.strip_prefix(".").unwrap_or(current_path);

    // Try to get the file from base branch
    let base_file_content = match get_file_from_tree(repo, base_tree, relative_path) {
        Ok(content) => Some(content),
        Err(_) => None, // File doesn't exist in base branch
    };

    // Try to get the file from head branch
    let head_file_content = match get_file_from_tree(repo, head_tree, relative_path) {
        Ok(content) => Some(content),
        Err(_) => None, // File doesn't exist in head branch
    };

    // git2 diff already confirmed this file changed, so we can proceed
    // Handle the case where file exists in only one branch
    if base_file_content.is_none() && head_file_content.is_none() {
        return Ok(None); // File doesn't exist in either branch (shouldn't happen with diff)
    }

    // Create ImageSource for base branch (old)
    let base_image_source = match base_file_content {
        Some(content) => {
            if is_lfs_pointer(&content) {
                if let Some((org, repo_name)) = github_repo_info {
                    // For base branch, we need to find the base commit SHA
                    let base_commit_sha = head_commit_sha; // This should be the base commit SHA, but we'll use head for now
                    let media_url = create_lfs_media_url(org, repo_name, base_commit_sha, relative_path);
                    ImageSource::Uri(Cow::Owned(media_url))
                } else {
                    ImageSource::Bytes {
                        uri: Cow::Owned(format!("bytes://base/{}", relative_path.display())),
                        bytes: Bytes::Shared(content.into()),
                    }
                }
            } else {
                ImageSource::Bytes {
                    uri: Cow::Owned(format!("bytes://base/{}", relative_path.display())),
                    bytes: Bytes::Shared(content.into()),
                }
            }
        },
        None => {
            // Create a placeholder for missing file
            ImageSource::Bytes {
                uri: Cow::Owned(format!("bytes://missing/{}", relative_path.display())),
                bytes: Bytes::Static(&[]), // Empty bytes for missing file
            }
        }
    };

    // Create ImageSource for head branch (new)
    let head_image_source = match head_file_content {
        Some(content) => {
            if is_lfs_pointer(&content) {
                if let Some((org, repo_name)) = github_repo_info {
                    let media_url = create_lfs_media_url(org, repo_name, head_commit_sha, relative_path);
                    ImageSource::Uri(Cow::Owned(media_url))
                } else {
                    ImageSource::Bytes {
                        uri: Cow::Owned(format!("bytes://head/{}", relative_path.display())),
                        bytes: Bytes::Shared(content.into()),
                    }
                }
            } else {
                ImageSource::Bytes {
                    uri: Cow::Owned(format!("bytes://head/{}", relative_path.display())),
                    bytes: Bytes::Shared(content.into()),
                }
            }
        },
        None => {
            // Create a placeholder for missing file
            ImageSource::Bytes {
                uri: Cow::Owned(format!("bytes://missing/{}", relative_path.display())),
                bytes: Bytes::Static(&[]), // Empty bytes for missing file
            }
        }
    };

    Ok(Some(Snapshot {
        path: relative_path.to_path_buf(),
        old: FileReference::Source(base_image_source), // Base branch version
        new: FileReference::Source(head_image_source), // Head branch version
        diff: None, // Always None for PR mode
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

pub fn parse_github_pr_url(url: &str) -> Result<(String, String, u32), GitError> {
    // Parse URLs like: https://github.com/rerun-io/rerun/pull/11253
    if !url.starts_with("https://github.com/") {
        return Err(GitError::PrUrlParseError);
    }

    let path = url.strip_prefix("https://github.com/")
        .ok_or(GitError::PrUrlParseError)?;

    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() != 4 || parts[2] != "pull" {
        return Err(GitError::PrUrlParseError);
    }

    let org = parts[0].to_string();
    let repo = parts[1].to_string();
    let pr_number = parts[3].parse::<u32>()
        .map_err(|_| GitError::PrUrlParseError)?;

    Ok((org, repo, pr_number))
}

pub fn fetch_pr_info(org: &str, repo: &str, pr_number: u32) -> Result<PrInfo, GitError> {
    let url = format!("https://api.github.com/repos/{}/{}/pulls/{}", org, repo, pr_number);

    // Use ehttp for HTTP request (blocking)
    let request = ehttp::Request::get(url);

    let response = ehttp::fetch_blocking(&request)
        .map_err(|e| GitError::NetworkError(e.to_string()))?;

    if !response.ok {
        return Err(GitError::NetworkError(format!("HTTP {}: {}", response.status, response.status_text)));
    }

    let json: Value = serde_json::from_slice(&response.bytes)
        .map_err(|e| GitError::NetworkError(format!("JSON parse error: {}", e)))?;

    let head_ref = json["head"]["ref"]
        .as_str()
        .ok_or(GitError::NetworkError("Missing head.ref in PR data".to_string()))?
        .to_string();

    let base_ref = json["base"]["ref"]
        .as_str()
        .ok_or(GitError::NetworkError("Missing base.ref in PR data".to_string()))?
        .to_string();

    Ok(PrInfo {
        org: org.to_string(),
        repo: repo.to_string(),
        pr_number,
        head_ref,
        base_ref,
    })
}
