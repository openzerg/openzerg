use crate::protocol::{FileEntry, FileTreeData};
use std::path::Path;
use walkdir::WalkDir;

pub fn scan_directory(path: &str) -> FileTreeData {
    let base_path = Path::new(path);
    let mut entries = Vec::new();

    if !base_path.exists() {
        return FileTreeData {
            path: path.to_string(),
            entries,
        };
    }

    for entry in WalkDir::new(base_path)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path_buf = entry.path();
        let relative = path_buf.strip_prefix(base_path).unwrap_or(path_buf);

        if relative.to_str() == Some("") {
            continue;
        }

        let metadata = entry.metadata().ok();
        let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

        entries.push(FileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: relative.to_string_lossy().to_string(),
            is_dir,
            size,
            modified: metadata
                .and_then(|m| m.modified().ok())
                .map(|t| chrono::DateTime::from(t)),
        });
    }

    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    FileTreeData {
        path: path.to_string(),
        entries,
    }
}
