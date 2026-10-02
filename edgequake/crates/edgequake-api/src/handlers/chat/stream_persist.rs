//! Helpers for persisting assistant messages after chat SSE (SPEC-155 B3).

/// Finish reason stored on assistant messages after streaming.
pub fn assistant_stream_finish_reason(client_disconnected_mid_stream: bool) -> Option<String> {
    if client_disconnected_mid_stream {
        Some("interrupted".to_string())
    } else {
        Some("stop".to_string())
    }
}

/// Whether to write an assistant row after the SSE channel closes (needs accumulated text).
pub fn should_persist_stream_assistant(content: &str) -> bool {
    !content.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_reason_interrupted_when_client_drops() {
        assert_eq!(
            assistant_stream_finish_reason(true).as_deref(),
            Some("interrupted")
        );
    }

    #[test]
    fn finish_reason_stop_when_stream_completes() {
        assert_eq!(
            assistant_stream_finish_reason(false).as_deref(),
            Some("stop")
        );
    }

    #[test]
    fn skip_persist_when_no_content() {
        assert!(!should_persist_stream_assistant(""));
    }

    #[test]
    fn persist_partial_content() {
        assert!(should_persist_stream_assistant("partial"));
    }
}
