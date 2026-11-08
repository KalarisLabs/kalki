//! OpenAI-compatible Chat Completions wire mapping. TASK P3 fills these in.
use serde_json::Value;

use super::{ProviderError, ProviderEvent};

/// Map a `data:` frame from the Chat Completions API into a normalized event.
///
/// # Errors
/// Unknown or malformed event types return a protocol error.
pub(crate) fn map_compat_event(v: &Value) -> Result<ProviderEvent, ProviderError> {
    let _ = v;
    Err(ProviderError::Protocol {
        provider: "openai_compat",
        message: "wire not implemented yet (task P3)".to_string(),
    })
}
