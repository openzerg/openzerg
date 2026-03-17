use crate::protocol::GitRepo;
use std::path::Path;
use walkdir::WalkDir;

pub fn scan_repos(workspace: &str) -> Vec<GitRepo> {
    let mut repos = Vec::new();
    let workspace_path = Path::new(workspace);

    if !workspace_path.exists() {
        return repos;
    }

    for entry in WalkDir::new(workspace_path)
        .max_depth(5)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if path.join(".git").is_dir() {
            let repo = scan_repo(path);
            repos.push(repo);
        }
    }

    repos
}

fn scan_repo(path: &Path) -> GitRepo {
    let relative_path = path.to_string_lossy().to_string();

    let (remote_url, branch, status, ahead, behind) = git2::Repository::open(path)
        .map(|repo| {
            let remote_url = repo
                .find_remote("origin")
                .ok()
                .and_then(|r| r.url().map(|s| s.to_string()));

            let branch = repo
                .head()
                .ok()
                .and_then(|h| h.shorthand().map(|s| s.to_string()));

            let (ahead, behind) = (0u32, 0u32);

            let status = "synced".to_string();

            (remote_url, branch, status, ahead, behind)
        })
        .unwrap_or((None, None, "unknown".to_string(), 0, 0));

    GitRepo {
        path: relative_path,
        remote_url,
        branch,
        status,
        ahead,
        behind,
    }
}
