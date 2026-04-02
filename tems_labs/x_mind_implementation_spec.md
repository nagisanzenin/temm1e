# X-Mind Implementation Specification

**Tem's Labs — Technical Specification TL-2026-001-IMPL**
**Date:** 2026-04-02

---

## 1. New Files

### 1.1 `crates/temm1e-agent/src/x_mind.rs`
Core types and traits for the X-Mind system.

```rust
// Types:
- XMindKind          — enum of built-in mind types (Architect, Analyst, Sentinel)
- XMindConfig        — per-mind configuration (enabled, cadence, token_budget)
- XMindsConfig       — top-level config for the entire X-Mind system
- MindArtifact       — persistent output of a mind observation
- MindObservation    — input context for a mind's observe() call
- MindInjection      — what gets injected into the system prompt
- XMindTrait         — trait that all minds implement
```

### 1.2 `crates/temm1e-agent/src/x_mind_engine.rs`
The orchestrator and built-in mind implementations.

```rust
// Types:
- XMindOrchestrator  — manages multiple minds, runs them concurrently
- ArchitectMind      — implements XMindTrait for architecture observation
- AnalystMind        — implements XMindTrait for analytical reasoning
- SentinelMind       — implements XMindTrait for safety monitoring

// Key methods:
- XMindOrchestrator::pre_observe()  — run relevant minds before LLM call
- XMindOrchestrator::post_update()  — update mind artifacts after LLM call
- XMindOrchestrator::build_injection() — synthesize {x_minds} block
```

## 2. Modified Files

### 2.1 `crates/temm1e-agent/src/lib.rs`
Add module declarations:
```rust
pub mod x_mind;
pub mod x_mind_engine;
```

### 2.2 `crates/temm1e-agent/src/runtime.rs`
- Add `x_mind_orchestrator: Option<XMindOrchestrator>` field
- Add `with_x_minds()` builder method
- Insert X-Mind pre-observation before consciousness pre-observation
- Insert X-Mind post-update after consciousness post-observation
- Pass X-Mind artifacts to consciousness for its own observation

### 2.3 `crates/temm1e-core/src/types/config.rs`
- Add `XMindsConfig` and `XMindConfig` structs
- Add `x_minds: XMindsConfig` field to `Temm1eConfig`

### 2.4 `src/main.rs`
- Initialize `XMindOrchestrator` when `config.x_minds.enabled`
- Wire into `AgentRuntime` via `with_x_minds()`

### 2.5 `config/default.toml`
Add default X-Mind configuration section.

## 3. Artifact Storage

Location: `~/.temm1e/x_minds/{mind_name}/artifact.json`

Loaded on orchestrator creation, saved after post_update when changed.

## 4. Token Budget

Default total budget: 500 tokens for all X-Mind injections combined.
Each mind's injection is capped at its share of the budget.
Orchestrator enforces the cap by truncating lower-priority injections.

## 5. Concurrency

All active minds run via `tokio::join!` with individual 10-second timeouts.
On timeout, the mind's previous artifact is used without update.

## 6. Testing Strategy

- Unit tests for all types and serialization
- Unit tests for orchestrator logic (mind selection, injection building)
- Integration test with mock provider (verify injection format)
- CLI self-test: 10-turn conversation with gemini-3-flash-preview
  - Baseline run: consciousness ON, X-Mind OFF
  - Treatment run: consciousness ON, X-Mind ON
  - Compare: tokens, cost, response quality, error count
