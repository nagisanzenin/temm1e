# Eigen-Tune — Local Routing Safety Protocol

> **Purpose:** spell out the complete safety chain that must be in place before the agent serves a user with a distilled local model. This document is the ZERO-RISK contract for local serving.
> **Companion docs:** `INTEGRATION_PLAN.md` (master plan), `CODE_ANCHORS.md` (paste-ready snippets).
> **Verified:** 2026-04-10, branch `eigen-revisit`.
> **Rule:** every gate in this document must be implemented and verified before the local routing code path can fire even once. No "we'll add the guard later." If a sub-agent tries to ship local routing without one of these gates, **stop and report**.

---

## 0. The threat model

What can go wrong when we serve a user with a local model instead of the cloud provider they expect?

| Failure | Mechanism | User-visible impact |
|---|---|---|
| **F1** Local model gives a worse answer | Distillation didn't capture full cloud capability | Wrong/incomplete reply, possibly costly action taken on wrong info |
| **F2** Local model hallucinates differently than cloud | Smaller models hallucinate more | Wrong reply |
| **F3** Local model fails to call a tool | Many small models lack function calling | Tool-bearing query gets a useless text-only reply |
| **F4** Ollama down or crashed | OS-level issue, disk full, OOM | Long wait, then timeout, then user sees an error |
| **F5** Local model cold-start latency | First request loads weights into RAM | 5-10s extra latency on first call |
| **F6** Drift after deployment | Domain shift, new types of queries | Quality degrades over time, undetected |
| **F7** Adapter file corruption | Disk issue, partial write | Local model produces gibberish or refuses to start |
| **F8** Wrong base model loaded | Modelfile points to deleted Ollama model | Local call fails immediately |
| **F9** Resource starvation | Local model competing for GPU/CPU with other processes | Slow or hung |
| **F10** Misclassified request reaches local | Classifier said "Simple" when query was actually complex | User gets a small-model answer to a hard question |

The safety chain below has a specific gate for each.

---

## 1. The seven-gate safety chain

Local routing only fires when **all seven gates** pass for a given (user, request, tier) tuple. Each gate has a specific code-level enforcement point.

### Gate 1 — Master kill switch (binary on/off)

**What:** the entire Eigen-Tune subsystem is gated by `[eigentune] enabled = false` (the default). Setting `enabled = false` and restarting the daemon stops all eigentune activity, including local routing.

**Where enforced:** `src/main.rs` Phase 16 — `if eigentune_cfg.enabled { ... }`. When false, the engine is never instantiated. The agent runtime's `eigen_tune` field is `None`. Every hook in the runtime checks `if let Some(et) = &self.eigen_tune` and skips when `None`.

**Recovery:** edit `temm1e.toml`, set `enabled = false`, restart `temm1e start`. Effect: immediate.

**Catches:** F1–F10 — turning the system off stops all of them.

### Gate 2 — Tool-use guard (request-level)

**What:** any request with `request.tools` non-empty is forced to `RouteDecision::Cloud`, regardless of tier state. Local models in MVP do not support function calling.

**Where enforced:** `crates/temm1e-agent/src/runtime.rs` Phase 12 routing block:
```rust
let route_decision = if let Some(et) = &self.eigen_tune {
    if request.tools.is_empty() {        // <-- Gate 2
        et.route(&eigentune_complexity).await
    } else {
        RouteDecision::Cloud              // <-- forced cloud for tool-bearing requests
    }
} else {
    RouteDecision::Cloud
};
```

**Recovery:** none needed — every tool-bearing request bypasses local automatically. Even users who have a graduated Simple tier still go to cloud when they ask for tool use.

**Catches:** F3 — eliminates the tool-call failure mode entirely.

**Future:** Phase Q (deferred) trains tool-use-capable local models and lifts this restriction per-tier. Not in this PR.

### Gate 3 — Statistical graduation gates (tier-level)

**What:** a tier only reaches `Graduated` state (where `route()` returns `Local`) after passing four mathematical gates in sequence:

1. **Collection gate:** ≥`min_pairs` (default 200) high-quality pairs collected, AND Shannon entropy of category distribution ≥ `diversity_target` (default 0.7).
   - **Where enforced:** `crates/temm1e-distill/src/engine/state_machine.rs:46-78` (`check_collecting_transition`)
   - **Catches:** insufficient training data, over-fitting to one category

2. **Training gate:** the trainer subprocess (mlx_lm.lora or unsloth) must exit with status 0. Any failure transitions the tier back to Collecting.
   - **Where enforced:** `crates/temm1e-distill/src/engine/trainer.rs` Phase 4
   - **Catches:** broken training data, broken backend, broken hardware

3. **Evaluation gate:** Wilson lower bound on holdout accuracy ≥ `graduation_accuracy` (default 0.95) at confidence level `graduation_confidence` (default 0.99).
   - **Where enforced:** `crates/temm1e-distill/src/engine/state_machine.rs:97-118` (`check_evaluating_transition`), uses `stats::wilson::wilson_lower`
   - **Concrete number:** with 30 eval samples, Wilson lower ≥ 0.95 at 99% CI requires **at least 29 of 30** to pass — that's 96.7% observed accuracy minimum
   - **Catches:** F1 (worse answers) — the math literally requires equivalent quality

4. **Shadow SPRT gate:** Sequential Probability Ratio Test must accept H₁ (`p₁ = 0.95`) over H₀ (`p₀ = 0.85`) at error rates `α = 0.05`, `β = 0.10`. Truncated at `sprt_max_samples = 200`.
   - **Where enforced:** `crates/temm1e-distill/src/engine/state_machine.rs:128-159` (`check_shadowing_transition`), uses `stats::sprt::Sprt`
   - **Concrete number:** SPRT typically needs 50-150 observations to make a decision. Each observation is a real user request where the local model's response was compared to the cloud's response.
   - **Catches:** F2 (different hallucinations) — measured against actual user queries, not synthetic eval

**Where the route decision reads tier state:** `crates/temm1e-distill/src/engine/router.rs:27-50`. Only `TierState::Graduated` returns `RouteDecision::Local`.

**Recovery:** if any gate fails, the tier automatically reverts to `Collecting`. The system never serves locally without proof.

**Catches:** F1, F2, F10 — proven equivalent quality at the complexity tier the user actually queries.

### Gate 4 — Continuous CUSUM drift detection (post-graduation)

**What:** after a tier graduates, every served local request feeds an observation into a CUSUM (Cumulative Sum) drift detector. If the cumulative deviation from `graduation_accuracy` exceeds `cusum_threshold` (default 5.0), the tier is **automatically demoted** back to Collecting.

**Where enforced:** `crates/temm1e-distill/src/engine/monitor.rs:33-80` and `crates/temm1e-distill/src/lib.rs:113-127` (`on_monitor_observation` callback). The observation comes from comparing the local response to a sampled cloud response (5% sample rate by default — `monitor_sample_rate`).

**Concrete behavior:**
- 5% of graduated-tier calls also trigger a parallel cloud call (fire-and-forget).
- The two responses are compared via `judge::embedding::cheap_equivalence_check` (free) or via Ollama embedding similarity (also free).
- The comparison feeds CUSUM. After ~50 disagreements, CUSUM alarms.
- On alarm: the tier is demoted, all subsequent requests for that tier go to cloud, the daemon logs a `warn!` line.

**Where to monitor:** `temm1e eigentune status` shows CUSUM s-statistic for each Graduated tier. `tail -f /tmp/temm1e.log | grep "CUSUM alarm"`.

**Recovery:** automatic. No human intervention needed.

**Catches:** F6 (drift) — the canonical statistical detector for slow degradation.

### Gate 5 — Per-call timeout + cloud fallback

**What:** every local provider call is wrapped in `tokio::time::timeout(Duration::from_secs(30), ...)`. If the local call times out OR returns an error, the routing logic immediately falls back to the cloud provider with the same request.

**Where enforced:** `crates/temm1e-agent/src/runtime.rs` Phase 12, `RouteDecision::Local` branch:
```rust
match tokio::time::timeout(
    std::time::Duration::from_secs(30),
    local_provider.complete(local_req),
).await {
    Ok(Ok(resp)) => resp,
    _ => {
        tracing::warn!("Eigen-Tune: local call failed/timed out, falling back to cloud");
        // Cloud call here ...
    }
}
```

**Concrete behavior:**
- Cold-start: first call to a freshly-loaded Ollama model can take 5-10s. 30s timeout absorbs this.
- Local crash: timeout triggers, cloud call serves the user. Total user wait: ~30s + cloud latency.
- Local syntax error in response: the OpenAI-compat provider returns Err, fallback fires.
- The fallback NEVER fails silently — `tracing::warn!` makes it visible in `/tmp/temm1e.log`.

**Recovery:** automatic per-request. Repeated failures within a tier eventually trip CUSUM (Gate 4).

**Catches:** F4 (Ollama down), F5 (cold start), F7 (corruption), F8 (model missing), F9 (starvation).

### Gate 6 — Adapter file integrity (write-then-validate)

**What:** the trainer writes the safetensors adapter to a UUID-named workdir, validates the file exists and has non-zero size, THEN tells Ollama to load it via Modelfile. Ollama itself validates the adapter at load time and rejects malformed files.

**Where enforced:** `crates/temm1e-distill/src/engine/trainer.rs` Phase 4 — after the backend's `train()` returns:
```rust
let artifacts = backend.train(&job).await?;
if !artifacts.adapter_path.exists() {
    return Err(Temm1eError::Tool("adapter file missing after successful train".into()));
}
let metadata = std::fs::metadata(&artifacts.adapter_path)?;
if metadata.len() == 0 {
    return Err(Temm1eError::Tool("adapter file is empty".into()));
}
// Then call ollama::create_model — Ollama also validates
```

**Recovery:** if validation fails, the trainer marks the run as Failed and the tier reverts to Collecting. No corrupted adapter ever reaches the routing layer.

**Catches:** F7 (corruption), partial subprocess failures.

### Gate 7 — Manual emergency demotion command

**What:** the `temm1e eigentune demote <tier>` CLI command and the `/eigentune demote <tier>` slash command force-revert a tier to Collecting. The user can immediately stop local routing for any tier without restarting the daemon.

**Where enforced:** `crates/temm1e-distill/src/engine/graduation.rs:55-72` already has a `demote()` method. Phase 15 (CLI subcommand) and Phase 18 (slash command) expose it.

**Usage:**
```bash
# Stop local routing for the Simple tier immediately:
temm1e eigentune demote simple

# Or via slash command:
/eigentune demote simple
```

**Recovery:** immediate. The next request for that tier returns `RouteDecision::Cloud`.

**Catches:** anything the automatic gates miss — gives the human an override.

---

## 2. Double opt-in: `enabled` AND `enable_local_routing`

The default config has `enabled = false`. To activate Eigen-Tune at all, the user sets `enabled = true`. But even then, **local routing requires a separate explicit opt-in**:

```toml
[eigentune]
enabled = true               # Phase 1: collection, training, evaluation, shadow phase
enable_local_routing = false # Phase 2: actually serve users locally (default OFF)
```

**Why:** the first opt-in lets the user observe the system work for several training cycles without any user-facing change. They see pairs accumulate, they see tiers transition through Training → Evaluating → Shadowing in the logs and `eigentune status`. They can manually inspect a trained model with `ollama run eigentune-simple-<id>` to verify it's reasonable. ONLY when they're satisfied do they flip the second switch and let the routing actually fire.

**Where enforced:** in Phase 12's routing block:
```rust
let route_decision = if let Some(et) = &self.eigen_tune {
    if request.tools.is_empty() && eigentune_cfg.enable_local_routing {  // <-- second opt-in
        et.route(&eigentune_complexity).await
    } else {
        RouteDecision::Cloud
    }
} else {
    RouteDecision::Cloud
};
```

The `enable_local_routing` flag is added to `EigenTuneConfig` in Phase 1 with `default_false`:
```rust
// crates/temm1e-distill/src/config.rs
/// Master switch for local routing. When false, the engine still collects,
/// trains, evaluates, and shadow-tests, but route() always returns Cloud.
#[serde(default = "default_false")]
pub enable_local_routing: bool,
```

**Note for sub-agents:** `EigenTuneConfig` does not currently have this field. **Phase 1 must add it** as part of the implementation. The field name and default are non-negotiable — they're referenced from runtime.rs by exact name.

---

## 3. Scenario matrix (verified)

Each scenario, the gates that protect the user, and the worst-case outcome.

| # | Scenario | Gates triggered | Worst-case outcome |
|---|---|---|---|
| 1 | User asks a question, default config | Gate 1 (off) | Cloud serves, no eigentune activity |
| 2 | User enables eigentune, asks a question, no graduated tier yet | Gate 1 ✓, tier in Collecting → router returns Cloud | Cloud serves, pair captured |
| 3 | User enables eigentune+local_routing, asks a question, Simple tier graduated, no tools | Gates 1✓, 2✓, 3✓, 5✓, 7✓ | **Local serves the user** — proven equivalent quality |
| 4 | User enables both, asks a question with tool use, Simple tier graduated | Gate 2 forces cloud | Cloud serves (correct — local doesn't do tools) |
| 5 | Local model timeout (cold start) | Gate 5 fallback | Cloud serves after ~30s wait + warn log |
| 6 | Local model returns gibberish for one query | Gate 5 (provider returns Err) → cloud fallback | Cloud serves, warn log |
| 7 | Local model gradually degrades (drift) | Gate 4 (CUSUM) detects and demotes after ~50 disagreements | Tier reverts to Collecting, all future requests go to cloud, system retrains later |
| 8 | Ollama crashes mid-request | Gate 5 timeout/error → cloud fallback | Cloud serves |
| 9 | Adapter file corrupted on disk | Gate 6 (validation at trainer) → run marked failed → tier reverts | Tier never reaches Graduated, no local routing |
| 10 | User wants to stop local routing immediately without restart | Gate 7 (manual demote) | Tier reverts in <1s |
| 11 | User wants to disable Eigen-Tune entirely | Gate 1 + restart | Engine never instantiated next launch |
| 12 | Misclassified Complex query routed as Simple | Gate 3 (graduation gates apply per-tier — Complex tier has its own gates and may never graduate) | Worst case: Simple-graduated tier handles a Complex query, output is mediocre. CUSUM eventually catches if this happens often. |
| 13 | Tool-use request reaches a non-tool model | Cannot happen — Gate 2 hard-blocks | n/a |
| 14 | First request after Ollama restart (model not loaded) | Gate 5 timeout (30s tolerates load) | If load > 30s: cloud fallback. Subsequent requests are fast. |
| 15 | Tier briefly graduated then drift detected | Gate 4 demotes, system trains again later | Brief window of degraded quality (~50 calls), then resolves automatically |
| 16 | User wants to see what's happening | `temm1e eigentune status` + `tail /tmp/temm1e.log | grep Eigen-Tune` | Full transparency at any time |

**No scenario produces an unrecoverable user-facing failure.** Every failure mode either falls back to cloud automatically OR is gated behind an opt-in switch.

---

## 4. Observability requirements

The user must be able to see what's happening at any time. This is non-negotiable for trust.

### 4.1 Log lines (mandatory)

| Event | Log level | Message | Where emitted |
|---|---|---|---|
| Engine initialized | `info!` | `Eigen-Tune: engine initialized (db = ...)` | `main.rs` Phase 16 |
| Tier transition | `info!` | `Eigen-Tune: tier {} {} → {}` | `lib.rs::tick` (existing) |
| Training started | `info!` | `Eigen-Tune: training cycle started for tier {}` | `trainer.rs::run` |
| Training completed | `info!` | `Eigen-Tune: training completed (loss = {}, model = {})` | `trainer.rs::run` |
| Training failed | `warn!` | `Eigen-Tune: training cycle failed: {}` | `trainer.rs::run` |
| Local call served | `info!` | `Eigen-Tune: served from local model (model = {}, tier = {})` | `runtime.rs` Phase 12 |
| Local call timeout/error | `warn!` | `Eigen-Tune: local call failed/timed out, falling back to cloud` | `runtime.rs` Phase 12 |
| CUSUM alarm | `warn!` | `Eigen-Tune: CUSUM alarm! Quality drift detected` | `monitor.rs` (existing) |
| Demotion | `warn!` | `Eigen-Tune: tier {} demoted to Collecting` | `graduation.rs` (existing) |
| Periodic tick | `debug!` | (only if there are transitions) | tick task |
| Collection (per pair) | `debug!` | `Eigen-Tune: collected training pair` | `collector.rs` (existing) |

### 4.2 Status command output (mandatory)

`temm1e eigentune status` must show, at minimum:

```
EIGEN-TUNE STATUS

Master switches:
  enabled                = true
  enable_local_routing   = false   <- shows BOTH opt-ins explicitly

Prerequisites:
  ✓ Ollama: running
  ✓ MLX: installed (Apple Silicon)
  ✓ Python 3.12.4

Data:
  Total pairs: 423 (high-quality: 287)
  Diversity: J = 0.74

Tiers:
  ● simple    GRADUATED   pairs=287  acc=0.97 (CI 0.94-0.99)  cusum_s=0.3  serving: eigentune-simple-a3f2
  ◐ standard  SHADOWING   pairs=89   sprt=15/200 (lambda=2.1)  serving: eigentune-standard-b8c1
  ○ complex   COLLECTING  pairs=4    (need 200)
```

The serving model name (`eigentune-simple-a3f2`) makes it auditable: the user can `ollama run eigentune-simple-a3f2` and test the model directly.

### 4.3 Database queries the user can run (documented in SETUP.md)

```bash
# How many pairs collected so far?
sqlite3 ~/.temm1e/eigentune.db "SELECT complexity, COUNT(*), AVG(quality_score) FROM eigentune_pairs GROUP BY complexity;"

# How many training runs have happened?
sqlite3 ~/.temm1e/eigentune.db "SELECT id, status, base_model, train_loss, started_at, completed_at FROM eigentune_runs ORDER BY started_at DESC;"

# Current tier states
sqlite3 ~/.temm1e/eigentune.db "SELECT tier, state, pair_count, eval_accuracy, eval_n, sprt_lambda, cusum_s, serving_run_id FROM eigentune_tiers;"
```

---

## 5. Implementation checklist (for sub-agents)

Before any line of local routing code is written, verify all seven gates have implementation tasks:

```
☐ Gate 1: master kill switch
   ☐ EigenTuneConfig.enabled defaults to false (already done in config.rs:200)
   ☐ Phase 16: main.rs only instantiates engine if enabled
   ☐ Phase 10: AgentRuntime.eigen_tune defaults to None
   ☐ Phase 11-14: every hook checks `if let Some(et) = &self.eigen_tune`

☐ Gate 2: tool-use guard
   ☐ Phase 12: routing block has `if request.tools.is_empty()` check
   ☐ Verify with a test: tool-bearing request never reaches Local branch

☐ Gate 3: statistical graduation gates
   ☐ Already in state_machine.rs (verified at lines 46-78, 83-118, 121-159)
   ☐ Phase 4 trainer: writes TrainingRun row, transitions Training → Evaluating on success only
   ☐ Phase 5 evaluator: writes eval_accuracy + eval_n, lets state machine read them
   ☐ Phase 6: state_machine recovery for stuck Training tiers

☐ Gate 4: CUSUM drift detection
   ☐ Already in monitor.rs (verified at lines 33-80)
   ☐ Phase 12 Monitor branch: spawns cloud comparison + on_monitor_observation
   ☐ Verify: tier state goes Graduated → Collecting on alarm

☐ Gate 5: per-call timeout + fallback
   ☐ Phase 12 Local branch: 30s timeout
   ☐ Phase 12 Local branch: cloud fallback on Err or timeout
   ☐ Verify with a test: unreachable Ollama → cloud fallback fires

☐ Gate 6: adapter file integrity
   ☐ Phase 4 trainer: validates adapter file exists + non-zero
   ☐ Phase 4 trainer: marks run Failed on validation error
   ☐ Phase 4 trainer: tier reverts to Collecting on failure

☐ Gate 7: manual emergency demote
   ☐ engine/graduation.rs already has demote() method (verified at lines 55-72)
   ☐ Phase 15 CLI: `Commands::Eigentune { command: EigentuneCommands::Demote { tier } }`
   ☐ Phase 18 slash command: `/eigentune demote <tier>` branch
   ☐ Verify: graduated tier reverts to Collecting in <1s

☐ Double opt-in
   ☐ Phase 1: add `enable_local_routing: bool` to EigenTuneConfig (default false)
   ☐ Phase 12: routing block checks BOTH `enabled` (via Some(et)) AND `enable_local_routing`
   ☐ Phase 17: default temm1e.toml example shows BOTH switches with comments

☐ Observability
   ☐ All log lines from §4.1 implemented at the right severity
   ☐ Phase 7: format_status() shows both opt-in switches
   ☐ Phase 17: SETUP.md documents the SQLite queries
```

---

## 6. Test plan for the safety chain

### 6.1 Unit tests (per gate)

```rust
// Gate 2: tool-use guard
#[tokio::test]
async fn tool_bearing_request_always_routes_cloud() {
    // Set up an engine with a Graduated Simple tier
    // Build a request with non-empty tools
    // Verify route_decision is Cloud, not Local
}

// Gate 3: graduation gates
#[tokio::test]
async fn evaluating_below_wilson_threshold_demotes() {
    // Seed a tier with eval_accuracy = 0.85, eval_n = 30
    // wilson_lower(85% of 30, 30, 0.99) = ~0.66 < 0.95
    // Verify check_evaluating_transition returns Some(Collecting)
}

#[tokio::test]
async fn shadowing_below_sprt_threshold_continues() {
    // Seed a tier with sprt_lambda = -0.5, sprt_n = 10
    // SPRT should return Continue (not enough evidence)
    // Verify check_shadowing_transition returns Ok(None)
}

// Gate 4: CUSUM
#[tokio::test]
async fn cusum_alarms_after_repeated_disagreement() {
    // Simulate 60 monitor observations with agree=false
    // CUSUM s should exceed cusum_threshold
    // Verify on_monitor_observation triggers demote
}

// Gate 5: timeout fallback
#[tokio::test]
async fn local_call_timeout_falls_back_to_cloud() {
    // Mock provider that sleeps 60 seconds
    // 30s timeout should fire, cloud fallback should serve
    // Verify circuit breaker records success (cloud), not failure
}

// Gate 6: adapter validation
#[tokio::test]
async fn trainer_marks_run_failed_when_adapter_missing() {
    // Mock backend that succeeds but doesn't write the file
    // Verify TrainingRun status = Failed, tier reverts to Collecting
}

// Gate 7: manual demote
#[tokio::test]
async fn demote_command_reverts_graduated_tier() {
    // Seed a Graduated tier
    // Call graduation.demote(EigenTier::Simple)
    // Verify tier state is now Collecting
}

// Double opt-in
#[tokio::test]
async fn local_routing_disabled_when_enable_local_routing_false() {
    // enabled=true, enable_local_routing=false
    // Even with a Graduated tier, route() should return Cloud
    // Wait, route() doesn't know about enable_local_routing — it's a runtime guard
    // Verify the runtime check at Phase 12 forces Cloud
}
```

### 6.2 Integration tests

```rust
// Full pipeline + safety chain
#[tokio::test]
async fn full_pipeline_with_safety_chain() {
    // 1. Set up in-mem store
    // 2. Seed 100 high-quality pairs across categories
    // 3. Trigger tick → tier transitions to Training
    // 4. Mock backend succeeds → tier transitions to Evaluating
    // 5. Mock evaluator writes eval_accuracy=0.97, eval_n=30 → state machine transitions to Shadowing
    // 6. Mock 60 shadow observations all agreeing → SPRT accepts H1 → Graduated
    // 7. Now route(EigenTier::Simple) should return Local
    // 8. But with enable_local_routing=false, runtime should still use cloud
    // 9. Set enable_local_routing=true, route again → Local serves
    // 10. Inject one bad observation, then another, then 60 → CUSUM alarms → tier demotes
    // 11. Verify route() now returns Cloud
}
```

### 6.3 Manual verification (developer machines)

```bash
# Set up a graduated tier the cheap way (skip the long collection):
sqlite3 ~/.temm1e/eigentune.db <<EOF
UPDATE eigentune_tiers SET
  state = 'graduated',
  serving_run_id = 'manual-test-run',
  eval_accuracy = 0.97,
  eval_n = 30,
  sprt_lambda = 5.0,
  sprt_n = 50
WHERE tier = 'simple';

INSERT INTO eigentune_runs (
  id, started_at, status, base_model, backend, method,
  dataset_version, pair_count, general_mix_pct, ollama_model_name
) VALUES (
  'manual-test-run', '2026-04-10T00:00:00Z', 'completed',
  'mlx-community/Llama-3.2-1B-Instruct-4bit', 'mlx', 'lora',
  1, 200, 0.0, 'eigentune-simple-test'
);
EOF

# Pull the model into Ollama (if not already done):
ollama pull llama3.2:1b
ollama tag llama3.2:1b eigentune-simple-test

# Now enable local routing in temm1e.toml:
# [eigentune]
# enabled = true
# enable_local_routing = true

# Restart temm1e and ask a Simple-tier question:
./target/release/temm1e chat
> What's 2+2?

# Verify the log shows:
grep "served from local model" /tmp/temm1e.log

# Now break Ollama (kill the server):
pkill ollama

# Ask another Simple-tier question:
> What's 3+3?

# Verify the log shows the fallback:
grep "local call failed/timed out, falling back to cloud" /tmp/temm1e.log

# And the user got an answer (the cloud model served):
# (visible in the chat output)

# Restart Ollama, demote the tier:
ollama serve &
./target/release/temm1e eigentune demote simple

# Verify it's now Collecting:
./target/release/temm1e eigentune status
```

---

## 7. What this safety protocol does NOT cover

- ❌ **Privacy of captured data.** The collector stores user messages in `~/.temm1e/eigentune.db`. This is local-only and never transmitted, but it IS user data on disk. SETUP.md must document this clearly. Users with strict data hygiene can disable the feature entirely or set short retention.
- ❌ **Subjective quality regressions.** A user might subjectively prefer the cloud model's "voice" even if it's mathematically equivalent. We can't measure subjective preference. The user can demote any tier they don't like via Gate 7.
- ❌ **Adversarial inputs.** A user crafting requests specifically to confuse the local model is out of scope. Eigen-Tune assumes good-faith use.
- ❌ **Multi-user concurrency.** If many users hit a graduated tier simultaneously, Ollama may saturate. Gate 5 (timeout) absorbs the worst case. Future enhancement: rate-limit local routing per second.
- ❌ **Cost accounting for local calls.** Local model calls have $0 LLM cost but consume local CPU/GPU. The budget tracker doesn't currently model this. Future enhancement.

---

## 8. The trust contract

By following this protocol, the user gets the following guarantees:

1. **No surprise routing.** The system never serves locally unless the user has explicitly opted in twice (`enabled` AND `enable_local_routing`).
2. **No silent failures.** Every fallback emits a `warn!` log line. The user can `tail /tmp/temm1e.log | grep Eigen-Tune` and see exactly what's happening.
3. **No quality regression beyond statistical noise.** Wilson lower bound + SPRT + CUSUM together guarantee that local routing only happens for tiers proven equivalent to cloud, and detects drift in real time.
4. **No tool-use breakage.** Tool-bearing requests are unconditionally cloud-routed.
5. **No unrecoverable state.** Every failure mode has an automatic recovery path (fallback or demote). The user can also force-demote any tier in <1 second.
6. **Full transparency.** `temm1e eigentune status` and the log lines tell the user exactly what's happening and why.
7. **Zero blast radius for default-config users.** Users who never enable Eigen-Tune see no behavior change, no new files on disk, no new background tasks, no compile-time additions.

If any of these guarantees can be broken by a single line of code in the implementation, **stop and report**. The plan is wrong, not the code.
