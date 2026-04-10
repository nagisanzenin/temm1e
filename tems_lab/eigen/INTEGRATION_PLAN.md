# Eigen-Tune — Integration & Completion Plan

> **Status:** design — supersedes the unimplemented Phase 10 of `IMPLEMENTATION.md`.
> **Verified against codebase:** branch `eigen-revisit` from `e4681ca` (workspace v4.8.0), 2026-04-10.
> **Tracks:** issue #35 (`temm1e-labs/temm1e`) — *"Eigen-Tune: feature advertised as functional but pipeline is incomplete and not wired into the runtime."*
> **Scope:** complete every missing module, wire the engine into the runtime, expose CLI + slash commands, **serve users with the distilled local model after the statistical gates pass**, align documentation. **Zero behavior change for any user who does not opt in.**
> **Goal:** every phase classified ZERO risk (purely additive or only active under explicit double opt-in `enabled = true` AND `enable_local_routing = true`); the local routing path is gated by the seven-gate safety chain documented in `LOCAL_ROUTING_SAFETY.md`.
>
> **Companion documents (read in this order):**
> 1. **`CODE_ANCHORS.md`** — verified file:line citations + paste-ready code snippets per phase. Sub-agents reference this instead of re-researching.
> 2. **`LOCAL_ROUTING_SAFETY.md`** — the seven-gate safety chain protecting local serving. Every gate has an enforcement point and a recovery path.
> 3. **This file (`INTEGRATION_PLAN.md`)** — phased master plan, scenario matrix, security audit, risk summary.

---

## 0. TL;DR

Eigen-Tune ships ~7 900 LOC of statistical machinery, SQLite storage, and a per-tier state machine that all work in isolation (1 405 LOC of integration tests pass). What's missing is the half of the pipeline that turns collected pairs into a deployed local model, plus the entire integration with the agent runtime. The state machine has a literal dead-end at `state_machine.rs:33` (`TierState::Training => Ok(None) // handled by trainer` — no trainer exists), the binary has zero `use temm1e_distill` imports anywhere, and the user-facing setup guide describes a feature that produces zero observable effect today.

This plan completes the missing modules (`curator.rs`, `engine/trainer.rs`, `engine/evaluator.rs`, `backends/mlx.rs`, `backends/unsloth.rs`), wires the existing `EigenTuneEngine` into `crates/temm1e-agent/src/runtime.rs:1191` and `src/main.rs:2131`, **adds local routing so the distilled model actually serves users after the statistical gates pass**, exposes the engine via a clap subcommand and dual-path slash command (`/eigentune`), and updates SETUP.md/README.md/CLAUDE.md so the public claims match shipped behavior.

**Local routing IS in scope** — earlier drafts of this plan deferred it as MEDIUM risk, but with the seven-gate safety chain documented in `LOCAL_ROUTING_SAFETY.md` (master kill switch + tool-use guard + Wilson 99% CI + SPRT + CUSUM drift detection + 30s timeout with cloud fallback + manual emergency demote), the path is bounded to LOW risk and gated behind a double opt-in (`enabled = true` AND `enable_local_routing = true`). Default-config users see zero behavior change; users who opt in twice get a system that can prove its local model is at least as good as cloud before serving them and automatically falls back on any failure.

The work splits into 22 phases, **all classified ZERO risk** (purely additive new files OR runtime branches gated behind `if let Some(et) = &self.eigen_tune` AND the double opt-in). The slash command parser edits in Phase 18 are LOW risk in isolation but mitigated by an explicit regression test for every existing slash command.

---

## 1. Current state (verified, with file:line citations)

### 1.1 What works today (do not touch)

| Subsystem | File | Status |
|---|---|---|
| Public API surface | `crates/temm1e-distill/src/lib.rs:39-475` | `EigenTuneEngine` with 5 hooks + status + tick + model discovery |
| Pair collector | `crates/temm1e-distill/src/collector.rs:31-255` | `collect()`, `observe_signal()`, `classify_domain()`, `is_likely_retry()`, `is_rejection()` |
| SQLite store | `crates/temm1e-distill/src/store.rs:21-730` | 4 tables, full CRUD, retention/eviction (`evict_if_full`, `prune_old_low_quality`) |
| State machine | `crates/temm1e-distill/src/engine/state_machine.rs:13-263` | All 4 transitions working except the Training dead end |
| Router | `crates/temm1e-distill/src/engine/router.rs:9-69` | `route()` returns `Cloud` / `Local` / `Shadow` / `Monitor` based on tier state + 5% sample rate |
| Shadow / Monitor | `crates/temm1e-distill/src/engine/shadow.rs`, `monitor.rs` | SPRT + CUSUM observers, fully wired to store |
| Graduation | `crates/temm1e-distill/src/engine/graduation.rs:10-73` | `tick()` and `demote()` against state machine |
| Embedding judge | `crates/temm1e-distill/src/judge/embedding.rs` | Cosine + cheap-equivalence shortcuts, no LLM cost |
| Behavior judge | `crates/temm1e-distill/src/judge/behavior.rs` | Tier 1 (instant) + Tier 2 (semantic) detection, multilingual rejection prototypes |
| Ollama backend (inference + create) | `crates/temm1e-distill/src/backends/ollama.rs` | `is_available`, `list_models`, `create_model`, `delete_model`, `embed`, `ensure_embedding_model` |
| Stats engines | `crates/temm1e-distill/src/stats/{sprt,cusum,wilson,entropy,thompson,beta,power}.rs` | All pure math, full unit coverage |
| Config | `crates/temm1e-distill/src/config.rs:1-447` | Every field has a serde default; empty `[eigentune]` section is valid |
| Model discovery / hardware detect | `crates/temm1e-distill/src/lib.rs:368-630` | Recommends models per RAM/chip |
| Existing tests | `crates/temm1e-distill/tests/{proof_of_pipeline,bench_eigentune}.rs` | 16 tests, all passing in isolation |

### 1.2 What's missing (verified by `ls`, by `Grep`, and against `IMPLEMENTATION.md` Phase 1.1)

| Planned file | `IMPLEMENTATION.md` line | Verified missing |
|---|---|---|
| `crates/temm1e-distill/src/curator.rs` | 20 | yes — dataset build logic only inside `tests/proof_of_pipeline.rs` |
| `crates/temm1e-distill/src/status.rs` | 24 | yes — status logic inlined in `lib.rs::format_status` (acceptable, not blocking) |
| `crates/temm1e-distill/src/engine/trainer.rs` | 39 | **yes — critical: state machine has a dead end without it** |
| `crates/temm1e-distill/src/engine/evaluator.rs` | 40 | **yes — critical: `eval_accuracy`/`eval_n` are read but never written in production** |
| `crates/temm1e-distill/src/backends/unsloth.rs` | 48 | yes |
| `crates/temm1e-distill/src/backends/mlx.rs` | 49 | yes |
| `crates/temm1e-distill/src/backends/hf_autotrain.rs` | 50 | yes — out of scope for this plan, defer |
| `crates/temm1e-distill/src/judge/teacher.rs` | 57 | yes — opt-in premium, defer |
| `crates/temm1e-distill/src/lib.rs::EigenTuneEngine::train()` | `IMPLEMENTATION.md:1306` | yes — public method advertised, never written |
| `scripts/eigentune_unsloth.py` | (implied) | yes — Unsloth is a Python lib not a CLI; need a thin wrapper |

### 1.3 What's broken (state machine dead ends)

`crates/temm1e-distill/src/engine/state_machine.rs:33`:
```rust
TierState::Training => Ok(None), // Training transitions handled by trainer
```
No trainer exists. A tier that enters `Training` is stuck forever.

`crates/temm1e-distill/src/engine/state_machine.rs:88-91` (the Evaluating gate):
```rust
let (accuracy, n) = match (record.eval_accuracy, record.eval_n) {
    (Some(acc), Some(n)) => (acc, n),
    _ => return Ok(None), // Not enough eval data yet
};
```
And `state_machine.rs:221-222` resets both fields to `None` when entering Evaluating. No production code path writes them back. Even if a tier somehow reached Evaluating, it would never leave.

### 1.4 What's NOT wired (verified by `Grep "use temm1e_distill"`)

Outside `crates/temm1e-distill/**` and its own tests, **zero `use temm1e_distill` imports exist**.

| Crate | Mentions distill? |
|---|---|
| `crates/temm1e-agent/**` | none |
| `crates/temm1e-gateway/**` | none |
| `crates/temm1e-channels/**` | none (no `/eigentune` slash command parser) |
| `src/main.rs` | none (no `eigentune` clap subcommand) |
| `crates/temm1e-perpetuum/src/conscience.rs:163` | comment only — *"Dream completes externally (EigenTune signals done)"* |
| `crates/temm1e-core/src/types/config.rs` | no `eigentune` field on `Temm1eConfig` |
| `crates/temm1e-core/Cargo.toml` | does not depend on `temm1e-distill` |
| Workspace `Cargo.toml:143` | `temm1e-distill` declared as workspace dep but never consumed by the root binary |
| Workspace `Cargo.toml:207` | no `eigentune` feature flag in default features |

### 1.5 What's overclaimed (the part that causes the user-visible "snake oil" problem)

| Claim | Source | Reality |
|---|---|---|
| `"v3.1.0  Eigen-Tune … proven on M2 with real LoRA fine-tune"` | `README.md:1046` | the M2 fine-tune was a manual one-off; no in-product code path can reproduce it |
| `"That's it. Restart TEMM1E and the system begins collecting"` | `tems_lab/eigen/SETUP.md:71-78` | no row is ever inserted into `eigentune_pairs` because the collector hook is never called |
| `temm1e-distill -- Eigen-Tune: self-tuning distillation engine` listed as a runtime crate | `CLAUDE.md:71` | not wired into the runtime |
| `"Distill quality scoring | temm1e-distill | Detects quality degradation"` | `docs/lab/cambium/THEORY.md:302` | no production code feeds CUSUM observations |
| `"EigenTune (distillation closed-loop) | Built, integrated"` | `tems_lab/perpetuum/IMPLEMENTATION_PLAN.md:57` | half-built, zero integration |

The plan's Phase S (Doc Alignment) fixes every one of these.

---

## 2. Architecture decisions

### A1 — Where does `EigenTuneConfig` live? (resolves circular-dep risk)

**Decision:** keep `EigenTuneConfig` in `crates/temm1e-distill/src/config.rs` (no move), and have **the binary** (`src/main.rs`) load the `[eigentune]` section in a second pass instead of adding it as a field on `Temm1eConfig`.

**Why:**
- `temm1e-distill` already depends on `temm1e-core` (for `Temm1eError`, line 12 of store.rs / state_machine.rs / etc.). Adding `temm1e-distill` to `temm1e-core` would create a Cargo circular dep — hard reject.
- Moving `EigenTuneConfig` to `temm1e-core` is possible (it has no deps beyond serde) but would change the public type path. Existing tests/users referencing `temm1e_distill::config::EigenTuneConfig` would need a re-export. Workable but adds two file moves and a public-API touch.
- The cleanest option: the binary already depends on both `temm1e-core` and (after Phase 0) `temm1e-distill`. It can deserialize the same TOML file twice — once into `Temm1eConfig` (existing flow, **unchanged**), once into a tiny wrapper struct that picks up `[eigentune]`. TOML allows unknown sections, so `Temm1eConfig`'s deserializer already silently ignores `[eigentune]`. Two-pass parsing has zero effect on the existing parse path.

**Implementation:** in `src/main.rs` near the existing `load_config()` call (line ~1455 per the explore agent's map), add:
```rust
#[derive(serde::Deserialize, Default)]
struct EigenTuneRoot {
    #[serde(default)]
    eigentune: temm1e_distill::config::EigenTuneConfig,
}
let raw = std::fs::read_to_string(&config_path).unwrap_or_default();
let expanded = temm1e_core::config::env::expand_env_vars(&raw);
let eigentune_cfg = toml::from_str::<EigenTuneRoot>(&expanded)
    .map(|r| r.eigentune)
    .unwrap_or_default();
```
This is a 6-line read, zero touch on `temm1e-core`.

**How to apply:** in Phase 0 dep wiring + Phase R construction site.

### A2 — Cargo feature flag strategy

**Decision:** **no new Cargo feature flag.** Eigen-Tune is gated by the runtime config field `[eigentune] enabled = false` (default), not by a compile-time flag. The crate compiles in every build but the engine is only instantiated when the user opts in.

**Why:**
- A `#[cfg(feature = "eigentune")]` gate would require touching every default-features list (workspace + binary + agent crate). Feature flags also fragment CI — clippy/test runs need both `--features` and `--no-default-features` to cover both code paths. This is overhead with no benefit because the additional binary size of the distill crate is small (~80 KB compiled) and it brings no new heavy runtime deps (sqlx is already used by `temm1e-memory`, reqwest by everything).
- The original design doc (`IMPLEMENTATION.md:108-110`) called for a feature flag, but the project has since standardized on runtime config gating for other subsystems (consciousness, perpetuum, social, cambium are all `Option<...>` fields on `AgentRuntime`, not feature flags). Following the project convention.

**How to apply:** Phase 0 just adds `temm1e-distill = { workspace = true }` to `crates/temm1e-agent/Cargo.toml` and to the root `[dependencies]` in the workspace `Cargo.toml` of the `temm1e` binary. No new `[features]` entries.

### A3 — Trainer backend dispatch

**Decision:** trait-based dispatch with **two backends in this plan**: `mlx` (Apple Silicon, native CLI) and `unsloth` (NVIDIA/CUDA via Python wrapper script). `hf_autotrain.rs` is out of scope (deferred — needs HF API key handling, unrelated complexity).

**Trait:**
```rust
// crates/temm1e-distill/src/backends/mod.rs
#[async_trait]
pub trait TrainingBackend: Send + Sync {
    fn name(&self) -> &'static str;
    /// Probe whether this backend can run on the current host (no side effects).
    async fn is_available(&self) -> bool;
    /// Spawn the training subprocess. Streams stdout/stderr to tracing.
    async fn train(&self, job: &TrainJob) -> Result<TrainArtifacts, Temm1eError>;
}

pub struct TrainJob {
    pub base_model: String,           // e.g. "mlx-community/SmolLM2-135M-Instruct-4bit"
    pub dataset_dir: PathBuf,         // contains train.jsonl + valid.jsonl
    pub output_dir: PathBuf,          // adapter weights go here
    pub epochs: i32,
    pub learning_rate: f64,
    pub lora_r: i32,
    pub lora_alpha: i32,
    pub batch_size: i32,
    pub grad_accumulation: i32,
    pub max_seq_len: i32,
}

pub struct TrainArtifacts {
    pub adapter_path: PathBuf,        // .safetensors or .npz
    pub fused_model_dir: Option<PathBuf>, // if backend can fuse (mlx_lm.fuse)
    pub train_loss: Option<f64>,
    pub eval_loss: Option<f64>,
    pub epochs_completed: i32,
}
```

**Dispatch:**
```rust
pub async fn select_backend(config: &EigenTuneConfig) -> Option<Box<dyn TrainingBackend>> {
    let mlx = backends::mlx::MlxBackend;
    let unsloth = backends::unsloth::UnslothBackend;
    match config.backend.as_str() {
        "mlx" if mlx.is_available().await => Some(Box::new(mlx)),
        "unsloth" if unsloth.is_available().await => Some(Box::new(unsloth)),
        // "auto" — try platform-native first
        "auto" if cfg!(all(target_os = "macos", target_arch = "aarch64"))
                  && mlx.is_available().await => Some(Box::new(mlx)),
        "auto" if unsloth.is_available().await => Some(Box::new(unsloth)),
        _ => None,
    }
}
```

**Why dispatch (not direct call):** future backends (HF AutoTrain, Axolotl, llama.cpp-finetune) plug in without touching the trainer orchestrator.

### A4 — GGUF vs safetensors-with-`ADAPTER` for Ollama

**Decision:** prefer **Ollama Modelfile `ADAPTER` directive with safetensors** for Llama / Mistral / Gemma family base models; fall back to GGUF conversion only when the base model family is unsupported.

**Why:**
- Ollama's Modelfile supports `FROM <base>` + `ADAPTER <path-to-safetensors-dir>` natively for Llama/Mistral/Gemma families ([Modelfile docs](https://docs.ollama.com/modelfile)). This skips the entire GGUF conversion pipeline, which would otherwise need llama.cpp's `convert_hf_to_gguf.py` (a Python tool that adds another dependency surface).
- For unsupported families (Qwen, Phi, SmolLM, etc.), we need GGUF. The simplest path: `mlx_lm.fuse --de-quantize` to materialize a fused model, then `python -m llama_cpp.convert` (if installed) — but this is brittle. Defer GGUF conversion to a follow-up phase; for the MVP, **the recommended models are restricted to families with ADAPTER support**.
- The default `recommend_models` in `lib.rs:567-630` already steers users toward `mlx-community` and `unsloth` quantized variants. We update the recommendations to prefer Llama-3.2 / Gemma-2 / Mistral-7B variants for the MVP and document the family restriction in SETUP.md.

**How to apply:**
- In `engine/trainer.rs::commit_to_ollama()`, write a Modelfile with `FROM` + `ADAPTER` paths (no GGUF conversion).
- In `lib.rs::recommend_models`, update the default list to Llama-3.2-1B / Llama-3.2-3B / Gemma-2-2B variants.
- In SETUP.md, add a "Supported base model families for MVP" section.
- Future phase: implement GGUF conversion in `engine/trainer.rs::convert_to_gguf()` for unsupported families (gated by config `auto_gguf = true`).

### A5 — Hook injection placement in `process_message()`

**Decision:** five hook points injected, all fire-and-forget (`tokio::spawn` + ignored `Result`):

1. **Pre-call** (`crates/temm1e-agent/src/runtime.rs:1180-1191`): `route()` is called BEFORE the provider call. Phase L is **observe-only** — the result is logged via `tracing::info!` but the agent continues with cloud. Phase N (deferred, MEDIUM risk) acts on the decision and switches the provider.
2. **Post-call** (`runtime.rs:1234-1236`, immediately after `response` is bound): build `EigenTunePairData`, `tokio::spawn(engine.on_completion(data))`. Fire-and-forget.
3. **User-message-arrival** (`runtime.rs:~400-450`): on each new user message, run `behavior_observation` (Tier 1) against the previous user message; if it returns `(false, "explicit_rejection")` or `(false, "retry_rephrase")`, `tokio::spawn(engine.on_signal(chat_id, QualitySignal::UserRejected | UserRetried))`.
4. **Tool-result** (`runtime.rs:1879-1905`): immediately after `execute_tool()` returns, on success → `on_signal(QualitySignal::ToolCallSucceeded)`; on `is_error == true` → `on_signal(QualitySignal::ResponseError)`. Fire-and-forget.
5. **Conversation-extended** (turn count crossing threshold): once per conversation, when `session.history.len() == config.eigentune.conversation_extended_threshold` (default 6), `on_signal(QualitySignal::ConversationExtended)`. Idempotency tracked in a `HashSet<chat_id>` on the engine's `Arc<Mutex<...>>` — or simpler, embedded in the per-pair `user_continued` column already.

**Why fire-and-forget:** the user's `feedback_no_stubs.md` and `feedback_zero_snake_oil.md` rules require this NOT to add latency or risk to the user-facing path. Wrapping every hook in `tokio::spawn` ensures even if the SQLite write hangs or Ollama is unreachable, the user's reply goes out immediately. Errors are logged at `debug!` level (not `error!`) per the existing collector pattern (`lib.rs:82-85`).

**No new latency, no new failure modes for users with `enabled=false`.**

### A6 — Cross-platform: how to handle missing MLX / Unsloth

**Decision:** the **collection** path (Phases A–K) works on every supported OS — macOS (Intel + ARM), Linux, Windows — because it only writes to SQLite and runs Rust code. **Training** is platform-gated:

| Backend | macOS-arm64 | macOS-x86 | Linux x86_64 | Windows x86_64 |
|---|---|---|---|---|
| MLX (`mlx_lm.lora`) | ✅ supported | ❌ unsupported | ❌ unsupported | ❌ unsupported |
| Unsloth (Python wrapper) | ✅ supported (CPU/MPS) | ⚠️ slow (CPU only) | ✅ supported (CUDA + CPU) | ⚠️ supported but flaky |

**On Windows specifically:** Unsloth officially supports Windows but has known issues with `bitsandbytes` 4-bit quant. For the MVP, we document Windows as "collection works, training requires WSL2 or Linux." This matches the user's `Cross-Platform Requirement` rule (collection works everywhere; training degrades gracefully with a clear error message).

**Failure mode:** if no backend is available, `engine.train(tier)` returns `Err(Temm1eError::Tool("no training backend available"))` and the tier reverts from `Training` → `Collecting` (handled by Phase E orchestrator). The user sees the error in `/eigentune status` ("Training: no backend (install mlx-lm or unsloth)") and the system continues collecting. **Zero user-visible regression.**

**How to apply:**
- Trainer dispatch (A3) returns `None` if no backend matches.
- Trainer orchestrator handles `None` by transitioning the tier back to `Collecting` and writing a `TrainingRun` row with `status=failed, error_message="no_backend"`.
- The status display (`lib.rs::format_status`) reads the most recent `TrainingRun` and shows the failure reason.

### A7 — How does `EigenTuneEngine` reach the agent vs the CLI subcommand?

**Two access patterns:**

1. **Agent runtime** (live): `AgentRuntime` gains an `eigen_tune: Option<Arc<EigenTuneEngine>>` field, mirroring the existing pattern for `consciousness: Option<ConsciousnessEngine>` (`runtime.rs:131`). The engine is constructed in `src/main.rs` near the agent construction site (line ~2131-2171) and injected via a new `.with_eigen_tune(Arc<EigenTuneEngine>)` builder method.

2. **CLI subcommand** (offline): `temm1e eigentune status` runs in `Commands::Eigentune { ... }` block. It does NOT need a live agent — it constructs its own `EigenTuneEngine` from the same SQLite database file the agent writes to. This is identical to how `temm1e status` reads agent state from disk without a running gateway.

**The shared database file:** `~/.temm1e/eigentune.db` (configurable via `[eigentune] database_url`). Both the live agent and the CLI subcommand connect to the same file. SQLite handles concurrent reads natively, and the writes are infrequent (every collection event + every state transition), so contention is not a concern.

**Why this matters:** the CLI handler can answer `/eigentune status` even when the gateway daemon is not running, which matches the existing pattern where `temm1e status` works without a daemon.

### A8 — Periodic `tick()` task ownership

**Decision:** the tick task is spawned **inside `Commands::Start`** in `src/main.rs`, near the existing heartbeat-spawn code (per the agent loop explore map, ~line 2350). It only runs when the daemon is running. The CLI subcommand `temm1e eigentune tick` provides an out-of-band trigger for testing.

**Period:** `60 seconds` (configurable via `[eigentune] tick_interval_secs`, but we don't add this knob in MVP — hardcoded 60s is fine).

**Pattern (from the existing social-facts spawn at `runtime.rs:578-585` and the heartbeat at `main.rs:2344-2357`):**
```rust
if let Some(et_engine) = eigen_tune_engine.clone() {
    task_handles.push(tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            for (tier, from, to) in et_engine.tick().await {
                tracing::info!(
                    tier = %tier.as_str(),
                    from = %from.as_str(),
                    to = %to.as_str(),
                    "Eigen-Tune: tier transition"
                );
                // If transition is into Training, kick off the trainer
                if to == TierState::Training {
                    let engine = et_engine.clone();
                    tokio::spawn(async move {
                        if let Err(e) = engine.train(tier).await {
                            tracing::warn!(error = %e, tier = %tier.as_str(),
                                "Eigen-Tune: training failed (tier reverts to Collecting)");
                        }
                    });
                }
            }
        }
    }));
}
```

The trainer is spawned as a child task so the tick loop is not blocked by a multi-minute training run.

### A9 — Curator extraction strategy

**Decision:** lift the dataset-building logic out of `tests/proof_of_pipeline.rs` into a new `crates/temm1e-distill/src/curator.rs` module. The test file keeps its end-to-end smoke test but switches to importing curator functions.

**Why:** the test's inline logic is the de-facto curator already; lifting it makes it reusable by `engine/trainer.rs` without code duplication. The test remains the regression net.

**Functions to extract** (from the curator-extraction explore agent's spec, validated against `proof_of_pipeline.rs:266-344`):

```rust
// crates/temm1e-distill/src/curator.rs

pub async fn load_tier_pairs(
    store: &EigenTuneStore,
    tier: &str,
    min_quality: f64,
) -> Result<Vec<TrainingPair>, Temm1eError>;

pub fn dedup_by_messages_hash(pairs: Vec<TrainingPair>) -> Vec<TrainingPair>;

pub fn compute_diversity_entropy(pairs: &[TrainingPair]) -> f64;

pub fn split_holdout_set(
    pairs: Vec<TrainingPair>,
    holdout_pct: f64,
    rng_seed: Option<u64>,
) -> (Vec<TrainingPair>, Vec<TrainingPair>);  // (eval, train) — stratified by (tier, category)

pub fn balance_by_thompson_sampling(
    pairs: &[TrainingPair],
    target_count: usize,
    rng_seed: Option<u64>,
) -> Vec<TrainingPair>;

pub async fn export_chatml_jsonl(
    pairs: &[TrainingPair],
    output_path: &Path,
) -> Result<usize, Temm1eError>;  // returns lines written

pub fn validate_chatml_jsonl(file_path: &Path) -> Result<(usize, usize), Temm1eError>;

/// Top-level pipeline used by the trainer.
pub async fn build_training_dataset(
    store: &EigenTuneStore,
    config: &EigenTuneConfig,
    tier: EigenTier,
    output_dir: &Path,
) -> Result<CuratorOutput, Temm1eError>;

pub struct CuratorOutput {
    pub train_path: PathBuf,    // <output_dir>/train.jsonl
    pub valid_path: PathBuf,    // <output_dir>/valid.jsonl  (subset of training, for in-loop eval)
    pub eval_path: PathBuf,     // <output_dir>/eval.jsonl   (held out for evaluator.rs)
    pub train_count: usize,
    pub eval_count: usize,
    pub diversity_j: f64,       // computed entropy at curation time
    pub category_distribution: Vec<(String, f64)>,
}
```

**General-mix data:** the original spec mentioned `general_mix_pct = 0.1` to prevent catastrophic forgetting. **Defer** for MVP — we don't have a general-purpose dataset bundled, and pulling one at runtime adds a download dep. The trainer still respects the config field but always sets the actual general mix count to 0 in MVP. Document this in SETUP.md as a future improvement.

**RNG seed:** all curator functions take an optional `rng_seed: Option<u64>` so tests are deterministic (passes `Some(42)`); production passes `None` and uses `rand::thread_rng()`.

---

## 3. Phased implementation

Each phase has: scope, files touched, risk level, dependencies, rollback. **The order is dependency-strict** — phases later in the list assume the prior ones are merged.

### Phase 0 — Pre-flight: Cargo dependency wiring

**Scope:** make `temm1e-distill` reachable from the binary and the agent crate. No code that runs.

**Files:**
- `Cargo.toml` (workspace, top-level): add `temm1e-distill.workspace = true` to the binary `[dependencies]` block (around line 184). Already declared as a workspace dep at line 143, so the workspace registration is fine.
- `crates/temm1e-agent/Cargo.toml`: add `temm1e-distill = { workspace = true }` to `[dependencies]`.
- `crates/temm1e-distill/Cargo.toml`: add `sha2 = { workspace = true }` (already in workspace deps at line 73), `tempfile = { version = "3" }` to dev-deps.

**Risk:** ZERO. Cargo dep additions only. Compiles cleanly with no functional change.

**Rollback:** revert the Cargo.toml edits.

**Verification:**
```bash
cargo check --workspace
cargo build --workspace
```
Both must succeed with no warnings.

---

### Phase 1 — Curator module (`src/curator.rs`) + `enable_local_routing` config field

**Scope:** new file (~400 LOC) PLUS one new field added to `EigenTuneConfig`.

**Files:**
- New: `crates/temm1e-distill/src/curator.rs` — implements all functions in §A9.
- Edit: `crates/temm1e-distill/src/lib.rs:14-22` — add `pub mod curator;`.
- Edit: `crates/temm1e-distill/src/config.rs` — add `pub enable_local_routing: bool` field with `default_false`. This is the second of the double opt-in switches required by the seven-gate safety chain (`LOCAL_ROUTING_SAFETY.md` §2). It defaults to `false` so even users who set `enabled = true` get observation-only mode until they explicitly enable local serving.

**Config field addition (`config.rs`):**
```rust
// Add to the EigenTuneConfig struct (after `pub enabled: bool` at line 16):
    /// Master switch for local routing. When false, the engine still collects,
    /// trains, evaluates, and shadow-tests, but route() always returns Cloud
    /// at the runtime layer (Phase 13). Default false. Required to be true
    /// for the agent to actually serve users with the distilled local model.
    #[serde(default = "default_false")]
    pub enable_local_routing: bool,
```

And add the corresponding initializer in `impl Default for EigenTuneConfig` at line 313:
```rust
            enable_local_routing: default_false(),
```

**Implementation notes:**
- `load_tier_pairs`: thin wrapper around `EigenTuneStore::get_pairs_for_tier(tier, min_quality)` (verified at `store.rs:305-326`).
- `dedup_by_messages_hash`: SHA-256 of `pair.messages_json` (already a normalized JSON string from collection time). `HashSet<String>` insertion-order preservation via `Vec`. Test against an inline list of 5 pairs with 1 duplicate.
- `compute_diversity_entropy`: forwards to `crate::stats::entropy::normalized_entropy` (`stats/entropy.rs:31-42`). Matches existing usage in `state_machine.rs:59`.
- `split_holdout_set`: stratified by `(EigenTier, domain_category)` tuple. Each stratum is shuffled with a deterministic RNG (`StdRng::seed_from_u64(seed)` if `seed.is_some()`). First `ceil(stratum.len() * holdout_pct)` pairs become eval. Sets `pair.is_eval_holdout = true` on eval pairs.
- `balance_by_thompson_sampling`: uses the existing `crate::stats::thompson::ThompsonSampler` (must verify the API exists — if it doesn't, fall back to proportional category sampling weighted by `quality_score`). **TODO in this phase: read `stats/thompson.rs` once and either use it as-is or adjust the curator function to match its actual API.**
- `export_chatml_jsonl`: opens file, iterates pairs, writes `{"messages": <parsed messages_json>}` + `\n` per pair. Returns line count. Matches the test's existing logic at `proof_of_pipeline.rs:276-289`.
- `validate_chatml_jsonl`: re-reads the file, parses each line as JSON, asserts `{"messages": [...]}` shape with valid roles. Returns `(valid, total)`.
- `build_training_dataset`: top-level orchestrator. Sequence: load → dedup → check entropy gate → balance → split → write three files (train.jsonl, valid.jsonl = 90% of train, eval.jsonl). Returns `CuratorOutput`.

**Tests (in `crates/temm1e-distill/src/curator.rs::tests`):**
1. `dedup_removes_exact_duplicates`
2. `dedup_preserves_order`
3. `compute_diversity_entropy_uniform_returns_one`
4. `compute_diversity_entropy_monoculture_returns_zero`
5. `split_holdout_pct_15_yields_15pct_eval`
6. `split_holdout_is_stratified_per_category`
7. `split_holdout_marks_is_eval_holdout`
8. `export_chatml_jsonl_one_per_line`
9. `export_chatml_jsonl_validates_round_trip`
10. `build_training_dataset_full_pipeline_inmem` (uses `sqlite::memory:` store, 100 fake pairs across 3 tiers and 5 categories, asserts files written, J ≥ threshold)

**Risk:** ZERO. New file, no public API change to existing types. The curator module is unreachable from any production code path until Phase 4 imports it.

**Rollback:** delete the new file + revert the `mod` declaration in `lib.rs`.

---

### Phase 2 — MLX backend (`src/backends/mlx.rs`)

**Scope:** new file, ~150 LOC. Implements `TrainingBackend` trait by spawning `mlx_lm.lora` as a subprocess.

**Files:**
- New: `crates/temm1e-distill/src/backends/mlx.rs`.
- Edit: `crates/temm1e-distill/src/backends/mod.rs` — add the trait definition (§A3) and `pub mod mlx;`.

**Subprocess invocation** (verified via [mlx-lm/LORA.md](https://github.com/ml-explore/mlx-lm/blob/main/mlx_lm/LORA.md)):
```rust
async fn train(&self, job: &TrainJob) -> Result<TrainArtifacts, Temm1eError> {
    // mlx_lm.lora --train \
    //     --model <base_model> \
    //     --data <dataset_dir> \           # contains train.jsonl + valid.jsonl
    //     --adapter-path <output_dir> \    # adapters.safetensors written here
    //     --fine-tune-type lora \
    //     --num-layers 16 \                # default; tunable in future
    //     --batch-size <batch_size> \
    //     --iters <iters>                  # iters = epochs * (train_count / batch_size)
    let mut cmd = tokio::process::Command::new("python3");
    cmd.arg("-m").arg("mlx_lm.lora")
       .arg("--train")
       .arg("--model").arg(&job.base_model)
       .arg("--data").arg(&job.dataset_dir)
       .arg("--adapter-path").arg(&job.output_dir)
       .arg("--fine-tune-type").arg("lora")
       .arg("--batch-size").arg(job.batch_size.to_string())
       .arg("--iters").arg(compute_iters(job).to_string());
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn()
        .map_err(|e| Temm1eError::Tool(format!("mlx_lm.lora spawn: {e}")))?;
    // Stream stdout/stderr to tracing in two background tasks
    stream_to_tracing(child.stdout.take().unwrap(), "mlx_lm.lora.stdout");
    stream_to_tracing(child.stderr.take().unwrap(), "mlx_lm.lora.stderr");
    let status = child.wait().await
        .map_err(|e| Temm1eError::Tool(format!("mlx_lm.lora wait: {e}")))?;
    if !status.success() {
        return Err(Temm1eError::Tool(format!("mlx_lm.lora exit {}", status.code().unwrap_or(-1))));
    }
    let adapter_path = job.output_dir.join("adapters.safetensors");
    if !adapter_path.exists() {
        return Err(Temm1eError::Tool("mlx_lm.lora: adapter file missing after success".into()));
    }
    Ok(TrainArtifacts {
        adapter_path,
        fused_model_dir: None, // Phase: optionally call mlx_lm.fuse here
        train_loss: None,     // Phase: parse from stdout
        eval_loss: None,
        epochs_completed: job.epochs,
    })
}

async fn is_available(&self) -> bool {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) { return false; }
    tokio::process::Command::new("python3")
        .args(["-c", "import mlx_lm"])
        .output().await
        .map(|o| o.status.success()).unwrap_or(false)
}
```

**Tests:**
1. `is_available_returns_false_on_non_arm64_mac`
2. `train_command_construction_no_spawn` (build the command, inspect args via `cmd.as_std()`, do not actually spawn)
3. `train_returns_error_when_python3_missing` (use a `PATH` override in test)

**Risk:** ZERO. New file, no existing code touched. The trainer orchestrator (Phase 4) is what calls into it.

**Rollback:** delete `mlx.rs`, revert `mod.rs`.

---

### Phase 3 — Unsloth backend (`src/backends/unsloth.rs` + `scripts/eigentune_unsloth.py`)

**Scope:** Unsloth is a Python library, not a CLI binary. We ship a thin Python wrapper script that accepts CLI args and drives Unsloth's `FastLanguageModel.get_peft_model()` + `SFTTrainer`. The Rust backend spawns the wrapper.

**Files:**
- New: `crates/temm1e-distill/src/backends/unsloth.rs` (~120 LOC).
- New: `scripts/eigentune_unsloth.py` (~80 LOC, vendored Python).
- Edit: `crates/temm1e-distill/src/backends/mod.rs` — add `pub mod unsloth;`.

**Python wrapper sketch** (`scripts/eigentune_unsloth.py`):
```python
#!/usr/bin/env python3
import argparse, json, os, sys
from pathlib import Path

def main():
    p = argparse.ArgumentParser()
    p.add_argument("--model", required=True)
    p.add_argument("--data", required=True)
    p.add_argument("--output", required=True)
    p.add_argument("--epochs", type=int, default=3)
    p.add_argument("--lr", type=float, default=2e-4)
    p.add_argument("--lora-r", type=int, default=32)
    p.add_argument("--lora-alpha", type=int, default=64)
    p.add_argument("--batch-size", type=int, default=4)
    p.add_argument("--max-seq-len", type=int, default=4096)
    args = p.parse_args()

    from unsloth import FastLanguageModel
    from trl import SFTTrainer
    from transformers import TrainingArguments
    from datasets import load_dataset

    model, tokenizer = FastLanguageModel.from_pretrained(
        args.model, max_seq_length=args.max_seq_len,
        dtype=None, load_in_4bit=True,
    )
    model = FastLanguageModel.get_peft_model(
        model, r=args.lora_r, lora_alpha=args.lora_alpha,
        target_modules=["q_proj","k_proj","v_proj","o_proj","gate_proj","up_proj","down_proj"],
        use_gradient_checkpointing="unsloth",
    )
    train_ds = load_dataset("json", data_files=str(Path(args.data)/"train.jsonl"), split="train")
    eval_ds = load_dataset("json", data_files=str(Path(args.data)/"valid.jsonl"), split="train") \
        if (Path(args.data)/"valid.jsonl").exists() else None

    def to_text(ex):
        # ChatML messages → text via tokenizer's chat template
        return {"text": tokenizer.apply_chat_template(ex["messages"], tokenize=False)}
    train_ds = train_ds.map(to_text)
    if eval_ds: eval_ds = eval_ds.map(to_text)

    trainer = SFTTrainer(
        model=model, tokenizer=tokenizer,
        train_dataset=train_ds, eval_dataset=eval_ds,
        dataset_text_field="text", max_seq_length=args.max_seq_len,
        args=TrainingArguments(
            per_device_train_batch_size=args.batch_size,
            num_train_epochs=args.epochs, learning_rate=args.lr,
            output_dir=args.output, save_strategy="epoch",
            logging_steps=10, optim="adamw_8bit",
            report_to="none",
        ),
    )
    result = trainer.train()
    model.save_pretrained(args.output)  # writes adapter_model.safetensors
    tokenizer.save_pretrained(args.output)
    # Emit a parseable summary line for the Rust caller
    summary = {"train_loss": result.training_loss, "epochs_completed": args.epochs}
    print("EIGENTUNE_RESULT " + json.dumps(summary))

if __name__ == "__main__":
    main()
```

**Rust backend** (`backends/unsloth.rs`):
```rust
async fn is_available(&self) -> bool {
    // python3 -c "import unsloth; import trl; import datasets"
    tokio::process::Command::new("python3")
        .args(["-c", "import unsloth, trl, datasets"])
        .output().await
        .map(|o| o.status.success()).unwrap_or(false)
}

async fn train(&self, job: &TrainJob) -> Result<TrainArtifacts, Temm1eError> {
    let script = locate_script("eigentune_unsloth.py")?;  // search relative to exe + cargo manifest
    let mut cmd = tokio::process::Command::new("python3");
    cmd.arg(&script)
       .arg("--model").arg(&job.base_model)
       .arg("--data").arg(&job.dataset_dir)
       .arg("--output").arg(&job.output_dir)
       .arg("--epochs").arg(job.epochs.to_string())
       .arg("--lr").arg(job.learning_rate.to_string())
       .arg("--lora-r").arg(job.lora_r.to_string())
       .arg("--lora-alpha").arg(job.lora_alpha.to_string())
       .arg("--batch-size").arg(job.batch_size.to_string())
       .arg("--max-seq-len").arg(job.max_seq_len.to_string());
    // Same stdout/stderr streaming as MLX backend
    let output = cmd.output().await
        .map_err(|e| Temm1eError::Tool(format!("unsloth spawn: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Temm1eError::Tool(format!("unsloth exit {}: {}",
            output.status.code().unwrap_or(-1), stderr)));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let summary = parse_eigentune_result(&stdout);
    let adapter_path = job.output_dir.join("adapter_model.safetensors");
    if !adapter_path.exists() {
        return Err(Temm1eError::Tool("unsloth: adapter file missing".into()));
    }
    Ok(TrainArtifacts {
        adapter_path, fused_model_dir: None,
        train_loss: summary.train_loss, eval_loss: None,
        epochs_completed: summary.epochs_completed,
    })
}
```

**Script discovery (`locate_script`):** searches in this order:
1. `$TEMM1E_SCRIPTS_DIR/eigentune_unsloth.py` (env override for tests + production install)
2. `<exe_dir>/scripts/eigentune_unsloth.py` (alongside the binary in distribution)
3. `<exe_dir>/../scripts/eigentune_unsloth.py` (cargo target/release layout)
4. `<cargo_manifest_dir>/scripts/eigentune_unsloth.py` (cargo dev workflow)

If not found, returns `Err(Temm1eError::Tool("eigentune_unsloth.py not found"))` and the trainer falls back to next backend.

**Tests:**
1. `is_available_false_when_unsloth_missing`
2. `train_command_construction`
3. `parse_eigentune_result_well_formed`
4. `parse_eigentune_result_handles_missing_summary_line`
5. `locate_script_finds_in_env_override`

**Risk:** ZERO. New files, no touch on existing code. Python script is vendored (no runtime download).

**Rollback:** delete the new files, revert `mod.rs`.

---

### Phase 4 — Trainer orchestrator (`src/engine/trainer.rs`)

**Scope:** the orchestrator that consumes `curator::build_training_dataset` and calls into `TrainingBackend::train()`. Writes `TrainingRun` rows. Updates the tier record on success/failure. Closes the Training → Evaluating loop.

**Files:**
- New: `crates/temm1e-distill/src/engine/trainer.rs` (~250 LOC).
- Edit: `crates/temm1e-distill/src/engine/mod.rs` — add `pub mod trainer;`.

**API:**
```rust
pub struct TrainerOrchestrator {
    store: Arc<EigenTuneStore>,
    config: EigenTuneConfig,
}

impl TrainerOrchestrator {
    pub fn new(store: Arc<EigenTuneStore>, config: EigenTuneConfig) -> Self;

    /// Run a complete training cycle for a tier.
    /// Assumes the tier is already in `Training` state.
    /// On success: tier moves to `Evaluating` (caller must trigger evaluator next).
    /// On failure: tier moves back to `Collecting`, TrainingRun row is `failed`.
    pub async fn run(&self, tier: EigenTier) -> Result<TrainArtifacts, Temm1eError>;
}
```

**Sequence (numbered):**
1. **Pre-flight checks.**
   - `let backend = backends::select_backend(&self.config).await.ok_or(Temm1eError::Tool("no backend"))?;`
   - `let _ollama = backends::ollama::is_available().await;` (warn if not running, but continue — Ollama is only needed at the very end)
2. **Curate the dataset.**
   - `let workdir = self.config.artifacts_dir / format!("run_{run_id}");`  (UUID, timestamped)
   - `let curator_out = curator::build_training_dataset(&self.store, &self.config, tier, &workdir).await?;`
   - Verify `curator_out.train_count >= self.config.min_pairs as usize` (else return `Err`)
   - Verify `curator_out.diversity_j >= self.config.diversity_target` (else return `Err`)
3. **Insert TrainingRun row (`status=running`).**
   - Construct a `TrainingRun` with the run_id, base_model from `config.base_model`, backend name, method, dataset_version, pair_count, general_mix_pct, started_at=now.
   - `self.store.save_run(&run).await?;`
   - Update the tier record: `record.current_run_id = Some(run_id); self.store.update_tier(&record).await?;`
4. **Build the TrainJob.**
   - Resolve `base_model`: if `"auto"`, call `recommend_models()` and pick the smallest one that fits the tier (Simple → 1B, Standard → 3B, Complex → 7B). Document this mapping in SETUP.md.
   - Construct TrainJob with config values (`epochs`, `learning_rate`, `lora_r`, `lora_alpha`, `batch_size`, `gradient_accumulation`, `max_seq_length`).
5. **Spawn the backend.**
   - `let artifacts = backend.train(&job).await?;`
   - This may take minutes. The tick task spawns the trainer in a child task (§A8) so the tick loop is not blocked.
6. **Commit to Ollama.**
   - Decide commit strategy based on base model family:
     - Llama / Mistral / Gemma → write Modelfile with `FROM <base>` + `ADAPTER <artifacts.adapter_path>` and call `ollama::create_model(model_name, &modelfile_path)`.
     - Other families → log warning, leave the run as `completed_local` (artifacts on disk, not in Ollama). User can manually convert to GGUF later.
   - `let model_name = format!("eigentune-{tier}-{run_id_short}");`
   - `ollama::create_model(&model_name, ...).await?;`
7. **Update the TrainingRun row (`status=completed`).**
   - `run.status = Completed; run.completed_at = Some(Utc::now()); run.train_loss = artifacts.train_loss; run.ollama_model_name = Some(model_name);`
   - `self.store.update_run(&run).await?;`
8. **Transition tier `Training → Evaluating`.**
   - The state machine guard: must use `state_machine.transition(tier, Training, Evaluating).await`.
   - This resets `eval_accuracy` and `eval_n` to None (line 221-222 of state_machine.rs) and bumps `last_trained_at`.
9. **Return artifacts.** Caller (the periodic tick task or `engine.train(tier)`) is expected to immediately call the evaluator.

**Failure handling:** every step is wrapped in `?` and on error:
- The tier transitions back to `Collecting` via `state_machine.transition(current_state, Collecting)`.
- The `TrainingRun` row's status is updated to `Failed` with `error_message = format!("{e}")`.
- The error is logged at `warn!` level and propagated to the caller.

**Tests:**
1. `run_fails_when_no_backend_available` (uses a config with `backend = "nonexistent"`)
2. `run_fails_when_min_pairs_not_met` (in-mem store with 5 pairs, min_pairs = 100)
3. `run_fails_when_diversity_below_threshold` (in-mem store with 100 pairs all in one category)
4. `run_writes_running_then_failed_on_backend_error` (mock backend that returns Err)
5. `run_transitions_tier_back_to_collecting_on_failure`
6. **No happy-path test in unit tests** — the happy path requires a real MLX or Unsloth install. That test lives in Phase 14 (feature-gated `MLX_AVAILABLE=1`).

**Risk:** ZERO. The trainer is only invoked when a tier is in `Training` state, which only happens when `[eigentune] enabled = true` and a tier accumulates enough pairs. Default = unreachable.

**Rollback:** delete `trainer.rs`, revert `engine/mod.rs`.

---

### Phase 5 — Evaluator (`src/engine/evaluator.rs`)

**Scope:** runs the eval holdout set against the freshly created Ollama model, compares with `judge::embedding`, computes accuracy + n, writes them to the tier record. Closes the Evaluating → Shadowing loop.

**Files:**
- New: `crates/temm1e-distill/src/engine/evaluator.rs` (~200 LOC).
- Edit: `crates/temm1e-distill/src/engine/mod.rs` — add `pub mod evaluator;`.

**API:**
```rust
pub struct EvaluatorOrchestrator {
    store: Arc<EigenTuneStore>,
    config: EigenTuneConfig,
}

impl EvaluatorOrchestrator {
    pub fn new(store: Arc<EigenTuneStore>, config: EigenTuneConfig) -> Self;

    pub async fn run(&self, tier: EigenTier, run_id: &str) -> Result<EvalReport, Temm1eError>;
}

pub struct EvalReport {
    pub tier: EigenTier,
    pub run_id: String,
    pub n: i32,
    pub accuracy: f64,
    pub wilson_lower: f64,
    pub passed: bool,
}
```

**Sequence:**
1. Load the run: `let run = self.store.get_run(run_id).await?.ok_or(...)?;`
2. Load eval holdout pairs: `let eval_pairs = self.store.get_pairs_for_tier(tier.as_str(), 0.0).await?` filtered to `is_eval_holdout == true`.
3. **Verify n is sufficient:** if `eval_pairs.len() < self.config.min_eval_samples as usize`, return `Err` (caller transitions tier back to Collecting).
4. For each eval pair:
   - Extract the user message from `pair.messages_json` (parse, find role=user, last one).
   - Call the freshly trained Ollama model: `let local_response = ollama::chat(&run.ollama_model_name?, &user_message).await?;` — this is a NEW function we add to `backends/ollama.rs` (a thin wrapper around `POST /api/chat`).
   - Compare against the stored `pair.response_json`'s assistant message via:
     - First the cheap check: `judge::embedding::cheap_equivalence_check(&local, &cloud)` (already exists at `judge/embedding.rs:30-54`).
     - If `None` (cheap check inconclusive): embed both with `ollama::embed("nomic-embed-text", &local)` and `ollama::embed("nomic-embed-text", &cloud)`, compute `cosine_similarity`, compare to `config.graduation_accuracy` (or a separate threshold).
   - Tally a `passed` count.
5. Compute metrics:
   - `accuracy = passed as f64 / n as f64`
   - `wilson_lower = wilson::wilson_lower(passed, n, config.graduation_confidence)`
6. Write back to the tier record:
   - `record.eval_accuracy = Some(accuracy); record.eval_n = Some(n as i32);`
   - `self.store.update_tier(&record).await?;`
7. The next tick of the state machine will see these fields populated and execute `Evaluating → Shadowing` (if `wilson_lower >= graduation_accuracy`) or `Evaluating → Collecting` (if not).
8. Return `EvalReport`.

**New helper added to `backends/ollama.rs`:**
```rust
pub async fn chat(model: &str, user_message: &str) -> Result<String, Temm1eError> {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(60)).build()?;
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": user_message}],
        "stream": false,
    });
    let resp = client.post(format!("{OLLAMA_BASE}/api/chat")).json(&body).send().await?;
    let parsed: serde_json::Value = resp.json().await?;
    Ok(parsed["message"]["content"].as_str().unwrap_or("").to_string())
}
```
This is the only addition to the existing `ollama.rs` file. It mirrors the existing `embed()` function.

**Tests:**
1. `run_fails_when_eval_pairs_below_min`
2. `run_writes_eval_accuracy_and_eval_n_to_tier_record` (with a fake model that always agrees)
3. `run_handles_ollama_unavailable` (returns Err, leaves tier state alone)
4. `chat_endpoint_request_construction`

**Risk:** ZERO. New file. Reads existing tier records and writes only `eval_accuracy` + `eval_n` fields. No production code path triggers this until Phase 7.

**Rollback:** delete `evaluator.rs`, revert `engine/mod.rs`, revert the `chat()` addition to `ollama.rs`.

---

### Phase 6 — Fix the state machine Training transition

**Scope:** replace the literal `Ok(None)` dead end at `state_machine.rs:33` with a check that lets the trainer drive the transition.

**File:** `crates/temm1e-distill/src/engine/state_machine.rs` line 33.

**Before:**
```rust
TierState::Training => Ok(None), // Training transitions handled by trainer
```

**After:**
```rust
TierState::Training => self.check_training_transition(tier, &record).await,
```

And add a new method:
```rust
async fn check_training_transition(
    &self,
    _tier: EigenTier,
    record: &TierRecord,
) -> Result<Option<TierState>, Temm1eError> {
    // The trainer is responsible for transitioning Training → Evaluating
    // (on success) or Training → Collecting (on failure). The state machine
    // tick should NOT auto-transition. However, we add a safety net: if
    // a tier has been Training for more than 1 hour with no current_run_id,
    // it's almost certainly stuck (trainer crashed) — recover to Collecting.
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

**Why this is still ZERO risk:** the new method preserves the original semantics (returns `None` in the common case) and only adds a safety net for stuck tiers. The trainer remains the authoritative driver.

**Test added:**
- `tier_stuck_in_training_for_an_hour_recovers_to_collecting`

**Risk:** ZERO. Behavior change is strictly additive (adds a recovery path that didn't exist; the original "stuck forever" behavior was a bug).

**Rollback:** revert the line 33 edit and remove the new method.

---

### Phase 7 — `lib.rs` `EigenTuneEngine::train()` public method

**Scope:** add the missing `pub async fn train(&self, tier: EigenTier) -> Result<(), Temm1eError>` method that calls the trainer + evaluator orchestrators in sequence.

**File:** `crates/temm1e-distill/src/lib.rs` (after line 138, near the existing hooks).

**Implementation:**
```rust
/// Run a complete training cycle for a tier.
///
/// Sequence: trainer → evaluator. Both must succeed for the tier to
/// reach Evaluating. On any failure the tier reverts to Collecting and
/// the error is propagated.
pub async fn train(&self, tier: EigenTier) -> Result<(), Temm1eError> {
    let trainer = engine::trainer::TrainerOrchestrator::new(
        self.store.clone(), self.config.clone());
    let artifacts = trainer.run(tier).await?;

    // The trainer transitions Training → Evaluating on success.
    // Now run the evaluator immediately. The evaluator writes
    // eval_accuracy/eval_n; the next tick() picks them up and
    // transitions Evaluating → Shadowing or → Collecting.
    let record = self.store.get_tier(tier.as_str()).await?;
    let run_id = record.current_run_id.clone()
        .ok_or_else(|| temm1e_core::types::error::Temm1eError::Internal(
            "Eigen-Tune: train completed without a run_id".into()))?;

    let evaluator = engine::evaluator::EvaluatorOrchestrator::new(
        self.store.clone(), self.config.clone());
    evaluator.run(tier, &run_id).await?;
    Ok(())
}
```

**Also import the new modules at the top of lib.rs:**
```rust
use crate::engine::trainer::TrainerOrchestrator;
use crate::engine::evaluator::EvaluatorOrchestrator;
```

**Tests:** integration test in `tests/proof_of_pipeline.rs` that constructs an in-mem store, seeds 100 pairs across categories, calls `engine.train(EigenTier::Simple).await` and asserts the tier transitions through the state machine. **Skipped on non-MLX hosts** via `#[cfg(target_os = "macos")]` + an environment guard `if std::env::var("EIGENTUNE_LIVE").is_err() { return; }`.

**Risk:** ZERO. New public method. Unreachable until Phase 8 wires it into the tick task.

**Rollback:** delete the method + the imports.

---

### Phase 8 — Periodic tick task in main.rs

**Scope:** spawn the tick loop (§A8) inside `Commands::Start`, gated by `eigentune_cfg.enabled`.

**File:** `src/main.rs`, near the existing heartbeat-spawn block (~line 2350 per the explore agent's map).

**New code (inserted near other `task_handles.push(...)` calls):**
```rust
// ── Eigen-Tune periodic tick ─────────────────────────────────────
if eigentune_cfg.enabled {
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
                    if to == temm1e_distill::types::TierState::Training {
                        let engine = et_engine.clone();
                        tokio::spawn(async move {
                            if let Err(e) = engine.train(tier).await {
                                tracing::warn!(
                                    error = %e, tier = %tier.as_str(),
                                    "Eigen-Tune: training cycle failed"
                                );
                            }
                        });
                    }
                }
            }
        }));
        tracing::info!("Eigen-Tune: periodic tick task spawned (60s interval)");
    }
}
```

**Risk:** ZERO. Only runs when `eigentune_cfg.enabled = true`. The default config has `enabled = false`, so users on default config see no behavior change. The task is a pure background loop with no user-visible effects beyond log lines.

**Rollback:** delete the `if eigentune_cfg.enabled { ... }` block.

---

### Phase 9 — Construct EigenTuneEngine in main.rs

**Scope:** load the `[eigentune]` config (per §A1, two-pass parsing), construct the engine if enabled, and inject it into the agent runtime.

**File:** `src/main.rs`, near the existing agent construction site (lines 2131-2171 per the explore agent's map).

**New code (inserted before `let agent = Arc::new(runtime);` at ~line 2170):**

```rust
// ── Load Eigen-Tune config (second pass — see plan A1) ──────────
let eigentune_cfg: temm1e_distill::config::EigenTuneConfig = {
    #[derive(serde::Deserialize, Default)]
    struct Root {
        #[serde(default)]
        eigentune: temm1e_distill::config::EigenTuneConfig,
    }
    let raw = std::fs::read_to_string(&config_path).unwrap_or_default();
    let expanded = temm1e_core::config::env::expand_env_vars(&raw);
    toml::from_str::<Root>(&expanded).map(|r| r.eigentune).unwrap_or_default()
};

// ── Instantiate EigenTuneEngine if enabled ──────────────────────
let eigen_tune_engine: Option<std::sync::Arc<temm1e_distill::EigenTuneEngine>> =
    if eigentune_cfg.enabled {
        let db_path = dirs::home_dir()
            .map(|h| h.join(".temm1e").join("eigentune.db"))
            .unwrap_or_else(|| std::path::PathBuf::from("eigentune.db"));
        // Ensure parent dir exists
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

// Inject into the runtime via the new builder
let runtime = if let Some(et) = eigen_tune_engine.clone() {
    runtime.with_eigen_tune(et)
} else {
    runtime
};

let agent = std::sync::Arc::new(runtime);
*agent_state.write().await = Some(agent);
```

**Note:** the tick task spawn (Phase 8) needs `eigen_tune_engine.clone()` so this construction MUST happen BEFORE the tick spawn block. Order in `main.rs` is: load eigentune_cfg → construct engine → inject into runtime → store in agent_state → spawn tick task.

**The `?` operator works:** the existing `Commands::Start` handler is `async fn` returning `Result`. If `EigenTuneEngine::new()` fails (e.g. SQLite write fails), we log + degrade to `None` rather than crashing the daemon. Existing users see no impact.

**Risk:** ZERO. The new code only executes when `eigentune_cfg.enabled = true`. With the default config:
- `eigentune_cfg = EigenTuneConfig::default()` (enabled = false)
- The `if eigentune_cfg.enabled` branch is never taken
- `eigen_tune_engine = None`
- The runtime is unchanged
- The tick task is not spawned
- **Net effect for existing users: zero new code paths exercised, zero new files touched on disk, zero new background tasks.**

**Rollback:** delete the new code block.

---

### Phase 10 — AgentRuntime field + builder method

**Scope:** add `eigen_tune: Option<Arc<EigenTuneEngine>>` field and `.with_eigen_tune()` builder. Mirror the existing pattern for `consciousness` (`runtime.rs:131`).

**File:** `crates/temm1e-agent/src/runtime.rs`.

**Edits:**

1. Add to the use list at the top:
```rust
use temm1e_distill::EigenTuneEngine;
```

2. Add to the struct (after line 146, the last existing field):
```rust
    /// Eigen-Tune self-tuning distillation engine. None = disabled.
    /// Hooks fire after every provider call to capture training pairs and
    /// observe quality signals. Fire-and-forget — never blocks the user.
    eigen_tune: Option<Arc<EigenTuneEngine>>,
```

3. Add to `Self::new()` (around line 187, the last field initialization):
```rust
            eigen_tune: None,
```

4. Add to `Self::with_limits()` (similar — end of the `Self { ... }` block around line 290):
```rust
            eigen_tune: None,
```

5. Add the builder method (next to `.with_consciousness()` and other `.with_*` methods):
```rust
    /// Inject the Eigen-Tune engine. When set, all five hooks fire
    /// after each provider call and tool execution. Fire-and-forget —
    /// errors are logged but never propagated to the user.
    pub fn with_eigen_tune(mut self, engine: Arc<EigenTuneEngine>) -> Self {
        self.eigen_tune = Some(engine);
        self
    }
```

**Risk:** ZERO. New field defaults to `None`. No existing code paths read it. Adding a struct field does not change the existing constructors' signatures (both are positional, no exhaustive struct expression in callers).

**Rollback:** revert all four edits.

---

### Phase 11 — Hook injection: collection (`on_completion`)

**Scope:** wire the post-call collection hook in `runtime.rs:1234` (immediately after `response` is bound, before `turn_api_calls += 1`).

**File:** `crates/temm1e-agent/src/runtime.rs`.

**New code (inserted at line 1235, just after `};` that closes the `match self.provider.complete(request)`):**
```rust
            // ── Eigen-Tune: collection hook (fire-and-forget) ──────
            if let Some(et) = &self.eigen_tune {
                let engine = et.clone();
                let pair_data = temm1e_distill::collector::EigenTunePairData {
                    messages_json: serde_json::to_string(&request.messages)
                        .unwrap_or_default(),
                    system_prompt: request.system.clone(),
                    tools_json: if request.tools.is_empty() {
                        None
                    } else {
                        Some(serde_json::to_string(&request.tools)
                            .unwrap_or_default())
                    },
                    response_json: serde_json::to_string(&response)
                        .unwrap_or_default(),
                    model: self.model.clone(),
                    provider: self.provider.name().to_string(),
                    complexity: complexity_str.clone(),  // captured at top of round, see Phase 13
                    conversation_id: msg.chat_id.clone(),
                    turn: session.history.len() as i32,
                    tokens_in: Some(response.usage.input_tokens as u32),
                    tokens_out: Some(response.usage.output_tokens as u32),
                    cost_usd: Some(call_cost),
                };
                tokio::spawn(async move {
                    engine.on_completion(pair_data).await;
                });
            }
```

**Note on `complexity_str`:** the variable is created in Phase 13 (the route hook). For now, this code references it as if it exists; Phase 13 is what introduces it. The phases are merged in order, so this is fine.

**Risk:** ZERO. The block only runs when `self.eigen_tune.is_some()`, which is only true when the user enabled `[eigentune]`. The hook is a `tokio::spawn`, so even if the engine's collector is slow or fails, the agent loop continues immediately to `turn_api_calls += 1` (line 1249).

**No latency added** for users with `enabled=false` (the entire `if let Some(et)` block is skipped).
**No latency added** for users with `enabled=true` (the work happens in a spawned task).

**Rollback:** delete the new block.

---

### Phase 12 — Hook injection: signals from tool execution

**Scope:** wire `ToolCallSucceeded` / `ResponseError` signals from the tool execution result branches.

**File:** `crates/temm1e-agent/src/runtime.rs:1879-1905`.

**New code (inserted immediately after the existing `let result = execute_tool(...)` line, in both the success and failure branches):**

Around line 1915-1949 (where the existing code matches `Ok` vs `Err`):
```rust
            // ── Eigen-Tune: tool result signal (fire-and-forget) ───
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

**Risk:** ZERO. Only runs when engine is enabled. Fire-and-forget. No effect on tool execution result handling.

**Rollback:** delete the new block.

---

### Phase 13 — Hook injection: full routing wrapper around `provider.complete()` (with shadow + monitor + local serve)

**Scope:** wrap `self.provider.complete(request).await` at `runtime.rs:1191` with the routing-aware version that handles all four `RouteDecision` cases: `Cloud` (default), `Local` (serve from distilled model), `Shadow` (cloud serves, local runs in parallel for SPRT evidence), `Monitor` (local serves, cloud sampled for CUSUM drift detection).

**Pre-requisite — capture `eigentune_complexity` variable.** The original explore agent's report was incorrect: the `complexity` variable at `runtime.rs:847` is local to the LLM-failure fallback `Err(e)` block (lines 843-867) and goes out of scope at line 867. The success path at lines 810-840 uses a different enum (`crate::llm_classifier::TaskDifficulty`). To get a tier string at line 1191, we must declare a new mut variable at line 416 (alongside `classification_label` and `difficulty_label`) and set it explicitly in BOTH classification branches. Full code in `CODE_ANCHORS.md` §3 Phase 11.

**File:** `crates/temm1e-agent/src/runtime.rs`, lines 1180-1234.

**The full replacement is large (~120 lines).** It is documented verbatim in `CODE_ANCHORS.md` §3 Phase 12. The key shape:

```rust
// Determine routing
let route_decision = if let Some(et) = &self.eigen_tune {
    if request.tools.is_empty() && eigentune_cfg.enable_local_routing {
        et.route(&eigentune_complexity).await
    } else {
        RouteDecision::Cloud  // tools-bearing requests OR local routing not opted in
    }
} else {
    RouteDecision::Cloud
};

let response = match route_decision {
    RouteDecision::Cloud => /* default path - existing logic */,
    RouteDecision::Local(endpoint) => /* try local with 30s timeout, fallback to cloud */,
    RouteDecision::Shadow(endpoint) => /* cloud serves; local runs in parallel; on_shadow_observation */,
    RouteDecision::Monitor(endpoint) => /* local serves; cloud sampled at 5%; on_monitor_observation */,
};
```

**Safety chain enforcement** (see `LOCAL_ROUTING_SAFETY.md` for full detail):
- **Gate 2** (tool-use guard): `if request.tools.is_empty()` — tool-bearing requests never route locally
- **Gate 5** (timeout + fallback): every local call wrapped in `tokio::time::timeout(Duration::from_secs(30), ...)` with automatic cloud fallback on Err or timeout
- **Double opt-in**: requires `enabled = true` (engine instantiated → `Some(et)`) AND `enable_local_routing = true` (config field added in Phase 1)
- **Gate 4** (CUSUM): the Monitor branch spawns a fire-and-forget cloud comparison that feeds `engine.on_monitor_observation()`, which auto-demotes the tier on alarm

**Borrow-checker note:** the Cloud, Local, Monitor, Shadow branches all need access to `request`, so we use `request.clone()` everywhere. `CompletionRequest` derives `Clone` (verified at `crates/temm1e-core/src/types/message.rs:43`).

**Risk:** ZERO for default-config users (the `Some(et)` check fails, we go through the unchanged Cloud branch). ZERO for `enabled=true, enable_local_routing=false` users (the second opt-in fails, we go through Cloud). LOW for `enable_local_routing=true` users — local calls are timeout-bounded and have cloud fallback; the seven-gate safety chain protects them; CUSUM detects any drift.

**Rollback:** revert Phase 13's edit. The original `let response = match self.provider.complete(request).await { ... }` block is preserved verbatim inside the `RouteDecision::Cloud` branch.

---

### Phase 14 — Hook injection: signals from user message arrival

**Scope:** detect retry/rejection on incoming user messages, fire `UserRetried` / `UserRejected` signals.

**File:** `crates/temm1e-agent/src/runtime.rs`, near the start of `process_message()` (~line 400-450 per the explore agent's map).

**New code (inserted after the user message is added to history but before the provider call):**

```rust
        // ── Eigen-Tune: user-message signal (fire-and-forget) ────────
        if let Some(et) = &self.eigen_tune {
            // Find the previous user message in this session, if any.
            let prev_user = session.history.iter().rev()
                .find(|m| matches!(m.role, MessageRole::User))
                .and_then(|m| m.content.first().and_then(|c| match c {
                    ContentPart::Text(t) => Some(t.clone()),
                    _ => None,
                }));
            let elapsed_secs = session.last_user_message_at
                .map(|ts| (chrono::Utc::now() - ts).num_seconds().max(0) as u64)
                .unwrap_or(0);
            let (agree, signal_kind) = temm1e_distill::judge::behavior::behavior_observation(
                &user_text,
                prev_user.as_deref(),
                elapsed_secs,
                false, // tool_failed: no tool yet on incoming msg
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

**Pre-requisite:** the `Session` struct needs a `last_user_message_at: Option<DateTime<Utc>>` field. **Verify in the actual `runtime.rs` code** — if the field doesn't exist, this is a NEW field on the session struct (small additive change), or we use the timestamp on the latest history entry. The cleanest path: read the timestamp from `session.history.last().map(|m| m.timestamp)` if `Message` already has a timestamp field. **If neither exists, fall back to passing `0` for `elapsed_secs` (which makes the retry detection always-on regardless of time).**

**Risk:** ZERO if the existing Session has timestamp data. LOW (acceptable) if we have to add a new optional field. Signals fire-and-forget.

**Rollback:** delete the new block.

---

### Phase 15 — CLI subcommand `temm1e eigentune ...`

**Scope:** new clap subcommand with four nested commands: `status`, `setup`, `model [name]`, `tick`.

**Files:** `src/main.rs`.

**Edits:**

1. Add to the existing `Commands` enum (around line 108, after the `Status` variant):
```rust
    /// Manage Eigen-Tune (self-tuning knowledge distillation)
    Eigentune {
        #[command(subcommand)]
        command: EigentuneCommands,
    },
```

2. Add the new subcommand enum (after the existing `SkillCommands` enum, around line 158-167):
```rust
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
}
```

3. Add the dispatch handler in the main `match cli.command { ... }` block (~line 1461). Add a new arm:
```rust
        Commands::Eigentune { command } => {
            // Read [eigentune] config from disk
            let config_path = config.clone().unwrap_or_else(|| "temm1e.toml".to_string());
            let eigentune_cfg = load_eigentune_config(&config_path).unwrap_or_default();
            // Open the same SQLite database the daemon uses
            let db_path = dirs::home_dir()
                .map(|h| h.join(".temm1e").join("eigentune.db"))
                .unwrap_or_else(|| std::path::PathBuf::from("eigentune.db"));
            let db_url = format!("sqlite:{}", db_path.display());
            // Create the engine read-only-ish (it'll create the file if missing)
            let engine = match temm1e_distill::EigenTuneEngine::new(&eigentune_cfg, &db_url).await {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Eigen-Tune: failed to open store: {e}");
                    std::process::exit(1);
                }
            };

            match command {
                EigentuneCommands::Status => {
                    println!("{}", engine.format_status().await);
                }
                EigentuneCommands::Setup => {
                    println!("{}", format_setup_instructions(&engine).await);
                }
                EigentuneCommands::Model { name } => {
                    if let Some(name) = name {
                        let mut engine = engine;
                        let msg = engine.set_model(&name);
                        println!("{}", msg);
                        // Note: this only updates in-memory; persistent change
                        // requires editing temm1e.toml manually. Document this.
                        println!("(Note: this is a session-only change. To persist,");
                        println!(" edit [eigentune] base_model = \"{}\" in temm1e.toml)", name);
                    } else {
                        println!("{}", engine.format_model_status().await);
                    }
                }
                EigentuneCommands::Tick => {
                    let transitions = engine.tick().await;
                    if transitions.is_empty() {
                        println!("Eigen-Tune: no tier transitions");
                    } else {
                        for (tier, from, to) in transitions {
                            println!("Eigen-Tune: {} {} → {}",
                                tier.as_str(), from.as_str(), to.as_str());
                        }
                    }
                }
            }
        }
```

4. Add the helper functions at module scope:
```rust
fn load_eigentune_config(config_path: &str) -> Option<temm1e_distill::config::EigenTuneConfig> {
    #[derive(serde::Deserialize, Default)]
    struct Root {
        #[serde(default)]
        eigentune: temm1e_distill::config::EigenTuneConfig,
    }
    let raw = std::fs::read_to_string(config_path).ok()?;
    let expanded = temm1e_core::config::env::expand_env_vars(&raw);
    toml::from_str::<Root>(&expanded).ok().map(|r| r.eigentune)
}

async fn format_setup_instructions(engine: &temm1e_distill::EigenTuneEngine) -> String {
    let prereqs = engine.check_prerequisites().await;
    let mut out = String::from("EIGEN-TUNE SETUP\n\n");
    out.push_str(&format!("Ollama: {}\n",
        if prereqs.ollama_running { "running ✓" } else { "not running — brew install ollama && ollama serve" }));
    out.push_str(&format!("Python: {}\n", prereqs.python_version.as_deref().unwrap_or("not found")));
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        out.push_str(&format!("MLX: {}\n",
            if prereqs.mlx_installed { "installed ✓" } else { "not found — pip install mlx-lm" }));
    } else {
        out.push_str(&format!("Unsloth: {}\n",
            if prereqs.unsloth_installed { "installed ✓" } else { "not found — pip install unsloth" }));
    }
    out.push_str(&format!("Can collect: {}\nCan train: {}\nCan serve: {}\n",
        prereqs.can_collect, prereqs.can_train, prereqs.can_serve));
    out
}
```

**Risk:** ZERO. New subcommand only runs when the user explicitly types `temm1e eigentune ...`. Existing subcommands are untouched.

**Rollback:** delete all four edits.

---

### Phase 16 — Slash command dispatch (gateway path + CLI chat path)

**Scope:** add `/eigentune` slash command handlers in BOTH parsers (per the explore agent's finding that the parsers are duplicated).

**File:** `src/main.rs`.

**Edits:**

1. **Gateway path** (~line 3361, in the `/help` text and ~line 2977 in the dispatch). Add the dispatch branch BEFORE the existing `/help` branch:
```rust
            if cmd_lower.starts_with("/eigentune") {
                let arg = msg_text_cmd.trim()["/eigentune".len()..].trim().to_string();
                let reply_text = handle_eigentune_slash(&arg, agent_state.clone(),
                    eigen_tune_engine.clone()).await;
                let reply = temm1e_core::types::message::OutboundMessage {
                    chat_id: msg.chat_id.clone(),
                    text: reply_text,
                    reply_to: Some(msg.id.clone()),
                    parse_mode: None,
                };
                send_with_retry(&*sender, reply).await;
                is_heartbeat_clone.store(false, Ordering::Relaxed);
                return;
            }
```

2. **CLI chat path** (~line 6033 in the `/help` text and ~line 5955 in the dispatch). Add a similar branch.

3. Add the shared handler at module scope:
```rust
async fn handle_eigentune_slash(
    arg: &str,
    _agent_state: Arc<RwLock<Option<Arc<temm1e_agent::AgentRuntime>>>>,
    engine: Option<Arc<temm1e_distill::EigenTuneEngine>>,
) -> String {
    let engine = match engine {
        Some(e) => e,
        None => return "Eigen-Tune is not enabled. Set [eigentune] enabled = true in temm1e.toml and restart.".to_string(),
    };
    let parts: Vec<&str> = arg.split_whitespace().collect();
    match parts.as_slice() {
        [] | ["status"] => engine.format_status().await,
        ["setup"] => format_setup_instructions(&engine).await,
        ["model"] => engine.format_model_status().await,
        ["model", name] => {
            // Note: cannot mutate Arc<EigenTuneEngine>. The slash command
            // can only display; persistent change requires editing temm1e.toml.
            format!("To change the base model, edit [eigentune] base_model = \"{name}\" in temm1e.toml and restart.")
        }
        ["tick"] => {
            let t = engine.tick().await;
            if t.is_empty() { "Eigen-Tune: no tier transitions".to_string() }
            else {
                let mut out = String::new();
                for (tier, from, to) in t {
                    out.push_str(&format!("Eigen-Tune: {} {} → {}\n",
                        tier.as_str(), from.as_str(), to.as_str()));
                }
                out
            }
        }
        _ => "Eigen-Tune: usage: /eigentune [status|setup|model [name]|tick]".to_string(),
    }
}
```

4. **Pass `eigen_tune_engine` into the slash command scope.** In the gateway dispatch closure, capture `eigen_tune_engine.clone()` alongside other captured variables.

5. **Update the `/help` text in BOTH parsers** to add the eigentune lines.

**Risk:** LOW. The dispatch lookup runs on every incoming message (both paths). The new branch only fires on `/eigentune` prefix and even then degrades gracefully when engine is None. Adding a branch to a chain of `if cmd_lower == "/foo"` checks is purely additive — existing branches are unchanged.

**The reason this is LOW not ZERO:** the slash command parsers are critical message-handling code. Any edit to them needs careful review. The change is small (4 lines per parser) and isolated, but the blast radius is "every incoming message." For ZERO classification, we'd want a separate test that exercises all existing slash commands to confirm none broke. **Phase 13's test suite includes such a regression test.**

**Rollback:** revert all four edits.

---

### Phase 17 — Documentation alignment (fixes the "snake oil" problem)

**Scope:** update every doc that overclaims Eigen-Tune to match reality.

**Files:**
- `tems_lab/eigen/SETUP.md` — replace the misleading "That's it. Restart and it works." with the actual setup steps. Add the "Supported base model families" section (Llama/Mistral/Gemma for MVP). Add a "Status: production beta" banner.
- `README.md:1046` — update the v3.1.0 changelog entry. The "proven on M2" claim is true (the manual fine-tune happened), but reword to clarify it was a research proof, not a shipped feature. Add a v4.9.0 entry: *"Eigen-Tune: closed-loop pipeline now wired into runtime — collection, training, evaluation, graduation all functional. Issue #35 fixed."*
- `README.md:821` — the architecture tree entry is accurate after this PR ships, no change needed (verify after Phase 16).
- `CLAUDE.md:71` — add a parenthetical: `temm1e-distill -- Eigen-Tune: self-tuning distillation engine (gated by [eigentune] enabled = true)`.
- `tems_lab/perpetuum/IMPLEMENTATION_PLAN.md:57` — change `"EigenTune (distillation closed-loop) | Built, integrated"` to `"EigenTune (distillation closed-loop) | Built, integrated as of v4.9.0"` (or whatever version this ships in).
- `docs/lab/cambium/THEORY.md:302` — no change required after this PR (the claim becomes accurate).
- `crates/temm1e-perpetuum/src/conscience.rs:163` — the comment is fine, just clarifies that EigenTune signals dream completion externally.

**Risk:** ZERO. Documentation only. No code paths affected.

**Rollback:** revert the doc edits.

---

### Phase 18 — Test suite

**Scope:** add unit tests per new module + an integration test for the full pipeline.

**Files:**
- `crates/temm1e-distill/src/curator.rs` (10 unit tests, listed in Phase 1)
- `crates/temm1e-distill/src/backends/mlx.rs` (3 unit tests, listed in Phase 2)
- `crates/temm1e-distill/src/backends/unsloth.rs` (5 unit tests, listed in Phase 3)
- `crates/temm1e-distill/src/engine/trainer.rs` (5 unit tests, listed in Phase 4)
- `crates/temm1e-distill/src/engine/evaluator.rs` (4 unit tests, listed in Phase 5)
- `crates/temm1e-distill/src/engine/state_machine.rs` (1 new test for the recovery path, Phase 6)
- `crates/temm1e-distill/tests/integration_full_loop.rs` (NEW): in-mem store + mock backend → 100 fake pairs → engine.train() → asserts tier transitions through Collecting → Training → Evaluating → (passes Wilson lower bound) → Shadowing.

**Plus a regression test for the slash command parser:**
- `tests/slash_command_dispatch.rs` in `src/`: parses every existing slash command (`/addkey`, `/keys`, `/removekey`, `/help`, `/usage`, `/memory`, `/cambium`, `/mcp`, `/browser`, `/timelimit`, `/reload`, `/reset`, `/restart`, `/eigentune`) and asserts they all dispatch to handlers (no "unknown command" responses).

**Total new tests:** 33.

**Risk:** ZERO. New tests only.

**Verification:**
```bash
cargo test --workspace -p temm1e-distill
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

---

### Phase 19 — 10-turn live self-test

**Scope:** the user's `Multi-turn CLI self-test protocol` from CLAUDE.md memory. After all phases compile and tests pass, run a live 10-turn conversation against the binary with `[eigentune] enabled = true` and verify:

1. Build release: `cargo build --release --bin temm1e`
2. Reset: `rm -f ~/.temm1e/{memory.db,eigentune.db}`
3. Source env, run the 10-turn script (per CLAUDE.md memory protocol)
4. Verify:
   - All 10 turns receive responses (no panics, no provider errors)
   - `sqlite3 ~/.temm1e/eigentune.db "SELECT COUNT(*) FROM eigentune_pairs;"` returns ≥ 8 (not all turns generate pairs — some are rejected by collector domain classification)
   - `sqlite3 ~/.temm1e/eigentune.db "SELECT tier, state, pair_count FROM eigentune_tiers;"` shows non-zero pair counts
   - `temm1e eigentune status` prints a non-empty report
   - The `/tmp/temm1e.log` contains `Eigen-Tune: collected training pair` and no error/warn lines from eigentune subsystem

**Risk:** ZERO. This is a verification step, not a code change. If anything fails, we go back and fix the relevant phase before merging.

---

### Phase 20 — Live training smoke test (gated, Apple Silicon only)

**Scope:** on a Mac with `mlx-lm` installed, manually trigger a tier transition to Training and verify the trainer actually runs.

**Steps:**
1. Lower `min_pairs` to 8 in temm1e.toml (just for this test)
2. Run a 10-turn conversation that produces pairs in the Simple tier
3. Run `temm1e eigentune tick`
4. Observe tier transition to Training in `/tmp/temm1e.log`
5. Verify a `TrainingRun` row appears with `status=running`, then `completed`
6. Verify a new Ollama model `eigentune-simple-<run_id>` is registered (`ollama list`)
7. Reset min_pairs to default

**Risk:** ZERO for the codebase. This test runs only on developer machines and validates the trainer end-to-end without affecting production.

---

### Phase 21 — Doc finalization & PR

**Scope:** write the PR description, update CHANGELOG, run the release protocol from CLAUDE.md memory.

**Files:**
- `CHANGELOG.md` (if exists) or PR body — list every phase
- `Cargo.toml` workspace version bump (per the user's release protocol — version is the source of truth)
- `README.md` — update the version badge, hero line, metrics table per the release protocol
- `CLAUDE.md` — update the test count

**Risk:** ZERO. Doc + version changes only. Release protocol handles the rest.

---

## 4. Deferred / opt-in phases (NOT in this PR)

These phases exist for completeness but are explicitly NOT in scope for this PR. **Local routing and shadow comparison are NO LONGER deferred — they are folded into Phase 13** with the seven-gate safety chain documented in `LOCAL_ROUTING_SAFETY.md`. The remaining deferrals are nice-to-haves with their own dependency surfaces.

### Phase O (deferred) — General-purpose data mixing

**What:** the curator mixes ~10% general-purpose instruction-following data into training datasets to prevent catastrophic forgetting.

**Why deferred:** requires a curated general dataset (e.g. OpenOrca subset). Either bundled as a JSONL file (~50 MB) or downloaded at first training. Adds dependency footprint.

**Risk if shipped:** LOW. Pure curator addition. Defer until users report forgetting.

### Phase P (deferred) — GGUF conversion fallback for unsupported model families

**What:** if the base model family doesn't support Ollama's `ADAPTER` directive (Qwen, Phi, SmolLM), call `mlx_lm.fuse --de-quantize` then `python -m llama_cpp.convert` to produce a GGUF file.

**Why deferred:** requires `llama_cpp` Python package, adds another subprocess, brittle quantization conversion. MVP restricts base models to ADAPTER-supported families (Llama, Mistral, Gemma).

### Phase Q (deferred) — Teacher judge

**What:** `judge/teacher.rs` — opt-in LLM-as-judge for higher-confidence shadow comparisons. Uses position debiasing (compares both A,B and B,A orderings).

**Why deferred:** costs LLM API money. Opt-in via `teacher_enabled = true`. The behavior + embedding judges are sufficient for MVP.

### Phase R (deferred) — `hf_autotrain.rs` backend

**What:** HuggingFace AutoTrain as a third backend (cloud GPU bursts).

**Why deferred:** requires HF API key, billing setup, network reliability assumptions. MLX + Unsloth cover the local case.

### Phase S (deferred) — Tool-use trained local models

**What:** train and evaluate local models on tool-use data, gate per-tier graduation on tool-use accuracy, lift the Gate 2 tool-use guard for tiers proven capable.

**Why deferred:** small open models historically struggle with function calling. Verifying capability adds another evaluator step. The MVP's "tools always go to cloud" guard is the safer initial position.

---

## 5. Scenario matrix — every existing user scenario

Verifying that **no existing user behavior changes** under any of the 19 ZERO-risk phases.

| # | Scenario | Default config (`enabled=false`)? | After this PR? |
|---|---|---|---|
| 1 | Telegram user sends message → Anthropic provider → reply | ✅ unchanged | ✅ unchanged (engine = None, no hooks fire) |
| 2 | CLI chat user sends message → reply | ✅ unchanged | ✅ unchanged |
| 3 | User runs `temm1e start` and the daemon starts | ✅ unchanged | ✅ unchanged (no eigentune init unless enabled) |
| 4 | User runs `temm1e status` | ✅ unchanged | ✅ unchanged (status is a sibling subcommand) |
| 5 | User runs any existing slash command (`/addkey`, `/keys`, `/help`, etc.) | ✅ unchanged | ✅ unchanged (the new `/eigentune` branch is added BEFORE the others, but only matches its own prefix; existing branches are untouched) |
| 6 | User has `[eigentune] enabled = true` for the first time | (didn't work before) | ✅ pairs are collected, tier states advance, training runs (if backend present) |
| 7 | User has `[eigentune] enabled = false` (default) | ✅ unchanged | ✅ unchanged — engine is `None`, every hook check is `if let Some(et) = ...` and skips |
| 8 | User restarts daemon — does the eigentune SQLite file appear? | (didn't exist) | ⚠️ ONLY if `enabled=true`. If false, no file is created. **Verified** by Phase 9 code path. |
| 9 | Tool execution fails | ✅ unchanged | ✅ unchanged for non-eigentune users; eigentune users see a `ResponseError` signal recorded |
| 10 | Provider returns 400 → fallback to prompted mode | ✅ unchanged | ✅ unchanged (the collection hook is INSIDE the `Ok` branch, so failed calls don't generate pairs) |
| 11 | User panics on Vietnamese text (the historical `ẹ` boundary bug) | ✅ unchanged (catch_unwind catches it) | ✅ unchanged (the eigentune hook is post-response, so the panic happens before it) |
| 12 | Existing 1638 tests | ✅ pass | ✅ pass (no existing test touches the new code; new tests are additive) |
| 13 | `cargo clippy --workspace -- -D warnings` | ✅ pass | ✅ pass (new code is clippy-clean by construction) |
| 14 | `cargo fmt --all -- --check` | ✅ pass | ✅ pass |
| 15 | User on Windows with eigentune disabled | ✅ unchanged | ✅ unchanged (no MLX/Unsloth check happens) |
| 16 | User on Windows with eigentune enabled | (didn't work) | ⚠️ collection works, training fails with clear error, status shows "no backend" |
| 17 | Cargo build time | ✅ baseline | +~5s (the distill crate is now compiled by the binary; was already compiled by `cargo test --workspace`) |
| 18 | Binary size | ✅ baseline | +~80 KB (the distill crate's code, no new deps) |
| 19 | Memory footprint when `enabled=false` | ✅ baseline | ✅ unchanged (engine never instantiated; the struct field is `Option<Arc<...>>` = 8 bytes) |
| 20 | Memory footprint when `enabled=true` | (didn't apply) | +~5 MB (SQLite connection pool, in-memory caches) |

**No scenario regresses.** The PR is purely additive for default-config users.

---

## 6. Cross-platform compatibility matrix

| Component | macOS-arm64 | macOS-x86_64 | Linux x86_64 | Windows x86_64 |
|---|---|---|---|---|
| Collection (collector + store + state machine) | ✅ | ✅ | ✅ | ✅ |
| Periodic tick task | ✅ | ✅ | ✅ | ✅ |
| CLI subcommand `temm1e eigentune status` | ✅ | ✅ | ✅ | ✅ |
| Slash command `/eigentune status` | ✅ | ✅ | ✅ | ✅ |
| MLX backend (Phase 2) | ✅ (when `mlx-lm` installed) | ❌ | ❌ | ❌ |
| Unsloth backend (Phase 3) | ⚠️ slow (CPU/MPS only) | ⚠️ slow (CPU only) | ✅ (CUDA) | ⚠️ flaky (use WSL2) |
| Trainer end-to-end | ✅ via MLX | ❌ | ✅ via Unsloth | ⚠️ |
| Evaluator (Ollama chat) | ✅ | ✅ | ✅ | ✅ |
| Ollama model creation via `ADAPTER` directive | ✅ | ✅ | ✅ | ✅ |
| Live SPRT/CUSUM monitoring | ✅ | ✅ | ✅ | ✅ |

**Failure mode on platforms without a training backend:** the trainer returns `Err(Temm1eError::Tool("no backend"))`, the tier transitions back to `Collecting`, and `temm1e eigentune status` shows the failure reason. **The user is never confused about why training isn't progressing.**

---

## 7. Security audit

### S1 — Subprocess spawning (trainer)

**Risk:** the trainer spawns `python3 -m mlx_lm.lora` and `python3 scripts/eigentune_unsloth.py`. Subprocess execution is a security boundary.

**Mitigations:**
- All arguments are passed via `Command::arg()` (not via shell), so there's no shell interpolation. No risk of command injection from base model names or paths.
- Base model names come from `EigenTuneConfig::base_model`, which is loaded from `temm1e.toml`. This file is owned by the user — if an attacker can write to it, they already have full control. **Not a new attack surface.**
- Dataset paths are constructed from `config.artifacts_dir + UUID`. The UUID is generated locally with `uuid::Uuid::new_v4()`. No user input flows into the path.
- Python script (`scripts/eigentune_unsloth.py`) is vendored in the repo. Verified at install time (sha256 hash in CI). No runtime download.
- The Python script does NOT receive any user message text directly. It reads JSONL files from a directory the trainer wrote. The JSONL files contain training pair messages, but those are user data the user already chose to capture by enabling `[eigentune]`.

**Assessment:** ZERO new attack surface. The trainer's only path-traversal risk is if `config.artifacts_dir` is set to a sensitive location and a path-aware attacker controls the UUID — but UUIDs are non-attacker-controlled.

### S2 — Conversation data persistence

**Risk:** every captured pair is stored in `~/.temm1e/eigentune.db`, including user messages and assistant responses. This is sensitive data.

**Mitigations:**
- Storage is local-only (SQLite file). No network transmission unless the user explicitly enables `teacher_enabled` (deferred phase Q).
- The collector's `classify_domain` function uses heuristic keyword matching (`collector.rs:171-254`). It does not call any LLM or send data anywhere.
- Retention policy already exists: `prune_old_low_quality(retention_days)` deletes old low-quality pairs. Default `retention_days = 180`.
- Disk space: each pair is ~1-5 KB. With `max_pairs_per_tier = 5000` and 3 tiers, the worst case is ~75 MB.

**New requirement:** add a `[eigentune]` config example to SETUP.md that documents:
- "Captured data lives in `~/.temm1e/eigentune.db` and never leaves your machine."
- "To disable and remove all data: set `enabled = false`, restart, then `rm ~/.temm1e/eigentune.db`."

**Assessment:** ZERO new risk over what the user already accepted by enabling the feature. The data is what they already chose to type into the AI agent.

### S3 — Adapter file integrity

**Risk:** the trainer outputs `adapter.safetensors`, which gets loaded by Ollama via the Modelfile `ADAPTER` directive. If an attacker can modify this file between training and Ollama load, they can inject behavior.

**Mitigations:**
- Output dir is under `~/.temm1e/eigentune/` (user-owned).
- Time-of-check / time-of-use window between trainer write and Ollama read is < 1 second.
- An attacker with write access to `~/.temm1e/` already has full control.

**Assessment:** not a new risk surface.

### S4 — Credential isolation

**Risk:** the eigentune engine has access to provider responses, which may contain rendered API keys or credentials.

**Mitigations:**
- The existing `SecretCensorChannel` wrapper at `src/main.rs:25-60` censors known API keys from outbound messages. The eigentune collector reads from `request.messages` and `response`, which are PRE-censor — but those are also already in agent memory, history database, and budget logs. No new exposure.
- The `domain_category` classifier explicitly looks for `/keys`, `/addkey`, `/eigen` in messages (see `collector.rs:243-249`) and routes them to `meta` category. Future enhancement: skip `meta` category from training datasets entirely (single-line filter in curator).

**Assessment:** ZERO new exposure. Eigentune sees the same data as the agent's memory backend already does.

### S5 — Embedding model download (`nomic-embed-text`)

**Risk:** the evaluator depends on a local embedding model served by Ollama. The first time it's called, it pulls `nomic-embed-text` from the Ollama registry (~270 MB).

**Mitigations:**
- The pull only happens during evaluation (not during collection). Users with `enabled=true` but never reaching Evaluating state never trigger it.
- The embedding model is signed by Ollama's registry. No supply chain risk beyond what Ollama users already accept.
- Disk usage: ~270 MB.

**Assessment:** acceptable. Document in SETUP.md.

### S6 — SQLite injection

**Risk:** the store uses `sqlx::query("...").bind(...)` everywhere except `update_signal()` which uses string interpolation for the column name (`store.rs:281-291`).

**Mitigation:** the column name is allowlist-validated against a hardcoded set (`user_continued`, `user_retried`, `tool_success`, `response_error`). Any other value returns `Err`. **No injection possible.**

**Assessment:** existing code is safe. Verified in Phase 1 review.

### S7 — Resource exhaustion

**Risk:** the trainer can consume large amounts of CPU/GPU/memory during a training run.

**Mitigations:**
- Training only runs when a tier reaches the `min_pairs` threshold (default 200) AND diversity entropy passes (default 0.7). This is a high bar; rapid-fire training is impossible.
- Only ONE tier trains at a time (the trainer is spawned in a child task per transition).
- Failure of a training run is graceful (logged, tier reverts). No crash or hang.
- The user can `kill -TERM <temm1e>` to stop the daemon; the training subprocess inherits the parent's process group and dies with it.

**Assessment:** acceptable. Document in SETUP.md that "training runs may take 5-30 minutes depending on hardware."

---

## 8. Open questions / decisions needed before implementation

The user's `feedback_zero_snake_oil.md` and `feedback_no_stubs.md` rules require me to surface every uncertainty BEFORE implementation. Here are the questions that must be answered (or accepted as documented limitations):

### Q1 — Two-pass TOML parsing or move EigenTuneConfig to temm1e-core?

§A1 chose the two-pass approach. The alternative is moving `EigenTuneConfig` to `temm1e-core` and having `temm1e-distill::config` re-export it. The move is cleaner architecturally but touches more files. **Recommendation:** stick with two-pass for the MVP; revisit in a refactor PR after the feature is shipped and stable.

### Q2 — Should the slash command `/eigentune model <name>` be allowed to mutate config?

The CLI subcommand and slash command can DISPLAY the current model and SUGGEST a change, but neither can mutate `temm1e.toml` (file is user-owned, edits would race with the user's editor). **Recommendation:** display only. Document the manual edit step. Future: add `temm1e config set [eigentune] base_model <name>` as a separate phase.

### Q3 — What happens if the training subprocess hangs forever?

The trainer's subprocess `wait()` has no timeout. A hung MLX or Unsloth process would prevent the tier from ever leaving Training (until the 1-hour state machine recovery in Phase 6 kicks in).

**Recommendation:** add a per-config training timeout `[eigentune] max_training_minutes = 60`. The trainer wraps the subprocess in `tokio::time::timeout()`. On timeout, the subprocess is killed (`child.kill()`) and the run is marked failed. **This adds 1 line of code to the trainer; do it in Phase 4.**

### Q4 — Should we add an `eigentune` feature flag despite §A2's recommendation?

The user might prefer compile-time gating for binary size reasons. The crate adds ~80 KB. **Recommendation:** no feature flag — runtime gating is simpler and matches the consciousness/perpetuum/social pattern. If the user wants compile-time gating, that's a separate refactor PR.

### Q5 — Should the `[eigentune]` SQLite database live alongside `memory.db` or in its own `eigentune.db`?

Currently planned as `~/.temm1e/eigentune.db` (separate file). Pros: clear ownership, easy to delete/reset. Cons: two SQLite connection pools.

**Recommendation:** separate file. Matches the existing pattern where each subsystem has its own table set in its own file (or in memory.db). The eigentune store has 4 tables and they're independent of agent memory.

### Q6 — How does the agent runtime get `complexity_str` if the existing classifier already produced it?

Phase 13 adds an inline match on `task_complexity`. **Verify:** the actual variable name in `runtime.rs` near line 1180. If the variable is `classification.difficulty` (a string) or `classification.task_complexity` (an enum), adjust the match accordingly. **If the variable doesn't exist at line 1180** (e.g. classification happens later), move the match to the right place and capture into a local variable.

This is a 5-minute verification during Phase 13 implementation.

### Q7 — Does `Session::history.last().map(|m| m.timestamp)` work for Phase 14?

Phase 14 needs a timestamp on the most recent user message to compute `elapsed_secs` for retry detection. **Verify:** does the existing `Message` type in `temm1e-core::types::message` have a timestamp field? If yes, use it. If no, pass `0` as a fallback (which means retry detection uses only edit-distance heuristics, not the time window — slightly weaker but still functional).

### Q8 — Should Phase 19's 10-turn self-test be mandatory for merge?

Per the user's `Multi-turn CLI self-test protocol` rule: yes. The test is mandatory after every code-touching PR. Run it before merging.

### Q9 — What should the default `base_model` be for "auto"?

Currently `lib.rs::recommend_models()` returns SmolLM2-135M-Instruct for low-RAM systems. **Should the trainer use this when `base_model = "auto"`?** Yes. Document in SETUP.md.

But: SmolLM2 is NOT in the ADAPTER-supported family list (Llama/Mistral/Gemma). For MVP, the auto recommendation should be:
- **Apple Silicon, ≤8 GB RAM:** `mlx-community/Llama-3.2-1B-Instruct-4bit`
- **Apple Silicon, ≤16 GB RAM:** `mlx-community/Llama-3.2-3B-Instruct-4bit`
- **Apple Silicon, ≥16 GB RAM:** `mlx-community/Mistral-7B-Instruct-v0.3-4bit`
- **NVIDIA, CUDA:** `unsloth/Llama-3.2-1B-Instruct-bnb-4bit` or `unsloth/Mistral-7B-Instruct-v0.3-bnb-4bit`

Update `lib.rs::recommend_models()` (line 567-630) to reflect this. **This is a small change (~20 lines), include in Phase 1 or Phase 2.**

### Q10 — How do we test the Ollama-create step end-to-end without hitting a real Ollama?

The tests in Phase 4 (trainer) use mock backends and skip the Ollama step. Phase 19 (10-turn live test) doesn't reach the Ollama step either (training takes minutes, doesn't fit in a 5-minute test).

**Recommendation:** Phase 20 (live training smoke test) is the only end-to-end test that exercises Ollama. It runs on developer machines, not in CI. **Acceptable** — CI can't realistically run a full training pipeline anyway.

---

## 9. Risk summary

| Phase | Description | Risk | Justification |
|---|---|---|---|
| 0 | Cargo dependency wiring | ZERO | Dep declarations only, no code runs |
| 1 | Curator module | ZERO | New file, unreachable from production until Phase 4 |
| 2 | MLX backend | ZERO | New file, unreachable until Phase 4 |
| 3 | Unsloth backend + Python wrapper | ZERO | New files, unreachable until Phase 4 |
| 4 | Trainer orchestrator | ZERO | New file, only invoked when tier in Training state (impossible without enabled=true) |
| 5 | Evaluator + ollama::chat helper | ZERO | New file + 1 helper function. Helper is unreachable until trainer calls it |
| 6 | State machine Training-stuck recovery | ZERO | Adds a recovery path for a state that was previously a dead end. Strictly additive |
| 7 | EigenTuneEngine::train() public method | ZERO | New method, unreachable until tick task spawns |
| 8 | Periodic tick task | ZERO | Spawned only when enabled=true |
| 9 | Construct engine in main.rs | ZERO | Only runs when enabled=true |
| 10 | AgentRuntime field + builder | ZERO | New optional field defaults to None |
| 11 | Collection hook | ZERO | Inside `if let Some(et)`. Default-disabled |
| 12 | Tool result signal hook | ZERO | Inside `if let Some(et)`. Default-disabled |
| 13 | Full routing wrapper (Cloud/Local/Shadow/Monitor) + complexity capture | LOW | Wraps `provider.complete()` at line 1191. Default-config users still hit the unchanged Cloud branch. Local/Shadow/Monitor branches gated by double opt-in (`enabled` AND `enable_local_routing`). Seven-gate safety chain enforced (see `LOCAL_ROUTING_SAFETY.md`). 30s timeout + automatic cloud fallback on every local call. |
| 14 | User message signal hook | ZERO | Gated on enabled |
| 15 | CLI subcommand | ZERO | New clap variant, dispatches to its own handler |
| 16 | Slash command (gateway + CLI) | LOW | Touches the slash dispatch paths in two places. Mitigated by Phase 18 regression test |
| 17 | Documentation alignment | ZERO | Doc-only |
| 18 | Test suite | ZERO | New tests only |
| 19 | 10-turn live self-test | ZERO | Verification step, no code change |
| 20 | Live training smoke test | ZERO | Manual verification, developer machines only |
| 21 | PR + release protocol | ZERO | Doc + version bump |

**Aggregate risk: ZERO for default-config users (no behavior change), LOW for users who flip BOTH opt-in switches.** The two LOW phases (13: routing wrapper, 16: slash commands) are mitigated by:
- Phase 13: seven-gate safety chain (`LOCAL_ROUTING_SAFETY.md`), automatic cloud fallback on any local failure, automatic CUSUM-driven demotion on drift
- Phase 16: explicit regression test exercising every existing slash command dispatch

**Default-config user impact:** zero new code paths exercised, zero new files on disk, zero new background tasks, +~80 KB binary size, +~5s cold-build time. The plan compiles and runs identically to v4.8.0 for any user who does not enable Eigen-Tune.

---

## 10. Implementation checklist (run order)

```
☐ Phase 0  — Cargo deps (Cargo.toml × 3)
☐ Phase 1  — curator.rs + 10 unit tests + add `enable_local_routing` field to EigenTuneConfig
☐ Phase 2  — backends/mlx.rs + 3 unit tests
☐ Phase 3  — backends/unsloth.rs + scripts/eigentune_unsloth.py + 5 unit tests
☐ Phase 4  — engine/trainer.rs + 5 unit tests + Q3 timeout + adapter integrity validation (Gate 6)
☐ Phase 5  — engine/evaluator.rs + ollama::chat helper + 4 unit tests
☐ Phase 6  — state_machine.rs Training recovery + 1 unit test
☐ Phase 7  — lib.rs::EigenTuneEngine::train() + 1 integration test
☐ Phase 8  — main.rs periodic tick task
☐ Phase 9  — main.rs engine construction + two-pass config load (eigentune_cfg available downstream)
☐ Phase 10 — runtime.rs AgentRuntime field + with_eigen_tune builder
☐ Phase 11 — runtime.rs eigentune_complexity capture (success + fallback paths) + collection hook
☐ Phase 12 — runtime.rs tool result signal hook
☐ Phase 13 — runtime.rs FULL routing wrapper (Cloud/Local/Shadow/Monitor) with seven-gate safety chain
☐ Phase 14 — runtime.rs user message signal hook
☐ Phase 15 — main.rs CLI subcommand (status, setup, model, tick, demote)
☐ Phase 16 — main.rs slash command (gateway + CLI, includes /eigentune demote)
☐ Phase 17 — Doc alignment (SETUP.md, README.md, CLAUDE.md)
☐ Phase 18 — Test suite (33 new tests + slash command regression test + LOCAL_ROUTING_SAFETY.md unit tests)
☐ Phase 19 — 10-turn live self-test (mandatory per CLAUDE.md protocol)
☐ Phase 20 — Live training smoke test (developer machines only)
☐ Phase 21 — PR + release protocol (version bump, README updates)
```

**Total LOC estimate:**
- New Rust code: ~1 600 LOC across 5 new files (curator, mlx, unsloth, trainer, evaluator) + ~50 LOC of edits to existing files
- New Python: ~80 LOC (eigentune_unsloth.py)
- New tests: ~600 LOC across 6 new test modules
- Doc updates: ~200 LOC

**Total touchpoint count:**
- New files: 6 (curator.rs, mlx.rs, unsloth.rs, trainer.rs, evaluator.rs, eigentune_unsloth.py) + 7 doc files
- Edited files: 9 (lib.rs, state_machine.rs, engine/mod.rs, backends/mod.rs, ollama.rs, runtime.rs, model_router.rs (maybe), main.rs, 3 Cargo.tomls)

**Touchpoints to existing critical paths:**
- `runtime.rs` — 5 hook injection points, all gated `if let Some(et)`
- `main.rs` — 1 construction site, 1 tick spawn, 1 CLI dispatch arm, 2 slash command branches
- `state_machine.rs` — 1 line edit (replace `Ok(None)` with method call) + 1 new method
- Everything else is purely additive

---

## 11. What this plan does NOT do

**Explicitly out of scope** (named so the user knows what to expect):

- ❌ No general-purpose data mixing (Phase O deferred). Trained models may exhibit catastrophic forgetting on out-of-distribution queries.
- ❌ No GGUF conversion fallback (Phase P deferred). Restricted to Llama/Mistral/Gemma family base models for MVP.
- ❌ No teacher judge (Phase Q deferred). Behavior + embedding judges only.
- ❌ No HF AutoTrain backend (Phase R deferred). MLX + Unsloth only.
- ❌ No tool-use trained local models (Phase S deferred). Tool-bearing requests always route to cloud (Gate 2 of the safety chain).
- ❌ No automatic config mutation. `temm1e eigentune model <name>` displays the current config and suggests an edit; the user must edit `temm1e.toml` manually.

**The MVP shipped by this plan is therefore:** "Eigen-Tune now collects, trains, evaluates, and **serves users with the distilled local model after the seven-gate safety chain passes**, when you opt in twice (`enabled = true` AND `enable_local_routing = true`). Tool-bearing requests always go to cloud. Drift is detected automatically and the tier auto-demotes. Manual emergency demote is available via `temm1e eigentune demote <tier>`." This is the full "Option B" resolution from issue #35 — the half-pipeline deferral is no longer needed because the safety chain (`LOCAL_ROUTING_SAFETY.md`) bounds the risk to LOW with automatic recovery on every failure mode.

---

## 12. Approval gates

Approved by the user on 2026-04-10:

1. ✅ **The two-pass TOML parsing approach** (§A1, Q1) — accepted.
2. ✅ **No Cargo feature flag** (§A2, Q4) — accepted, runtime gating only.
3. ✅ **Llama/Mistral/Gemma family restriction for MVP** (§A4, Q9) — accepted, GGUF conversion deferred.
4. ✅ **Slash command included** — Phase 18 ships in the main PR with a regression test.
5. ✅ **Local routing INCLUDED** — folded into Phase 13 with the seven-gate safety chain (`LOCAL_ROUTING_SAFETY.md`). Double opt-in (`enabled` AND `enable_local_routing`) defaults to off.

Implementation begins Phase 0 immediately upon handoff to `/production-grade`.

---

## 13. References

- Issue: temm1e-labs/temm1e#35 — *"Eigen-Tune: feature advertised as functional but pipeline is incomplete"*
- Original design: `tems_lab/eigen/DESIGN.md`
- Original implementation plan (which never finished): `tems_lab/eigen/IMPLEMENTATION.md`
- Setup guide (currently inaccurate, fixed in Phase 17): `tems_lab/eigen/SETUP.md`
- Research paper: `tems_lab/eigen/RESEARCH_PAPER.md`
- Technical reference: `tems_lab/eigen/TECHNICAL_REFERENCE.md`

External:
- [mlx-lm/LORA.md](https://github.com/ml-explore/mlx-lm/blob/main/mlx_lm/LORA.md) — MLX `lora` CLI reference
- [Ollama Modelfile reference](https://docs.ollama.com/modelfile) — `FROM` + `ADAPTER` directive
- [Unsloth docs](https://unsloth.ai/docs/basics/inference-and-deployment/saving-to-ollama) — Unsloth → Ollama deployment
- [mlx_lm.fuse + GGUF discussion](https://github.com/ml-explore/mlx/discussions/1507) — adapter fusion path (deferred)

---

**Plan author:** Claude (claude-opus-4-6[1m])
**Researched:** 2026-04-10
**Status:** awaiting user approval to begin Phase 0
