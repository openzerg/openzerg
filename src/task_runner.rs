use std::process::Command;
use crate::protocol::HostExecuteTask;

pub struct TaskResult {
    pub success: bool,
    pub output: String,
}

pub async fn execute_task(task: &HostExecuteTask, workspace: &str) -> TaskResult {
    let cwd = task.cwd.as_ref()
        .map(|s| s.as_str())
        .unwrap_or(workspace);

    let output = Command::new("sh")
        .arg("-c")
        .arg(&task.command)
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            let mut combined = stdout.to_string();
            if !stderr.is_empty() {
                if !combined.is_empty() {
                    combined.push('\n');
                }
                combined.push_str(&stderr);
            }

            TaskResult {
                success: output.status.success(),
                output: combined,
            }
        }
        Err(e) => {
            TaskResult {
                success: false,
                output: format!("Failed to execute: {}", e),
            }
        }
    }
}