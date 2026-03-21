mod handler;

pub use handler::AgentGrpcServer;

pub mod agent {
    tonic::include_proto!("agent");
}

pub use agent::agent_service_server::AgentServiceServer;