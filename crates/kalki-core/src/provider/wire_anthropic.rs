//! Anthropic Messages wire: request body, HTTP transport, and SSE frame mapping.
//!
//! Frame contract (verified): `message_start` carries `message.usage.input_tokens`;
//! `content_block_start` with `content_block.type == "tool_use"` opens a tool call
//! (id + name); `content_block_delta` carries `text_delta` (text) and
//! `input_json_delta` (argument fragments); `message_delta` carries the fold-in
//! `usage.output_tokens`; `message_stop` terminates; `error` is a protocol error.
//! Unknown event types (`ping`, block stops, thinking deltas) are ignored.
//! Honest simplification: argument deltas cannot resolve the block's tool-call id
//! (mapping is stateless), so `ToolCallArgsDelta.id` is `idx:<index>`; the session
//! loop correlates `idx:N` with the `content_block_start` id. `message_delta` reports
//! output tokens only — the session loop folds usages across both frames.

use serde_json::{Value, json};

use super::{
    ChatRequest, Provider, ProviderError, ProviderEvent, ProviderKind, ProviderStream, ToolSpec,
};

/// Wire identifier used in error variants.
const WIRE: &str = "anthropic_messages";

/// Anthropic Messages provider (`{base}/v1/messages`).
pub struct AnthropicMessagesProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl AnthropicMessagesProvider {
    /// Build from the environment: `ANTHROPIC_API_KEY` (required) and
    /// `ANTHROPIC_BASE_URL` (optional, defaults to `https://api.anthropic.com`).
    ///
    /// # Errors
    /// `ProviderError::MissingConfig` when the key is absent or blank.
    pub fn from_env() -> Result<Self, ProviderError> {
        from_env_vars(
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_BASE_URL",
            "https://api.anthropic.com",
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
) -> Result<AnthropicMessagesProvider, ProviderError> {
    let api_key = get(key_var)
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .ok_or_else(|| ProviderError::MissingConfig(format!("{key_var} is not set or empty")))?;
    let base_url = get(base_var)
        .map(|b| b.trim().to_string())
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| default_base.to_string());
    Ok(AnthropicMessagesProvider {
        client: reqwest::Client::new(),
        base_url: base_url.trim_end_matches('/').to_string(),
        api_key,
    })
}

impl Provider for AnthropicMessagesProvider {
    fn name(&self) -> &'static str {
        WIRE
    }

    async fn open(&self, request: ChatRequest) -> Result<ProviderStream, ProviderError> {
        let body = build_body(&request)?;
        let url = format!("{}/v1/messages", self.base_url);
        let builder = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .json(&body);
        send_streaming(builder, ProviderKind::AnthropicMessages).await
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
    // Anthropic tool results travel as `tool_result` blocks; tool calls as `tool_use` blocks.
    // Neither is assembled here (no tool-result replay in this task) — plain text turns only.
    let messages: Vec<Value> = request
        .messages
        .iter()
        .map(|m| json!({"role": m.role, "content": m.content}))
        .collect();
    let mut body = json!({
        "model": request.model,
        // Anthropic requires an explicit budget; 1024 is the wire minimum.
        "max_tokens": request.max_output_tokens.unwrap_or(1024),
        "messages": messages,
        "stream": true,
    });
    if let Some(system) = &request.system {
        body["system"] = json!(system);
    }
    let tools = tool_values(&request.tools)?;
    if !tools.is_empty() {
        body["tools"] = Value::Array(tools);
    }
    if let Some(effort) = &request.reasoning_effort {
        // Anthropic has no effort string; map to a thinking budget (numeric strings verbatim,
        // named efforts to ≥1024 budgets). Budget must stay below max_tokens or the API rejects.
        body["thinking"] = json!({
            "type": "enabled",
            "budget_tokens": thinking_budget(effort)?,
        });
    }
    Ok(body)
}

fn thinking_budget(effort: &str) -> Result<u64, ProviderError> {
    if let Ok(budget) = effort.trim().parse::<u64>() {
        return Ok(budget);
    }
    match effort.trim().to_ascii_lowercase().as_str() {
        "low" => Ok(1024),
        "medium" => Ok(4096),
        "high" => Ok(16384),
        other => Err(malformed(format!(
            "unrecognized reasoning_effort `{other}` (use low | medium | high or a token budget)"
        ))),
    }
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
                "name": t.name,
                "description": t.description,
                "input_schema": parameters,
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

/// Map an SSE `data:` frame from the Messages API into a normalized event.
///
/// Returns `Ok(None)` for unknown event types; `Err` for known types with
/// malformed payloads and for the provider-reported `error` event.
pub(crate) fn map_messages_event(v: &Value) -> Result<Option<ProviderEvent>, ProviderError> {
    let ty = v
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| malformed("missing `type`".to_string()))?;
    match ty {
        "message_start" => {
            let usage = v
                .pointer("/message/usage")
                .ok_or_else(|| malformed("missing `message.usage`".to_string()))?;
            Ok(Some(ProviderEvent::Usage {
                input_tokens: req_u64(usage, "input_tokens")?,
                output_tokens: 0,
                cached_tokens: usage.get("cache_read_input_tokens").and_then(Value::as_u64),
            }))
        }
        "content_block_start" => {
            let block = v
                .get("content_block")
                .ok_or_else(|| malformed("missing `content_block`".to_string()))?;
            if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                Ok(Some(ProviderEvent::ToolCallStart {
                    id: req_str(block, "id")?,
                    name: req_str(block, "name")?,
                }))
            } else {
                // Text blocks open implicitly with their first delta.
                Ok(None)
            }
        }
        "content_block_delta" => {
            let delta = v
                .get("delta")
                .ok_or_else(|| malformed("missing `delta`".to_string()))?;
            match delta.get("type").and_then(Value::as_str) {
                Some("text_delta") => Ok(Some(ProviderEvent::TextDelta(req_str(delta, "text")?))),
                Some("input_json_delta") => {
                    let index = v
                        .get("index")
                        .and_then(Value::as_u64)
                        .ok_or_else(|| malformed("missing `index`".to_string()))?;
                    // See module docs: stateless mapping keys deltas by index; the
                    // session loop correlates `idx:N` with the content_block id.
                    Ok(Some(ProviderEvent::ToolCallArgsDelta {
                        id: format!("idx:{index}"),
                        delta: req_str(delta, "partial_json")?,
                    }))
                }
                _ => Ok(None),
            }
        }
        "message_delta" => {
            // `usage` here carries the running output total; input arrived at message_start.
            let usage = v
                .get("usage")
                .ok_or_else(|| malformed("missing `usage`".to_string()))?;
            Ok(Some(ProviderEvent::Usage {
                input_tokens: 0,
                output_tokens: req_u64(usage, "output_tokens")?,
                cached_tokens: None,
            }))
        }
        "message_stop" => Ok(Some(ProviderEvent::Finish {
            reason: "stop".to_string(),
        })),
        "error" => {
            let kind = v
                .pointer("/error/type")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let message = v
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("unspecified provider error");
            Err(malformed(format!("{kind}: {message}")))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn map_messages_event_returns_input_usage_when_message_start() {
        let frame = json!({
            "type": "message_start",
            "message": {"usage": {"input_tokens": 42, "output_tokens": 1, "cache_read_input_tokens": 7}},
        });
        let mapped = map_messages_event(&frame).unwrap();
        assert_eq!(
            mapped,
            Some(ProviderEvent::Usage {
                input_tokens: 42,
                output_tokens: 0,
                cached_tokens: Some(7),
            })
        );
    }

    #[test]
    fn map_messages_event_returns_usage_without_cache_when_cache_absent() {
        let frame = json!({
            "type": "message_start",
            "message": {"usage": {"input_tokens": 10, "output_tokens": 1}},
        });
        let mapped = map_messages_event(&frame).unwrap();
        assert_eq!(
            mapped,
            Some(ProviderEvent::Usage {
                input_tokens: 10,
                output_tokens: 0,
                cached_tokens: None,
            })
        );
    }

    #[test]
    fn map_messages_event_returns_tool_call_start_when_tool_use_block_starts() {
        let frame = json!({
            "type": "content_block_start",
            "index": 1,
            "content_block": {"type": "tool_use", "id": "toolu_01", "name": "bash", "input": {}},
        });
        let mapped = map_messages_event(&frame).unwrap();
        assert_eq!(
            mapped,
            Some(ProviderEvent::ToolCallStart {
                id: "toolu_01".to_string(),
                name: "bash".to_string(),
            })
        );
    }

    #[test]
    fn map_messages_event_ignores_text_block_start() {
        let frame = json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": {"type": "text", "text": ""},
        });
        assert_eq!(map_messages_event(&frame).unwrap(), None);
    }

    #[test]
    fn map_messages_event_returns_text_delta_when_text_delta() {
        let frame = json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "text_delta", "text": "chunk"},
        });
        let mapped = map_messages_event(&frame).unwrap();
        assert_eq!(mapped, Some(ProviderEvent::TextDelta("chunk".to_string())));
    }

    #[test]
    fn map_messages_event_returns_indexed_args_delta_when_input_json_delta() {
        let frame = json!({
            "type": "content_block_delta",
            "index": 2,
            "delta": {"type": "input_json_delta", "partial_json": "{\"cmd\":"},
        });
        let mapped = map_messages_event(&frame).unwrap();
        assert_eq!(
            mapped,
            Some(ProviderEvent::ToolCallArgsDelta {
                id: "idx:2".to_string(),
                delta: "{\"cmd\":".to_string(),
            })
        );
    }

    #[test]
    fn map_messages_event_returns_output_usage_when_message_delta() {
        let frame = json!({
            "type": "message_delta",
            "delta": {"stop_reason": "tool_use"},
            "usage": {"output_tokens": 17},
        });
        let mapped = map_messages_event(&frame).unwrap();
        assert_eq!(
            mapped,
            Some(ProviderEvent::Usage {
                input_tokens: 0,
                output_tokens: 17,
                cached_tokens: None,
            })
        );
    }

    #[test]
    fn map_messages_event_returns_finish_stop_when_message_stop() {
        let frame = json!({"type": "message_stop"});
        let mapped = map_messages_event(&frame).unwrap();
        assert_eq!(
            mapped,
            Some(ProviderEvent::Finish {
                reason: "stop".to_string()
            })
        );
    }

    #[test]
    fn map_messages_event_errors_when_error_event() {
        let frame = json!({
            "type": "error",
            "error": {"type": "overloaded_error", "message": "Overloaded"},
        });
        match map_messages_event(&frame) {
            Err(ProviderError::Protocol { provider, message }) => {
                assert_eq!(provider, WIRE);
                assert!(message.contains("overloaded_error"));
            }
            other => panic!("expected protocol error, got {other:?}"),
        }
    }

    #[test]
    fn map_messages_event_ignores_unknown_event_types() {
        for frame in [
            json!({"type": "ping"}),
            json!({"type": "content_block_stop", "index": 0}),
            json!({"type": "some_future_event"}),
        ] {
            assert_eq!(map_messages_event(&frame).unwrap(), None);
        }
    }

    #[test]
    fn map_messages_event_errors_when_type_missing() {
        let frame = json!({"index": 0});
        assert!(matches!(
            map_messages_event(&frame),
            Err(ProviderError::Protocol { .. })
        ));
    }

    #[test]
    fn map_messages_event_errors_when_json_delta_missing_index() {
        let frame = json!({
            "type": "content_block_delta",
            "delta": {"type": "input_json_delta", "partial_json": "{}"},
        });
        assert!(matches!(
            map_messages_event(&frame),
            Err(ProviderError::Protocol { .. })
        ));
    }

    #[test]
    fn thinking_budget_maps_named_efforts_and_numeric_strings() {
        assert_eq!(thinking_budget("low").unwrap(), 1024);
        assert_eq!(thinking_budget("medium").unwrap(), 4096);
        assert_eq!(thinking_budget("high").unwrap(), 16384);
        assert_eq!(thinking_budget("2048").unwrap(), 2048);
        assert!(thinking_budget("extreme").is_err());
    }

    #[test]
    fn from_env_vars_errors_when_api_key_missing() {
        let result = from_env_vars(
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_BASE_URL",
            "https://api.anthropic.com",
            |_| None,
        );
        assert!(matches!(result, Err(ProviderError::MissingConfig(_))));
    }

    #[test]
    fn from_env_vars_uses_default_base_when_override_absent() {
        let provider = from_env_vars(
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_BASE_URL",
            "https://api.anthropic.com",
            |key| (key == "ANTHROPIC_API_KEY").then(|| "sk-ant-test".to_string()),
        )
        .unwrap();
        assert_eq!(provider.base_url, "https://api.anthropic.com");
    }

    #[test]
    fn from_env_vars_honors_base_url_override_and_trims_trailing_slash() {
        let provider = from_env_vars(
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_BASE_URL",
            "https://api.anthropic.com",
            |key| match key {
                "ANTHROPIC_API_KEY" => Some("sk-ant-test".to_string()),
                "ANTHROPIC_BASE_URL" => Some("http://127.0.0.1:9090/".to_string()),
                _ => None,
            },
        )
        .unwrap();
        assert_eq!(provider.base_url, "http://127.0.0.1:9090");
    }
}
