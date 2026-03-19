use openzerg::{
    event::EventDispatcher,
    protocol::{AgentEvent, HostEvent, ProcessEvent, Priority, ResourceType, Message},
    sse::SseManager,
};
use std::sync::Arc;

#[tokio::test]
async fn test_event_dispatcher_basic() {
    let dispatcher = EventDispatcher::new("test-agent".to_string());
    
    let mut event_rx = dispatcher.subscribe_events();
    let mut interrupt_rx = dispatcher.subscribe_interrupts();
    
    // Send interrupt
    let host_event = HostEvent {
        event_id: "e1".to_string(),
        event: AgentEvent::Interrupt {
            message: "stop now".to_string(),
            target_session: None,
        },
    };
    
    let response = dispatcher.dispatch(host_event).await.unwrap();
    match response {
        Message::VmEventAck(ack) => {
            assert!(ack.accepted);
            assert_eq!(ack.event_id, "e1");
        }
        _ => panic!("Wrong message type"),
    }
    
    // Receive interrupt signal
    let signal = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        interrupt_rx.recv()
    ).await.unwrap().unwrap();
    assert_eq!(signal.message, "stop now");
}

#[tokio::test]
async fn test_event_dispatcher_message() {
    let dispatcher = EventDispatcher::new("test-agent".to_string());
    let mut event_rx = dispatcher.subscribe_events();
    
    let host_event = HostEvent {
        event_id: "e2".to_string(),
        event: AgentEvent::Message {
            content: "Hello".to_string(),
            from: "user".to_string(),
        },
    };
    
    dispatcher.dispatch(host_event).await.unwrap();
    
    let event = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        event_rx.recv()
    ).await.unwrap().unwrap();
    
    match event {
        AgentEvent::Message { content, from } => {
            assert_eq!(content, "Hello");
            assert_eq!(from, "user");
        }
        _ => panic!("Wrong event type"),
    }
}

#[tokio::test]
async fn test_event_dispatcher_query() {
    let dispatcher = EventDispatcher::new("test-agent".to_string());
    let mut event_rx = dispatcher.subscribe_events();
    
    let host_event = HostEvent {
        event_id: "e3".to_string(),
        event: AgentEvent::Query {
            query_id: "q1".to_string(),
            question: "What is 2+2?".to_string(),
        },
    };
    
    dispatcher.dispatch(host_event).await.unwrap();
    
    let event = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        event_rx.recv()
    ).await.unwrap().unwrap();
    
    match event {
        AgentEvent::Query { query_id, question } => {
            assert_eq!(query_id, "q1");
            assert_eq!(question, "What is 2+2?");
        }
        _ => panic!("Wrong event type"),
    }
}

#[tokio::test]
async fn test_event_dispatcher_assign_task() {
    let dispatcher = EventDispatcher::new("test-agent".to_string());
    let mut event_rx = dispatcher.subscribe_events();
    
    let host_event = HostEvent {
        event_id: "e4".to_string(),
        event: AgentEvent::AssignTask {
            task_id: "t1".to_string(),
            title: "Build feature".to_string(),
            description: "Implement X".to_string(),
            priority: Priority::High,
            deadline: None,
            context: Some(serde_json::json!({"repo": "test"})),
        },
    };
    
    dispatcher.dispatch(host_event).await.unwrap();
    
    let event = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        event_rx.recv()
    ).await.unwrap().unwrap();
    
    match event {
        AgentEvent::AssignTask { task_id, priority, .. } => {
            assert_eq!(task_id, "t1");
            assert_eq!(priority, Priority::High);
        }
        _ => panic!("Wrong event type"),
    }
}

#[tokio::test]
async fn test_event_dispatcher_remind() {
    let dispatcher = EventDispatcher::new("test-agent".to_string());
    let mut event_rx = dispatcher.subscribe_events();
    
    let host_event = HostEvent {
        event_id: "e5".to_string(),
        event: AgentEvent::Remind {
            id: "r1".to_string(),
            message: "Don't forget".to_string(),
        },
    };
    
    dispatcher.dispatch(host_event).await.unwrap();
    
    let event = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        event_rx.recv()
    ).await.unwrap().unwrap();
    
    match event {
        AgentEvent::Remind { id, message } => {
            assert_eq!(id, "r1");
            assert_eq!(message, "Don't forget");
        }
        _ => panic!("Wrong event type"),
    }
}

#[tokio::test]
async fn test_event_dispatcher_config_update() {
    let dispatcher = EventDispatcher::new("test-agent".to_string());
    let mut event_rx = dispatcher.subscribe_events();
    
    let host_event = HostEvent {
        event_id: "e6".to_string(),
        event: AgentEvent::ConfigUpdate {
            llm_base_url: Some("http://new-api".to_string()),
            llm_api_key: None,
            llm_model: Some("gpt-4o".to_string()),
        },
    };
    
    dispatcher.dispatch(host_event).await.unwrap();
    
    let event = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        event_rx.recv()
    ).await.unwrap().unwrap();
    
    match event {
        AgentEvent::ConfigUpdate { llm_base_url, llm_model, .. } => {
            assert_eq!(llm_base_url, Some("http://new-api".to_string()));
            assert_eq!(llm_model, Some("gpt-4o".to_string()));
        }
        _ => panic!("Wrong event type"),
    }
}

#[tokio::test]
async fn test_event_dispatcher_process_notification() {
    let dispatcher = EventDispatcher::new("test-agent".to_string());
    let mut event_rx = dispatcher.subscribe_events();
    
    let host_event = HostEvent {
        event_id: "e7".to_string(),
        event: AgentEvent::ProcessNotification {
            process_id: "p1".to_string(),
            event: ProcessEvent::Completed { exit_code: 0 },
            output_preview: Some("Success".to_string()),
        },
    };
    
    dispatcher.dispatch(host_event).await.unwrap();
    
    let event = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        event_rx.recv()
    ).await.unwrap().unwrap();
    
    match event {
        AgentEvent::ProcessNotification { process_id, event, output_preview } => {
            assert_eq!(process_id, "p1");
            match event {
                ProcessEvent::Completed { exit_code } => assert_eq!(exit_code, 0),
                _ => panic!("Wrong process event"),
            }
            assert_eq!(output_preview, Some("Success".to_string()));
        }
        _ => panic!("Wrong event type"),
    }
}

#[tokio::test]
async fn test_event_dispatcher_resource_warning() {
    let dispatcher = EventDispatcher::new("test-agent".to_string());
    let mut event_rx = dispatcher.subscribe_events();
    
    let host_event = HostEvent {
        event_id: "e8".to_string(),
        event: AgentEvent::ResourceWarning {
            resource: ResourceType::Memory,
            message: "Memory usage high".to_string(),
        },
    };
    
    dispatcher.dispatch(host_event).await.unwrap();
    
    let event = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        event_rx.recv()
    ).await.unwrap().unwrap();
    
    match event {
        AgentEvent::ResourceWarning { resource, message } => {
            assert_eq!(resource, ResourceType::Memory);
            assert_eq!(message, "Memory usage high");
        }
        _ => panic!("Wrong event type"),
    }
}

#[tokio::test]
async fn test_event_dispatcher_multiple_events() {
    let dispatcher = EventDispatcher::new("test-agent".to_string());
    let mut event_rx = dispatcher.subscribe_events();
    
    // Send multiple events
    for i in 0..3 {
        let host_event = HostEvent {
            event_id: format!("e{}", i),
            event: AgentEvent::Message {
                content: format!("Message {}", i),
                from: "user".to_string(),
            },
        };
        dispatcher.dispatch(host_event).await.unwrap();
    }
    
    // Receive all events
    for i in 0..3 {
        let event = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            event_rx.recv()
        ).await.unwrap().unwrap();
        
        match event {
            AgentEvent::Message { content, .. } => {
                assert_eq!(content, format!("Message {}", i));
            }
            _ => panic!("Wrong event type"),
        }
    }
}