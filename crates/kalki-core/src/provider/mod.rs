//! Provider wires. Every call site in kalki-core uses this module (ADR-0002);
//! async-openai stays quarantined behind the trait until the conformance suite signs off.
//!
//! Streaming: [`ProviderStream`] is a hand-rolled async iterator (no `Stream` trait) —
//! `bytes_stream()` + line decoding, normalized into [`ProviderEvent`].

use serde::{Deserialize, Serialize};

/// A chat request normalized across wires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// Model identifier (e.g. `gpt-5.6`, `claude-sonnet-4.6`, `kimi-k3`).
    pub model: String,
    /// System/developer prompt.
    pub system: Option<String>,
    /// Ordered conversation.
    pub messages: Vec<ChatMessage>,
    /// Tool schemas exposed to the model (lowest-common-denominator JSON schema).
    pub tools: Vec<ToolSpec>,
    /// Hard cap on completion tokens.
    pub max_output_tokens: Option<u64>,
    /// Provider-specific reasoning effort hint (pass-through).
    pub reasoning_effort: Option<String>,
}

/// One message turn.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    /// `user` | `assistant` | `tool` | `system`
    pub role: String,
    /// Plain text content (tool results are stringified JSON).
    pub content: String,
    /// For `tool` messages: the matching `ToolCall::id`.
    pub tool_call_id: Option<String>,
}

/// A tool schema in the lowest-common-denominator form (`snake_case`, closed objects, `additionalProperties: false`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    /// Tool name, ≤ 48 chars `snake_case`.
    pub name: String,
    /// One-sentence description.
    pub description: String,
    /// JSON-schema object as a string (serialized bytes must be stable for cache hits).
    pub parameters_json_schema: String,
}

/// Normalized streaming event from any wire.
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderEvent {
    /// Text token(s) for the current assistant turn.
    TextDelta(String),
    /// Reasoning/thinking token(s), when the wire exposes them.
    ReasoningDelta(String),
    /// A tool call begins.
    ToolCallStart {
        /// Wire-assigned or generated id.
        id: String,
        /// Tool name.
        name: String,
    },
    /// Incremental JSON arguments for an in-flight tool call.
    ToolCallArgsDelta {
        /// Matching `ToolCallStart::id`.
        id: String,
        /// Partial JSON fragment.
        delta: String,
    },
    /// Terminal usage accounting for this response.
    Usage {
        /// Input tokens (post-breakpoint for cached models).
        input_tokens: u64,
        /// Output tokens.
        output_tokens: u64,
        /// Cached tokens when reported.
        cached_tokens: Option<u64>,
    },
    /// Stream ended; `reason` is the provider-normalized finish reason.
    Finish {
        /// e.g. `stop` | `tool_calls` | `length` | `error`.
        reason: String,
    },
}

/// Errors on the provider path.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// Non-2xx HTTP with a truncated body message.
    #[error("http {status}: {message}")]
    Http {
        /// HTTP status code.
        status: u16,
        /// Truncated body excerpt (≤ 512 bytes), secrets scrubbed by the caller.
        message: String,
    },
    /// SSE frame could not be decoded.
    #[error("sse decode: {0}")]
    Sse(String),
    /// Wire protocol violation.
    #[error("protocol violation ({provider}): {message}")]
    Protocol {
        /// Wire name.
        provider: &'static str,
        /// Details.
        message: String,
    },
    /// Transport I/O failure.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// Configuration missing (e.g. no API key).
    #[error("missing configuration: {0}")]
    MissingConfig(String),
}

/// A single provider wire implementation.
pub trait Provider: Send + Sync {
    /// Wire identifier (`openai_responses` | `anthropic_messages` | `openai_compat`).
    fn name(&self) -> &'static str;

    /// Open a streaming chat completion. Errors surface before the first byte is consumed.
    fn open(
        &self,
        request: ChatRequest,
    ) -> impl std::future::Future<Output = Result<ProviderStream, ProviderError>> + Send;
}

/// Concrete streaming handle across wires (enum dispatch inside).
pub struct ProviderStream {
    kind: ProviderKind,
    stream: std::pin::Pin<Box<dyn futures_util::Stream<Item = reqwest::Result<Vec<u8>>> + Send>>,
    buf: Vec<u8>,
    data_lines: Vec<String>,
}

/// Which wire a [`ProviderStream`] decodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    /// `OpenAI` Responses API (SSE, `evented`).
    OpenAiResponses,
    /// Anthropic Messages API (SSE, evented).
    AnthropicMessages,
    /// `OpenAI-compatible Chat Completions` wire (SSE, `data:` lines).
    OpenAiCompat,
}

impl ProviderStream {
    /// Construct a stream from a live response (used by wire implementations).
    ///
    /// The response body is converted into an owned byte stream; nothing besides
    /// the network owns it afterwards.
    #[must_use]
    pub fn new(kind: ProviderKind, response: reqwest::Response) -> Self {
        use futures_util::StreamExt;
        let stream = response
            .bytes_stream()
            .map(|r| r.map(|b| b.to_vec()))
            .boxed();
        Self {
            kind,
            stream,
            buf: Vec::with_capacity(8192),
            data_lines: Vec::new(),
        }
    }

    /// Decode the next normalized event, or `None` at end of stream.
    ///
    /// # Errors
    /// Decode or I/O failure mid-stream.
    pub async fn next_event(&mut self) -> Option<Result<ProviderEvent, ProviderError>> {
        use futures_util::StreamExt;
        loop {
            if let Some(line) = take_line(&mut self.buf) {
                if let Some(event) = parse_sse_line(self.kind, &line, &mut self.data_lines) {
                    match event {
                        Ok(ev) => return Some(Ok(ev)),
                        Err(e) => return Some(Err(e)),
                    }
                }
                continue;
            }
            match self.stream.next().await {
                Some(Ok(chunk)) => self.buf.extend_from_slice(&chunk),
                Some(Err(e)) => {
                    return Some(Err(ProviderError::Sse(format!("read error: {e}"))));
                }
                None => {
                    return if self.data_lines.is_empty() {
                        None
                    } else {
                        Some(Err(ProviderError::Sse(
                            "unterminated data frame".to_string(),
                        )))
                    };
                }
            }
        }
    }
}

fn take_line(buf: &mut Vec<u8>) -> Option<String> {
    if let Some(pos) = buf.iter().position(|&b| b == b'\n') {
        let mut line = buf.drain(..=pos).collect::<Vec<_>>();
        if line.last() == Some(&b'\n') {
            line.pop();
        }
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        String::from_utf8(line).ok()
    } else {
        None
    }
}

fn parse_sse_line(
    kind: ProviderKind,
    line: &str,
    data_lines: &mut Vec<String>,
) -> Option<Result<ProviderEvent, ProviderError>> {
    if let Some(rest) = line.strip_prefix("data: ") {
        data_lines.push(rest.to_string());
        if rest == "[DONE]" {
            return Some(Ok(ProviderEvent::Finish {
                reason: "stop".to_string(),
            }));
        }
        // A line-level fast path: most wires terminate a frame with a single data: field.
        if kind == ProviderKind::OpenAiCompat {
            return decode_openai_compat_event(data_lines);
        }
        return None; // multi-line frames: caller waits for event: terminator
    }
    if line.strip_prefix("event:").is_some() {
        return decode_frame_complete(kind, data_lines);
    }
    None
}

// Shared with the `event:`-terminated path in decode_frame_complete; both
// delegate to wire_compat::map_compat_event so the fast path cannot drift.
fn decode_openai_compat_event(
    data_lines: &mut Vec<String>,
) -> Option<Result<ProviderEvent, ProviderError>> {
    let raw = data_lines.pop()?;
    let v: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => return Some(Err(ProviderError::Sse(format!("bad json: {e}")))),
    };
    match wire_compat::map_compat_event(&v) {
        Ok(Some(ev)) => Some(Ok(ev)),
        Ok(None) => None,
        Err(e) => Some(Err(e)),
    }
}

fn decode_frame_complete(
    kind: ProviderKind,
    data_lines: &mut Vec<String>,
) -> Option<Result<ProviderEvent, ProviderError>> {
    let raw = data_lines.pop()?;
    match serde_json::from_str::<serde_json::Value>(&raw) {
        Ok(v) => {
            // Map fns return Ok(None) for wire frames we deliberately ignore
            // (unknown event types); skipping them keeps the stream alive.
            let mapped = match kind {
                ProviderKind::OpenAiResponses => wire_openai::map_responses_event(&v),
                ProviderKind::AnthropicMessages => wire_anthropic::map_messages_event(&v),
                ProviderKind::OpenAiCompat => wire_compat::map_compat_event(&v),
            };
            match mapped {
                Ok(Some(ev)) => Some(Ok(ev)),
                Ok(None) => None,
                Err(e) => Some(Err(e)),
            }
        }
        Err(e) => Some(Err(ProviderError::Sse(format!("bad frame json: {e}")))),
    }
}

/// Anthropic Messages wire mapping (implemented in `wire_anthropic`).
pub(crate) mod wire_anthropic;
/// OpenAI-compatible Chat Completions mapping (implemented in `wire_compat`).
pub(crate) mod wire_compat;
/// OpenAI Responses wire mapping (implemented in `wire_openai`).
pub(crate) mod wire_openai;

// Mod.rs re-exports only (AGENTS.md §4); the wire files own their transport + mapping.
pub use wire_anthropic::AnthropicMessagesProvider;
pub use wire_compat::OpenAiCompatProvider;
pub use wire_openai::OpenAiResponsesProvider;
