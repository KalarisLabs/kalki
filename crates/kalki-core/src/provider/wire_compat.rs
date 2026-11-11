//! OpenAI-compatible Chat Completions wire: request body, HTTP transport, and SSE mapping.
//!
//! Frame contract (mirrors the `data:` fast path in `mod.rs`): `choices[0].delta.content`
//! → text; `usage` (present when the server emits usage frames) → usage accounting;
//! `choices[0].delta.tool_calls` → tool-call start / argument deltas;
//! `choices[0].finish_reason` → finish. Unknown/absent shapes are ignored
//! (`Ok(None)`); structurally broken frames are protocol errors.
//!
//! Tool-call argument deltas key on `idx:<index>` (compat wires repeat `index`,
//! rarely the id, per delta); if a wire does repeat `id` it is used verbatim.
//! The session loop correlates `idx:N` the same way as the Anthropic wire.

use serde_json::{Value, json};

use super::{
    ChatRequest, Provider, ProviderError, ProviderEvent, ProviderKind, ProviderStream, ToolSpec,
};

/// Wire identifier used in error variants.
const WIRE: &str = "openai_compat";

/// OpenAI-compatible Chat Completions provider (`{base}/chat/completions`).
pub struct OpenAiCompatProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl OpenAiCompatProvider {
    /// Build from the environment: `OPENAI_COMPAT_API_KEY` (required) and
    /// `OPENAI_COMPAT_BASE_URL` (optional, defaults to `http://localhost:1234/v1`,
    /// the LM Studio default; vLLM/Ollama users should set it, e.g. to `http://localhost:11434/v1`).
    ///
    /// # Errors
    /// `ProviderError::MissingConfig` when the key is absent or blank.
    pub fn from_env() -> Result<Self, ProviderError> {
        from_env_vars(
            "OPENAI_COMPAT_API_KEY",
            "OPENAI_COMPAT_BASE_URL",
            "http://localhost:1234/v1",
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
) -> Result<OpenAiCompatProvider, ProviderError> {
    let api_key = get(key_var)
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .ok_or_else(|| ProviderError::MissingConfig(format!("{key_var} is not set or empty")))?;
    let base_url = get(base_var)
        .map(|b| b.trim().to_string())
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| default_base.to_string());
    Ok(OpenAiCompatProvider {
        client: reqwest::Client::new(),
        base_url: base_url.trim_end_matches('/').to_string(),
        api_key,
    })
}

impl Provider for OpenAiCompatProvider {
    fn name(&self) -> &'static str {
        WIRE
    }

    async fn open(&self, request: ChatRequest) -> Result<ProviderStream, ProviderError> {
        let body = build_body(&request)?;
        let url = format!("{}/chat/completions", self.base_url);
        let builder = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .json(&body);
        send_streaming(builder, ProviderKind::OpenAiCompat).await
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
    let mut messages: Vec<Value> = Vec::with_capacity(request.messages.len() + 1);
    if let Some(system) = &request.system {
        messages.push(json!({"role": "system", "content": system}));
    }
    for m in &request.messages {
        // `tool` turns need tool_call_id to satisfy compat servers; other roles stay bare.
        if m.role == "tool" {
            let mut msg = json!({"role": "tool", "content": m.content});
            if let Some(id) = &m.tool_call_id {
                msg["tool_call_id"] = json!(id);
            }
            messages.push(msg);
        } else {
            messages.push(json!({"role": m.role, "content": m.content}));
        }
    }
    // `max_output_tokens` is intentionally not forwarded: compat servers split on
    // `max_tokens` vs `max_completion_tokens` and o-series endpoints reject both.
    let mut body = json!({
        "model": request.model,
        "messages": messages,
        "stream": true,
    });
    let tools = tool_values(&request.tools)?;
    if !tools.is_empty() {
        body["tools"] = Value::Array(tools);
    }
    if let Some(effort) = &request.reasoning_effort {
        body["reasoning_effort"] = json!(effort);
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
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": parameters,
                },
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

/// Map a `data:` frame from the Chat Completions API into a normalized event.
///
/// Returns `Ok(None)` for frames we do not consume; `Err` for structurally broken
/// frames (wrong `usage`/`choices` shapes).
pub(crate) fn map_compat_event(v: &Value) -> Result<Option<ProviderEvent>, ProviderError> {
    if let Some(usage) = v.get("usage") {
        if !usage.is_object() {
            return Err(malformed("`usage` must be an object".to_string()));
        }
        let get = |k: &str| usage.get(k).and_then(Value::as_u64).unwrap_or(0);
        return Ok(Some(ProviderEvent::Usage {
            input_tokens: get("prompt_tokens"),
            output_tokens: get("completion_tokens"),
            cached_tokens: usage
                .pointer("/prompt_tokens_details/cached_tokens")
                .and_then(Value::as_u64),
        }));
    }
    let Some(choices) = v.get("choices") else {
        // e.g. role-only deltas with nothing usable, or future frame types.
        return Ok(None);
    };
    let choices = choices
        .as_array()
        .ok_or_else(|| malformed("`choices` must be an array".to_string()))?;
    let Some(first) = choices.first() else {
        return Ok(None);
    };
    if let Some(content) = first.pointer("/delta/content") {
        match content {
            Value::String(text) => return Ok(Some(ProviderEvent::TextDelta(text.clone()))),
            Value::Null => {} // servers emit explicit nulls on role-only frames
            _ => {
                return Err(malformed(
                    "`delta.content` must be a string or null".to_string(),
                ));
            }
        }
    }
    if let Some(tool_calls) = first.pointer("/delta/tool_calls") {
        let calls = tool_calls
            .as_array()
            .ok_or_else(|| malformed("`delta.tool_calls` must be an array".to_string()))?;
        // Compat frames carry one call per delta in practice; parallel calls arrive interleaved.
        let Some(call) = calls.first() else {
            return Ok(None);
        };
        let index = call.get("index").and_then(Value::as_u64).unwrap_or(0);
        let id = call.get("id").and_then(Value::as_str);
        let name = call.pointer("/function/name").and_then(Value::as_str);
        if let (Some(id), Some(name)) = (id, name) {
            return Ok(Some(ProviderEvent::ToolCallStart {
                id: id.to_string(),
                name: name.to_string(),
            }));
        }
        return match call.pointer("/function/arguments") {
            Some(Value::String(args)) => Ok(Some(ProviderEvent::ToolCallArgsDelta {
                id: id.map_or_else(|| format!("idx:{index}"), str::to_string),
                delta: args.clone(),
            })),
            Some(_) => Err(malformed(
                "`function.arguments` must be a string".to_string(),
            )),
            None => Ok(None),
        };
    }
    Ok(first
        .pointer("/finish_reason")
        .and_then(Value::as_str)
        .map(|reason| ProviderEvent::Finish {
            reason: reason.to_string(),
        }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn map_compat_event_returns_text_delta_when_delta_content() {
        let frame = json!({"choices": [{"delta": {"content": "hello"}}]});
        let mapped = map_compat_event(&frame).unwrap();
        assert_eq!(mapped, Some(ProviderEvent::TextDelta("hello".to_string())));
    }

    #[test]
    fn map_compat_event_ignores_role_only_delta() {
        let frame = json!({"choices": [{"delta": {"role": "assistant"}}]});
        assert_eq!(map_compat_event(&frame).unwrap(), None);
    }

    #[test]
    fn map_compat_event_returns_usage_when_usage_frame() {
        let frame = json!({
            "choices": [],
            "usage": {
                "prompt_tokens": 30,
                "completion_tokens": 12,
                "prompt_tokens_details": {"cached_tokens": 24},
            },
        });
        let mapped = map_compat_event(&frame).unwrap();
        assert_eq!(
            mapped,
            Some(ProviderEvent::Usage {
                input_tokens: 30,
                output_tokens: 12,
                cached_tokens: Some(24),
            })
        );
    }

    #[test]
    fn map_compat_event_returns_usage_without_cache_when_details_absent() {
        let frame = json!({"usage": {"prompt_tokens": 3, "completion_tokens": 4}});
        let mapped = map_compat_event(&frame).unwrap();
        assert_eq!(
            mapped,
            Some(ProviderEvent::Usage {
                input_tokens: 3,
                output_tokens: 4,
                cached_tokens: None,
            })
        );
    }

    #[test]
    fn map_compat_event_returns_finish_when_finish_reason() {
        let frame = json!({"choices": [{"delta": {}, "finish_reason": "tool_calls"}]});
        let mapped = map_compat_event(&frame).unwrap();
        assert_eq!(
            mapped,
            Some(ProviderEvent::Finish {
                reason: "tool_calls".to_string()
            })
        );
    }

    #[test]
    fn map_compat_event_returns_tool_call_start_when_id_and_name_present() {
        let frame = json!({
            "choices": [{"delta": {"tool_calls": [{
                "index": 0,
                "id": "call_abc",
                "type": "function",
                "function": {"name": "bash", "arguments": ""},
            }]}}],
        });
        let mapped = map_compat_event(&frame).unwrap();
        assert_eq!(
            mapped,
            Some(ProviderEvent::ToolCallStart {
                id: "call_abc".to_string(),
                name: "bash".to_string(),
            })
        );
    }

    #[test]
    fn map_compat_event_returns_indexed_args_delta_when_continuation_frame() {
        let frame = json!({
            "choices": [{"delta": {"tool_calls": [{
                "index": 1,
                "function": {"arguments": "{\"cmd\":"},
            }]}}],
        });
        let mapped = map_compat_event(&frame).unwrap();
        assert_eq!(
            mapped,
            Some(ProviderEvent::ToolCallArgsDelta {
                id: "idx:1".to_string(),
                delta: "{\"cmd\":".to_string(),
            })
        );
    }

    #[test]
    fn map_compat_event_ignores_unknown_frames() {
        for frame in [
            json!({"choices": [{"delta": {}}]}),
            json!({"system_fingerprint": "fp_123"}),
            json!({"choices": []}),
        ] {
            assert_eq!(map_compat_event(&frame).unwrap(), None);
        }
    }

    #[test]
    fn map_compat_event_errors_when_usage_wrong_shape() {
        let frame = json!({"usage": "not-an-object"});
        assert!(matches!(
            map_compat_event(&frame),
            Err(ProviderError::Protocol { .. })
        ));
    }

    #[test]
    fn map_compat_event_errors_when_choices_wrong_shape() {
        let frame = json!({"choices": "not-an-array"});
        assert!(matches!(
            map_compat_event(&frame),
            Err(ProviderError::Protocol { .. })
        ));
    }

    #[test]
    fn map_compat_event_errors_when_delta_content_wrong_type() {
        let frame = json!({"choices": [{"delta": {"content": 42}}]});
        assert!(matches!(
            map_compat_event(&frame),
            Err(ProviderError::Protocol { .. })
        ));
    }

    #[test]
    fn from_env_vars_errors_when_api_key_missing() {
        let result = from_env_vars(
            "OPENAI_COMPAT_API_KEY",
            "OPENAI_COMPAT_BASE_URL",
            "http://localhost:1234/v1",
            |_| None,
        );
        assert!(matches!(result, Err(ProviderError::MissingConfig(_))));
    }

    #[test]
    fn from_env_vars_uses_default_base_when_override_absent() {
        let provider = from_env_vars(
            "OPENAI_COMPAT_API_KEY",
            "OPENAI_COMPAT_BASE_URL",
            "http://localhost:1234/v1",
            |key| (key == "OPENAI_COMPAT_API_KEY").then(|| "compat-key".to_string()),
        )
        .unwrap();
        assert_eq!(provider.base_url, "http://localhost:1234/v1");
    }

    #[test]
    fn from_env_vars_honors_base_url_override_and_trims_trailing_slash() {
        let provider = from_env_vars(
            "OPENAI_COMPAT_API_KEY",
            "OPENAI_COMPAT_BASE_URL",
            "http://localhost:1234/v1",
            |key| match key {
                "OPENAI_COMPAT_API_KEY" => Some("compat-key".to_string()),
                "OPENAI_COMPAT_BASE_URL" => Some("http://127.0.0.1:11434/v1/".to_string()),
                _ => None,
            },
        )
        .unwrap();
        assert_eq!(provider.base_url, "http://127.0.0.1:11434/v1");
    }
}
