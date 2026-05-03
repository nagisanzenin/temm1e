//! Self-Audit Pass — one-shot verification that catches "stalled promise"
//! turns where a weak model emits intent text without calling a tool.
//!
//! ## Why this exists
//!
//! The agent loop's stop condition is "model emitted no tool call ⇒ done"
//! (`runtime.rs::process_message` near the `tool_uses.is_empty()` branch).
//! Flagship models (Claude / GPT / Gemini) honor an implicit "say-it-or-
//! call-it" contract — if they say "Let me X", they emit the X tool call
//! in the same turn. Smaller open-weight models (e.g. Qwen 27B / 35B-A3B,
//! reported in GH-62) routinely break that contract: they emit "Let me X"
//! as text and forget the tool call. The loop then exits as if done; the
//! worker flips `is_busy=false`; `/status` correctly reports "Idle"; the
//! user sees a promise and a silent bot.
//!
//! ## Design
//!
//! Per the One Model Rule (`feedback_one_model_rule.md`), the audit uses
//! the SAME active provider+model as the main loop — no cheap-fallback
//! classifier. To bound cost, the audit:
//!
//! - Runs at most ONCE per turn (hard cap, enforced at the call site).
//! - Only triggers when tools were available, the response had text, and
//!   no tool call was emitted (would be redundant otherwise).
//! - Fail-open: any malformed audit response degrades to today's baseline
//!   behavior — the loop exits with the original text. No worse than
//!   v5.5.5; never a regression.
//!
//! ## Marker
//!
//! Synthetic audit messages are tagged with [`AUDIT_MARKER_PREFIX`] so
//! they can be filtered from user-visible history dumps and from any
//! tooling that wants to ignore them.

use temm1e_core::types::message::{ChatMessage, MessageContent, Role};

/// Prefix that marks an internal Self-Audit user message so it can be
/// distinguished from real user input. Survives in `session.history` for
/// continuity but is filterable for display purposes.
pub const AUDIT_MARKER_PREFIX: &str = "[__INTERNAL_AUDIT__]";

/// The exact token the model must emit to confirm completion. Anything
/// else (tool call OR malformed text) is treated by [`classify_audit_response`].
pub const AUDIT_DONE_TOKEN: &str = "[DONE]";

/// Build the synthetic user-role message that asks the model to audit
/// its previous turn.
///
/// `prev_assistant_text` is currently unused in the prompt body — the
/// model already sees its own previous response above this message in
/// the conversation history. The parameter is kept for future iterations
/// that may want to quote the response inline (e.g. to handle reasoning
/// models whose previous "text" was actually `reasoning_content`).
pub fn format_audit_message(prev_assistant_text: &str) -> ChatMessage {
    let _ = prev_assistant_text;
    let body = format!(
        "{AUDIT_MARKER_PREFIX}\n\
         Reflect on your last response above. Choose ONE:\n\
         \n\
         A) If you completed the user's request and have nothing more to do, \
         reply with exactly \"{AUDIT_DONE_TOKEN}\" and nothing else. The \
         user will see your previous response, not this one.\n\
         B) If you stated intent (\"Let me X\", \"I'll check Y\") but did \
         not call a tool, emit the tool call now. The loop will execute it \
         and continue.\n\
         \n\
         Do NOT explain. Do NOT apologize. Pick A or B."
    );
    ChatMessage {
        role: Role::User,
        content: MessageContent::Text(body),
    }
}

/// Outcome of one Self-Audit round.
///
/// Mirrors [`temm1e_core::AuditOutcomeKind`]; this enum is the in-runtime
/// view that drives the loop decision, while the core enum is the
/// telemetry-friendly variant stored in `model_discipline`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditOutcome {
    /// Model confirmed completion with the [`AUDIT_DONE_TOKEN`]. The loop
    /// should exit and serve the ORIGINAL pre-audit text to the user.
    Done,
    /// Model emitted a tool call. The loop should execute it normally
    /// — exactly as if the audit round had been a regular turn.
    ToolCallTriggered,
    /// Audit response was malformed (no [`AUDIT_DONE_TOKEN`], no tool
    /// call). Fail-open: the loop should exit with the original text.
    /// Identical to today's baseline; never a regression.
    FailedOpen,
}

impl AuditOutcome {
    /// Convert to the telemetry-side enum for `record_audit_outcome`.
    pub fn to_kind(self) -> temm1e_core::AuditOutcomeKind {
        match self {
            AuditOutcome::Done => temm1e_core::AuditOutcomeKind::Done,
            AuditOutcome::ToolCallTriggered => temm1e_core::AuditOutcomeKind::ToolCallTriggered,
            AuditOutcome::FailedOpen => temm1e_core::AuditOutcomeKind::FailedOpen,
        }
    }
}

/// Classify an audit-round response.
///
/// `text_parts` are the assistant text content parts from the audit
/// round (joined with `\n` for substring search).
/// `had_tool_call` is `true` iff the audit round emitted at least one
/// `ContentPart::ToolUse`.
pub fn classify_audit_response(text_parts: &[String], had_tool_call: bool) -> AuditOutcome {
    if had_tool_call {
        return AuditOutcome::ToolCallTriggered;
    }
    let combined = text_parts.join("\n");
    if combined.contains(AUDIT_DONE_TOKEN) {
        AuditOutcome::Done
    } else {
        AuditOutcome::FailedOpen
    }
}

/// Test if a chat message is an internal audit message.
///
/// Useful for display filters and for stripping audit messages from
/// memory snapshots if a future release wants to keep them out of
/// long-term storage.
pub fn is_audit_message(msg: &ChatMessage) -> bool {
    matches!(&msg.content,
        MessageContent::Text(t) if t.starts_with(AUDIT_MARKER_PREFIX))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_audit_message_includes_marker_and_done_token() {
        let msg = format_audit_message("Let me check that file");
        let MessageContent::Text(body) = &msg.content else {
            panic!("expected text content");
        };
        assert!(body.starts_with(AUDIT_MARKER_PREFIX));
        assert!(body.contains(AUDIT_DONE_TOKEN));
        assert!(matches!(msg.role, Role::User));
    }

    #[test]
    fn classify_audit_done_token() {
        let parts = vec![AUDIT_DONE_TOKEN.to_string()];
        assert_eq!(classify_audit_response(&parts, false), AuditOutcome::Done);
    }

    #[test]
    fn classify_audit_done_token_with_surrounding_text() {
        // Some weak models won't perfectly emit only [DONE]; tolerate it.
        let parts = vec!["Yes, I'm done. [DONE]".to_string()];
        assert_eq!(classify_audit_response(&parts, false), AuditOutcome::Done);
    }

    #[test]
    fn classify_audit_tool_call() {
        let parts: Vec<String> = vec![];
        assert_eq!(
            classify_audit_response(&parts, true),
            AuditOutcome::ToolCallTriggered
        );
    }

    #[test]
    fn classify_audit_tool_call_overrides_text() {
        // If the model emits BOTH a tool call AND incidental text, treat
        // it as ToolCallTriggered — the action is what matters.
        let parts = vec!["Sure, calling now.".to_string()];
        assert_eq!(
            classify_audit_response(&parts, true),
            AuditOutcome::ToolCallTriggered
        );
    }

    #[test]
    fn classify_audit_malformed() {
        let parts = vec!["I dunno what to do here".to_string()];
        assert_eq!(
            classify_audit_response(&parts, false),
            AuditOutcome::FailedOpen
        );
    }

    #[test]
    fn classify_audit_empty() {
        let parts: Vec<String> = vec![];
        assert_eq!(
            classify_audit_response(&parts, false),
            AuditOutcome::FailedOpen
        );
    }

    #[test]
    fn is_audit_message_filter_works() {
        let audit = format_audit_message("");
        assert!(is_audit_message(&audit));

        let regular_user = ChatMessage {
            role: Role::User,
            content: MessageContent::Text("hello".to_string()),
        };
        assert!(!is_audit_message(&regular_user));

        let regular_assistant = ChatMessage {
            role: Role::Assistant,
            content: MessageContent::Text(format!("{AUDIT_MARKER_PREFIX} fake")),
        };
        // Marker check is content-based, not role-based, so this also
        // matches — but that's fine: assistant messages are never created
        // with the prefix in practice.
        assert!(is_audit_message(&regular_assistant));
    }

    #[test]
    fn outcome_to_kind_round_trip() {
        assert_eq!(
            AuditOutcome::Done.to_kind(),
            temm1e_core::AuditOutcomeKind::Done
        );
        assert_eq!(
            AuditOutcome::ToolCallTriggered.to_kind(),
            temm1e_core::AuditOutcomeKind::ToolCallTriggered
        );
        assert_eq!(
            AuditOutcome::FailedOpen.to_kind(),
            temm1e_core::AuditOutcomeKind::FailedOpen
        );
    }
}
