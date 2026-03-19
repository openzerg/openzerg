use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl RpcError {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;

    pub const SESSION_NOT_FOUND: i32 = -32001;
    pub const PROCESS_NOT_FOUND: i32 = -32002;
    pub const TASK_NOT_FOUND: i32 = -32003;

    pub fn parse_error() -> Self {
        Self {
            code: Self::PARSE_ERROR,
            message: "Parse error".into(),
            data: None,
        }
    }

    pub fn invalid_request() -> Self {
        Self {
            code: Self::INVALID_REQUEST,
            message: "Invalid Request".into(),
            data: None,
        }
    }

    pub fn method_not_found() -> Self {
        Self {
            code: Self::METHOD_NOT_FOUND,
            message: "Method not found".into(),
            data: None,
        }
    }

    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self {
            code: Self::INVALID_PARAMS,
            message: msg.into(),
            data: None,
        }
    }

    pub fn internal_error(msg: impl Into<String>) -> Self {
        Self {
            code: Self::INTERNAL_ERROR,
            message: msg.into(),
            data: None,
        }
    }

    pub fn session_not_found(id: &str) -> Self {
        Self {
            code: Self::SESSION_NOT_FOUND,
            message: format!("Session '{}' not found", id),
            data: None,
        }
    }

    pub fn process_not_found(id: &str) -> Self {
        Self {
            code: Self::PROCESS_NOT_FOUND,
            message: format!("Process '{}' not found", id),
            data: None,
        }
    }

    pub fn task_not_found(id: &str) -> Self {
        Self {
            code: Self::TASK_NOT_FOUND,
            message: format!("Task '{}' not found", id),
            data: None,
        }
    }
}

impl RpcResponse {
    pub fn success(id: Option<i64>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<i64>, error: RpcError) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

impl RpcRequest {
    pub fn parse(json: &str) -> Result<Self, RpcError> {
        serde_json::from_str(json).map_err(|_| RpcError::parse_error())
    }

    pub fn to_json(&self) -> Result<String, RpcError> {
        serde_json::to_string(self).map_err(|_| RpcError::internal_error("Serialization failed"))
    }
}

impl RpcResponse {
    pub fn to_json(&self) -> Result<String, RpcError> {
        serde_json::to_string(self).map_err(|_| RpcError::internal_error("Serialization failed"))
    }
}
