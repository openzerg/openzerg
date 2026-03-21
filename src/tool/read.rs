use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use crate::error::{Error, Result};
use super::traits::{Tool, ToolContext, ToolResult, Attachment};
use super::schema::generate_schema;
use super::truncation::{truncate_output, MAX_BYTES, MAX_LINE_LENGTH};

const DEFAULT_READ_LIMIT: usize = 2000;
const DESCRIPTION: &str = include_str!("prompts/read.md");

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadParams {
    #[schemars(description = "The absolute path to the file or directory to read")]
    pub filePath: String,
    #[schemars(description = "The line number to start reading from (1-indexed)")]
    pub offset: Option<usize>,
    #[schemars(description = "The maximum number of lines to read (defaults to 2000)")]
    pub limit: Option<usize>,
}

pub struct ReadTool;

impl ReadTool {
    pub fn new() -> Self { Self }
    
    fn is_binary_extension(ext: &str) -> bool {
        matches!(ext.to_lowercase().as_str(),
            "zip" | "tar" | "gz" | "exe" | "dll" | "so" | "wasm" | "pyc" | "pyo" |
            "jar" | "war" | "7z" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" |
            "odt" | "ods" | "odp" | "bin" | "dat" | "obj" | "o" | "a" | "lib" | "class"
        )
    }
    
    async fn is_binary(&self, path: &PathBuf) -> Result<bool> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        
        if Self::is_binary_extension(ext) {
            return Ok(true);
        }
        
        let metadata = tokio::fs::metadata(path).await
            .map_err(|e| Error::File(format!("Failed to read file metadata: {}", e)))?;
        
        if metadata.len() == 0 {
            return Ok(false);
        }
        
        let mut file = tokio::fs::File::open(path).await
            .map_err(|e| Error::File(format!("Failed to open file: {}", e)))?;
        
        let mut sample = [0u8; 4096];
        let n = tokio::io::AsyncReadExt::read(&mut file, &mut sample).await
            .map_err(|e| Error::File(format!("Failed to read file sample: {}", e)))?;
        
        if n == 0 {
            return Ok(false);
        }
        
        if sample[..n].iter().any(|&b| b == 0) {
            return Ok(true);
        }
        
        let non_printable = sample[..n].iter()
            .filter(|&&b| b < 9 || (b > 13 && b < 32))
            .count();
        
        Ok(non_printable as f64 / n as f64 > 0.3)
    }
    
    async fn read_directory(&self, path: &PathBuf, params: &ReadParams) -> Result<ToolResult> {
        let mut entries = Vec::new();
        
        let mut dir = tokio::fs::read_dir(path).await
            .map_err(|e| Error::File(format!("Failed to read directory: {}", e)))?;
        
        while let Some(entry) = dir.next_entry().await.map_err(|e| Error::File(format!("Failed to read entry: {}", e)))? {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().await
                .map(|t| t.is_dir())
                .unwrap_or(false);
            
            entries.push(if is_dir { format!("{}/", name) } else { name });
        }
        
        entries.sort();
        
        let total = entries.len();
        let offset = params.offset.unwrap_or(1).saturating_sub(1);
        let limit = params.limit.unwrap_or(DEFAULT_READ_LIMIT);
        
        let sliced: Vec<String> = entries.into_iter().skip(offset).take(limit).collect();
        let truncated = offset + sliced.len() < total;
        
        let mut output = format!("<path>{}</path>\n<type>directory</type>\n<entries>\n", path.display());
        output.push_str(&sliced.join("\n"));
        
        if truncated {
            output.push_str(&format!("\n\n(Showing {} of {} entries. Use offset to continue.)", sliced.len(), total));
        } else {
            output.push_str(&format!("\n\n({} entries)", total));
        }
        output.push_str("\n</entries>");
        
        Ok(ToolResult {
            title: path.display().to_string(),
            output,
            metadata: Value::Object([
                ("preview".to_string(), Value::String(sliced.iter().take(20).cloned().collect::<Vec<_>>().join("\n"))),
                ("truncated".to_string(), Value::Bool(truncated)),
            ].into_iter().collect()),
            attachments: vec![],
            truncated: false,
        })
    }
    
    async fn read_image(&self, path: &PathBuf, mime: &str) -> Result<ToolResult> {
        let data = tokio::fs::read(path).await
            .map_err(|e| Error::File(format!("Failed to read image: {}", e)))?;
        
        let title = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("image")
            .to_string();
        
        Ok(ToolResult {
            title: title.clone(),
            output: format!("Image read successfully: {} ({} bytes)", title, data.len()),
            metadata: serde_json::json!({
                "mime": mime,
                "size": data.len(),
            }),
            attachments: vec![Attachment::image(mime, &data)],
            truncated: false,
        })
    }
    
    async fn read_text_file(&self, path: &PathBuf, params: &ReadParams) -> Result<ToolResult> {
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| Error::File(format!("Failed to read file: {}", e)))?;
        
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();
        
        let offset = params.offset.unwrap_or(1).saturating_sub(1);
        let limit = params.limit.unwrap_or(DEFAULT_READ_LIMIT);
        
        if offset > total_lines {
            return Err(Error::File(format!("Offset {} is out of range (file has {} lines)", offset + 1, total_lines)));
        }
        
        let mut result_lines = Vec::new();
        let mut bytes = 0;
        let mut truncated_by_bytes = false;
        
        for (idx, line) in lines.iter().skip(offset).enumerate() {
            if idx >= limit {
                break;
            }
            
            let line_display = if line.len() > MAX_LINE_LENGTH {
                format!("{}... (line truncated to {} chars)", &line[..MAX_LINE_LENGTH], MAX_LINE_LENGTH)
            } else {
                line.to_string()
            };
            
            let line_size = line_display.len() + 1;
            if bytes + line_size > MAX_BYTES {
                truncated_by_bytes = true;
                break;
            }
            
            result_lines.push(format!("{}: {}", offset + idx + 1, line_display));
            bytes += line_size;
        }
        
        let last_read_line = offset + result_lines.len();
        let has_more = last_read_line < total_lines;
        let truncated = has_more || truncated_by_bytes;
        
        let mut output = format!("<path>{}</path>\n<type>file</type>\n<content>\n", path.display());
        output.push_str(&result_lines.join("\n"));
        
        if truncated_by_bytes {
            output.push_str(&format!("\n\n(Output capped at {}KB. Showing lines {}-{}. Use offset={} to continue.)", 
                MAX_BYTES / 1024, offset + 1, last_read_line, last_read_line + 1));
        } else if has_more {
            output.push_str(&format!("\n\n(Showing lines {}-{} of {}. Use offset={} to continue.)",
                offset + 1, last_read_line, total_lines, last_read_line + 1));
        } else {
            output.push_str(&format!("\n\n(End of file - total {} lines)", total_lines));
        }
        output.push_str("\n</content>");
        
        Ok(ToolResult {
            title: path.display().to_string(),
            output,
            metadata: serde_json::json!({
                "preview": lines.iter().skip(offset).take(20).cloned().collect::<Vec<_>>().join("\n"),
                "truncated": truncated,
            }),
            attachments: vec![],
            truncated,
        })
    }
}

#[async_trait]
impl Tool for ReadTool {
    fn id(&self) -> &str { "read" }
    
    fn description(&self) -> &str {
        DESCRIPTION
    }
    
    fn parameters_schema(&self) -> Value {
        generate_schema::<ReadParams>()
    }
    
    async fn execute(&self, args: Value, ctx: ToolContext) -> Result<ToolResult> {
        let params: ReadParams = serde_json::from_value(args)
            .map_err(|e| Error::Tool(format!("Invalid parameters: {}", e)))?;
        
        if params.offset.map(|o| o < 1).unwrap_or(false) {
            return Err(Error::Tool("offset must be greater than or equal to 1".into()));
        }
        
        let path = if std::path::Path::new(&params.filePath).is_absolute() {
            PathBuf::from(&params.filePath)
        } else {
            ctx.workspace.join(&params.filePath)
        };
        
        if !path.exists() {
            let dir = path.parent().unwrap_or(&path);
            let base = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            
            let mut suggestions = Vec::new();
            if let Ok(mut entries) = std::fs::read_dir(dir) {
                while let Some(Ok(entry)) = entries.next() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.to_lowercase().contains(&base.to_lowercase()) || 
                       base.to_lowercase().contains(&name.to_lowercase()) {
                        suggestions.push(dir.join(name).display().to_string());
                        if suggestions.len() >= 3 { break; }
                    }
                }
            }
            
            if !suggestions.is_empty() {
                return Err(Error::File(format!(
                    "File not found: {}\n\nDid you mean one of these?\n{}",
                    path.display(),
                    suggestions.join("\n")
                )));
            }
            
            return Err(Error::File(format!("File not found: {}", path.display())));
        }
        
        if path.is_dir() {
            return self.read_directory(&path, &params).await;
        }
        
        let mime = mime_guess::from_path(&path)
            .first()
            .map(|m| m.to_string())
            .unwrap_or_else(|| "text/plain".to_string());
        
        if mime.starts_with("image/") && mime != "image/svg+xml" {
            return self.read_image(&path, &mime).await;
        }
        
        if mime == "application/pdf" {
            let data = tokio::fs::read(&path).await
                .map_err(|e| Error::File(format!("Failed to read PDF: {}", e)))?;
            
            let title = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("pdf")
                .to_string();
            
            return Ok(ToolResult {
                title: title.clone(),
                output: format!("PDF read successfully: {} ({} bytes)", title, data.len()),
                metadata: serde_json::json!({ "mime": mime, "size": data.len() }),
                attachments: vec![Attachment::image(&mime, &data)],
                truncated: false,
            });
        }
        
        if self.is_binary(&path).await? {
            return Err(Error::File(format!("Cannot read binary file: {}", path.display())));
        }
        
        self.read_text_file(&path, &params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_binary_extension_zip() {
        assert!(ReadTool::is_binary_extension("zip"));
    }

    #[test]
    fn test_is_binary_extension_exe() {
        assert!(ReadTool::is_binary_extension("exe"));
    }

    #[test]
    fn test_is_binary_extension_txt() {
        assert!(!ReadTool::is_binary_extension("txt"));
    }

    #[test]
    fn test_is_binary_extension_rs() {
        assert!(!ReadTool::is_binary_extension("rs"));
    }

    #[test]
    fn test_is_binary_extension_pyc() {
        assert!(ReadTool::is_binary_extension("pyc"));
    }

    #[test]
    fn test_is_binary_extension_case_insensitive() {
        assert!(ReadTool::is_binary_extension("ZIP"));
        assert!(ReadTool::is_binary_extension("Exe"));
    }

    #[test]
    fn test_is_binary_extension_dll() {
        assert!(ReadTool::is_binary_extension("dll"));
    }

    #[test]
    fn test_is_binary_extension_so() {
        assert!(ReadTool::is_binary_extension("so"));
    }

    #[test]
    fn test_is_binary_extension_wasm() {
        assert!(ReadTool::is_binary_extension("wasm"));
    }

    #[test]
    fn test_is_binary_extension_jar() {
        assert!(ReadTool::is_binary_extension("jar"));
    }

    #[test]
    fn test_is_binary_extension_docx() {
        assert!(ReadTool::is_binary_extension("docx"));
    }

    #[test]
    fn test_is_binary_extension_bin() {
        assert!(ReadTool::is_binary_extension("bin"));
    }

    #[test]
    fn test_read_params_deserialize() {
        let json = serde_json::json!({
            "filePath": "/tmp/test.txt"
        });
        let params: ReadParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.filePath, "/tmp/test.txt");
        assert!(params.offset.is_none());
        assert!(params.limit.is_none());
    }

    #[test]
    fn test_read_params_deserialize_with_options() {
        let json = serde_json::json!({
            "filePath": "/tmp/test.txt",
            "offset": 10,
            "limit": 100
        });
        let params: ReadParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.filePath, "/tmp/test.txt");
        assert_eq!(params.offset, Some(10));
        assert_eq!(params.limit, Some(100));
    }

    #[test]
    fn test_read_tool_id() {
        let tool = ReadTool::new();
        assert_eq!(tool.id(), "read");
    }

    #[test]
    fn test_read_tool_description() {
        let tool = ReadTool::new();
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_read_params_schema() {
        let tool = ReadTool::new();
        let schema = tool.parameters_schema();
        assert!(schema.is_object());
    }

    #[tokio::test]
    async fn test_is_binary_empty_file() {
        let tool = ReadTool::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let empty_file = temp_dir.path().join("empty.txt");
        tokio::fs::write(&empty_file, "").await.unwrap();
        
        let result = tool.is_binary(&empty_file).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_is_binary_text_file() {
        let tool = ReadTool::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let text_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&text_file, "Hello, World!").await.unwrap();
        
        let result = tool.is_binary(&text_file).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_is_binary_file_with_null() {
        let tool = ReadTool::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let bin_file = temp_dir.path().join("test.bin");
        tokio::fs::write(&bin_file, vec![0, 1, 2, 3, 0, 5]).await.unwrap();
        
        let result = tool.is_binary(&bin_file).await.unwrap();
        assert!(result);
    }
}