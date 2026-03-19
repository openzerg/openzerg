use tonic::{Request, Response, Status};
use std::sync::Arc;

use crate::grpc::agent::*;
use crate::grpc::agent::agent_service_server::AgentService;
use crate::session::SessionManager;
use crate::process::ProcessManager;
use crate::tool::ToolRegistry;

pub struct AgentGrpcServer {
    session_manager: Arc<SessionManager>,
    process_manager: Arc<ProcessManager>,
    tool_registry: Arc<ToolRegistry>,
}

impl AgentGrpcServer {
    pub fn new(
        session_manager: Arc<SessionManager>,
        process_manager: Arc<ProcessManager>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        Self {
            session_manager,
            process_manager,
            tool_registry,
        }
    }
}

#[tonic::async_trait]
impl AgentService for AgentGrpcServer {
    async fn list_sessions(&self, _request: Request<ListSessionsRequest>) -> Result<Response<SessionListResponse>, Status> {
        Ok(Response::new(SessionListResponse { sessions: vec![], total: 0 }))
    }

    async fn get_session(&self, _request: Request<GetSessionRequest>) -> Result<Response<SessionInfo>, Status> {
        Err(Status::not_found("Session not found"))
    }

    async fn get_session_messages(&self, _request: Request<GetSessionMessagesRequest>) -> Result<Response<MessageListResponse>, Status> {
        Ok(Response::new(MessageListResponse { messages: vec![], total: 0 }))
    }

    async fn send_session_chat(&self, _request: Request<SendSessionChatRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn interrupt_session(&self, _request: Request<InterruptSessionRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn get_session_context(&self, _request: Request<GetSessionContextRequest>) -> Result<Response<SessionContextResponse>, Status> {
        Ok(Response::new(SessionContextResponse { context_json: "{}".to_string() }))
    }

    async fn list_processes(&self, _request: Request<ListProcessesRequest>) -> Result<Response<ProcessListResponse>, Status> {
        Ok(Response::new(ProcessListResponse { processes: vec![], total: 0 }))
    }

    async fn get_process(&self, _request: Request<GetProcessRequest>) -> Result<Response<ProcessInfo>, Status> {
        Err(Status::not_found("Process not found"))
    }

    async fn get_process_output(&self, _request: Request<GetProcessOutputRequest>) -> Result<Response<ProcessOutputResponse>, Status> {
        Ok(Response::new(ProcessOutputResponse { content: "".to_string(), total_size: 0 }))
    }

    async fn list_tasks(&self, _request: Request<ListTasksRequest>) -> Result<Response<TaskListResponse>, Status> {
        Ok(Response::new(TaskListResponse { tasks: vec![], total: 0 }))
    }

    async fn get_task(&self, _request: Request<GetTaskRequest>) -> Result<Response<TaskInfo>, Status> {
        Err(Status::not_found("Task not found"))
    }

    async fn list_activities(&self, _request: Request<ListActivitiesRequest>) -> Result<Response<ActivityListResponse>, Status> {
        Ok(Response::new(ActivityListResponse { activities: vec![], total: 0 }))
    }

    async fn send_message(&self, _request: Request<SendMessageRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn send_remind(&self, _request: Request<SendRemindRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }

    async fn list_builtin_tools(&self, _request: Request<Empty>) -> Result<Response<BuiltinToolListResponse>, Status> {
        let tools = self.tool_registry.tool_definitions().await;
        
        let response = BuiltinToolListResponse {
            tools: tools.iter().map(|t| BuiltinToolInfo {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters_json: serde_json::to_string(&t.parameters).unwrap_or_default(),
            }).collect(),
        };
        
        Ok(Response::new(response))
    }

    async fn execute_builtin_tool(&self, _request: Request<ExecuteBuiltinToolRequest>) -> Result<Response<ExecuteBuiltinToolResponse>, Status> {
        Ok(Response::new(ExecuteBuiltinToolResponse {
            title: "".to_string(),
            output: "".to_string(),
            metadata_json: "{}".to_string(),
            attachments_json: "[]".to_string(),
            truncated: false,
        }))
    }

    async fn check_health(&self, _request: Request<Empty>) -> Result<Response<HealthResponse>, Status> {
        Ok(Response::new(HealthResponse {
            healthy: true,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }
}