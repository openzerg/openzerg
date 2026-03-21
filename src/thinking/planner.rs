use std::sync::Arc;
use crate::protocol::{AgentEvent, Priority};
use crate::llm::LLMClient;
use crate::session::SessionManager;
use crate::task::{Task, TaskManager};
use crate::error::Result;
use serde::{Deserialize, Serialize};

const PLANNER_PROMPT: &str = include_str!("prompts/planner.md");

pub struct ThinkingLayer {
    llm_client: Arc<LLMClient>,
    session_manager: Arc<SessionManager>,
    task_manager: Arc<TaskManager>,
}

impl ThinkingLayer {
    pub fn new(
        llm_client: Arc<LLMClient>,
        session_manager: Arc<SessionManager>,
        task_manager: Arc<TaskManager>,
    ) -> Self {
        Self {
            llm_client,
            session_manager,
            task_manager,
        }
    }

    pub async fn process_event(&self, event: AgentEvent) -> Result<TaskPlan> {
        let context = self.build_context().await;
        
        let prompt = self.build_planning_prompt(&event, &context);
        
        let response = self.llm_client.complete(prompt).await?;
        
        let plan = self.parse_response(&response).await;
        
        Ok(plan)
    }

    async fn build_context(&self) -> ThinkingContext {
        let sessions = self.session_manager.get_summaries().await;
        let tasks = self.task_manager.list_summaries(None).await;
        
        ThinkingContext {
            sessions,
            tasks,
        }
    }

    fn build_planning_prompt(&self, event: &AgentEvent, context: &ThinkingContext) -> Vec<crate::llm::Message> {
        let context_str = format!(
            "## Current State\n\n### Sessions:\n{}\n\n### Tasks:\n{}",
            serde_json::to_string_pretty(&context.sessions).unwrap_or_default(),
            serde_json::to_string_pretty(&context.tasks).unwrap_or_default(),
        );

        let event_str = serde_json::to_string_pretty(event).unwrap_or_default();

        vec![
            crate::llm::Message::system(PLANNER_PROMPT),
            crate::llm::Message::user(&format!("{}\n\n## Received Event\n\n{}", context_str, event_str)),
        ]
    }

    async fn parse_response(&self, response: &str) -> TaskPlan {
        let clean_response = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        match serde_json::from_str::<ParsedPlan>(clean_response) {
            Ok(parsed) => {
                let mut tasks = Vec::new();
                let mut assignments = std::collections::HashMap::new();

                for t in parsed.tasks.into_iter() {
                    let task_id = uuid::Uuid::new_v4().to_string();
                    let priority = match t.priority.as_str() {
                        "high" => Priority::High,
                        "urgent" => Priority::Urgent,
                        "low" => Priority::Low,
                        _ => Priority::Medium,
                    };

                    let task = Task::new(
                        task_id.clone(),
                        t.title,
                        t.description,
                        priority,
                    );

                    if let Some(session_id) = &t.assign_to {
                        assignments.insert(task_id.clone(), Assignment::ToSession(session_id.clone()));
                    } else {
                        assignments.insert(task_id.clone(), Assignment::NewSession);
                    }

                    tasks.push(task);
                }

                TaskPlan {
                    analysis: parsed.analysis,
                    tasks,
                    assignments,
                }
            }
            Err(e) => {
                tracing::warn!("Failed to parse LLM response: {}", e);
                TaskPlan {
                    analysis: response.to_string(),
                    tasks: Vec::new(),
                    assignments: std::collections::HashMap::new(),
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ThinkingContext {
    pub sessions: Vec<crate::session::SessionSummary>,
    pub tasks: Vec<crate::task::TaskSummary>,
}

#[derive(Debug, Clone)]
pub struct TaskPlan {
    pub analysis: String,
    pub tasks: Vec<Task>,
    pub assignments: std::collections::HashMap<String, Assignment>,
}

#[derive(Debug, Clone)]
pub enum Assignment {
    ToSession(String),
    NewSession,
    Defer,
}

#[derive(Debug, Clone, Deserialize)]
struct ParsedPlan {
    analysis: String,
    tasks: Vec<ParsedTask>,
}

#[derive(Debug, Clone, Deserialize)]
struct ParsedTask {
    title: String,
    description: String,
    priority: String,
    assign_to: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assignment_variants() {
        let to_session = Assignment::ToSession("s1".to_string());
        let new_session = Assignment::NewSession;
        let defer = Assignment::Defer;
        
        match to_session {
            Assignment::ToSession(id) => assert_eq!(id, "s1"),
            _ => panic!("Wrong variant"),
        }
        
        match new_session {
            Assignment::NewSession => {}
            _ => panic!("Wrong variant"),
        }
        
        match defer {
            Assignment::Defer => {}
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_task_plan_creation() {
        let plan = TaskPlan {
            analysis: "test analysis".to_string(),
            tasks: vec![],
            assignments: std::collections::HashMap::new(),
        };
        assert_eq!(plan.analysis, "test analysis");
        assert!(plan.tasks.is_empty());
    }

    #[test]
    fn test_thinking_context_creation() {
        let ctx = ThinkingContext {
            sessions: vec![],
            tasks: vec![],
        };
        assert!(ctx.sessions.is_empty());
        assert!(ctx.tasks.is_empty());
    }

    #[test]
    fn test_parsed_plan_deserialization() {
        let json = r#"{"analysis":"test","tasks":[]}"#;
        let plan: ParsedPlan = serde_json::from_str(json).unwrap();
        assert_eq!(plan.analysis, "test");
        assert!(plan.tasks.is_empty());
    }

    #[test]
    fn test_parsed_task_deserialization() {
        let json = r#"{"title":"Test","description":"Desc","priority":"high","assign_to":"s1"}"#;
        let task: ParsedTask = serde_json::from_str(json).unwrap();
        assert_eq!(task.title, "Test");
        assert_eq!(task.description, "Desc");
        assert_eq!(task.priority, "high");
        assert_eq!(task.assign_to, Some("s1".to_string()));
    }

    #[test]
    fn test_parsed_task_without_assign() {
        let json = r#"{"title":"Test","description":"Desc","priority":"medium"}"#;
        let task: ParsedTask = serde_json::from_str(json).unwrap();
        assert!(task.assign_to.is_none());
    }
}