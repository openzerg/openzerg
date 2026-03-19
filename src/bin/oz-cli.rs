use clap::{Parser, Subcommand};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

#[derive(Parser)]
#[command(name = "oz-cli")]
#[command(about = "OpenZerg Agent CLI", long_about = None)]
struct Cli {
    #[arg(short, long, env = "AGENT_URL", default_value = "http://localhost:8081")]
    url: String,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Sessions {
        #[arg(short, long, default_value = "0")]
        offset: usize,
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    
    Session {
        id: String,
    },
    
    Messages {
        id: String,
        #[arg(short, long, default_value = "0")]
        offset: usize,
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
    
    Chat {
        #[arg(short, long)]
        session: Option<String>,
        #[arg(short, long)]
        message: Option<String>,
    },
    
    Interrupt {
        session: String,
        #[arg(short, long)]
        message: String,
        #[arg(short = 'L', long, default_value = "medium")]
        level: String,
    },
    
    Processes {
        #[arg(short, long, default_value = "0")]
        offset: usize,
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    
    Process {
        id: String,
    },
    
    Output {
        id: String,
        #[arg(short, long, default_value = "stdout")]
        stream: String,
        #[arg(short, long)]
        follow: bool,
    },
    
    Tasks {
        #[arg(short, long, default_value = "0")]
        offset: usize,
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    
    Task {
        id: String,
    },
    
    Activities {
        #[arg(short, long, default_value = "0")]
        offset: usize,
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
    
    Message {
        #[arg(short, long)]
        message: String,
    },
    
    Remind {
        #[arg(short, long)]
        message: String,
    },
    
    Status,
}

#[derive(Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct Session {
    id: String,
    purpose: String,
    state: String,
    created_at: String,
    message_count: usize,
}

#[derive(Deserialize)]
struct Message {
    id: String,
    session_id: String,
    role: String,
    content: String,
    timestamp: String,
}

#[derive(Deserialize)]
struct Process {
    id: String,
    command: String,
    status: String,
    exit_code: Option<i32>,
    started_at: String,
}

#[derive(Deserialize)]
struct Task {
    id: String,
    content: String,
    status: String,
    priority: String,
}

#[derive(Deserialize)]
struct Activity {
    id: String,
    activity_type: String,
    description: String,
    timestamp: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let client = Client::new();
    
    match cli.command {
        Commands::Sessions { offset, limit } => {
            let url = format!("{}/api/sessions?offset={}&limit={}", cli.url, offset, limit);
            let resp = client.get(&url).send().await?;
            let api: ApiResponse<serde_json::Value> = resp.json().await?;
            
            if api.success {
                if let Some(data) = api.data {
                    if let Some(sessions) = data.get("sessions").and_then(|s| s.as_array()) {
                        println!("{:<36} {:<10} {:<12} {:<6} {}", "ID", "PURPOSE", "STATE", "MSGS", "CREATED");
                        println!("{}", "-".repeat(90));
                        for s in sessions {
                            let id = s.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            let purpose = s.get("purpose").and_then(|v| v.as_str()).unwrap_or("");
                            let state = s.get("state").and_then(|v| v.as_str()).unwrap_or("");
                            let count = s.get("message_count").and_then(|v| v.as_u64()).unwrap_or(0);
                            let created = s.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                            println!("{:<36} {:<10} {:<12} {:<6} {}", 
                                &id[..8.min(id.len())], purpose, state, count, 
                                &created[..19.min(created.len())]);
                        }
                    }
                }
            } else {
                eprintln!("Error: {:?}", api.error);
            }
        }
        
        Commands::Session { id } => {
            let url = format!("{}/api/sessions/{}", cli.url, id);
            let resp = client.get(&url).send().await?;
            let api: ApiResponse<Session> = resp.json().await?;
            
            if api.success {
                if let Some(s) = api.data {
                    println!("ID: {}", s.id);
                    println!("Purpose: {}", s.purpose);
                    println!("State: {}", s.state);
                    println!("Created: {}", s.created_at);
                    println!("Messages: {}", s.message_count);
                }
            } else {
                eprintln!("Error: {:?}", api.error);
            }
        }
        
        Commands::Messages { id, offset, limit } => {
            let url = format!("{}/api/sessions/{}/messages?offset={}&limit={}", cli.url, id, offset, limit);
            let resp = client.get(&url).send().await?;
            let api: ApiResponse<serde_json::Value> = resp.json().await?;
            
            if api.success {
                if let Some(data) = api.data {
                    if let Some(messages) = data.get("messages").and_then(|m| m.as_array()) {
                        for m in messages {
                            let role = m.get("role").and_then(|v| v.as_str()).unwrap_or("");
                            let content = m.get("content").and_then(|v| v.as_str()).unwrap_or("");
                            let time = m.get("timestamp").and_then(|v| v.as_str()).unwrap_or("");
                            println!("[{}] {:<8}: {}", &time[..19.min(time.len())], role, content);
                            println!();
                        }
                    }
                }
            } else {
                eprintln!("Error: {:?}", api.error);
            }
        }
        
        Commands::Chat { session, message } => {
            let msg = match message {
                Some(m) => m,
                None => {
                    print!("Enter message: ");
                    io::stdout().flush()?;
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    input.trim().to_string()
                }
            };
            
            if let Some(sid) = session {
                let url = format!("{}/api/sessions/{}/chat", cli.url, sid);
                let resp = client.post(&url).json(&serde_json::json!({ "content": msg })).send().await?;
                let api: ApiResponse<serde_json::Value> = resp.json().await?;
                if api.success {
                    println!("Message sent to session {}", sid);
                } else {
                    eprintln!("Error: {:?}", api.error);
                }
            } else {
                let url = format!("{}/api/message", cli.url);
                let resp = client.post(&url).json(&serde_json::json!({ "content": msg })).send().await?;
                let api: ApiResponse<serde_json::Value> = resp.json().await?;
                if api.success {
                    println!("Message sent to agent");
                } else {
                    eprintln!("Error: {:?}", api.error);
                }
            }
        }
        
        Commands::Interrupt { session, message, level } => {
            let url = format!("{}/api/sessions/{}/interrupt", cli.url, session);
            let resp = client.post(&url).json(&serde_json::json!({ 
                "message": message,
                "level": level 
            })).send().await?;
            let api: ApiResponse<serde_json::Value> = resp.json().await?;
            if api.success {
                println!("Interrupt sent to session {}", session);
            } else {
                eprintln!("Error: {:?}", api.error);
            }
        }
        
        Commands::Processes { offset, limit } => {
            let url = format!("{}/api/processes?offset={}&limit={}", cli.url, offset, limit);
            let resp = client.get(&url).send().await?;
            let api: ApiResponse<serde_json::Value> = resp.json().await?;
            
            if api.success {
                if let Some(data) = api.data {
                    if let Some(processes) = data.get("processes").and_then(|p| p.as_array()) {
                        println!("{:<36} {:<20} {:<12} {:<6} {}", "ID", "COMMAND", "STATUS", "EXIT", "STARTED");
                        println!("{}", "-".repeat(100));
                        for p in processes {
                            let id = p.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            let cmd = p.get("command").and_then(|v| v.as_str()).unwrap_or("");
                            let status = p.get("status").and_then(|v| v.as_str()).unwrap_or("");
                            let exit = p.get("exit_code").and_then(|v| v.as_i64()).map(|e| e.to_string()).unwrap_or("-".to_string());
                            let started = p.get("started_at").and_then(|v| v.as_str()).unwrap_or("");
                            println!("{:<36} {:<20} {:<12} {:<6} {}", 
                                &id[..8.min(id.len())], 
                                &cmd[..20.min(cmd.len())], 
                                status, exit,
                                &started[..19.min(started.len())]);
                        }
                    }
                }
            } else {
                eprintln!("Error: {:?}", api.error);
            }
        }
        
        Commands::Process { id } => {
            let url = format!("{}/api/processes/{}", cli.url, id);
            let resp = client.get(&url).send().await?;
            let api: ApiResponse<Process> = resp.json().await?;
            
            if api.success {
                if let Some(p) = api.data {
                    println!("ID: {}", p.id);
                    println!("Command: {}", p.command);
                    println!("Status: {}", p.status);
                    if let Some(code) = p.exit_code {
                        println!("Exit Code: {}", code);
                    }
                    println!("Started: {}", p.started_at);
                }
            } else {
                eprintln!("Error: {:?}", api.error);
            }
        }
        
        Commands::Output { id, stream, follow: _ } => {
            let url = format!("{}/api/processes/{}/output?stream={}", cli.url, id, stream);
            let resp = client.get(&url).send().await?;
            let api: ApiResponse<serde_json::Value> = resp.json().await?;
            
            if api.success {
                if let Some(data) = api.data {
                    if let Some(content) = data.get("content").and_then(|c| c.as_str()) {
                        println!("{}", content);
                    }
                }
            } else {
                eprintln!("Error: {:?}", api.error);
            }
        }
        
        Commands::Tasks { offset, limit } => {
            let url = format!("{}/api/tasks?offset={}&limit={}", cli.url, offset, limit);
            let resp = client.get(&url).send().await?;
            let api: ApiResponse<serde_json::Value> = resp.json().await?;
            
            if api.success {
                if let Some(data) = api.data {
                    if let Some(tasks) = data.get("tasks").and_then(|t| t.as_array()) {
                        println!("{:<36} {:<12} {:<8} {}", "ID", "STATUS", "PRIORITY", "CONTENT");
                        println!("{}", "-".repeat(80));
                        for t in tasks {
                            let id = t.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            let status = t.get("status").and_then(|v| v.as_str()).unwrap_or("");
                            let priority = t.get("priority").and_then(|v| v.as_str()).unwrap_or("");
                            let content = t.get("content").and_then(|v| v.as_str()).unwrap_or("");
                            println!("{:<36} {:<12} {:<8} {}", 
                                &id[..8.min(id.len())], status, priority, content);
                        }
                    }
                }
            } else {
                eprintln!("Error: {:?}", api.error);
            }
        }
        
        Commands::Task { id } => {
            let url = format!("{}/api/tasks/{}", cli.url, id);
            let resp = client.get(&url).send().await?;
            let api: ApiResponse<Task> = resp.json().await?;
            
            if api.success {
                if let Some(t) = api.data {
                    println!("ID: {}", t.id);
                    println!("Content: {}", t.content);
                    println!("Status: {}", t.status);
                    println!("Priority: {}", t.priority);
                }
            } else {
                eprintln!("Error: {:?}", api.error);
            }
        }
        
        Commands::Activities { offset, limit } => {
            let url = format!("{}/api/activities?offset={}&limit={}", cli.url, offset, limit);
            let resp = client.get(&url).send().await?;
            let api: ApiResponse<serde_json::Value> = resp.json().await?;
            
            if api.success {
                if let Some(data) = api.data {
                    if let Some(activities) = data.get("activities").and_then(|a| a.as_array()) {
                        println!("{:<36} {:<15} {}", "ID", "TYPE", "DESCRIPTION");
                        println!("{}", "-".repeat(100));
                        for a in activities {
                            let id = a.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            let typ = a.get("activity_type").and_then(|v| v.as_str()).unwrap_or("");
                            let desc = a.get("description").and_then(|v| v.as_str()).unwrap_or("");
                            println!("{:<36} {:<15} {}", &id[..8.min(id.len())], typ, desc);
                        }
                    }
                }
            } else {
                eprintln!("Error: {:?}", api.error);
            }
        }
        
        Commands::Message { message } => {
            let url = format!("{}/api/message", cli.url);
            let resp = client.post(&url).json(&serde_json::json!({ "content": message })).send().await?;
            let api: ApiResponse<serde_json::Value> = resp.json().await?;
            if api.success {
                println!("Message sent to agent");
            } else {
                eprintln!("Error: {:?}", api.error);
            }
        }
        
        Commands::Remind { message } => {
            let url = format!("{}/api/remind", cli.url);
            let resp = client.post(&url).json(&serde_json::json!({ "message": message })).send().await?;
            let api: ApiResponse<serde_json::Value> = resp.json().await?;
            if api.success {
                println!("Remind sent to agent");
            } else {
                eprintln!("Error: {:?}", api.error);
            }
        }
        
        Commands::Status => {
            let url = format!("{}/health", cli.url);
            let resp = client.get(&url).send().await?;
            let api: serde_json::Value = resp.json().await?;
            println!("Status: {}", serde_json::to_string_pretty(&api)?);
        }
    }
    
    Ok(())
}