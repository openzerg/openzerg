use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct CachedFile {
    pub content: String,
    pub hash: String,
    pub last_read: DateTime<Utc>,
    pub dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WriteResult {
    Success,
    Conflict {
        cached_content: String,
        disk_content: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cached_file_creation() {
        let cached = CachedFile {
            content: "test content".to_string(),
            hash: "abc123".to_string(),
            last_read: Utc::now(),
            dirty: false,
        };
        assert_eq!(cached.content, "test content");
        assert_eq!(cached.hash, "abc123");
        assert!(!cached.dirty);
    }

    #[test]
    fn test_write_result_success() {
        let result = WriteResult::Success;
        match result {
            WriteResult::Success => assert!(true),
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn test_write_result_conflict() {
        let result = WriteResult::Conflict {
            cached_content: "old".to_string(),
            disk_content: "new".to_string(),
            message: "conflict".to_string(),
        };
        match result {
            WriteResult::Conflict { message, .. } => assert_eq!(message, "conflict"),
            _ => panic!("Expected Conflict"),
        }
    }

    #[test]
    fn test_write_result_serialization() {
        let result = WriteResult::Success;
        let json = serde_json::to_string(&result).unwrap();
        assert_eq!(json, "\"Success\"");
    }

    #[test]
    fn test_write_result_conflict_serialization() {
        let result = WriteResult::Conflict {
            cached_content: "a".to_string(),
            disk_content: "b".to_string(),
            message: "m".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Conflict"));
    }

    #[test]
    fn test_file_info_creation() {
        let info = FileInfo {
            path: "/tmp/test.txt".to_string(),
            name: "test.txt".to_string(),
            is_dir: false,
            size: 100,
            modified: Some(Utc::now()),
        };
        assert_eq!(info.path, "/tmp/test.txt");
        assert!(!info.is_dir);
        assert_eq!(info.size, 100);
    }

    #[test]
    fn test_file_info_serialization() {
        let info = FileInfo {
            path: "/tmp".to_string(),
            name: "tmp".to_string(),
            is_dir: true,
            size: 0,
            modified: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("/tmp"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_file_info_deserialization() {
        let json = r#"{"path":"/x","name":"x","is_dir":false,"size":50,"modified":null}"#;
        let info: FileInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.path, "/x");
        assert_eq!(info.size, 50);
    }
}
