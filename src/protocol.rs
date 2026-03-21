use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    VmConnect(VmConnect),
    VmHeartbeat(VmHeartbeat),
    VmStatusReport(VmStatusReport),
    VmEventAck(VmEventAck),

    HostEvent(HostEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConnect {
    pub agent_name: String,
    pub internal_token: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmHeartbeat {
    pub agent_name: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmStatusReport {
    pub agent_name: String,
    pub timestamp: DateTime<Utc>,
    pub data: AgentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub online: bool,
    pub cpu_percent: f32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub disk_used_gb: f64,
    pub disk_total_gb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmEventAck {
    pub event_id: String,
    pub accepted: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostEvent {
    pub event_id: String,
    pub event: AgentEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    Interrupt {
        message: String,
        target_session: Option<String>,
    },

    ProcessNotification {
        process_id: String,
        event: ProcessEvent,
        output_preview: Option<String>,
    },

    Message {
        content: String,
        from: String,
    },

    AssignTask {
        task_id: String,
        title: String,
        description: String,
        priority: Priority,
        deadline: Option<DateTime<Utc>>,
        context: Option<serde_json::Value>,
    },

    Remind {
        id: String,
        message: String,
    },

    Query {
        query_id: String,
        question: String,
    },

    ConfigUpdate {
        llm_base_url: Option<String>,
        llm_api_key: Option<String>,
        llm_model: Option<String>,
    },

    ResourceWarning {
        resource: ResourceType,
        message: String,
    },

    SessionCreated {
        session_id: String,
        purpose: String,
    },

    Thinking {
        session_id: String,
        content: String,
    },

    Response {
        session_id: String,
        content: String,
    },

    Done {
        session_id: String,
    },

    Error {
        session_id: String,
        message: String,
    },

    SubSessionResult {
        parent_session_id: String,
        child_session_id: String,
        child_session_type: String,
        status: String,
        summary: String,
        details: String,
    },

    SessionTask {
        session_id: String,
        task: String,
        context: Option<serde_json::Value>,
    },

    UserMessage {
        content: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessEvent {
    Started,
    Completed { exit_code: i32 },
    Failed { error: String },
    OutputLimitReached,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Medium,
    High,
    Urgent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Cpu,
    Memory,
    Disk,
    Processes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterruptLevel {
    Low,
    Medium,
    High,
}

impl Message {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_variants() {
        assert_eq!(Priority::Low, Priority::Low);
        assert_ne!(Priority::Low, Priority::Medium);
        assert_ne!(Priority::High, Priority::Urgent);
    }

    #[test]
    fn test_priority_serialization() {
        let priority = Priority::High;
        let json = serde_json::to_string(&priority).unwrap();
        assert_eq!(json, "\"high\"");
    }

    #[test]
    fn test_priority_deserialization() {
        let json = "\"urgent\"";
        let priority: Priority = serde_json::from_str(json).unwrap();
        assert_eq!(priority, Priority::Urgent);
    }

    #[test]
    fn test_resource_type_serialization() {
        let resource = ResourceType::Cpu;
        let json = serde_json::to_string(&resource).unwrap();
        assert_eq!(json, "\"cpu\"");
    }

    #[test]
    fn test_process_event_started() {
        let event = ProcessEvent::Started;
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, "\"started\"");
    }

    #[test]
    fn test_process_event_completed() {
        let event = ProcessEvent::Completed { exit_code: 0 };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("completed"));
        assert!(json.contains("0"));
    }

    #[test]
    fn test_process_event_failed() {
        let event = ProcessEvent::Failed {
            error: "timeout".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("failed"));
        assert!(json.contains("timeout"));
    }

    #[test]
    fn test_agent_event_interrupt() {
        let event = AgentEvent::Interrupt {
            message: "stop".to_string(),
            target_session: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("interrupt"));
        assert!(json.contains("stop"));
    }

    #[test]
    fn test_agent_event_message() {
        let event = AgentEvent::Message {
            content: "hello".to_string(),
            from: "user".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("hello"));
        assert!(json.contains("user"));
    }

    #[test]
    fn test_agent_event_assign_task() {
        let event = AgentEvent::AssignTask {
            task_id: "t1".to_string(),
            title: "Task".to_string(),
            description: "Desc".to_string(),
            priority: Priority::High,
            deadline: None,
            context: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("t1"));
        assert!(json.contains("high"));
    }

    #[test]
    fn test_message_to_json() {
        let msg = Message::VmHeartbeat(VmHeartbeat {
            agent_name: "agent".to_string(),
            timestamp: Utc::now(),
        });
        let json = msg.to_json().unwrap();
        assert!(json.contains("vm_heartbeat"));
    }

    #[test]
    fn test_message_from_json() {
        let json =
            r#"{"type":"vm_heartbeat","agent_name":"test","timestamp":"2024-01-01T00:00:00Z"}"#;
        let msg = Message::from_json(json).unwrap();
        match msg {
            Message::VmHeartbeat(hb) => assert_eq!(hb.agent_name, "test"),
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_vm_connect_serialization() {
        let connect = VmConnect {
            agent_name: "agent1".to_string(),
            internal_token: "token123".to_string(),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&connect).unwrap();
        assert!(json.contains("agent1"));
        assert!(json.contains("token123"));
    }

    #[test]
    fn test_agent_status_serialization() {
        let status = AgentStatus {
            online: true,
            cpu_percent: 50.0,
            memory_used_mb: 1000,
            memory_total_mb: 8000,
            disk_used_gb: 100.0,
            disk_total_gb: 500.0,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("online"));
        assert!(json.contains("50.0"));
    }

    #[test]
    fn test_host_event_serialization() {
        let event = HostEvent {
            event_id: "e1".to_string(),
            event: AgentEvent::Message {
                content: "test".to_string(),
                from: "system".to_string(),
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("e1"));
    }
}
