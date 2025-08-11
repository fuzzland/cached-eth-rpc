use std::hash::Hash;

use actix_web::HttpResponse;
use serde::Serialize;
use serde_json::{Number, Value};

const DEFAULT_JSON_RPC_VERSION: &str = "2.0";

#[derive(PartialEq, Hash, Debug, Clone)]
pub struct RequestId {
    id: StringOrNumber,
}

impl TryFrom<Value> for RequestId {
    type Error = eyre::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(number) => number
                .as_u64()
                .ok_or(eyre::eyre!("invalid request id"))
                .map(|number| Self {
                    id: StringOrNumber::Number(number),
                }),
            Value::String(string) => Ok(Self {
                id: StringOrNumber::String(string),
            }),
            _ => Err(eyre::eyre!("invalid request id")),
        }
    }
}

impl Serialize for RequestId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match &self.id {
            StringOrNumber::String(string) => string.serialize(serializer),
            StringOrNumber::Number(number) => number.serialize(serializer),
        }
    }
}

impl Eq for RequestId {}

#[derive(PartialEq, Hash, Debug, Clone)]
enum StringOrNumber {
    String(String),
    Number(u64),
}

#[derive(Serialize, Clone)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
    pub id: Option<RequestId>,
}

impl JsonRpcRequest {
    pub fn new(id: Option<RequestId>, method: String, params: Value) -> Self {
        Self {
            jsonrpc: DEFAULT_JSON_RPC_VERSION.to_string(),
            method,
            params,
            id,
        }
    }
}

// Assume A is some type you've defined
#[derive(Serialize, Clone)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<RequestId>,

    #[serde(flatten)]
    pub result: ResultOrError,
}

impl JsonRpcResponse {
    pub fn from_error(id: Option<RequestId>, error: DefinedError) -> Self {
        Self {
            jsonrpc: DEFAULT_JSON_RPC_VERSION.to_string(),
            id,
            result: ResultOrError::Error {
                error: DefinedOrCustomError::Defined(error),
            },
        }
    }

    pub fn from_custom_error(id: Option<RequestId>, error: Value) -> Self {
        Self {
            jsonrpc: DEFAULT_JSON_RPC_VERSION.to_string(),
            id,
            result: ResultOrError::Error {
                error: DefinedOrCustomError::Custom(error),
            },
        }
    }

    pub fn from_result(id: RequestId, result: Value) -> Self {
        Self {
            jsonrpc: DEFAULT_JSON_RPC_VERSION.to_string(),
            id: Some(id),
            result: ResultOrError::Result { result },
        }
    }
}

impl From<JsonRpcResponse> for HttpResponse {
    fn from(val: JsonRpcResponse) -> Self {
        HttpResponse::Ok().json(val)
    }
}

impl From<JsonRpcResponse> for Result<HttpResponse, actix_web::Error> {
    fn from(val: JsonRpcResponse) -> Self {
        Ok(val.into())
    }
}

#[derive(Serialize, Clone)]
#[serde(untagged)]
pub enum ResultOrError {
    Error {
        #[serde(rename = "error")]
        error: DefinedOrCustomError,
    },

    Result {
        #[serde(rename = "result")]
        result: Value,
    },
}

#[derive(Serialize, Clone)]
#[serde(untagged)]
pub enum DefinedOrCustomError {
    Defined(DefinedError),
    Custom(Value),
}

/// Source: https://www.jsonrpc.org/specification
#[derive(Clone)]
pub enum DefinedError {
    #[allow(dead_code)]
    InvalidJson,

    InvalidRequest,

    MethodNotFound,

    #[allow(dead_code)]
    InvalidParams,

    InternalError(Option<Value>),
}

impl DefinedError {
    pub fn code_and_message(&self) -> (i64, String) {
        match self {
            DefinedError::InvalidJson => (-32700, "Invalid JSON".to_string()),
            DefinedError::InvalidRequest => {
                (-32600, "JSON is not a valid request object".to_string())
            }
            DefinedError::MethodNotFound => (-32601, "Method does not exist".to_string()),
            DefinedError::InvalidParams => (-32602, "Invalid method parameters".to_string()),
            DefinedError::InternalError(_) => (-32603, "Internal JSON-RPC error".to_string()),
        }
    }

    pub fn data(&self) -> &Option<Value> {
        match self {
            DefinedError::InvalidJson => &None,
            DefinedError::InvalidRequest => &None,
            DefinedError::MethodNotFound => &None,
            DefinedError::InvalidParams => &None,
            DefinedError::InternalError(err) => err,
        }
    }
}

impl Serialize for DefinedError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (code, message) = self.code_and_message();

        let mut error = serde_json::Map::new();

        error.insert("code".to_string(), Value::Number(Number::from(code)));
        error.insert("message".to_string(), Value::String(message));

        if let Some(data) = self.data() {
            error.insert("data".to_string(), data.clone());
        }

        error.serialize(serializer)
    }
}
