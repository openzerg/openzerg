mod handler;

pub use handler::AgentGrpcServer;

pub mod agent {
    tonic::include_proto!("agent");
}