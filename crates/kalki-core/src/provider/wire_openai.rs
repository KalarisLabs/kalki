//! OpenAI Responses wire mapping. TASK P3 fills these in.
use serde_json::Value;

use super::{ProviderError, ProviderEvent};

/// Map an SSE `data:` frame from the Responses API into a normalized event.
///
/// # Errors
/// Unknown or malformed event types return a protocol error (caller surfaces as `ProviderError::Protocol`).
pub(crate) fn map_responses_event(v: &Value) -> Result<ProviderEvent, ProviderError> {
    let _ = v;
    Err(ProviderError::Protocol {
        provider: "openai_responses",
        message: "wire not implemented yet (task P3)".to_string(),
    })
}
