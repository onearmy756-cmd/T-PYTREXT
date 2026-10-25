//! Mid-run `/steer` support (Python `AIAgent.steer` parity subset).
//!
//! Fundi keeps user guidance in a pending buffer and injects it into the
//! **last tool result** in the conversation tail, prefixed with a provenance
//! marker (`User guidance:`) so models don't confuse it with tool output.

use fundi_core::{Message, MessageRole};

const USER_GUIDANCE_MARKER: &str = "User guidance:";

/// Normalize steer text for acceptance (mirrors Python tests).
pub fn normalize_accepted_steer_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

/// Append a new steer fragment to an existing pending buffer (newline separated).
pub fn concat_pending(existing: Option<String>, next: String) -> String {
    match existing {
        None => next,
        Some(prev) if prev.is_empty() => next,
        Some(prev) => format!("{prev}\n{next}"),
    }
}

fn append_user_guidance_to_text(base: &str, guidance: &str) -> String {
    let guidance_block = format!("\n\n{USER_GUIDANCE_MARKER}\n{guidance}");
    format!("{base}{guidance_block}")
}

/// Apply pending steer guidance to the last `MessageRole::Tool` message in `messages`.
///
/// Returns `true` when a tool message was found and mutated.
pub fn apply_pending_steer_to_last_tool_message(messages: &mut [Message], guidance: &str) -> bool {
    let Some(idx) = messages.iter().rposition(|m| m.role == MessageRole::Tool) else {
        return false;
    };

    let msg = &mut messages[idx];
    let base = msg.content.clone().unwrap_or_default();
    msg.content = Some(append_user_guidance_to_text(&base, guidance));
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use fundi_core::Message;

    #[test]
    fn accepts_trims_and_rejects_empty() {
        assert_eq!(
            normalize_accepted_steer_text("  hello \n").as_deref(),
            Some("hello")
        );
        assert_eq!(normalize_accepted_steer_text(""), None);
        assert_eq!(normalize_accepted_steer_text("  \n\t "), None);
    }

    #[test]
    fn concatenates_multiple_steers() {
        let mut cur: Option<String> = None;
        cur = Some(concat_pending(cur, "first".into()));
        cur = Some(concat_pending(cur, "second".into()));
        cur = Some(concat_pending(cur, "third".into()));
        assert_eq!(cur.as_deref(), Some("first\nsecond\nthird"));
    }

    #[test]
    fn appends_to_last_tool_result_only() {
        let mut msgs = vec![
            Message::user("u"),
            Message::tool_result("a", "A"),
            Message::tool_result("b", "B"),
        ];
        assert!(apply_pending_steer_to_last_tool_message(&mut msgs, "note"));
        assert_eq!(msgs[1].content.as_deref(), Some("A"));
        let b = msgs[2].content.as_deref().unwrap_or("");
        assert!(b.starts_with("B"));
        assert!(b.contains("User guidance:"));
        assert!(b.contains("note"));
    }
}
