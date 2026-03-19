use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use sha2::{Sha256, Digest};
use walkdir::WalkDir;
use crate::error::{Error, Result};
use super::types::{CachedFile, WriteResult, FileInfo};

pub struct FileManager {
    workspace: PathBuf,
    cache: Arc<RwLock<HashMap<String, CachedFile>>>,
}

impl FileManager {
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            workspace,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn read(&self, path: &str) -> Result<String> {
        let mut cache = self.cache.write().await;
        
        if let Some(cached) = cache.get(path) {
            return Ok(cached.content.clone());
        }
        
        let full_path = self.workspace.join(path);
        let content = tokio::fs::read_to_string(&full_path).await
            .map_err(|e| Error::File(format!("Failed to read {}: {}", path, e)))?;
        
        let hash = self.compute_hash(&content);
        
        cache.insert(path.to_string(), CachedFile {
            content: content.clone(),
            hash,
            last_read: chrono::Utc::now(),
            dirty: false,
        });
        
        Ok(content)
    }

    pub async fn write(&self, path: &str, new_content: &str) -> Result<WriteResult> {
        let mut cache = self.cache.write().await;
        
        let cached = cache.get(path).cloned();
        
        let full_path = self.workspace.join(path);
        
        if let Some(cached) = cached {
            let current_hash = self.compute_disk_hash(path).await?;
            
            if current_hash != cached.hash {
                let disk_content = tokio::fs::read_to_string(&full_path).await
                    .map_err(|e| Error::File(format!("Failed to read disk: {}", e)))?;
                
                return Ok(WriteResult::Conflict {
                    cached_content: cached.content,
                    disk_content,
                    message: "File has been modified by another session".into(),
                });
            }
        }
        
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| Error::File(format!("Failed to create directory: {}", e)))?;
        }
        
        tokio::fs::write(&full_path, new_content).await
            .map_err(|e| Error::File(format!("Failed to write {}: {}", path, e)))?;
        
        let new_hash = self.compute_hash(new_content);
        cache.insert(path.to_string(), CachedFile {
            content: new_content.to_string(),
            hash: new_hash,
            last_read: chrono::Utc::now(),
            dirty: false,
        });
        
        Ok(WriteResult::Success)
    }

    pub async fn append(&self, path: &str, content: &str) -> Result<()> {
        let full_path = self.workspace.join(path);
        
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&full_path).await
            .map_err(|e| Error::File(format!("Failed to open {}: {}", path, e)))?;
        
        use tokio::io::AsyncWriteExt;
        file.write_all(content.as_bytes()).await
            .map_err(|e| Error::File(format!("Failed to append: {}", e)))?;
        
        self.cache.write().await.remove(path);
        
        Ok(())
    }

    pub async fn refresh(&self, path: &str) -> Result<String> {
        self.cache.write().await.remove(path);
        self.read(path).await
    }

    pub async fn delete(&self, path: &str) -> Result<()> {
        let full_path = self.workspace.join(path);
        
        if full_path.is_dir() {
            tokio::fs::remove_dir_all(&full_path).await
                .map_err(|e| Error::File(format!("Failed to remove directory: {}", e)))?;
        } else {
            tokio::fs::remove_file(&full_path).await
                .map_err(|e| Error::File(format!("Failed to remove file: {}", e)))?;
        }
        
        self.cache.write().await.remove(path);
        
        Ok(())
    }

    pub async fn list(&self, dir: &str) -> Result<Vec<FileInfo>> {
        let full_path = self.workspace.join(dir);
        
        if !full_path.exists() {
            return Ok(Vec::new());
        }
        
        let mut entries = Vec::new();
        
        for entry in WalkDir::new(&full_path).max_depth(1).into_iter().skip(1) {
            let entry = entry.map_err(|e| Error::File(format!("Failed to read directory: {}", e)))?;
            
            let metadata = entry.metadata().ok();
            let name = entry.file_name().to_string_lossy().to_string();
            let relative = entry.path().strip_prefix(&self.workspace)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| name.clone());
            
            entries.push(FileInfo {
                path: relative,
                name,
                is_dir: entry.file_type().is_dir(),
                size: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                modified: metadata.as_ref().and_then(|m| {
                    m.modified().ok().map(|t| {
                        chrono::DateTime::from(t)
                    })
                }),
            });
        }
        
        entries.sort_by(|a, b| {
            b.is_dir.cmp(&a.is_dir)
                .then_with(|| a.name.cmp(&b.name))
        });
        
        Ok(entries)
    }

    pub async fn exists(&self, path: &str) -> bool {
        self.workspace.join(path).exists()
    }

    async fn compute_disk_hash(&self, path: &str) -> Result<String> {
        let full_path = self.workspace.join(path);
        
        if !full_path.exists() {
            return Ok(String::new());
        }
        
        let content = tokio::fs::read_to_string(&full_path).await
            .map_err(|e| Error::File(format!("Failed to read: {}", e)))?;
        
        Ok(self.compute_hash(&content))
    }

    fn compute_hash(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    pub fn workspace(&self) -> &PathBuf {
        &self.workspace
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_file_manager_new() {
        let temp = tempdir().unwrap();
        let manager = FileManager::new(temp.path().to_path_buf());
        assert_eq!(manager.workspace(), &temp.path().to_path_buf());
    }

    #[tokio::test]
    async fn test_exists_false() {
        let temp = tempdir().unwrap();
        let manager = FileManager::new(temp.path().to_path_buf());
        assert!(!manager.exists("nonexistent.txt").await);
    }

    #[tokio::test]
    async fn test_write_and_read() {
        let temp = tempdir().unwrap();
        let manager = FileManager::new(temp.path().to_path_buf());
        
        let result = manager.write("test.txt", "hello world").await.unwrap();
        assert!(matches!(result, WriteResult::Success));
        
        let content = manager.read("test.txt").await.unwrap();
        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn test_read_nonexistent() {
        let temp = tempdir().unwrap();
        let manager = FileManager::new(temp.path().to_path_buf());
        
        let result = manager.read("nonexistent.txt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_append() {
        let temp = tempdir().unwrap();
        let manager = FileManager::new(temp.path().to_path_buf());
        
        manager.write("test.txt", "hello").await.unwrap();
        manager.append("test.txt", " world").await.unwrap();
        
        // Read from disk (cache was cleared by append)
        tokio::fs::read_to_string(temp.path().join("test.txt")).await.unwrap();
    }

    #[tokio::test]
    async fn test_delete_file() {
        let temp = tempdir().unwrap();
        let manager = FileManager::new(temp.path().to_path_buf());
        
        manager.write("test.txt", "content").await.unwrap();
        manager.delete("test.txt").await.unwrap();
        
        assert!(!manager.exists("test.txt").await);
    }

    #[tokio::test]
    async fn test_list_empty() {
        let temp = tempdir().unwrap();
        let manager = FileManager::new(temp.path().to_path_buf());
        
        let entries = manager.list(".").await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_list_with_files() {
        let temp = tempdir().unwrap();
        let manager = FileManager::new(temp.path().to_path_buf());
        
        manager.write("file1.txt", "a").await.unwrap();
        manager.write("file2.txt", "b").await.unwrap();
        
        let entries = manager.list(".").await.unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn test_refresh() {
        let temp = tempdir().unwrap();
        let manager = FileManager::new(temp.path().to_path_buf());
        
        manager.write("test.txt", "original").await.unwrap();
        
        tokio::fs::write(temp.path().join("test.txt"), "modified").await.unwrap();
        
        let content = manager.refresh("test.txt").await.unwrap();
        assert_eq!(content, "modified");
    }

    #[test]
    fn test_compute_hash() {
        let temp = tempdir().unwrap();
        let manager = FileManager::new(temp.path().to_path_buf());
        
        let hash1 = manager.compute_hash("test");
        let hash2 = manager.compute_hash("test");
        let hash3 = manager.compute_hash("other");
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[tokio::test]
    async fn test_create_subdirectory() {
        let temp = tempdir().unwrap();
        let manager = FileManager::new(temp.path().to_path_buf());
        
        manager.write("subdir/test.txt", "content").await.unwrap();
        assert!(manager.exists("subdir/test.txt").await);
    }

    #[tokio::test]
    async fn test_write_creates_directory() {
        let temp = tempdir().unwrap();
        let manager = FileManager::new(temp.path().to_path_buf());
        
        manager.write("deep/nested/path/file.txt", "content").await.unwrap();
        assert!(manager.exists("deep/nested/path/file.txt").await);
    }
}