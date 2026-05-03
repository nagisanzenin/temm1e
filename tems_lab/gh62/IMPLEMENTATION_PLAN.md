# GH-62 Implementation Plan — Persist Through Multi-Step Tasks

**Branch:** `gh-62/persist-multistep`
**Confidence:** 100/0 (zero risk per CLAUDE.md zero-risk policy)
**Constraint:** ONE MODEL RULE — every sub-task uses the same active model (no cheap-classifier fallback)

---

## 1. Problem statement (final)

Agent loop terminates whenever the model emits a turn with no `tool_use` parts (`crates/temm1e-agent/src/runtime.rs:2009-2010`). Flagship models (Claude/GPT/Gemini) honor an implicit "say-it-or-call-it" contract; weak open-weight models (Qwen 27B / 35B-A3B) break it by emitting "Let me X" as text without the call. Result: worker correctly clears `is_busy=false` (`src/main.rs:5998`); `/status` correctly reports "Idle" (`src/main.rs:3135-3136`). The bug is a missing verifier between "no tool call" and "task done."

---

## 2. Design principles (locked)

1. **Implicit + Verify** contract — keep today's "no tool call = done" inference, add one verification before accepting it.
2. **One Model Rule** — verifier uses the same active model. No cheap-fallback classifier.
3. **Fail open** — any verifier error or malformed output exits via the existing path. No worse than today.
4. **Cache-safe** — additive changes go in stable system-prompt base, not volatile tail.
5. **Self-graduating** — telemetry lets reliable models bypass the verifier over time.
6. **Each phase independently shippable + reversible.**

---

## 3. Codebase facts that anchor this plan (all verified)

| Question | Answer |
|---|---|
| Where does the loop exit on text-only? | `runtime.rs:2009-2010` inside `loop {}` block starting at `runtime.rs:1288` |
| Where is the assistant message recorded to history on tool-call exit? | `runtime.rs:2509-2512` (Parts) and `runtime.rs:2500-2507` (prompted-mode Text) — runs ONLY when `tool_uses.is_empty()` is FALSE |
| Where does session.history persist long-term? | `src/main.rs:5664-5673` — entire history serialized as JSON `MemoryEntry` after each `process_message` |
| Does `is_compound_task` already have an LLM signal? | YES — `llm_classifier::TaskDifficulty::{Simple,Standard,Complex}` is computed at `runtime.rs:1075-1076`. We can derive `is_compound = difficulty != Simple` from the existing classifier call. Zero extra LLM round-trip. |
| Where is the system prompt stable base? | `src/main.rs:440` (`SYSTEM_PROMPT_BODY` const) — additions here stay cacheable. Volatile tail goes through `request.prepend_system_volatile` / `append_system_volatile` |
| Bug confirmed in `file_read`? | `crates/temm1e-tools/src/file.rs:113-128` — when 32KB byte cap fires, the inner shadowed `end` (byte index) doesn't propagate to the `[Showing lines]` footer trigger which checks the OUTER `end` (line index). Net: byte-cap truncations get only the cryptic `[output truncated at 32KB]` footer with no offset hint. |
| Memory schema upgrade path? | `crates/temm1e-memory/src/sqlite.rs:54-200` — bare `CREATE TABLE IF NOT EXISTS`. No migration framework needed. Backwards compatible. |
| Memory trait extension pattern? | `crates/temm1e-core/src/traits/memory.rs:177-227` — methods have default `Ok(())` impls so adding new methods doesn't break existing implementors (markdown.rs, failover.rs, MockMemory). |
| MockProvider for tests? | `crates/temm1e-test-utils/src/lib.rs:26-128` — single canned response only. **Must be extended** with `with_responses(Vec<CompletionResponse>)` queue for audit-round testing. |
| Release-protocol parity gate? | `docs/RELEASE_PROTOCOL.md:80-209` — every feature must show registration log in CLI / TUI / server smoke. Phase B (prompt change) requires no registration log. Phase D requires `[self-audit] enabled` log on init. |

---

## 4. Phases — implementation specs

### PHASE A — `file_read` truncation visibility (ZERO RISK, ship first)

**Files touched:**
- `crates/temm1e-tools/src/file.rs:90-141`

**Change:**
1. Replace lines 90-140 of `execute()` with logic that:
   - Computes `total_lines` once.
   - Computes `selected_end_line` = `min(start + limit, total_lines)`.
   - Builds line-numbered output as today.
   - When 32KB byte cap fires: count `\n` chars in the truncated string to get `lines_actually_emitted`; recompute `actual_end_line = start + lines_actually_emitted`. Use this for the footer + new header.
   - When EITHER condition truncates (line-limit OR byte-cap), emit a **header line** at the top AND keep the existing footer:
     - Header: `[TRUNCATED — showing lines {start}-{actual_end} of {total} total. To continue, call file_read with offset={actual_end + 1}]`
     - Footer (kept for backward-compat scanning): `[Showing lines {start}-{actual_end} of {total} total]` (always emitted on truncation now, not just on line-limit truncation)
   - When NOT truncated: no header, no footer (unchanged behavior for full reads).

**Tests added** (in `#[cfg(test)] mod tests` at end of `file.rs`):
- `truncation_header_appears_on_line_limit` — 5000-line fixture, limit=100 → header + footer present, offset=101
- `truncation_header_appears_on_byte_cap` — file with one 50KB line → header + footer present, math correct
- `no_truncation_no_header` — small file fully fits → neither header nor footer
- `byte_cap_offset_math_correct` — fixture sized so byte-cap fires mid-line → offset hint = next full line

**Risk:** Zero. Pure formatting. No control flow change. No provider impact. No existing tests for this code path → nothing to regress.

---

### PHASE B — Explicit "say-it-or-call-it" contract in stable system prompt (ZERO RISK)

**Files touched:**
- `src/main.rs:440-500` (`SYSTEM_PROMPT_BODY` const)

**Change:** Insert new section after `KEY RULES` block, before `PERSISTENT MEMORY`:

```
END-OF-TURN CONTRACT:
Every response either CALLS a tool (work continues) OR provides your final
answer (work is done). Pick exactly one. There is no third option.

- If you state intent ("Let me X", "I'll check Y", "checking now", "I should X"),
  you MUST emit the corresponding tool call in the SAME response. Stating intent
  without calling the tool ends the turn — the user sees your promise and an
  idle bot.
- If you have nothing more to do, just answer the user directly. Do not preface
  with "let me" if there's nothing to do.
- If a tool result was incomplete (e.g. file_read showed [TRUNCATED — showing
  lines X-Y of Z]), call the tool again with adjusted parameters in this same
  turn. Do not promise to retry later.
```

**Why this slot:** `SYSTEM_PROMPT_BODY` is the stable cacheable base. Volatile tail (mode, perpetuum, consciousness) is added per-turn via `request.prepend_system_volatile`/`append_system_volatile` (`runtime.rs:1378-1454`). Adding to the stable base preserves prompt-cache hits.

**Validation gate:** Run mandatory 10-turn CLI smoke (per CLAUDE.md protocol) on the active model. Cost delta must be < 5%, Turn-6 conversation-memory recall must work.

**Risk:** Zero. Pure prompt addition. No code path changes. Cache layout preserved.

---

### PHASE C — Compound-task detection via existing classifier (LOW RISK)

**Files touched:**
- `crates/temm1e-agent/src/runtime.rs:755-1203` — plumb classifier-derived `is_compound` out of the classification block
- `crates/temm1e-agent/src/done_criteria.rs:66` — keep `is_compound_task` as `is_compound_task_fallback`, mark as fallback in doc comment

**Change:**
1. At `runtime.rs:756`, add: `let mut classifier_compound: Option<bool> = None;`
2. Inside the `Order` arm at `runtime.rs:1122-1158`, after determining difficulty: `classifier_compound = Some(classification.difficulty != TaskDifficulty::Simple);`
3. Inside the `Chat` arm: `classifier_compound = Some(false);` (chat is never compound)
4. Inside the `Stop` arm: not reached — early return.
5. In the `Err(e)` rule-based fallback at `runtime.rs:1162-1194`: `classifier_compound = Some(matches!(complexity, TaskComplexity::Standard | TaskComplexity::Complex));`
6. At `runtime.rs:1203`, replace `let is_compound = done_criteria::is_compound_task(&user_text);` with:
   ```rust
   let is_compound = classifier_compound
       .unwrap_or_else(|| done_criteria::is_compound_task_fallback(&user_text));
   ```
7. In `done_criteria.rs`, rename `is_compound_task` → `is_compound_task_fallback`. Existing tests at lines 263-298 still call the function (just under the new name) — update them.

**Why safe:**
- Falls back to current keyword behavior when v2 classifier is disabled (`v2_optimizations = false` config) → identical behavior for that path.
- Single-verb requests like "check server.py for blueprint endpoints" now correctly classify as `Standard` (multi-step) per the classifier prompt at `llm_classifier.rs:79-82`, so they get DONE-criteria scaffolding for the first time.
- Zero added LLM cost — uses existing classifier output.

**Tests added:**
- Existing 6 tests in `done_criteria.rs:263-298` updated to call `is_compound_task_fallback` — must still pass.
- New unit test in `runtime.rs` tests module: `compound_derived_from_classifier_difficulty` using extended MockProvider.

**Risk:** Low. Changes one variable assignment; falls back to current behavior on classifier failure or v2-disabled.

---

### PHASE D — Self-Audit Pass (the actual fix; LOW RISK, feature-flagged)

**Files added:**
- `crates/temm1e-agent/src/self_audit.rs` (new, ~200 lines)

**Files touched:**
- `crates/temm1e-agent/src/lib.rs` — `pub mod self_audit;`
- `crates/temm1e-agent/src/runtime.rs:201-296` — add `self_audit_enabled: bool` field
- `crates/temm1e-agent/src/runtime.rs:298-507` — initialize `self_audit_enabled: false` in `new()` and `with_limits()`; add `with_self_audit_enabled(bool)` builder
- `crates/temm1e-agent/src/runtime.rs:1283-1287` — add turn-local counter `let mut audits_used_this_turn: u8 = 0;`
- `crates/temm1e-agent/src/runtime.rs:2010` — insert audit hook before existing finalization code
- `crates/temm1e-core/src/types/config.rs:819-878` — add `self_audit_enabled: bool` (default `false`) to `AgentConfig`
- `src/main.rs` — wire `cfg.agent.self_audit_enabled` into all 14 `AgentRuntime` construction sites (use grep `with_personality(&personality)` as anchor — same construction sites)

**`self_audit.rs` API:**
```rust
//! Self-Audit Pass — a one-shot verification that catches "stalled promise"
//! turns where a weak model emitted intent text without calling a tool.
//!
//! Per the One Model Rule (feedback_one_model_rule.md), the audit uses the
//! same active provider+model as the main loop. Hard cap of 1 audit per turn
//! bounds cost; fail-open semantics mean any audit error degrades to today's
//! baseline behavior.

use temm1e_core::types::message::{ChatMessage, MessageContent, Role};

pub const AUDIT_MARKER_PREFIX: &str = "[__INTERNAL_AUDIT__]";
pub const AUDIT_DONE_TOKEN: &str = "[DONE]";

/// Build the synthetic user-role message that asks the model to audit its
/// previous turn. Marker prefix lets us identify and filter these later.
pub fn format_audit_message(prev_assistant_text: &str) -> ChatMessage {
    let body = format!(
        "{AUDIT_MARKER_PREFIX}\n\
         Reflect on your last response above. Choose ONE:\n\
         \n\
         A) If you completed the user's request and have nothing more to do, \
            reply with exactly \"{AUDIT_DONE_TOKEN}\" and nothing else. The \
            user will see your previous response, not this one.\n\
         B) If you stated intent but did not call a tool, emit the tool call \
            now. The loop will execute it and continue.\n\
         \n\
         Do NOT explain. Do NOT apologize. Pick A or B."
    );
    let _ = prev_assistant_text; // currently unused; reserved for future quoting
    ChatMessage {
        role: Role::User,
        content: MessageContent::Text(body),
    }
}

/// Classify an audit-round response as DONE / TOOL_CALL_TRIGGERED / FAILED.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditOutcome {
    /// Model confirmed completion with [DONE] marker.
    Done,
    /// Model emitted a tool call — loop should execute it normally.
    ToolCallTriggered,
    /// Audit response was malformed or empty — fail open, treat as Done.
    FailedOpen,
}

pub fn classify_audit_response(text_parts: &[String], had_tool_call: bool) -> AuditOutcome {
    if had_tool_call {
        return AuditOutcome::ToolCallTriggered;
    }
    let combined = text_parts.join("\n");
    let trimmed = combined.trim();
    if trimmed.contains(AUDIT_DONE_TOKEN) {
        AuditOutcome::Done
    } else {
        AuditOutcome::FailedOpen
    }
}

/// Test if a chat message is an internal audit message (for filtering).
pub fn is_audit_message(msg: &ChatMessage) -> bool {
    matches!(&msg.content,
        MessageContent::Text(t) if t.starts_with(AUDIT_MARKER_PREFIX))
}

#[cfg(test)]
mod tests { /* ... */ }
```

**Hook in `runtime.rs:2010`** (replace the bare `if tool_uses.is_empty() {` block):

```rust
if tool_uses.is_empty() {
    // ── Self-Audit gate ────────────────────────────────────────
    // When enabled: if tools were available, the model produced
    // SOME text, and we haven't audited yet, run one audit round.
    // The audit asks the same model to confirm completion or
    // commit to a tool. Fail-open: any malformed result exits
    // normally with the original text.
    let should_audit = self.self_audit_enabled
        && audits_used_this_turn == 0
        && !effective_tools.is_empty()
        && !text_parts.is_empty()
        && !send_message_used
        // Cost cap: skip audit if we'd exceed budget.
        && (self.budget.max_spend_usd() == 0.0
            || self.budget.total_spend_usd() + (turn_cost_usd * 0.2)
                < self.budget.max_spend_usd());

    if should_audit {
        let pre_audit_text = text_parts.join("\n");
        // Record the pre-audit assistant turn so the model has
        // context for what it just said.
        if prompted_mode {
            session.history.push(ChatMessage {
                role: Role::Assistant,
                content: MessageContent::Text(if pre_audit_text.is_empty() {
                    "(no text)".to_string()
                } else {
                    pre_audit_text.clone()
                }),
            });
        } else {
            session.history.push(ChatMessage {
                role: Role::Assistant,
                content: MessageContent::Parts(response.content.clone()),
            });
        }
        // Push the synthetic audit user-role message.
        session.history.push(self_audit::format_audit_message(&pre_audit_text));
        audits_used_this_turn += 1;
        // Save the pre-audit text so the next exit can use it as
        // the user-facing reply (audit response stays internal).
        pending_audit_pre_text = Some(pre_audit_text);
        info!("Self-Audit: triggering one audit round");
        continue;
    }

    // ── existing finalization (status, witness, persistence, return) unchanged ──
    // If we reach here AFTER an audit round, replace text_parts with
    // pending_audit_pre_text so the AUDIT_DONE_TOKEN never reaches the user.
    if let Some(pre_text) = pending_audit_pre_text.take() {
        // The current text_parts should be the audit response (e.g. "[DONE]").
        // Discard it; serve the pre-audit text as the final reply.
        text_parts.clear();
        text_parts.push(pre_text);
        info!("Self-Audit: model returned [DONE]; serving pre-audit text");
    }

    // ... existing code from runtime.rs:2011-2491 unchanged ...
}
```

**Discipline event recording** (Phase E hookup):
At the same audit hook, after deciding the outcome on the second-pass exit, call `self.memory.record_audit_outcome(&self.provider.name(), &self.model, outcome).await` (Phase E adds this method).

**Config:**
```toml
# temm1e.toml — added to [agent] section
self_audit_enabled = false   # v5.6.0 default OFF; flip ON in v5.7.0 after telemetry
```

**Logging anchors** (per parity-gate requirement):
- Init: `info!("Self-Audit Pass enabled (1-audit-per-turn cap)");` when `self_audit_enabled` is true.
- Trigger: `info!("Self-Audit: triggering one audit round");`
- Outcome: `info!(outcome = ?outcome, "Self-Audit: round complete");`

**Tests added** (`crates/temm1e-agent/src/self_audit.rs#tests` + `crates/temm1e-agent/tests/self_audit_integration.rs`):
1. `format_audit_message_includes_marker_and_done_token` — string contents
2. `classify_audit_done_token` — `[DONE]` → `Done`
3. `classify_audit_tool_call` — tool call seen → `ToolCallTriggered`
4. `classify_audit_malformed` — random text → `FailedOpen`
5. `classify_audit_empty` → `FailedOpen`
6. `is_audit_message_filter_works`
7. **Integration**: `audit_catches_stalled_promise_with_qsim_provider`
   - QueuedMockProvider returning: round 1 = text-only "Let me check"; round 2 = tool call to file_read.
   - Expect: loop exits after round 2's tool succeeds, NOT after round 1.
   - Asserts: `provider.calls() == 2` not `== 1`; final reply contains the eventual answer.
8. **Integration**: `audit_done_token_serves_pre_audit_text`
   - QueuedMockProvider returning: round 1 = "Hello, your name is Alice."; round 2 = "[DONE]".
   - Expect: user-facing reply = "Hello, your name is Alice.", NOT "[DONE]".
9. **Integration**: `audit_failed_open_exits_normally`
   - QueuedMockProvider returning: round 1 = text; round 2 = "I dunno what to do".
   - Expect: exit with round 1's text. No infinite loop. Telemetry counter `audit_failed_responses += 1`.
10. **Integration**: `audit_disabled_no_extra_round`
    - `self_audit_enabled = false`. Round 1 = text-only.
    - Expect: `provider.calls() == 1`. Identical to today's behavior.
11. **Integration**: `audit_skipped_when_no_tools_available`
    - `effective_tools` empty (e.g. role-restricted session).
    - Expect: no audit even when enabled.
12. **Integration**: `audit_skipped_when_send_message_used`
    - `send_message_used = true`.
    - Expect: no audit (user already received content).
13. **Integration**: `audit_hard_cap_one_per_turn`
    - QueuedMockProvider returning: round 1 = text; round 2 = text (audit response that didn't say [DONE] AND didn't tool call).
    - Expect: `audits_used_this_turn` capped at 1 → fail-open exit. No loop forever.

**Required MockProvider extension** (`crates/temm1e-test-utils/src/lib.rs`):
```rust
pub struct QueuedMockProvider {
    pub responses: Arc<Mutex<VecDeque<CompletionResponse>>>,
    pub call_count: Arc<Mutex<usize>>,
    pub captured_requests: Arc<Mutex<Vec<CompletionRequest>>>,
}
impl QueuedMockProvider {
    pub fn with_responses(responses: Vec<CompletionResponse>) -> Self { ... }
}
// impl Provider — pop front of queue per call_count++; if queue empty, error.
```

**Risk analysis:**
| Risk | Mitigation |
|---|---|
| Flagship models pay extra round | Default OFF for v5.6.0. Telemetry (Phase E) shows reliability before flipping default ON. |
| Audit message bloats history | ~50 tokens per audit. Pruning at history_pruning.rs handles long-term. |
| `[DONE]` token leaks to user | `pending_audit_pre_text.take()` swap at exit. Test #8 verifies. |
| Loop runs forever | `audits_used_this_turn` capped at 1 at the branch condition. Test #13 verifies. |
| Internal audit message persisted to memory.db | Audit messages survive in `session.history` → JSON-serialized at `main.rs:5664` with marker prefix preserved. Future restoration sees them with marker; `is_audit_message()` filter can be wired in if it becomes a problem. **Decision: leave them in for v5.6.0** — they help the model maintain context across turns. Add explicit filter only if a real issue surfaces. |
| Spend budget bypass | Cost gate in `should_audit` skips audit if it would push over `max_spend_usd`. |
| Witness/learning/blueprint runs only once | The `continue` branch skips finalization. Final exit (post-audit) still runs finalization once, with original text. |
| Cache invalidation | Audit message goes to `session.history` (already-volatile per build_context). System prompt unchanged. Cache stays warm. |

---

### PHASE E — Per-model discipline telemetry (LOW RISK, observability-only)

**Files added:**
- None (logic lives in `self_audit.rs` + `Memory` trait extension)

**Files touched:**
- `crates/temm1e-core/src/traits/memory.rs:177-227` — add 2 new trait methods with default `Ok(())` impls (backward-compatible per existing pattern):
  ```rust
  async fn record_audit_outcome(
      &self,
      _provider: &str,
      _model: &str,
      _outcome: AuditOutcomeKind,  // Done | ToolCallTriggered | FailedOpen | Skipped
      _was_text_only: bool,
  ) -> Result<(), Temm1eError> { Ok(()) }

  async fn get_model_discipline(
      &self,
      _provider: &str,
      _model: &str,
  ) -> Result<Option<ModelDiscipline>, Temm1eError> { Ok(None) }
  ```
- `crates/temm1e-core/src/traits/memory.rs` — add `ModelDiscipline` + `AuditOutcomeKind` types (alongside existing `ToolReliabilityRecord`).
- `crates/temm1e-memory/src/sqlite.rs:130-200` — add new table:
  ```sql
  CREATE TABLE IF NOT EXISTS model_discipline (
      provider TEXT NOT NULL,
      model TEXT NOT NULL,
      text_only_exits INTEGER NOT NULL DEFAULT 0,
      audit_done_responses INTEGER NOT NULL DEFAULT 0,
      audit_tool_call_responses INTEGER NOT NULL DEFAULT 0,
      audit_failed_responses INTEGER NOT NULL DEFAULT 0,
      audit_skipped INTEGER NOT NULL DEFAULT 0,
      last_updated INTEGER NOT NULL,
      PRIMARY KEY (provider, model)
  )
  ```
  Implement `record_audit_outcome` and `get_model_discipline` for SqliteMemory. Other backends (markdown, mock, failover) inherit the default `Ok(())` impl — no breakage.
- `crates/temm1e-agent/src/runtime.rs:2010` — at audit-exit branches, fire-and-forget telemetry call.

**Adaptive auto-disable** (deferred to v5.7.0+ pending data):
After we have v5.6.0 telemetry, decide thresholds. Initial implementation in v5.6.0 is observability-only: counters increment, no behavioral changes triggered by them.

**Tests added:**
- `crates/temm1e-memory/src/sqlite.rs#tests` — `record_audit_outcome_increments_correct_counter`, `get_model_discipline_returns_none_for_unseen`, `concurrent_audit_record_safe`.

**Risk:** Low.
- Pure observability for v5.6.0; no control flow change.
- Default trait impls preserve backward compatibility for non-Sqlite backends.
- `CREATE TABLE IF NOT EXISTS` makes existing dbs safe.

---

## 5. Rollout sequence

| Step | Phase | Default | Validation Before Merge |
|---|---|---|---|
| 1 | Phase A — file_read truncation visibility | always on | new unit tests + workspace `cargo test` clean |
| 2 | Phase B — explicit contract prompt | always on | mandatory 10-turn CLI smoke on active model: cost delta <5%, Turn-6 recall intact |
| 3 | Phase C — classifier-based compound detect | always on (with keyword fallback) | renamed test pass + smoke |
| 4 | Phase E — discipline telemetry | always on (observability-only) | counter persistence test, concurrent-write test |
| 5 | Phase D — self-audit | **OFF by default in v5.6.0**; flip ON in v5.7.0 after telemetry | ALL phase D tests + full release-protocol smoke (CLI/TUI/server parity) + A/B with audit OFF (must show identical behavior) |

All 5 phases ship in **one v5.6.0 release** but Phase D's flag stays OFF. This bundles the ecosystem change (trait method additions) into one version bump while keeping behavior change opt-in.

---

## 6. Mandatory release-protocol gates (per `docs/RELEASE_PROTOCOL.md`)

1. **Compilation gates:**
   ```
   cargo check --workspace
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   cargo fmt --all -- --check
   cargo test --workspace
   ```
2. **Test count must increase** by at least: 4 (Phase A) + 6 (Phase D unit) + 7 (Phase D integration) + 3 (Phase E) = **+20 tests minimum**. Update README test badge.
3. **Multi-turn CLI self-test (mandatory)** — run with active provider (Gemini Flash): all 10 turns get responses, Turn 6 recalls Turn 1, cost ±5% of v5.5.5 baseline.
4. **Parity smoke** — Phase D init log "Self-Audit Pass enabled" must appear in CLI / TUI / server when feature flag is on (test by enabling flag temporarily).
5. **Version bump:** `Cargo.toml` workspace version → `5.6.0`.
6. **README.md updates** per the 14-row table in RELEASE_PROTOCOL.md.
7. **CLAUDE.md crate count** — unchanged (no new crate added; new files live inside existing crates).
8. **Release notes** — explicitly note "Self-Audit defaults to OFF; opt-in via `[agent] self_audit_enabled = true` for weaker tool-calling models like Qwen 27B/35B."

---

## 7. Empirical validation matrix

After implementation, before merge:

| Scenario | Expected Result | How |
|---|---|---|
| Active model (Gemini Flash) — normal multi-turn task | Identical behavior to v5.5.5; no extra rounds | 10-turn smoke; compare provider call count |
| Active model with `self_audit_enabled = true` | <2% of turns trigger audit; all return [DONE] | Same smoke + read `model_discipline` table |
| MockProvider simulating Qwen stalled-promise | Audit catches it; loop continues to tool call | Phase D integration test #7 |
| MockProvider returning [DONE] | Pre-audit text reaches user; "[DONE]" doesn't | Phase D integration test #8 |
| MockProvider returning malformed audit | Fail-open; identical to v5.5.5 baseline | Phase D integration test #9 |
| Cost budget exhausted mid-turn | Audit skipped | Verify via budget gate test |
| Workspace `cargo test --workspace` | All 1312+ existing tests pass + 20 new | CI |
| `is_compound_task_fallback` keyword path | All 6 existing tests pass | Phase C rename test |

---

## 8. What this plan does NOT do (and why)

- **Forced [DONE]/[CONTINUE] sentinel on every turn for all models** — would force flagship models that occasionally forget into infinite loops. Risk too high.
- **Multi-model verifier (e.g. Haiku for audit when on GPT)** — violates the One Model Rule.
- **Keyword/regex stalled-promise detection** — violates `feedback_no_keyword_matching.md`.
- **Synthetic `task_complete` tool** — changes the universal tool surface for one edge case; flagship models may resist; high blast radius.
- **Raising `MAX_PROMPTED_JSON_RETRIES`** — only fires in prompted_mode (provider 400'd on tools). Misses native-tool-calling Qwen.
- **Adaptive `max_tool_rounds`** — orthogonal; doesn't help when loop exits via `tool_uses.is_empty()`.

---

## 9. Confidence statement

**100/0** per `feedback_zero_risk_100_conf.md`:
- Every code location verified by reading the actual current code on the `gh-62/persist-multistep` branch.
- Every existing test interaction mapped (`is_compound_task` callers, MockProvider behavior, history persistence path).
- Every default-impl trait extension verified backward-compatible against `markdown.rs` / `failover.rs` / MockMemory.
- Every risk has a mitigation listed; risks with mitigation = ZERO RISK per the policy.
- Phase D opt-in default for one release cycle = empirical safety net before behavior flip.
- One Model Rule preserved throughout (saved to memory as `feedback_one_model_rule.md`).

Ready to implement on user approval.
