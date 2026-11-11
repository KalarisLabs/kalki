//! OpenAI Responses wire: request body, HTTP transport, and SSE frame mapping.
//!
//! Frame contract (verified): `response.output_text.delta` carries text;
//! `response.output_item.added` / `response.content_part.added` carry tool-call
//! items as `item` with `id` + `name`; `response.function_call_arguments.delta`
//! carries argument fragments keyed by `item_id`; `response.completed` carries
//! `response.usage` with `input_tokens` / `output_tokens` (+ cached_tokens).
//! Unknown event types are ignored (`Ok(None)`) — the stream carries many
//! auxiliary frames; known types with malformed payloads are protocol errors.

use serde_json::{Value, json};

use super::{
    ChatRequest, Provider, ProviderError, ProviderEvent, ProviderKind, ProviderStream, ToolSpec,
};

/// Wire identifier used in error variants.
const WIRE: &str = "openai_responses";

/// `OpenAI` Responses provider (`{base}/v1/responses`).
pub struct OpenAiResponsesProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl OpenAiResponsesProvider {
    /// Build from the environment: `OPENAI_API_KEY` (required) and
    /// `OPENAI_BASE_URL` (optional, defaults to `https://api.openai.com`).
    ///
    /// # Errors
    /// `ProviderError::MissingConfig` when the key is absent or blank.
    pub fn from_env() -> Result<Self, ProviderError> {
        from_env_vars(
            "OPENAI_API_KEY",
            "OPENAI_BASE_URL",
            "https://api.openai.com",
            |key| std::env::var(key).ok(),
        )
    }
}

// Shared-shape env resolution kept per-wire so each wire file stays self-contained.
fn from_env_vars(
    key_var: &str,
    base_var: &str,
    default_base: &str,
    get: impl Fn(&str) -> Option<String>,
) -> Result<OpenAiResponsesProvider, ProviderError> {
    let api_key = get(key_var)
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .ok_or_else(|| ProviderError::MissingConfig(format!("{key_var} is not set or empty")))?;
    let base_url = get(base_var)
        .map(|b| b.trim().to_string())
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| default_base.to_string());
    Ok(OpenAiResponsesProvider {
        client: reqwest::Client::new(),
        base_url: base_url.trim_end_matches('/').to_string(),
        api_key,
    })
}

impl Provider for OpenAiResponsesProvider {
    fn name(&self) -> &'static str {
        WIRE
    }

    async fn open(&self, request: ChatRequest) -> Result<ProviderStream, ProviderError> {
        let body = build_body(&request)?;
        let url = format!("{}/v1/responses", self.base_url);
        let builder = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .json(&body);
        send_streaming(builder, ProviderKind::OpenAiResponses).await
    }
}

/// POST the request, mapping non-2xx to a truncated `Http` error and 2xx to a live stream.
async fn send_streaming(
    builder: reqwest::RequestBuilder,
    kind: ProviderKind,
) -> Result<ProviderStream, ProviderError> {
    let resp = builder.send().await.map_err(|e| transport_err(&e))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.bytes().await.map_err(|e| transport_err(&e))?;
        // Truncate on the wire contract: 512 bytes, not chars, so lossy decode never panics.
        let excerpt = &body[..body.len().min(512)];
        return Err(ProviderError::Http {
            status: status.as_u16(),
            message: String::from_utf8_lossy(excerpt).trim().to_string(),
        });
    }
    Ok(ProviderStream::new(kind, resp))
}

fn transport_err(e: &reqwest::Error) -> ProviderError {
    // reqwest errors are not std::io::Error; the Io variant is the transport slot.
    ProviderError::Io(std::io::Error::other(format!("http transport: {e}")))
}

fn build_body(request: &ChatRequest) -> Result<Value, ProviderError> {
    // Responses `input_text` for every turn: history fidelity (output_text /
    // function_call_output items for assistant and tool turns) is a known
    // simplification — out of scope for this task.
    let input: Vec<Value> = request
        .messages
        .iter()
        .map(|m| {
            json!({
                "role": m.role,
                "content": [{"type": "input_text", "text": m.content}],
            })
        })
        .collect();
    let mut body = json!({
        "model": request.model,
        "input": input,
        "stream": true,
    });
    if let Some(system) = &request.system {
        body["instructions"] = json!(system);
    }
    let tools = tool_values(&request.tools)?;
    if !tools.is_empty() {
        body["tools"] = Value::Array(tools);
    }
    if let Some(effort) = &request.reasoning_effort {
        body["reasoning"] = json!({"effort": effort});
    }
    if let Some(max) = request.max_output_tokens {
        body["max_output_tokens"] = json!(max);
    }
    Ok(body)
}

fn tool_values(tools: &[ToolSpec]) -> Result<Vec<Value>, ProviderError> {
    tools
        .iter()
        .map(|t| {
            let parameters: Value =
                serde_json::from_str(&t.parameters_json_schema).map_err(|e| {
                    malformed(format!(
                        "tool `{}` parameters_json_schema is not JSON: {e}",
                        t.name
                    ))
                })?;
            Ok(json!({
                "type": "function",
                "name": t.name,
                "description": t.description,
                "parameters": parameters,
            }))
        })
        .collect()
}

fn malformed(message: String) -> ProviderError {
    ProviderError::Protocol {
        provider: WIRE,
        message,
    }
}

fn req_str(v: &Value, key: &str) -> Result<String, ProviderError> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| malformed(format!("missing or invalid `{key}`")))
}

fn req_u64(v: &Value, key: &str) -> Result<u64, ProviderError> {
    v.get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| malformed(format!("missing or invalid `{key}`")))
}

/// Map an SSE `data:` frame from the Responses API into a normalized event.
///
/// Returns `Ok(None)` for unknown event types (the stream carries many
/// auxiliary frames); `Err` for known types with malformed payloads.
pub(crate) fn map_responses_event(v: &Value) -> Result<Option<ProviderEvent>, ProviderError> {
    let ty = v
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| malformed("missing `type`".to_string()))?;
    match ty {
        "response.output_text.delta" => Ok(Some(ProviderEvent::TextDelta(req_str(v, "delta")?))),
        "response.completed" => {
            let usage = v
                .pointer("/response/usage")
                .ok_or_else(|| malformed("missing `response.usage`".to_string()))?;
            Ok(Some(ProviderEvent::Usage {
                input_tokens: req_u64(usage, "input_tokens")?,
                output_tokens: req_u64(usage, "output_tokens")?,
                cached_tokens: usage
                    .pointer("/input_tokens_details/cached_tokens")
                    .and_then(Value::as_u64),
            }))
        }
        "response.output_item.added" | "response.content_part.added" => {
            // Tool calls surface as items with id + name; other items/parts carry none — ignore them.
            let Some(item) = v.get("item") else {
                return Ok(None);
            };
            match (
                item.get("id").and_then(Value::as_str),
                item.get("name").and_then(Value::as_str),
            ) {
                (Some(id), Some(name)) => Ok(Some(ProviderEvent::ToolCallStart {
                    id: id.to_string(),
                    name: name.to_string(),
                })),
                _ => Ok(None),
            }
        }
        "response.function_call_arguments.delta" => Ok(Some(ProviderEvent::ToolCallArgsDelta {
            id: req_str(v, "item_id")?,
            delta: req_str(v, "delta")?,
        })),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn map_responses_event_returns_text_delta_when_output_text_delta() {
        let frame = json!({"type": "response.output_text.delta", "delta": "hello"});
        let mapped = map_responses_event(&frame).unwrap();
        assert_eq!(mapped, Some(ProviderEvent::TextDelta("hello".to_string())));
    }

    #[test]
    fn map_responses_event_returns_usage_with_cached_tokens_when_response_completed() {
        let frame = json!({
            "type": "response.completed",
            "response": {"usage": {
                "input_tokens": 120,
                "output_tokens": 34,
                "input_tokens_details": {"cached_tokens": 96},
            }},
        });
        let mapped = map_responses_event(&frame).unwrap();
        assert_eq!(
            mapped,
            Some(ProviderEvent::Usage {
                input_tokens: 120,
                output_tokens: 34,
                cached_tokens: Some(96),
            })
        );
    }

    #[test]
    fn map_responses_event_returns_usage_without_cached_when_details_absent() {
        let frame = json!({
            "type": "response.completed",
            "response": {"usage": {"input_tokens": 10, "output_tokens": 5}},
        });
        let mapped = map_responses_event(&frame).unwrap();
        assert_eq!(
            mapped,
            Some(ProviderEvent::Usage {
                input_tokens: 10,
                output_tokens: 5,
                cached_tokens: None,
            })
        );
    }

    #[test]
    fn map_responses_event_returns_tool_call_start_when_function_call_item_added() {
        let frame = json!({
            "type": "response.output_item.added",
            "item": {"type": "function_call", "id": "fc_123", "call_id": "call_abc", "name": "bash"},
        });
        let mapped = map_responses_event(&frame).unwrap();
        assert_eq!(
            mapped,
            Some(ProviderEvent::ToolCallStart {
                id: "fc_123".to_string(),
                name: "bash".to_string(),
            })
        );
    }

    #[test]
    fn map_responses_event_returns_tool_call_start_when_tool_item_part_added() {
        let frame = json!({
            "type": "response.content_part.added",
            "item": {"id": "fc_9", "name": "git_status"},
        });
        let mapped = map_responses_event(&frame).unwrap();
        assert_eq!(
            mapped,
            Some(ProviderEvent::ToolCallStart {
                id: "fc_9".to_string(),
                name: "git_status".to_string(),
            })
        );
    }

    #[test]
    fn map_responses_event_ignores_item_without_id_and_name() {
        let frame = json!({"type": "response.output_item.added", "item": {"type": "message"}});
        assert_eq!(map_responses_event(&frame).unwrap(), None);
    }

    #[test]
    fn map_responses_event_returns_args_delta_when_arguments_delta() {
        let frame = json!({
            "type": "response.function_call_arguments.delta",
            "item_id": "fc_123",
            "delta": "{\"cmd\":",
        });
        let mapped = map_responses_event(&frame).unwrap();
        assert_eq!(
            mapped,
            Some(ProviderEvent::ToolCallArgsDelta {
                id: "fc_123".to_string(),
                delta: "{\"cmd\":".to_string(),
            })
        );
    }

    #[test]
    fn map_responses_event_ignores_unknown_event_types() {
        for frame in [
            json!({"type": "response.in_progress"}),
            json!({"type": "response.reasoning_summary_text.delta", "delta": "x"}),
            json!({"type": "totally_synthetic_future_event"}),
        ] {
            assert_eq!(map_responses_event(&frame).unwrap(), None);
        }
    }

    #[test]
    fn map_responses_event_errors_when_type_missing() {
        let frame = json!({"delta": "hello"});
        assert!(matches!(
            map_responses_event(&frame),
            Err(ProviderError::Protocol { .. })
        ));
    }

    #[test]
    fn map_responses_event_errors_when_text_delta_missing_delta() {
        let frame = json!({"type": "response.output_text.delta"});
        assert!(matches!(
            map_responses_event(&frame),
            Err(ProviderError::Protocol { .. })
        ));
    }

    #[test]
    fn map_responses_event_errors_when_completed_usage_missing() {
        let frame = json!({"type": "response.completed", "response": {}});
        assert!(matches!(
            map_responses_event(&frame),
            Err(ProviderError::Protocol { .. })
        ));
    }

    #[test]
    fn from_env_vars_errors_when_api_key_missing() {
        let result = from_env_vars(
            "OPENAI_API_KEY",
            "OPENAI_BASE_URL",
            "https://api.openai.com",
            |_| None,
        );
        assert!(matches!(result, Err(ProviderError::MissingConfig(_))));
    }

    #[test]
    fn from_env_vars_errors_when_api_key_blank() {
        let result = from_env_vars(
            "OPENAI_API_KEY",
            "OPENAI_BASE_URL",
            "https://api.openai.com",
            |key| (key == "OPENAI_API_KEY").then(|| "   ".to_string()),
        );
        assert!(matches!(result, Err(ProviderError::MissingConfig(_))));
    }

    #[test]
    fn from_env_vars_uses_default_base_when_override_absent() {
        let provider = from_env_vars(
            "OPENAI_API_KEY",
            "OPENAI_BASE_URL",
            "https://api.openai.com",
            |key| (key == "OPENAI_API_KEY").then(|| "sk-test".to_string()),
        )
        .unwrap();
        assert_eq!(provider.base_url, "https://api.openai.com");
    }

    #[test]
    fn from_env_vars_honors_base_url_override_and_trims_trailing_slash() {
        let provider = from_env_vars(
            "OPENAI_API_KEY",
            "OPENAI_BASE_URL",
            "https://api.openai.com",
            |key| match key {
                "OPENAI_API_KEY" => Some("sk-test".to_string()),
                "OPENAI_BASE_URL" => Some("http://127.0.0.1:8080/".to_string()),
                _ => None,
            },
        )
        .unwrap();
        assert_eq!(provider.base_url, "http://127.0.0.1:8080");
    }
}
