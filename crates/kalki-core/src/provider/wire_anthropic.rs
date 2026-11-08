//! Anthropic Messages wire mapping. TASK P3 fills these in.
use serde_json::Value;

use super::{ProviderError, ProviderEvent};

/// Map an SSE `data:` frame from the Messages API into a normalized event.
///
/// # Errors
/// Unknown or malformed event types return a protocol error.
pub(crate) fn map_messages_event(v: &Value) -> Result<ProviderEvent, ProviderError> {
    let _ = v;
    Err(ProviderError::Protocol {
        provider: "anthropic_messages",
        message: "wire not implemented yet (task P3)".to_string(),
    })
}
