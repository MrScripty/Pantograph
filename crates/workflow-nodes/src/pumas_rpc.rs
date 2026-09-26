//! Closed, library-only JSON-RPC transport for an explicitly configured Pumas
//! service.
//!
//! This module deliberately exposes operations as an enum instead of accepting
//! arbitrary method names. It is a transport seam, not an inference client.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures_util::StreamExt;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

const MAX_RESPONSE_BYTES: usize = 32 * 1024 * 1024;
const MAX_REQUEST_BYTES: usize = 4 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub struct PumasRpcClient {
    endpoint: Url,
    http: reqwest::Client,
    next_request_id: std::sync::Arc<AtomicU64>,
}

impl PumasRpcClient {
    pub fn new(endpoint: &str) -> Result<Self, PumasRpcError> {
        let endpoint = Url::parse(endpoint).map_err(|_| PumasRpcError::InvalidEndpoint {
            reason: "Pumas RPC endpoint must be a valid URL".to_string(),
        })?;
        validate_endpoint(&endpoint)?;

        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|source| PumasRpcError::Transport {
                operation: "build HTTP client".to_string(),
                message: source.to_string(),
            })?;

        Ok(Self {
            endpoint,
            http,
            next_request_id: std::sync::Arc::new(AtomicU64::new(1)),
        })
    }

    pub fn endpoint(&self) -> &Url {
        &self.endpoint
    }

    pub async fn call(&self, operation: PumasRpcOperation) -> Result<Value, PumasRpcError> {
        operation.validate_input_bounds()?;
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: request_id,
            method: operation.method_name(),
            params: operation.params(),
        };
        let request_body =
            serde_json::to_vec(&request).map_err(|source| PumasRpcError::InvalidRequest {
                operation: operation.method_name().to_string(),
                reason: source.to_string(),
            })?;
        if request_body.len() > MAX_REQUEST_BYTES {
            return Err(PumasRpcError::RequestTooLarge {
                operation: operation.method_name().to_string(),
                max_bytes: MAX_REQUEST_BYTES,
            });
        }

        let response = self
            .http
            .post(self.endpoint.clone())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(request_body)
            .send()
            .await
            .map_err(|source| request_error(operation.method_name(), source))?;

        let status = response.status();
        let mut body = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|source| request_error(operation.method_name(), source))?;
            if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(PumasRpcError::ResponseTooLarge {
                    operation: operation.method_name().to_string(),
                    max_bytes: MAX_RESPONSE_BYTES,
                });
            }
            body.extend_from_slice(&chunk);
        }

        let response: JsonRpcResponse =
            serde_json::from_slice(&body).map_err(|source| PumasRpcError::InvalidResponse {
                operation: operation.method_name().to_string(),
                reason: format!(
                    "HTTP status {} response is not valid JSON: {source}",
                    status
                ),
            })?;
        let result = response.into_result(request_id, operation.method_name(), status.as_u16());
        if status.is_success() {
            result
        } else {
            match result {
                Ok(_) => Err(PumasRpcError::HttpStatus {
                    operation: operation.method_name().to_string(),
                    status: status.as_u16(),
                    body: bounded_error_body(&body),
                }),
                Err(error) => Err(error),
            }
        }
    }
}

fn request_error(operation: &str, source: reqwest::Error) -> PumasRpcError {
    if source.is_timeout() {
        PumasRpcError::Timeout {
            operation: operation.to_string(),
            timeout: REQUEST_TIMEOUT,
        }
    } else {
        PumasRpcError::Transport {
            operation: operation.to_string(),
            message: source.to_string(),
        }
    }
}

fn validate_endpoint(endpoint: &Url) -> Result<(), PumasRpcError> {
    let host = endpoint.host_str().unwrap_or_default();
    let host_literal = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    let loopback = host_literal
        .parse::<std::net::IpAddr>()
        .is_ok_and(|address| address.is_loopback());
    if !loopback {
        return Err(PumasRpcError::InvalidEndpoint {
            reason: "Pumas RPC is restricted to a loopback endpoint".to_string(),
        });
    }
    if !matches!(endpoint.scheme(), "http" | "https") {
        return Err(PumasRpcError::InvalidEndpoint {
            reason: "Pumas RPC requires an HTTP(S) endpoint".to_string(),
        });
    }
    if endpoint.path() != "/rpc" || endpoint.query().is_some() || endpoint.fragment().is_some() {
        return Err(PumasRpcError::InvalidEndpoint {
            reason: "Pumas RPC endpoint must be exactly a loopback /rpc URL".to_string(),
        });
    }
    if endpoint.username() != "" || endpoint.password().is_some() {
        return Err(PumasRpcError::InvalidEndpoint {
            reason: "Pumas RPC endpoint must not contain credentials".to_string(),
        });
    }
    Ok(())
}

fn bounded_error_body(body: &[u8]) -> String {
    const MAX_ERROR_TEXT_BYTES: usize = 512;
    String::from_utf8_lossy(&body[..body.len().min(MAX_ERROR_TEXT_BYTES)]).into_owned()
}

#[derive(Debug, Clone, Serialize)]
struct JsonRpcRequest<'a> {
    jsonrpc: &'static str,
    id: u64,
    method: &'a str,
    params: Value,
}

/// JSON-RPC envelope compatibility policy: unknown envelope fields are
/// forward-compatible and ignored, while `jsonrpc`, `id`, and the exclusive
/// result/error discriminators are validated below. Domain payloads are
/// decoded by their typed Pumas contracts after this transport boundary.
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(default, deserialize_with = "deserialize_rpc_field")]
    result: JsonRpcField<Value>,
    #[serde(default, deserialize_with = "deserialize_rpc_field")]
    error: JsonRpcField<JsonRpcErrorObject>,
}

#[derive(Debug, Default)]
enum JsonRpcField<T> {
    #[default]
    Missing,
    Null,
    Value(T),
}

fn deserialize_rpc_field<'de, D, T>(deserializer: D) -> Result<JsonRpcField<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(|value| match value {
        Some(value) => JsonRpcField::Value(value),
        None => JsonRpcField::Null,
    })
}

/// Unknown producer diagnostics remain forward-compatible; a present
/// `data.class` discriminator is nevertheless required to be a string.
#[derive(Debug, Deserialize)]
struct JsonRpcErrorObject {
    code: i32,
    message: String,
    #[serde(default)]
    data: Option<Value>,
}

impl JsonRpcResponse {
    fn into_result(
        self,
        request_id: u64,
        operation: &str,
        http_status: u16,
    ) -> Result<Value, PumasRpcError> {
        if self.jsonrpc != "2.0" {
            return Err(PumasRpcError::InvalidResponse {
                operation: operation.to_string(),
                reason: format!("unsupported JSON-RPC version {:?}", self.jsonrpc),
            });
        }
        if self.id != json!(request_id) {
            return Err(PumasRpcError::InvalidResponse {
                operation: operation.to_string(),
                reason: "response id does not match request id".to_string(),
            });
        }
        match (self.result, self.error) {
            (JsonRpcField::Value(result), JsonRpcField::Missing) => Ok(result),
            (JsonRpcField::Null, JsonRpcField::Missing) => Ok(Value::Null),
            (JsonRpcField::Missing, JsonRpcField::Value(error)) => {
                let class = match error.data.as_ref().and_then(|data| data.get("class")) {
                    None => None,
                    Some(value) => Some(
                        value
                            .as_str()
                            .ok_or_else(|| PumasRpcError::InvalidResponse {
                                operation: operation.to_string(),
                                reason: "remote error data.class is not a string".to_string(),
                            })?
                            .to_string(),
                    ),
                };
                Err(PumasRpcError::Remote {
                    operation: operation.to_string(),
                    code: error.code,
                    class,
                    data: error.data,
                    message: error.message,
                    http_status: (http_status != 200).then_some(http_status),
                })
            }
            (JsonRpcField::Value(_) | JsonRpcField::Null, JsonRpcField::Value(_))
            | (JsonRpcField::Value(_) | JsonRpcField::Null, JsonRpcField::Null)
            | (JsonRpcField::Missing, JsonRpcField::Null) => Err(PumasRpcError::InvalidResponse {
                operation: operation.to_string(),
                reason: "response contains both result and error".to_string(),
            }),
            (JsonRpcField::Missing, JsonRpcField::Missing) => Err(PumasRpcError::InvalidResponse {
                operation: operation.to_string(),
                reason: "response contains neither result nor error".to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub enum PumasRpcOperation {
    GetModels,
    SearchModelsFts {
        query: String,
        limit: usize,
        offset: usize,
    },
    ResolveModelExecutionDescriptor {
        model_id: String,
    },
    ResolveModelArtifactLoadTarget {
        request: pumas_library::models::ResolveModelArtifactLoadTargetRequest,
    },
    ResolveModelPackageFacts {
        model_id: String,
    },
    ResolveModelPackageFactsSummary {
        model_id: String,
    },
    ModelPackageFactsSummarySnapshot {
        limit: usize,
        offset: usize,
    },
    ListModelLibraryUpdatesSince {
        cursor: Option<String>,
        limit: usize,
    },
}

impl PumasRpcOperation {
    fn validate_input_bounds(&self) -> Result<(), PumasRpcError> {
        let oversized = match self {
            Self::GetModels => false,
            Self::SearchModelsFts { query, .. }
            | Self::ResolveModelExecutionDescriptor { model_id: query }
            | Self::ResolveModelPackageFacts { model_id: query }
            | Self::ResolveModelPackageFactsSummary { model_id: query } => {
                estimated_json_string_size(query) > MAX_REQUEST_BYTES
            }
            Self::ResolveModelArtifactLoadTarget { request } => {
                serde_json::to_vec(request)
                    .map_err(|source| PumasRpcError::InvalidRequest {
                        operation: self.method_name().to_string(),
                        reason: source.to_string(),
                    })?
                    .len()
                    > MAX_REQUEST_BYTES
            }
            Self::ModelPackageFactsSummarySnapshot { .. }
            | Self::ListModelLibraryUpdatesSince { cursor: None, .. } => false,
            Self::ListModelLibraryUpdatesSince {
                cursor: Some(cursor),
                ..
            } => estimated_json_string_size(cursor) > MAX_REQUEST_BYTES,
        };
        if oversized {
            return Err(PumasRpcError::RequestTooLarge {
                operation: self.method_name().to_string(),
                max_bytes: MAX_REQUEST_BYTES,
            });
        }
        Ok(())
    }

    fn method_name(&self) -> &'static str {
        match self {
            Self::GetModels => "get_models",
            Self::SearchModelsFts { .. } => "search_models_fts",
            Self::ResolveModelExecutionDescriptor { .. } => "resolve_model_execution_descriptor",
            Self::ResolveModelArtifactLoadTarget { .. } => "resolve_model_artifact_load_target",
            Self::ResolveModelPackageFacts { .. } => "resolve_model_package_facts",
            Self::ResolveModelPackageFactsSummary { .. } => "resolve_model_package_facts_summary",
            Self::ModelPackageFactsSummarySnapshot { .. } => "model_package_facts_summary_snapshot",
            Self::ListModelLibraryUpdatesSince { .. } => "list_model_library_updates_since",
        }
    }

    fn params(&self) -> Value {
        match self {
            Self::GetModels => json!({}),
            Self::SearchModelsFts {
                query,
                limit,
                offset,
            } => json!({ "query": query, "limit": limit, "offset": offset }),
            Self::ResolveModelExecutionDescriptor { model_id }
            | Self::ResolveModelPackageFacts { model_id }
            | Self::ResolveModelPackageFactsSummary { model_id } => {
                json!({ "model_id": model_id })
            }
            Self::ResolveModelArtifactLoadTarget { request } => json!({ "request": request }),
            Self::ModelPackageFactsSummarySnapshot { limit, offset } => {
                json!({ "limit": limit, "offset": offset })
            }
            Self::ListModelLibraryUpdatesSince { cursor, limit } => {
                json!({ "cursor": cursor, "limit": limit })
            }
        }
    }
}

fn estimated_json_string_size(value: &str) -> usize {
    // JSON string escaping can expand control characters substantially. A
    // conservative bound keeps validation ahead of request serialization.
    value.len().saturating_mul(6).saturating_add(64)
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PumasRpcError {
    #[error("invalid Pumas RPC endpoint: {reason}")]
    InvalidEndpoint { reason: String },

    #[error("Pumas RPC transport failed during {operation}: {message}")]
    Transport { operation: String, message: String },

    #[error("Pumas RPC {operation} timed out after {timeout:?}")]
    Timeout {
        operation: String,
        timeout: Duration,
    },

    #[error("Pumas RPC request for {operation} is invalid: {reason}")]
    InvalidRequest { operation: String, reason: String },

    #[error("Pumas RPC request for {operation} exceeded {max_bytes} bytes")]
    RequestTooLarge { operation: String, max_bytes: usize },

    #[error("Pumas RPC {operation} returned HTTP status {status}: {body}")]
    HttpStatus {
        operation: String,
        status: u16,
        body: String,
    },

    #[error("Pumas RPC response for {operation} exceeded {max_bytes} bytes")]
    ResponseTooLarge { operation: String, max_bytes: usize },

    #[error("invalid Pumas RPC response for {operation}: {reason}")]
    InvalidResponse { operation: String, reason: String },

    #[error("Pumas RPC operation {operation} failed ({code}, {class:?}): {message}")]
    Remote {
        operation: String,
        code: i32,
        class: Option<String>,
        data: Option<Value>,
        message: String,
        http_status: Option<u16>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn test_server(response: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let address = listener.local_addr().expect("test server address");
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept test request");
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request);
            let body = response.as_bytes();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                response
            )
            .expect("write test response");
        });
        format!("http://{address}/rpc")
    }

    #[test]
    fn endpoint_is_loopback_rpc_only() {
        assert!(PumasRpcClient::new("http://example.test/rpc").is_err());
        assert!(PumasRpcClient::new("http://localhost:34791/rpc").is_err());
        assert!(PumasRpcClient::new("http://127.0.0.1/not-rpc").is_err());
        assert!(PumasRpcClient::new("http://127.0.0.1/rpc?redirect=1").is_err());
        let error = match PumasRpcClient::new("http://user:secret@127.0.0.1/rpc") {
            Ok(_) => panic!("credentials must be rejected"),
            Err(error) => error,
        };
        assert!(!error.to_string().contains("secret"));
        assert!(!format!("{error:?}").contains("secret"));
        assert!(PumasRpcClient::new("http://127.0.0.1/rpc").is_ok());
        assert!(PumasRpcClient::new("http://[::1]:34791/rpc").is_ok());
    }

    #[test]
    fn encodes_only_the_admitted_operation_shape() {
        let operation = PumasRpcOperation::SearchModelsFts {
            query: "tiny".to_string(),
            limit: 3,
            offset: 1,
        };
        assert_eq!(operation.method_name(), "search_models_fts");
        assert_eq!(
            operation.params(),
            json!({ "query": "tiny", "limit": 3, "offset": 1 })
        );
    }

    #[tokio::test]
    async fn accepts_matching_success_envelope() {
        let client = PumasRpcClient::new(&test_server(
            r#"{"jsonrpc":"2.0","result":{"models":{}},"id":1}"#,
        ))
        .expect("client");
        let result = client
            .call(PumasRpcOperation::GetModels)
            .await
            .expect("success response");
        assert_eq!(result, json!({ "models": {} }));
    }

    #[tokio::test]
    async fn accepts_forward_compatible_envelope_fields() {
        let client = PumasRpcClient::new(&test_server(
            r#"{"jsonrpc":"2.0","result":{"models":{}},"id":1,"trace_id":"ignored"}"#,
        ))
        .expect("client");
        assert_eq!(
            client
                .call(PumasRpcOperation::GetModels)
                .await
                .expect("success response with extension field"),
            json!({ "models": {} })
        );
    }

    #[tokio::test]
    async fn preserves_typed_remote_error_class() {
        let client = PumasRpcClient::new(&test_server(
            r#"{"jsonrpc":"2.0","error":{"code":-32002,"message":"missing","data":{"class":"not_found"}},"id":1}"#,
        ))
        .expect("client");
        assert_eq!(
            client
                .call(PumasRpcOperation::ResolveModelPackageFacts {
                    model_id: "missing".to_string(),
                })
                .await,
            Err(PumasRpcError::Remote {
                operation: "resolve_model_package_facts".to_string(),
                code: -32002,
                class: Some("not_found".to_string()),
                data: Some(json!({ "class": "not_found" })),
                message: "missing".to_string(),
                http_status: None,
            })
        );
    }

    #[tokio::test]
    async fn rejects_mismatched_response_id() {
        let client = PumasRpcClient::new(&test_server(r#"{"jsonrpc":"2.0","result":{},"id":99}"#))
            .expect("client");
        let error = client
            .call(PumasRpcOperation::GetModels)
            .await
            .expect_err("mismatched response id");
        assert!(matches!(error, PumasRpcError::InvalidResponse { .. }));
    }

    #[tokio::test]
    async fn accepts_null_result_but_rejects_null_error_with_result() {
        let null_result =
            PumasRpcClient::new(&test_server(r#"{"jsonrpc":"2.0","result":null,"id":1}"#))
                .expect("client");
        assert_eq!(
            null_result
                .call(PumasRpcOperation::GetModels)
                .await
                .expect("null result is a valid result"),
            Value::Null
        );

        let both_fields = PumasRpcClient::new(&test_server(
            r#"{"jsonrpc":"2.0","result":{},"error":null,"id":1}"#,
        ))
        .expect("client");
        assert!(matches!(
            both_fields.call(PumasRpcOperation::GetModels).await,
            Err(PumasRpcError::InvalidResponse { .. })
        ));
    }

    #[tokio::test]
    async fn rejects_oversized_requests_before_network_io() {
        let client = PumasRpcClient::new("http://127.0.0.1:9/rpc").expect("client");
        let error = client
            .call(PumasRpcOperation::SearchModelsFts {
                query: "x".repeat(MAX_REQUEST_BYTES),
                limit: 1,
                offset: 0,
            })
            .await
            .expect_err("oversized request");
        assert!(matches!(error, PumasRpcError::RequestTooLarge { .. }));
    }

    #[tokio::test]
    async fn rejects_escape_expansion_before_network_io() {
        let client = PumasRpcClient::new("http://127.0.0.1:9/rpc").expect("client");
        let error = client
            .call(PumasRpcOperation::SearchModelsFts {
                query: "\u{0000}".repeat(MAX_REQUEST_BYTES / 6),
                limit: 1,
                offset: 0,
            })
            .await
            .expect_err("escape-expanded request");
        assert!(matches!(error, PumasRpcError::RequestTooLarge { .. }));
    }
}
