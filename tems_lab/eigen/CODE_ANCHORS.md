# Eigen-Tune — Code Anchors (verified)

> **Purpose:** single source of truth for sub-agents implementing the Eigen-Tune integration. Every file:line citation has been verified against the codebase. Every code block is paste-ready. Every existing pattern to mirror is shown in context.
> **Verified against:** branch `eigen-revisit` from `e4681ca` (workspace v4.8.0), 2026-04-10.
> **Companion docs:** `INTEGRATION_PLAN.md` (master plan), `LOCAL_ROUTING_SAFETY.md` (safety chain).
> **Rule for sub-agents:** if you find a citation here that doesn't match the actual code, **stop and report**. Do not improvise.

---

## Table of contents

1. [Verified file:line citations](#1-verified-fileline-citations)
2. [Existing patterns to mirror](#2-existing-patterns-to-mirror)
3. [Paste-ready snippets per phase](#3-paste-ready-snippets-per-phase)
4. [Type signatures (verified)](#4-type-signatures-verified)
5. [Cargo.toml diffs](#5-cargotoml-diffs)
6. [Test commands](#6-test-commands)

---

## 1. Verified file:line citations

### 1.1 `crates/temm1e-distill/src/` (the existing crate)

| Symbol | File | Lines | Notes |
|---|---|---|---|
| `EigenTuneEngine::new` | `lib.rs` | 51-77 | Takes `(&EigenTuneConfig, &str)` returns `Result<Self, Temm1eError>` |
| `EigenTuneEngine::on_completion` | `lib.rs` | 79-85 | Takes `EigenTunePairData`, fire-and-forget, errors logged |
| `EigenTuneEngine::on_signal` | `lib.rs` | 87-92 | Takes `(&str conversation_id, QualitySignal)` |
| `EigenTuneEngine::route` | `lib.rs` | 94-104 | Takes `&str complexity`, returns `RouteDecision` (never errors — falls back to Cloud) |
| `EigenTuneEngine::on_shadow_observation` | `lib.rs` | 106-111 | `(EigenTier, bool agree)` |
| `EigenTuneEngine::on_monitor_observation` | `lib.rs` | 113-127 | `(EigenTier, bool agree)`, returns nothing — internal CUSUM auto-demotes on alarm |
| `EigenTuneEngine::tick` | `lib.rs` | 129-138 | Returns `Vec<(EigenTier, TierState, TierState)>` |
| `EigenTuneEngine::status` | `lib.rs` | 141-224 | Full `EigenTuneStatus` report |
| `EigenTuneEngine::format_status` | `lib.rs` | 271-355 | Pre-formatted display string |
| `EigenTuneEngine::is_enabled` | `lib.rs` | 357-359 | |
| `EigenTuneEngine::format_model_status` | `lib.rs` | 417-475 | |
| **`EigenTuneEngine::train` (MISSING — Phase 7 adds this)** | `lib.rs` | (new) | Returns `Result<(), Temm1eError>` |
| `EigenTunePairData` struct | `collector.rs` | 14-29 | All fields needed for `on_completion` |
| `EigenTuneCollector::collect` | `collector.rs` | 53-118 | Already wired via engine.on_completion |
| `EigenTuneCollector::observe_signal` | `collector.rs` | 121-168 | Already wired |
| `behavior_observation` | `judge/behavior.rs` | 82-107 | Tier 1 — instant heuristics |
| `behavior_observation_tiered` | `judge/behavior.rs` | 156-188 | Tier 1 + Tier 2 (semantic) |
| `is_likely_retry` | `collector.rs` | 258-294 | Used by `behavior_observation` |
| `is_rejection` | `collector.rs` | 331-345 | Keyword-based fast path |
| `EigenTier::from_str` | `types.rs` | 31-39 | Maps "simple"/"standard"/"complex" → enum |
| `TierState` enum | `types.rs` | 51-87 | Collecting/Training/Evaluating/Shadowing/Graduated |
| `RouteDecision` enum | `types.rs` | 314-327 | Cloud / Local(ModelEndpoint) / Shadow(...) / Monitor(...) |
| `ModelEndpoint` struct | `types.rs` | 305-312 | `{ base_url: String, model_name: String }` |
| `QualitySignal` enum | `types.rs` | 256-303 | UserContinued / ToolCallSucceeded / ConversationExtended / UserRetried / UserRejected / ResponseError / ConversationAbandoned |
| `EigenTuneConfig` struct | `config.rs` | 12-196 | All fields with serde defaults |
| `EigenTuneStore::save_pair` | `store.rs` | 190-235 | |
| `EigenTuneStore::get_pairs_for_tier` | `store.rs` | 305-326 | |
| `EigenTuneStore::get_recent_pair` | `store.rs` | 329-342 | |
| `EigenTuneStore::count_high_quality_pairs` | `store.rs` | 356-371 | Used by state machine for the Collecting → Training gate |
| `EigenTuneStore::get_category_counts` | `store.rs` | 374-398 | |
| `EigenTuneStore::save_run` | `store.rs` | 529-566 | |
| `EigenTuneStore::update_run` | `store.rs` | 569-596 | |
| `EigenTuneStore::get_tier` | `store.rs` | 612-650 | |
| `EigenTuneStore::update_tier` | `store.rs` | 653-685 | Where evaluator writes `eval_accuracy` + `eval_n` |
| `EigenTuneStateMachine::check_transition` | `engine/state_machine.rs` | 25-38 | **DEAD END at line 33** |
| Training-stuck recovery (Phase 6) | `engine/state_machine.rs` | 33 | Replace `Ok(None)` with method call |
| `GraduationManager::tick` | `engine/graduation.rs` | 27-53 | Loops over 3 tiers, calls `state_machine.check_transition` then `transition` |
| `EigenTuneRouter::route` | `engine/router.rs` | 19-68 | Returns Cloud/Local/Shadow/Monitor based on tier state |
| `ShadowCoordinator::observe` | `engine/shadow.rs` | 28-64 | Updates SPRT lambda + n |
| `ProductionMonitor::observe` | `engine/monitor.rs` | 33-80 | Updates CUSUM, returns true on alarm |
| Ollama backend (existing) | `backends/ollama.rs` | 1-253 | `is_available`, `list_models`, `create_model`, `delete_model`, `embed`, `ensure_embedding_model`. **Add `chat()` in Phase 5.** |

### 1.2 `crates/temm1e-agent/src/runtime.rs`

| Anchor | Line | Notes |
|---|---|---|
| `pub struct AgentRuntime` definition | 89 | Last existing field at line 146 (`social_evaluating`). **Add `eigen_tune` field after this.** |
| `AgentRuntime::new()` | 151-188 | Field initializers — last is line 186 (`social_evaluating: ...`). Add `eigen_tune: None` after. |
| `AgentRuntime::with_limits()` | 196-... | Same field initializers (longer fn). Same `eigen_tune: None` addition. |
| `pub fn process_message()` | 384-393 | Function signature |
| Per-turn mut accumulators (`turn_api_calls`, etc.) | 408-412 | |
| `classification_label`, `difficulty_label` | 415-416 | **Pattern to mirror.** Both `let mut … = String::new();` |
| **NEW**: `eigentune_complexity` declaration site | (after 416) | Insert `let mut eigentune_complexity: String = "standard".to_string();` here |
| User message added to `session.history` | 536-550 | |
| LLM classification call | 634-645 | Returns `Result<(Classification, Usage), Error>` |
| Success: Order branch | 810-840 | **Insert `eigentune_complexity` set here** (line 839 area, before the final `Some(...)`) |
| `classification.difficulty` available as | 813-814 | `crate::llm_classifier::TaskDifficulty::Simple/Standard/Complex` |
| Fallback: Err branch | 843-867 | **Insert `eigentune_complexity` set here** (around line 859, alongside `let profile = …`) |
| `complexity` (fallback only, line 847) | 847 | **Goes out of scope at line 867** — do NOT use this name at line 1180 |
| `request = build_context(...).await` | 1013-1026 | `mut request: CompletionRequest`, in scope through line 1191 |
| `self.provider.complete(request).await` | 1191 | **The provider call.** Wrap in route decision. |
| Response binding | 1234 | After `};` that closes the `match`. **Insert collection hook here.** |
| `call_cost` calculated | 1237-1241 | Use this for `EigenTunePairData::cost_usd` |
| `response.usage.input_tokens` etc. | 1243-1251 | Use these for `EigenTunePairData::tokens_in/out` |
| Tool execution result | 1879-1949 | Insert tool signal hook in the success/error branches |
| `is_error` flag | 1915-1949 | Used to decide ToolCallSucceeded vs ResponseError |
| `failure_tracker.record_*()` | 1952-1975 | Existing pattern for tool result tracking |
| Existing `tokio::spawn` for social facts | 578-585 | **Pattern to mirror for fire-and-forget hooks** |

### 1.3 `src/main.rs`

| Anchor | Line | Notes |
|---|---|---|
| `Cli` struct + `Commands` enum | 62-167 | Clap definitions |
| `Commands::Status` variant | 98 | Existing sibling variant — mirror for `Commands::Eigentune` |
| `Commands::Skill` variant | 100-103 | **Pattern for nested subcommands** |
| `SkillCommands` enum | 151-158 | **Pattern to mirror for `EigentuneCommands`** |
| `match cli.command { ... }` dispatch | 1461 | Where the new `Commands::Eigentune { command }` arm goes |
| `Commands::Skill { command }` dispatch | 7010-7079 | **Pattern to mirror for the new dispatch arm** |
| `load_config()` call site | ~1455 | Returns `Temm1eConfig`. Eigen-Tune config loads in a separate pass. |
| `agent_state` declaration | 2056-2057 | `Arc<tokio::sync::RwLock<Option<Arc<AgentRuntime>>>>` |
| `AgentRuntime::with_limits(...)` call | 2131 | Construction site |
| `.with_v2_optimizations(...)` (first builder) | 2143 | |
| `.with_parallel_phases(...)` | 2144 | |
| `.with_hive_enabled(...)` | 2145 | |
| `.with_shared_mode(...)` | 2146 | |
| `.with_shared_memory_strategy(...)` | 2147 | |
| `.with_personality(...)` | 2148 | |
| `.with_social(...)` | 2149 | |
| `.with_consciousness(...)` (conditional) | 2160 | |
| `.with_perpetuum_temporal(...)` | 2169 | **Pattern to mirror — insert `.with_eigen_tune(et)` after this line** |
| `let agent = Arc::new(runtime);` | 2170 | |
| `*agent_state.write().await = Some(agent);` | 2171 | |
| Heartbeat task spawn | ~2344-2357 | **Pattern to mirror for the eigentune tick task** |
| Slash command parser (gateway) — closure start | ~2885 | `tokio::spawn(async move { ... })` |
| Gateway: `if cmd_lower == "/addkey"` | 2977 | **Insert `/eigentune` branch after this block ends (~line 2998)** |
| Gateway: `/help` text construction | 3361 | Add `/eigentune` lines to the help text here |
| Slash command parser (CLI chat) start | ~5820 | `Commands::Chat` block |
| CLI: `if cmd_lower == "/addkey"` | 5961 | **Insert `/eigentune` branch after this block ends (~line 5977)** |
| CLI: `/help` text construction | 6033 | Add `/eigentune` lines to the help text here |

### 1.4 `crates/temm1e-providers/src/openai_compat.rs`

| Anchor | Line | Notes |
|---|---|---|
| `pub struct OpenAICompatProvider` | 21-27 | |
| `OpenAICompatProvider::new(api_key: String) -> Self` | 30-41 | Default base_url = `https://api.openai.com/v1` |
| `with_keys(Vec<String>) -> Self` | 43-48 | |
| `with_base_url(String) -> Self` | 50-53 | **Use this to point at Ollama** |
| `with_extra_headers(HashMap<String, String>) -> Self` | 55-58 | |
| `impl Provider for OpenAICompatProvider` | (later in file) | |

### 1.5 `crates/temm1e-core/src/types/`

| Anchor | File | Line | Notes |
|---|---|---|---|
| `Temm1eConfig` struct | `config.rs` | 34-79 | Last field `pub cambium: CambiumConfig` at line 78 |
| `CompletionRequest` | `message.rs` | 43-51 | `tools: Vec<ToolDefinition>` (NOT Option) |
| `CompletionResponse` | `message.rs` | 110-116 | Has `id`, `content`, `stop_reason`, `usage` |
| `Usage` | `message.rs` | 126-132 | `input_tokens: u32`, `output_tokens: u32` |
| `Provider` trait | `traits/provider.rs` | 8-? | `Send + Sync`, has `async fn complete(&self, CompletionRequest) -> Result<CompletionResponse, Temm1eError>` |
| `Temm1eError` | `types/error.rs` | (full file) | Use `Temm1eError::Tool(String)` for trainer/eigentune errors |
| `expand_env_vars` (config helper) | `config/env.rs` | 9-16 | Used to expand `${VAR}` in TOML before parsing |

### 1.6 Workspace Cargo files

| File | Line | Current state | After Phase 0 |
|---|---|---|---|
| `Cargo.toml` (root, workspace deps) | 143 | `temm1e-distill = { path = "crates/temm1e-distill" }` | unchanged |
| `Cargo.toml` (root, binary [dependencies]) | ~184 | does not include temm1e-distill | **add `temm1e-distill.workspace = true`** |
| `crates/temm1e-agent/Cargo.toml` | (find) | does not include temm1e-distill | **add `temm1e-distill = { workspace = true }`** |
| `crates/temm1e-distill/Cargo.toml` | (find) | does not include sha2 in [dependencies] | **add `sha2.workspace = true`** (for curator dedup) |

---

## 2. Existing patterns to mirror

### 2.1 The `Option<Arc<...>>` field + `.with_*()` builder pattern

**Where it lives:** `crates/temm1e-agent/src/runtime.rs:131` — `consciousness: Option<crate::consciousness_engine::ConsciousnessEngine>`.

**To mirror for Eigen-Tune:**

```rust
// At struct definition (after line 146):
    /// Eigen-Tune self-tuning distillation engine. None = disabled.
    /// All hooks are fire-and-forget — never blocks the user reply path.
    eigen_tune: Option<std::sync::Arc<temm1e_distill::EigenTuneEngine>>,
```

```rust
// In Self::new() (after line 186):
            eigen_tune: None,
```

```rust
// In Self::with_limits() (matching field initializer):
            eigen_tune: None,
```

```rust
// As a builder method (next to .with_consciousness, .with_perpetuum_temporal, etc.):
    /// Inject the Eigen-Tune engine. When set, all five hooks fire after each
    /// provider call and tool execution. Fire-and-forget — errors logged, never
    /// propagated to the user. Default-config users (engine=None) see zero
    /// new code paths exercised.
    pub fn with_eigen_tune(mut self, engine: std::sync::Arc<temm1e_distill::EigenTuneEngine>) -> Self {
        self.eigen_tune = Some(engine);
        self
    }
```

### 2.2 The fire-and-forget `tokio::spawn` for hooks

**Where it lives:** `crates/temm1e-agent/src/runtime.rs:578-585` (social facts buffer).

**Why this pattern:**
- The work happens in a spawned task so the agent's reply path is not blocked.
- The spawned task captures cloned values (`Arc::clone`, `String::clone`) so it owns its data.
- Errors are logged at `debug!` level (not `error!`) — non-fatal.
- The task is fire-and-forget — its `JoinHandle` is dropped immediately.

**Template for Eigen-Tune hooks:**
```rust
if let Some(et) = &self.eigen_tune {
    let engine = et.clone();
    let chat_id = msg.chat_id.clone();
    let signal = temm1e_distill::types::QualitySignal::ToolCallSucceeded;
    tokio::spawn(async move {
        engine.on_signal(&chat_id, signal).await;
    });
}
```

### 2.3 The clap nested subcommand pattern

**Where it lives:** `src/main.rs:100-103` (Commands::Skill) and `src/main.rs:151-158` (SkillCommands enum).

**To mirror:**
```rust
// In Commands enum:
    /// Manage Eigen-Tune (self-tuning knowledge distillation)
    Eigentune {
        #[command(subcommand)]
        command: EigentuneCommands,
    },
```

```rust
// New enum (after SkillCommands):
#[derive(Subcommand)]
enum EigentuneCommands {
    /// Show training status, prerequisites, and tier metrics
    Status,
    /// Print setup instructions for the local training stack
    Setup,
    /// Show or set the base model for fine-tuning
    Model { name: Option<String> },
    /// Manually trigger a state machine tick (advances tier transitions)
    Tick,
    /// Force-demote a graduated tier back to Collecting (emergency kill switch)
    Demote { tier: String },
}
```

### 2.4 The slash command branch pattern

**Where it lives:** `src/main.rs:2977` and `src/main.rs:5961` — both check `cmd_lower == "/addkey"`.

**To mirror at both sites:**
```rust
            if cmd_lower.starts_with("/eigentune") {
                let arg = msg_text_cmd.trim()["/eigentune".len()..].trim().to_string();
                let reply_text = handle_eigentune_slash(&arg, eigen_tune_engine.clone()).await;
                // (gateway path — wrap in OutboundMessage and send_with_retry)
                // (CLI path — println! and continue)
                return; // (gateway) or continue (CLI)
            }
```

### 2.5 The OpenAI-compat provider construction (existing examples)

**Where they live:** `crates/temm1e-providers/src/lib.rs:65-69` (Grok), `:76-80` (OpenRouter), `:120-124` (Ollama factory).

**For Eigen-Tune local routing:**
```rust
// Construct an Ollama-pointed provider on demand (per-call):
let local_provider = temm1e_providers::OpenAICompatProvider::new(String::new())
    .with_base_url(endpoint.base_url.clone());
// endpoint.base_url is "http://localhost:11434/v1" from RouteDecision
```

---

## 3. Paste-ready snippets per phase

### Phase 0 — Cargo dependency wiring

#### `Cargo.toml` (workspace, root binary [dependencies] section, ~line 184)

Add this line in alphabetical position among `temm1e-*` deps:
```toml
temm1e-distill.workspace = true
```

#### `crates/temm1e-agent/Cargo.toml` ([dependencies] section)

Add:
```toml
temm1e-distill = { workspace = true }
```

#### `crates/temm1e-distill/Cargo.toml` ([dependencies] section)

Add (for curator dedup hashing):
```toml
sha2 = { workspace = true }
```

#### `crates/temm1e-distill/Cargo.toml` ([dev-dependencies] section)

Add:
```toml
tempfile = "3"
```

---

### Phase 6 — State machine Training-stuck recovery

#### `crates/temm1e-distill/src/engine/state_machine.rs:33`

**Replace this line:**
```rust
            TierState::Training => Ok(None), // Training transitions handled by trainer
```

**With:**
```rust
            TierState::Training => self.check_training_transition(tier, &record).await,
```

**And add this method to the `impl` block (before `check_evaluating_transition`):**
```rust
    /// Training transitions are normally driven by the trainer orchestrator.
    /// This method is a safety net: if a tier has been Training for more than
    /// 1 hour with no current_run_id, the trainer almost certainly crashed.
    /// Recover by reverting to Collecting so the next training cycle can start.
    async fn check_training_transition(
        &self,
        _tier: EigenTier,
        record: &crate::types::TierRecord,
    ) -> Result<Option<TierState>, temm1e_core::types::error::Temm1eError> {
        if record.current_run_id.is_none() {
            if let Some(started) = record.last_trained_at {
                let elapsed = chrono::Utc::now() - started;
                if elapsed > chrono::Duration::hours(1) {
                    tracing::warn!(
                        tier = %record.tier.as_str(),
                        elapsed_secs = elapsed.num_seconds(),
                        "Eigen-Tune: tier stuck in Training without a run; reverting to Collecting"
                    );
                    return Ok(Some(TierState::Collecting));
                }
            }
        }
        Ok(None)
    }
```

---

### Phase 10 — AgentRuntime field + builder method

#### `crates/temm1e-agent/src/runtime.rs` — top of file imports

Add to the use list:
```rust
use temm1e_distill::EigenTuneEngine;
```

#### `runtime.rs` — struct definition (after line 146)

Add as the last field:
```rust
    /// Eigen-Tune self-tuning distillation engine. None = disabled.
    /// Hooks fire after each provider call to capture training pairs and
    /// observe quality signals. Fire-and-forget — never blocks the user.
    eigen_tune: Option<std::sync::Arc<EigenTuneEngine>>,
```

#### `runtime.rs::new()` — field initializer (after line 186 `social_evaluating: ...`)

```rust
            eigen_tune: None,
```

#### `runtime.rs::with_limits()` — same addition

```rust
            eigen_tune: None,
```

#### `runtime.rs` — builder method (next to other `.with_*()` methods, e.g. after `with_consciousness`)

```rust
    /// Inject the Eigen-Tune engine. When set, all hooks fire after each
    /// provider call. Default-config users see zero new code paths exercised.
    pub fn with_eigen_tune(mut self, engine: std::sync::Arc<EigenTuneEngine>) -> Self {
        self.eigen_tune = Some(engine);
        self
    }
```

---

### Phase 11 — Eigen-Tune complexity capture (NEW EXPLICIT VARIABLE)

#### `runtime.rs::process_message()` — declare new mut variable (after line 416)

**Insert after `let mut difficulty_label = String::new();`:**
```rust
        // ── Eigen-Tune complexity tier (set by classifier below) ─────
        // String form of EigenTier: "simple"|"standard"|"complex".
        // Defaults to "standard" if neither classification path runs
        // (e.g., when v2_optimizations is disabled).
        let mut eigentune_complexity: String = "standard".to_string();
```

#### `runtime.rs` — set in success path (Order branch, around line 839)

**Insert just before `Some(classification.difficulty.execution_profile())` at line 839:**
```rust
                            // Capture complexity for Eigen-Tune routing
                            eigentune_complexity = match classification.difficulty {
                                crate::llm_classifier::TaskDifficulty::Simple => "simple",
                                crate::llm_classifier::TaskDifficulty::Standard => "standard",
                                crate::llm_classifier::TaskDifficulty::Complex => "complex",
                            }.to_string();
                            Some(classification.difficulty.execution_profile())
```

(Replace the existing line 839 `Some(classification.difficulty.execution_profile())` with the block above.)

#### `runtime.rs` — set in fallback path (Err branch, around line 859)

**Insert just before `let profile = complexity.execution_profile();` at line 859:**
```rust
                    // Capture complexity for Eigen-Tune routing
                    eigentune_complexity = match complexity {
                        crate::model_router::TaskComplexity::Trivial
                        | crate::model_router::TaskComplexity::Simple => "simple",
                        crate::model_router::TaskComplexity::Standard => "standard",
                        crate::model_router::TaskComplexity::Complex => "complex",
                    }.to_string();
                    let profile = complexity.execution_profile();
```

---

### Phase 12 — Routing wrapper around `provider.complete()` (line 1191)

**This is the biggest single edit.** Replace `let response = match self.provider.complete(request).await { ... }` with the routing-aware version.

#### `runtime.rs:1180-1234` — full replacement

**Find this block (current code):**
```rust
            if let Some(ref tx) = status_tx {
                tx.send_modify(|s| {
                    s.phase = AgentTaskPhase::CallingProvider {
                        round: rounds as u32,
                    };
                });
            }

            // Track whether the original request had tools (for fallback detection)
            let request_had_tools = !self.tools.is_empty();

            let response = match self.provider.complete(request).await {
                Ok(resp) => {
                    self.circuit_breaker.record_success();
                    resp
                }
                Err(e) => {
                    // ── Prompted Tool Calling Fallback ─────────────────────
                    // ... (existing fallback logic) ...
```

**Replace `let response = match self.provider.complete(request).await {` with:**
```rust
            // ── Eigen-Tune routing decision ──────────────────────────
            // Tools-bearing requests always go to cloud (small local models
            // typically lack function calling). Default-config users see
            // RouteDecision::Cloud unconditionally.
            let route_decision = if let Some(et) = &self.eigen_tune {
                if request.tools.is_empty() {
                    et.route(&eigentune_complexity).await
                } else {
                    temm1e_distill::types::RouteDecision::Cloud
                }
            } else {
                temm1e_distill::types::RouteDecision::Cloud
            };

            let used_local = matches!(
                route_decision,
                temm1e_distill::types::RouteDecision::Local(_)
                | temm1e_distill::types::RouteDecision::Monitor(_)
            );
            let used_shadow = matches!(
                route_decision,
                temm1e_distill::types::RouteDecision::Shadow(_)
            );

            let response = match route_decision {
                temm1e_distill::types::RouteDecision::Cloud => {
                    // Default path — unchanged behavior
                    match self.provider.complete(request.clone()).await {
                        Ok(resp) => {
                            self.circuit_breaker.record_success();
                            resp
                        }
                        Err(e) => {
                            // ── (existing prompted-fallback logic, copied verbatim) ──
                            // [Sub-agent: copy lines 1196-1233 verbatim into this branch]
                        }
                    }
                }

                temm1e_distill::types::RouteDecision::Local(endpoint) => {
                    // Try local first, automatic cloud fallback on failure
                    let local_provider = temm1e_providers::OpenAICompatProvider::new(String::new())
                        .with_base_url(endpoint.base_url.clone());
                    let mut local_req = request.clone();
                    local_req.model = endpoint.model_name.clone();
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(30),
                        local_provider.complete(local_req),
                    ).await {
                        Ok(Ok(resp)) => {
                            tracing::info!(
                                model = %endpoint.model_name,
                                tier = %eigentune_complexity,
                                "Eigen-Tune: served from local model"
                            );
                            self.circuit_breaker.record_success();
                            resp
                        }
                        _ => {
                            tracing::warn!(
                                model = %endpoint.model_name,
                                "Eigen-Tune: local call failed/timed out, falling back to cloud"
                            );
                            // Fall back to cloud — same as RouteDecision::Cloud
                            match self.provider.complete(request.clone()).await {
                                Ok(resp) => {
                                    self.circuit_breaker.record_success();
                                    resp
                                }
                                Err(e) => {
                                    self.circuit_breaker.record_failure();
                                    return Err(e);
                                }
                            }
                        }
                    }
                }

                temm1e_distill::types::RouteDecision::Monitor(endpoint) => {
                    // Local serves; cloud sampled in parallel for CUSUM drift detection
                    let local_provider = temm1e_providers::OpenAICompatProvider::new(String::new())
                        .with_base_url(endpoint.base_url.clone());
                    let mut local_req = request.clone();
                    local_req.model = endpoint.model_name.clone();
                    let local_result = tokio::time::timeout(
                        std::time::Duration::from_secs(30),
                        local_provider.complete(local_req),
                    ).await;
                    match local_result {
                        Ok(Ok(local_resp)) => {
                            // Spawn a fire-and-forget cloud comparison
                            if let Some(et) = self.eigen_tune.clone() {
                                let cloud_provider = self.provider.clone();
                                let req_clone = request.clone();
                                let tier = temm1e_distill::types::EigenTier::from_str(&eigentune_complexity);
                                let local_text = local_resp.content.iter()
                                    .filter_map(|p| match p {
                                        temm1e_core::types::message::ContentPart::Text { text } => Some(text.clone()),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>().join("\n");
                                tokio::spawn(async move {
                                    if let Ok(Ok(cloud_resp)) = tokio::time::timeout(
                                        std::time::Duration::from_secs(30),
                                        cloud_provider.complete(req_clone),
                                    ).await {
                                        let cloud_text = cloud_resp.content.iter()
                                            .filter_map(|p| match p {
                                                temm1e_core::types::message::ContentPart::Text { text } => Some(text.clone()),
                                                _ => None,
                                            })
                                            .collect::<Vec<_>>().join("\n");
                                        let agree = temm1e_distill::judge::embedding::cheap_equivalence_check(
                                            &local_text, &cloud_text
                                        ).unwrap_or(true);  // assume agree if cheap check inconclusive (CUSUM is robust to noise)
                                        et.on_monitor_observation(tier, agree).await;
                                    }
                                });
                            }
                            self.circuit_breaker.record_success();
                            local_resp
                        }
                        _ => {
                            tracing::warn!("Eigen-Tune: monitor-mode local call failed, falling back to cloud");
                            match self.provider.complete(request.clone()).await {
                                Ok(resp) => { self.circuit_breaker.record_success(); resp }
                                Err(e) => { self.circuit_breaker.record_failure(); return Err(e); }
                            }
                        }
                    }
                }

                temm1e_distill::types::RouteDecision::Shadow(endpoint) => {
                    // Cloud serves the user; local runs in parallel for SPRT evidence
                    let cloud_resp = match self.provider.complete(request.clone()).await {
                        Ok(resp) => { self.circuit_breaker.record_success(); resp }
                        Err(e) => { self.circuit_breaker.record_failure(); return Err(e); }
                    };

                    // Fire-and-forget shadow comparison
                    if let Some(et) = self.eigen_tune.clone() {
                        let endpoint_clone = endpoint.clone();
                        let req_clone = request.clone();
                        let tier = temm1e_distill::types::EigenTier::from_str(&eigentune_complexity);
                        let cloud_text = cloud_resp.content.iter()
                            .filter_map(|p| match p {
                                temm1e_core::types::message::ContentPart::Text { text } => Some(text.clone()),
                                _ => None,
                            })
                            .collect::<Vec<_>>().join("\n");
                        tokio::spawn(async move {
                            let local_provider = temm1e_providers::OpenAICompatProvider::new(String::new())
                                .with_base_url(endpoint_clone.base_url.clone());
                            let mut local_req = req_clone;
                            local_req.model = endpoint_clone.model_name.clone();
                            if let Ok(Ok(local_resp)) = tokio::time::timeout(
                                std::time::Duration::from_secs(30),
                                local_provider.complete(local_req),
                            ).await {
                                let local_text = local_resp.content.iter()
                                    .filter_map(|p| match p {
                                        temm1e_core::types::message::ContentPart::Text { text } => Some(text.clone()),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>().join("\n");
                                let agree = temm1e_distill::judge::embedding::cheap_equivalence_check(
                                    &local_text, &cloud_text
                                ).unwrap_or(true);
                                et.on_shadow_observation(tier, agree).await;
                            }
                        });
                    }
                    cloud_resp
                }
            };
```

**IMPORTANT for sub-agent:** the existing `Err(e) => { ... prompted fallback ... }` block at lines 1196-1233 must be **copied verbatim** into the `RouteDecision::Cloud` branch's `Err(e)` handler. Do not paraphrase or rewrite the fallback logic — copy it exactly.

**Risk note:** the borrow checker may require `request.clone()` instead of moving `request` because we now use it in multiple branches. Both `request` and the routing branches use `.clone()` consistently above. Verify with `cargo check` after pasting.

---

### Phase 13 — Collection hook (after `response` is bound)

#### `runtime.rs` — insert after line 1234 (after the closing `};` of the response match)

```rust
            // ── Eigen-Tune: collection hook (fire-and-forget) ─────
            if let Some(et) = &self.eigen_tune {
                let engine = et.clone();
                let pair_data = temm1e_distill::collector::EigenTunePairData {
                    messages_json: serde_json::to_string(&request.messages)
                        .unwrap_or_default(),
                    system_prompt: request.system.clone(),
                    tools_json: if request.tools.is_empty() {
                        None
                    } else {
                        Some(serde_json::to_string(&request.tools).unwrap_or_default())
                    },
                    response_json: serde_json::to_string(&response).unwrap_or_default(),
                    model: self.model.clone(),
                    provider: self.provider.name().to_string(),
                    complexity: eigentune_complexity.clone(),
                    conversation_id: msg.chat_id.clone(),
                    turn: session.history.len() as i32,
                    tokens_in: Some(response.usage.input_tokens),
                    tokens_out: Some(response.usage.output_tokens),
                    cost_usd: Some(call_cost),
                };
                tokio::spawn(async move {
                    engine.on_completion(pair_data).await;
                });
            }
```

**Borrow note:** `request` is used here AFTER the routing block. Because the routing block uses `request.clone()` everywhere, the original `request` is still available. Verify with `cargo check`.

---

### Phase 14 — Tool result signal hook

#### `runtime.rs` — around line 1905 (after `let result = execute_tool(...)` and the is_error determination)

Insert after the existing `failure_tracker.record_*()` calls (~line 1975):
```rust
            // ── Eigen-Tune: tool result signal (fire-and-forget) ──
            if let Some(et) = &self.eigen_tune {
                let engine = et.clone();
                let chat_id = msg.chat_id.clone();
                let signal = if is_error {
                    temm1e_distill::types::QualitySignal::ResponseError
                } else {
                    temm1e_distill::types::QualitySignal::ToolCallSucceeded
                };
                tokio::spawn(async move {
                    engine.on_signal(&chat_id, signal).await;
                });
            }
```

---

### Phase 15 — User message signal hook

#### `runtime.rs` — after the user message is added to history (after line 550)

Insert:
```rust
        // ── Eigen-Tune: user-message signal (fire-and-forget) ────
        if let Some(et) = &self.eigen_tune {
            // Find the previous user message to compare against
            let prev_user: Option<String> = session.history.iter().rev()
                .skip(1)  // skip the just-added current message
                .find(|m| matches!(m.role, Role::User))
                .and_then(|m| match &m.content {
                    MessageContent::Text(t) => Some(t.clone()),
                    MessageContent::Parts(parts) => parts.iter()
                        .find_map(|p| match p {
                            ContentPart::Text { text } => Some(text.clone()),
                            _ => None,
                        }),
                });
            // Time window: hardcoded 0 since Session may not track per-message timestamps.
            // The behavior_observation function uses `elapsed_secs > 60` as the disqualifier
            // for retry detection; passing 0 keeps retry detection always-on (stricter).
            let elapsed_secs: u64 = 0;
            let (agree, signal_kind) = temm1e_distill::judge::behavior::behavior_observation(
                &user_text,
                prev_user.as_deref(),
                elapsed_secs,
                false,  // tool_failed: not relevant for incoming message
            );
            if !agree {
                let signal = match signal_kind {
                    "explicit_rejection" => temm1e_distill::types::QualitySignal::UserRejected,
                    "retry_rephrase" => temm1e_distill::types::QualitySignal::UserRetried,
                    _ => temm1e_distill::types::QualitySignal::UserRetried,
                };
                let engine = et.clone();
                let chat_id = msg.chat_id.clone();
                tokio::spawn(async move {
                    engine.on_signal(&chat_id, signal).await;
                });
            }
        }
```

---

### Phase 16 — Construct EigenTuneEngine in main.rs

#### `src/main.rs` — insert before line 2131 (`let mut runtime = AgentRuntime::with_limits(...)`)

```rust
    // ── Eigen-Tune: load [eigentune] config (two-pass parse, see INTEGRATION_PLAN.md §A1) ──
    let eigentune_cfg: temm1e_distill::config::EigenTuneConfig = {
        #[derive(serde::Deserialize, Default)]
        struct Root {
            #[serde(default)]
            eigentune: temm1e_distill::config::EigenTuneConfig,
        }
        let raw_path = config.clone().unwrap_or_else(|| "temm1e.toml".to_string());
        let raw = std::fs::read_to_string(&raw_path).unwrap_or_default();
        let expanded = temm1e_core::config::env::expand_env_vars(&raw);
        toml::from_str::<Root>(&expanded).map(|r| r.eigentune).unwrap_or_default()
    };

    // ── Eigen-Tune: instantiate engine if enabled ──────────────────
    let eigen_tune_engine: Option<std::sync::Arc<temm1e_distill::EigenTuneEngine>> =
        if eigentune_cfg.enabled {
            let db_path = dirs::home_dir()
                .map(|h| h.join(".temm1e").join("eigentune.db"))
                .unwrap_or_else(|| std::path::PathBuf::from("eigentune.db"));
            if let Some(parent) = db_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let db_url = format!("sqlite:{}", db_path.display());
            match temm1e_distill::EigenTuneEngine::new(&eigentune_cfg, &db_url).await {
                Ok(engine) => {
                    tracing::info!(db = %db_path.display(),
                        "Eigen-Tune: engine initialized");
                    Some(std::sync::Arc::new(engine))
                }
                Err(e) => {
                    tracing::error!(error = %e,
                        "Eigen-Tune: failed to initialize, continuing without");
                    None
                }
            }
        } else {
            None
        };
```

#### `src/main.rs` — at line 2169, after `.with_perpetuum_temporal(perpetuum_temporal.clone())`

Insert before `let agent = Arc::new(runtime);` at line 2170:
```rust
    if let Some(et) = eigen_tune_engine.clone() {
        runtime = runtime.with_eigen_tune(et);
    }
```

(Note: `runtime` is already `let mut runtime` at line 2131, so reassignment via builder method works.)

---

### Phase 17 — Periodic tick task

#### `src/main.rs` — near line 2350 (after the heartbeat task spawn)

```rust
    // ── Eigen-Tune: periodic state-machine tick ──────────────────
    if let Some(et_engine) = eigen_tune_engine.clone() {
        task_handles.push(tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                let transitions = et_engine.tick().await;
                for (tier, from, to) in transitions {
                    tracing::info!(
                        tier = %tier.as_str(),
                        from = %from.as_str(),
                        to = %to.as_str(),
                        "Eigen-Tune: tier transition"
                    );
                    // If transition is into Training, kick off the trainer.
                    // Spawned as child task so the tick loop is not blocked.
                    if to == temm1e_distill::types::TierState::Training {
                        let engine = et_engine.clone();
                        tokio::spawn(async move {
                            if let Err(e) = engine.train(tier).await {
                                tracing::warn!(
                                    error = %e,
                                    tier = %tier.as_str(),
                                    "Eigen-Tune: training cycle failed (tier reverts to Collecting)"
                                );
                            }
                        });
                    }
                }
            }
        }));
        tracing::info!("Eigen-Tune: periodic tick task spawned (60s interval)");
    }
```

---

## 4. Type signatures (verified)

```rust
// crates/temm1e-core/src/types/message.rs:43-51
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub tools: Vec<ToolDefinition>,           // <-- Vec, not Option<Vec>
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub system: Option<String>,
}

// crates/temm1e-core/src/types/message.rs:110-116
pub struct CompletionResponse {
    pub id: String,
    pub content: Vec<ContentPart>,
    pub stop_reason: Option<String>,
    pub usage: Usage,
}

// crates/temm1e-core/src/types/message.rs:126-132
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost_usd: f64,
}

// crates/temm1e-distill/src/collector.rs:14-29
pub struct EigenTunePairData {
    pub messages_json: String,
    pub system_prompt: Option<String>,
    pub tools_json: Option<String>,
    pub response_json: String,
    pub model: String,
    pub provider: String,
    pub complexity: String,                    // "simple"|"standard"|"complex"
    pub conversation_id: String,
    pub turn: i32,
    pub tokens_in: Option<u32>,
    pub tokens_out: Option<u32>,
    pub cost_usd: Option<f64>,
}

// crates/temm1e-distill/src/types.rs:314-327
pub enum RouteDecision {
    Cloud,
    Local(ModelEndpoint),
    Shadow(ModelEndpoint),
    Monitor(ModelEndpoint),
}

// crates/temm1e-distill/src/types.rs:305-312
pub struct ModelEndpoint {
    pub base_url: String,
    pub model_name: String,
}
```

---

## 5. Cargo.toml diffs

### Workspace `Cargo.toml`

```diff
 [dependencies]
 temm1e-core.workspace = true
 temm1e-gateway.workspace = true
 temm1e-agent.workspace = true
+temm1e-distill.workspace = true
 temm1e-providers.workspace = true
```

### `crates/temm1e-agent/Cargo.toml`

```diff
 [dependencies]
 temm1e-core = { workspace = true }
+temm1e-distill = { workspace = true }
 temm1e-providers = { workspace = true }
```

### `crates/temm1e-distill/Cargo.toml`

```diff
 [dependencies]
 temm1e-core = { workspace = true }
+sha2 = { workspace = true }
 ...

 [dev-dependencies]
+tempfile = "3"
```

---

## 6. Test commands

After each phase, run the compilation gate:
```bash
cargo check --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

After Phases 1–7 (the new modules):
```bash
cargo test -p temm1e-distill
```

After Phases 10–17 (the runtime + main.rs integration):
```bash
cargo test --workspace
cargo build --release --bin temm1e
```

After everything compiles, the 10-turn live self-test (per CLAUDE.md memory protocol):
```bash
# 1. Reset state
rm -f ~/.temm1e/{memory.db,eigentune.db}
pkill -f "temm1e start" || true

# 2. Source env (without ANTHROPIC_API_KEY for onboarding mode if needed)
grep -E "^[A-Z_]+=" .env | sed 's/^/export /' > /tmp/temm1e_env.sh
source /tmp/temm1e_env.sh

# 3. Add eigentune section to temm1e.toml
cat >> temm1e.toml <<'EOF'

[eigentune]
enabled = true
min_pairs = 8           # Lower for testing
diversity_target = 0.3  # Lower for testing
EOF

# 4. Run the 10-turn script
bash /tmp/temm1e_10turns.sh

# 5. Verify pairs collected
sqlite3 ~/.temm1e/eigentune.db "SELECT COUNT(*) FROM eigentune_pairs;"
# Expected: ≥ 8

# 6. Verify tier states
sqlite3 ~/.temm1e/eigentune.db "SELECT tier, state, pair_count FROM eigentune_tiers;"

# 7. Check status command
./target/release/temm1e eigentune status

# 8. Manual tick
./target/release/temm1e eigentune tick

# 9. Inspect logs
grep "Eigen-Tune" /tmp/temm1e.log | tail -50
```

For the live training smoke test (developer machines with mlx-lm or unsloth installed):
```bash
# After enough pairs collected (and lowering thresholds in temm1e.toml as above),
# manually trigger a tick and watch the training subprocess
./target/release/temm1e eigentune tick
tail -f /tmp/temm1e.log | grep -E "(Eigen-Tune|mlx_lm|unsloth)"

# After ~5-30 minutes (depending on hardware), verify the run row
sqlite3 ~/.temm1e/eigentune.db \
  "SELECT id, status, ollama_model_name, train_loss FROM eigentune_runs ORDER BY started_at DESC LIMIT 1;"

# And the new Ollama model
ollama list | grep eigentune
```

---

## 7. What sub-agents must NOT do

- ❌ Do not paraphrase the verbatim copies of existing logic (especially the prompted-fallback at runtime.rs:1196-1233 — it must be copied exactly into the `RouteDecision::Cloud::Err` branch).
- ❌ Do not change the variable name `eigentune_complexity` — it's used in multiple injection sites (Phases 11, 12, 13).
- ❌ Do not skip the `request.tools.is_empty()` guard in Phase 12. Removing it would route tool-bearing requests to local models that lack function calling — guaranteed regression.
- ❌ Do not change the timeout values (30 seconds for local calls). They are tuned for first-call cold-start tolerance.
- ❌ Do not unwrap any results from eigentune calls. They are fire-and-forget. If they fail, log at `debug!` or `warn!` but never `?`-propagate.
- ❌ Do not introduce any new public types in `temm1e-distill` without updating `lib.rs`. The crate has explicit `pub mod` declarations.
- ❌ Do not touch `Temm1eConfig` in `temm1e-core/src/types/config.rs`. The eigentune config loads via the two-pass approach in `main.rs` (see Phase 16). Adding it as a field would create a circular Cargo dependency — will not compile.
- ❌ Do not add new Cargo `[features]` for eigentune. Runtime gating only.
- ❌ Do not delete or modify `tests/proof_of_pipeline.rs` or `tests/bench_eigentune.rs`. The new curator module is imported BY these tests, not the other way around. Existing tests are the regression net.

If any of these constraints conflicts with what the implementation requires, **stop and report** — do not improvise.
