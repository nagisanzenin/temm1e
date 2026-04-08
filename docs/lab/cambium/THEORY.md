# Cambium: Theoretical Foundation

> **Status:** Working theory — v1.0  
> **Date:** 2026-04-08  
> **Scope:** Foundational theory for Tem's gap-driven self-grow capability

---

## The Cambium Metaphor

In a tree, the **cambium** is the thin layer of growth tissue just under the bark. Each year the tree adds a new ring of cambium, getting larger and stronger while the **heartwood** (the core trunk) stays stable forever. The tree's history is recorded in its rings.

This is exactly how Tem's self-grow capability works:

| Botanical | Tem |
|-----------|-----|
| **Heartwood** | Immutable kernel: vault, core traits, security, the Cambium pipeline itself |
| **Cambium** | The growth layer: tools, skills, cores, integrations — where new capabilities are added |
| **Bark** | The runtime surface: channels, gateway, agent — what users interact with |
| **Rings** | `GrowthSession` history: one ring per growth cycle, permanently recorded |

Cambium adds layers at the edge while the heartwood remains untouched. Every growth cycle is recorded as a ring. The tree gets larger over time, but its identity is preserved.

---

## I. The Thesis

Tem is a runtime that orchestrates AI providers, tools, memory, and communication channels. The runtime is Rust code. The AI is external (Claude, GPT, Gemini). When Tem grows through **Cambium**, it does not make the AI smarter — it makes the **orchestration** better. Cambium improves the bridge between what users need and what the system can deliver.

Cambium is **gap-driven code evolution through a verification harness.**

The entity observes a gap between user need and system capability, designs a solution, writes code, and passes it through a deterministic verification pipeline that is itself immutable. The quality of generated code depends on the LLM. The safety of deployed code depends on the pipeline. These two concerns are separated by design.

This separation is what makes Cambium timeproof: as LLMs improve, the code quality improves, the growth rate accelerates — but the safety guarantees remain constant because they are mechanical, not probabilistic.

---

## II. First Principles

### Principle 1: Growth is Gap-Driven, Never Speculative

Every growth action closes a specific, observable gap between what a user needed and what Tem could deliver. There is no "this might be useful someday." The trigger is always empirical: an error was logged, a user asked and Tem couldn't answer, a quality metric degraded.

**Why this matters:** Speculative growth leads to bloat, unnecessary complexity, and wasted resources. Gap-driven growth ensures every change has a measurable before/after.

### Principle 2: The Growth System is Not Growable

The pipeline that enables growth is itself immutable. Tem cannot modify its own modification process. This is the foundational safety guarantee. If the growth system could modify itself, there would be no stable ground to reason about safety from.

**Why this matters:** This prevents the theoretical risk of recursive self-modification — an entity removing its own guardrails by modifying the guardrail-checking code.

### Principle 3: Verification is Deterministic and LLM-Agnostic

The verification pipeline does not use AI judgment. It uses mechanical checks: does the code compile? Do all tests pass? Does the linter approve? Is the diff within budget? These checks produce binary yes/no answers regardless of which LLM generated the code.

**Why this matters:** If verification depended on AI judgment, a more capable AI could potentially convince the verifier. Deterministic checks cannot be argued with.

### Principle 4: Trust is Earned Through Track Record

New growth capabilities start at maximum oversight. Successful changes build trust. Failed changes decrease trust. Certain zones never open regardless of track record. This creates a natural ramp: early growth is conservative and supervised, mature growth (after many verified successes) gains autonomy.

**Why this matters:** It allows the system to start cautiously and expand scope based on evidence, rather than requiring a human to decide upfront what to trust.

### Principle 5: Capability Expansion Does Not Equal Authority Expansion

Adding a new tool (capability) does not grant permission to use it unsupervised (authority). If Tem grows a "deploy to production" tool, it still requires user authorization to deploy. Capabilities and authorities are orthogonal axes.

**Why this matters:** This prevents capability growth from silently expanding what Tem is allowed to do without user awareness.

### Principle 6: The Entity Knows Itself

Tem maintains an accurate, current model of its own architecture — crate graph, trait contracts, message flow, tool registry. Growth without self-knowledge is blind mutation. Growth with self-knowledge is engineering.

**Why this matters:** A surgeon must know anatomy. An entity modifying its own code must understand its own architecture.

### Principle 7: Every Growth Action is Reversible

No cambium change creates irreversible state. Every change is committed to version control, every binary has a rollback, every database has a snapshot. If a change causes harm, it can be undone completely.

**Why this matters:** Reversibility is the ultimate safety net. It makes every growth action a safe experiment rather than a permanent commitment.

### Principle 8: Growth Has a Cost and Must Be Justified

Every growth cycle consumes tokens, compute, time, and carries risk. The expected benefit must exceed the expected cost. This is not a subjective judgment — it maps to: "how many users are affected by this gap?" and "how severe is the impact?"

**Why this matters:** Without cost accounting, the system could spend more on growth than the growth saves.

### Principle 9: Transparency is Non-Negotiable

Every growth action is logged with full context: what triggered it, what was changed, what tests passed/failed, what the outcome was. The user can inspect every change, understand every decision, and override any action.

**Why this matters:** An opaque self-modifying system is inherently untrustworthy. Transparency converts uncertainty into informed oversight.

### Principle 10: The Human Can Always Override

The user can disable cambium entirely, rollback any specific change, modify trust levels, or add/remove protected zones at any time. The system serves the user; the user does not serve the system.

**Why this matters:** Self-grow is a tool for human benefit. The moment the human loses control, it becomes a liability.

---

## III. What Cambium Is (and Is Not)

### Cambium IS:

**Supervised code evolution.** Tem observes a gap, reads its own architecture, designs a solution, writes Rust code, passes it through a deterministic verification pipeline, and deploys if verified. "Supervised" refers to the pipeline, not a human — the pipeline is mechanical, predictable, and immutable.

**An orchestration improvement loop.** Tem improves HOW it coordinates AI providers, tools, memory, and channels. It does not improve the AI itself — that is the provider's domain.

**A verification harness with a pluggable code generator.** The pipeline (compile, lint, test, verify) is fixed. The code generator (which LLM, which prompt, which strategy) is variable. Better generators → faster growth. But the harness guarantees safety regardless.

**An enabler for future LLMs.** Today's LLMs produce modest code improvements that pass the pipeline slowly. Future LLMs will produce better code that passes the pipeline faster. The pipeline does not need to change — it was designed for a capability level that doesn't exist yet.

### Cambium IS NOT:

**Recursive self-improvement.** Tem does not make the AI smarter or modify its own intelligence. The AI is external. Tem modifies its runtime orchestration — the equivalent of a developer improving their IDE, not improving their brain.

**Autonomous AI design.** Tem does not redesign its own architecture. The trait contracts, crate structure, and message flow are immutable. Tem works within the existing architecture, adding tools, skills, and optimizations.

**Unsupervised.** Every change passes through deterministic verification. Every change is logged. Every change is reversible. The human can inspect and override at any time.

**Unbounded.** Growth is rate-limited (max sessions per day), scope-limited (trust hierarchy), and cost-limited (token budget). The system cannot grow faster than the budget allows or wider than the trust level permits.

---

## IV. The Goal of Cambiuming

### Primary Goal

**Close the gap between user intent and system capability, one verified change at a time.**

This is the singular purpose. Every growth action must directly serve this goal. The gap is observable:

```
Gap = (what the user needed) - (what Tem could deliver)
```

### How Gaps Are Observed

| Signal | Source | Example |
|--------|--------|---------|
| **Explicit failure** | Error logs, panics | `panic: byte index 200 is not a char boundary` |
| **Capability miss** | Agent response | "I don't have a tool for that" |
| **User correction** | Anima profiling | User repeatedly reformats Tem's output |
| **Quality degradation** | Distill scoring | CUSUM alarm on response quality |
| **User request** | Natural language | "Can you monitor my Kubernetes pods?" |
| **Performance bottleneck** | Budget tracker | Token cost per message increasing over time |

### What Growth Sessions Produce

Each growth session produces exactly one of:

1. **New tool** — Tem gains a capability it didn't have (e.g., Kubernetes monitoring)
2. **Bug fix** — An error path is corrected (e.g., UTF-8 boundary handling)
3. **Optimization** — An existing capability becomes faster/cheaper (e.g., smarter context pruning)
4. **New skill** — A workflow Tem learned to execute is codified for reuse
5. **New integration** — Tem connects to a new service/channel/provider
6. **New specialist core** — A domain-specific sub-agent is created for TemDOS

Each output is concrete, testable, and measurable. "Make Tem better" is not a valid growth goal. "Add Kubernetes pod monitoring tool that returns pod status as JSON" is.

---

## V. Scope of Allowed Growth

### The Trust Hierarchy

The codebase is divided into zones based on modification risk. The zones are defined in `PROTECTED_ZONES.md` and enforced by the cambium pipeline (which is itself in the highest protection zone).

```
LEVEL 0 — IMMUTABLE KERNEL (never modifiable by cambium)
│
│  The cambium pipeline code itself
│  Guardrail enforcement and zone checking
│  temm1e-vault (encryption, credential management)
│  temm1e-core/src/traits/ (architectural contracts)
│  Security enforcement (allowlist, authentication)
│  Workspace Cargo.toml structure (cannot add/remove crates)
│  This file (THEORY.md) and PROTECTED_ZONES.md
│  The watchdog/supervisor process
│
│  Enforcement: SHA-256 checksums validated pre-commit.
│  If any Level 0 file's checksum changes, the entire
│  growth session is aborted and the user is alerted.
│
│  Rationale: These files define WHAT cambium is and
│  HOW it operates. If they could be modified, there
│  would be no stable safety guarantee.
│
├─ LEVEL 1 — APPROVAL REQUIRED (human must confirm)
│
│  temm1e-agent/ (runtime loop, classification, execution)
│  temm1e-gateway/ (dispatch, worker management, routing)
│  temm1e-providers/ (API integration, streaming)
│  temm1e-perpetuum/ (scheduling, autonomy, volition)
│  temm1e-memory/ (persistence, search)
│  src/main.rs (entry point, initialization)
│  Cargo.toml dependency changes (new external crates)
│
│  Enforcement: The pipeline generates a diff, sends it
│  to the user via their active channel, and waits for
│  explicit "approve" before committing.
│
│  Rationale: These files control core behavior. A bug
│  here affects every user, every message, every session.
│  Human judgment is required.
│
├─ LEVEL 2 — AUTONOMOUS WITH FULL PIPELINE
│
│  temm1e-tools/ (add new tools, fix existing tools)
│  temm1e-skills/ (skill registry, skill loading)
│  temm1e-channels/ (channel improvements)
│  temm1e-cores/ (new specialist sub-agents)
│  temm1e-anima/ (profiling improvements)
│  temm1e-observable/ (metrics, tracing)
│  temm1e-hive/ (swarm coordination)
│  temm1e-distill/ (learning pipeline)
│
│  Enforcement: Full pipeline (compile, lint, test, review
│  by CodeReviewer core, review by SecurityAuditor core).
│  No human gate — but human is notified of every change
│  and can rollback at any time.
│
│  Rationale: These are extension points. Adding a tool
│  or skill does not affect existing behavior if all
│  tests pass. The blast radius is contained.
│
└─ LEVEL 3 — AUTONOMOUS WITH BASIC PIPELINE
│
│  docs/ (documentation)
│  tests/ (add tests — NEVER remove)
│  ~/.temm1e/skills/*.md (runtime skill files)
│  ~/.temm1e/cores/*.md (runtime core definitions)
│
│  Enforcement: Compilation check only. No full test suite
│  required (docs/skills don't affect compilation).
│
│  Rationale: Zero runtime risk. Skills and core definitions
│  are interpreted text, not compiled code. Documentation
│  cannot break the system.
```

### What Cambium Can NEVER Do

Regardless of trust level, track record, or LLM capability:

1. **Modify the cambium pipeline** — the judge cannot modify itself
2. **Remove tests** — tests can be added, never deleted
3. **Add `unsafe` Rust blocks** — no escape from the type system
4. **Add external crate dependencies without approval** — supply chain risk
5. **Modify encryption/vault code** — credential security is sacrosanct
6. **Modify trait definitions** — architectural contracts are immutable
7. **Force-push to any branch** — git history is append-only
8. **Exceed the session budget** — hard ceiling on tokens/compute per session
9. **Run more than N sessions per day** — rate limiting prevents runaway growth
10. **Deploy without passing the full pipeline** — no shortcuts, no exceptions

### How Trust Levels Evolve

Trust is earned, not assumed. The evolution is based on track record:

```
Initial state:
  Level 2 = APPROVAL REQUIRED (same as Level 1)
  Level 3 = AUTONOMOUS WITH BASIC PIPELINE

After 10 consecutive successful Level 3 changes (zero rollbacks):
  Level 3 confirmed autonomous

After 25 consecutive successful Level 2 changes (zero rollbacks):
  Level 2 becomes fully autonomous (no human approval needed)

After ANY rollback at Level N:
  Level N reverts to requiring approval for next 10 changes
  Cooldown period: 24 hours before next growth session

After 3 rollbacks in 7 days at any level:
  ALL levels revert to APPROVAL REQUIRED
  User must explicitly re-enable autonomous growth
```

This creates a natural ramp: cautious start, earned freedom, rapid correction on failure.

---

## VI. Current Capabilities Assessment

What TEMM1E can do TODAY that is directly relevant to cambiuming:

### Already Enabled

| Capability | Crate | Relevance to Cambium |
|------------|-------|----------------------|
| **File read/write** | temm1e-tools | Tem can read its own source code and write new code |
| **Shell execution** | temm1e-tools | Tem can run `cargo check`, `cargo test`, `cargo build` |
| **Git operations** | temm1e-tools | Tem can branch, commit, diff, log (force-push blocked) |
| **Perpetuum scheduling** | temm1e-perpetuum | Tem can schedule recurring growth sessions |
| **Conscience states** | temm1e-perpetuum | Sleep/Dream states trigger self-work (bug analysis, introspection) |
| **Vigil bug reporter** | temm1e-perpetuum | Scans logs for errors, triages via LLM |
| **TemDOS cores** | temm1e-cores | Specialist sub-agents for code review, security audit |
| **Hive coordination** | temm1e-hive | Parallel execution of review + test + audit |
| **Distill quality scoring** | temm1e-distill | Detects quality degradation (CUSUM alarm) |
| **Anima user profiling** | temm1e-anima | Detects unmet user needs and preference patterns |
| **Skill loading** | temm1e-skills | Hot-loads `.md` skills from disk without restart |
| **Learning extraction** | temm1e-agent | Captures what works with value scoring V(a,t) |
| **Budget tracking** | temm1e-agent | Tracks cost per session, prevents runaway spending |
| **2127+ tests** | workspace-wide | Regression detection for any code change |

### Disabled by Default (Config Flip)

| Capability | Config Key | Why Disabled |
|------------|-----------|-------------|
| **Volition** | `perpetuum.volition.enabled` | Proactive agency — powerful but needs careful scoping |
| **Eigen-Tune** | `eigen_tune.enabled` | Full distillation pipeline — resource intensive |

### Not Yet Implemented (Required for Cambium)

| Capability | Purpose | Difficulty |
|------------|---------|-----------|
| **Self-grow pipeline** | Orchestrates the full grow cycle | New crate: `temm1e-cambium` |
| **Codebase self-model** | Machine-readable architecture description | Documentation + generation scripts |
| **Blue-green binary swap** | Deploy new binary without downtime | Process management + state serialization |
| **Watchdog process** | Monitors new binary, rollback on failure | Lightweight separate binary |
| **Protected zone enforcement** | SHA-256 checksums on immutable files | Pre-commit validation |
| **Trust level state machine** | Track record → earned autonomy | SQLite persistence |
| **Session budget tracking** | Rate limits + cooldowns for growth | Extension of existing BudgetTracker |
| **CodeReviewer core definition** | Reviews cambium diffs for quality | `.core` markdown file |
| **SecurityAuditor core definition** | Reviews diffs for vulnerabilities | `.core` markdown file |

### The Readiness Assessment

**Infrastructure readiness: ~80%.** Tem already has the tools to read code, write code, compile, test, commit, schedule, and coordinate. What's missing is the **pipeline orchestration** (temm1e-cambium crate) and the **safety infrastructure** (protected zones, watchdog, trust state machine).

**The hardest unsolved problem:** Binary swap. Rust compiles to a static binary. There is no hot-reload. The running process must be replaced with a new process while preserving state and minimizing downtime. This is a solved problem in deployment engineering (blue-green, canary) but novel in the context of an entity replacing itself.

---

## VII. Timeproof Architecture

### The Core Insight

The cambium pipeline is a **verification harness** with a **pluggable code generator.**

```
┌──────────────────────────┐
│     Code Generator       │  ← This is the LLM. It changes over time.
│  (Claude, GPT, Gemini,   │     Better LLMs → better code → faster pipeline passes.
│   future models)         │     The pipeline doesn't care which LLM wrote the code.
└──────────┬───────────────┘
           │ generated code
           ▼
┌──────────────────────────┐
│   Verification Pipeline  │  ← This is deterministic. It does NOT change.
│                          │     cargo check → clippy → fmt → test → review → deploy
│  Compile? ──── yes/no    │     Binary outcomes. No AI judgment. No persuasion possible.
│  Lint?    ──── yes/no    │
│  Test?    ──── yes/no    │
│  Budget?  ──── yes/no    │
│  Zone?    ──── yes/no    │
└──────────┬───────────────┘
           │ verified change
           ▼
┌──────────────────────────┐
│   Deployment Protocol    │  ← This is mechanical. Blue-green swap, health check,
│                          │     rollback on failure. No AI involvement.
│  Build → Health → Swap   │
│  Monitor → Promote/Roll  │
└──────────────────────────┘
```

**Why this is timeproof:**

The pipeline was designed for LLMs that don't exist yet. A 2030 model that writes perfect Rust will still go through the same pipeline — it will just pass faster. The pipeline doesn't need to "keep up" with LLM capability because it verifies outcomes, not process.

### Scaling with LLM Capability

| Era | Code Quality | Pipeline Behavior | Growth Rate |
|-----|-------------|-------------------|-------------|
| **2026** (current) | Good for simple tools, struggles with complex refactors | More retries, simpler changes, more human oversight | Slow: ~1-3 changes/week |
| **2027-2028** | Reliable for most features, some architectural changes | Fewer retries, moderate changes | Moderate: ~1-2 changes/day |
| **2029-2030** | Expert-level across full codebase | Near-zero retries, complex features | Fast: ~5-10 changes/day |
| **2030+** | Surpasses most human developers | Pipeline becomes the only bottleneck | Rate-limited by policy, not capability |

The pipeline is the **constant**. The LLM is the **variable**. As the variable improves, the system outputs more value per unit time — but the safety guarantees (the constant) never weaken.

### What Future LLMs Unlock (That Today's Cannot)

1. **Cross-crate refactoring** — changing a tool implementation AND updating agent runtime to use it optimally. Requires understanding multiple crates simultaneously.

2. **Architectural evolution** — proposing changes to Level 1 code (currently human-approval required) that are sophisticated enough to earn trust. The trust hierarchy allows this if track record is sufficient.

3. **Proactive gap detection** — today's LLMs need explicit error signals. Future LLMs could analyze usage patterns and predict gaps before users encounter them.

4. **Multi-step growth plans** — instead of single-change sessions, planning a sequence of related changes that together deliver a complex capability (with each step independently verified).

5. **Better self-knowledge** — future LLMs with larger context can hold more of the codebase in working memory, leading to better-informed growth decisions.

### What the System Provides That Future LLMs Need

Even a vastly more capable LLM running as Tem would be limited without cambium infrastructure:

- **Without cambium:** The LLM is brilliant but frozen in amber. It can orchestrate magnificently within existing tools but cannot add new ones. User asks for X → "I don't have that capability" → forever.

- **With cambium:** The LLM identifies the gap, reads the architecture, writes the tool, passes the pipeline, deploys. User asks for X → "Let me build that" → done.

**Self-grow is the highway. The LLM is the car. Building the highway takes effort but it serves every car that ever drives on it — including ones that haven't been built yet.**

---

## VIII. Trust But Verify: The Verification Pipeline

### The Pipeline Stages

Every cambium session passes through these stages in order. Each stage is a binary gate: pass or fail. There is no "partial pass."

```
STAGE 0: TRIGGER VALIDATION
  Input:  Growth trigger (bug report, user request, quality alarm)
  Check:  Is this a real gap? Is it within scope? Is it within budget?
  Output: Validated growth intent with clear goal statement
  Fail:   Session not started. No cost incurred.

STAGE 1: SELF-BRIEFING
  Input:  Validated growth intent
  Action: Read docs/cambium/ARCHITECTURE.md + relevant crate docs
  Check:  Does Tem understand the affected code paths?
  Output: Architecture-aware modification plan
  Fail:   Abort if understanding is insufficient. Report to user.

STAGE 2: CODE GENERATION
  Input:  Modification plan + relevant source files
  Action: LLM generates Rust code changes
  Check:  Diff is within budget (max lines, max files, correct zone)
  Output: Code diff ready for verification
  Fail:   Retry with smaller scope (max 3 attempts). Then abort.

STAGE 3: ZONE COMPLIANCE
  Input:  Code diff
  Check:  No Level 0 files modified. Level 1 files flagged for approval.
  Output: Zone-compliant diff
  Fail:   Hard abort. Cannot be retried. Zone violations are not negotiable.

STAGE 4: COMPILATION
  Input:  Code diff applied to worktree
  Action: cargo check --workspace
  Check:  Zero compilation errors
  Output: Type-checked code
  Fail:   LLM analyzes error, generates fix. Retry (max 3). Then abort.

STAGE 5: LINTING
  Input:  Type-checked code
  Action: cargo clippy --workspace --all-targets --all-features -- -D warnings
  Check:  Zero warnings
  Output: Lint-clean code
  Fail:   LLM analyzes warning, generates fix. Retry (max 3). Then abort.

STAGE 6: FORMATTING
  Input:  Lint-clean code
  Action: cargo fmt --all -- --check
  Check:  All code formatted
  Output: Formatted code
  Fail:   Auto-fix with cargo fmt --all. Always passes.

STAGE 7: TEST SUITE
  Input:  Formatted code
  Action: cargo test --workspace
  Check:  ALL existing tests pass. New tests exist for new code.
  Output: Regression-free code
  Fail:   LLM analyzes failing test. If test failure reveals a real bug
          in the generated code → fix and retry (max 3). Then abort.

STAGE 8: CODE REVIEW (TemDOS Core)
  Input:  Verified diff
  Action: CodeReviewer core evaluates: architecture compliance, code quality,
          naming conventions, error handling, test coverage
  Check:  Review score >= threshold (configurable, default: 7/10)
  Output: Reviewed code
  Fail:   LLM addresses review feedback. Retry (max 2). Then abort.

STAGE 9: SECURITY AUDIT (TemDOS Core)
  Input:  Reviewed diff
  Action: SecurityAuditor core evaluates: OWASP top 10, injection,
          credential exposure, unsafe patterns, input validation
  Check:  Zero security findings above "info" severity
  Output: Security-cleared code
  Fail:   Hard abort. Security findings require human review.

STAGE 10: INTEGRATION TEST
  Input:  Security-cleared code
  Action: Build release binary. Run 10-turn conversation test.
  Check:  All 10 turns get valid responses. Memory recall works (turn 6).
          Zero errors in logs.
  Output: Integration-verified binary
  Fail:   Abort. Integration failures indicate deeper issues.

STAGE 11: DEPLOYMENT
  Input:  Integration-verified binary
  Action: Blue-green binary swap protocol (see Section IX)
  Check:  Health check passes within 30 seconds
  Output: Running new binary
  Fail:   Automatic rollback to previous binary.

STAGE 12: POST-DEPLOY MONITORING
  Input:  Running new binary
  Action: Watch for 5 minutes. Check error rate, response quality,
          resource usage.
  Check:  Error rate <= previous. Quality score >= previous.
  Output: Confirmed successful growth
  Fail:   Automatic rollback. Change marked as FAILED in history.
```

### What "Trust" Means in Practice

Trust is not blind faith. Trust is a calibrated expectation based on evidence.

**We trust the LLM to:**
- Identify real gaps (validated by Distill scores and error logs)
- Write syntactically correct Rust (validated by compilation)
- Follow coding standards (validated by clippy and code review core)
- Not introduce vulnerabilities (validated by security audit core)
- Not break existing behavior (validated by test suite)

**We verify each of these claims mechanically.** The pipeline is the verification. If the LLM's code doesn't pass, it doesn't ship. The trust is in the pipeline's ability to catch problems, not in the LLM's ability to avoid them.

**The trust-but-verify inequality:**

```
P(pipeline catches bug | bug exists) >> P(LLM produces bug-free code)
```

The pipeline's false-negative rate (letting a bug through) must be lower than the LLM's bug rate. This is achievable because:
- Compilation catches type errors with P=1.0
- Tests catch behavioral regressions with P proportional to coverage
- Clippy catches known anti-patterns with P=1.0
- Security audit catches known vulnerability patterns

As test coverage increases, the pipeline's catch rate increases, which means the LLM can be trusted with riskier changes because the safety net is stronger.

---

## IX. The Deployment Protocol

### Blue-Green Binary Swap

The running Tem binary must be replaced without losing state or conversations. This is the operational heart of Cambium.

```
CURRENT STATE:
  temm1e (v4.6.1) running on port 8080
  Active conversations in memory
  Perpetuum concerns in SQLite
  Memory database in SQLite

PHASE A — BUILD (isolated, no impact on running system)
  1. Create git worktree: /tmp/temm1e-grow-<session-id>/
  2. Apply code changes in worktree
  3. cargo build --release in worktree
  4. New binary: /tmp/temm1e-grow-<session-id>/target/release/temm1e
  5. Copy to staging: ~/.temm1e/cambium/staging/temm1e-next

PHASE B — VERIFY (parallel, no impact on running system)
  6. Start staging binary on alternate port (8081) with test config
  7. Health check: GET :8081/health → 200
  8. Run integration test against staging binary
  9. If fail → delete staging, revert git changes, log failure, done

PHASE C — DRAIN (brief impact: no new conversations accepted)
  10. Signal running binary: enter drain mode
      - Stop accepting new messages from channels
      - Complete all in-flight message processing (timeout: 30s)
      - Persist in-flight state to SQLite checkpoint table
      - Flush Perpetuum state (already in SQLite, just ensure WAL sync)
      - Close channel connections gracefully
  11. Running binary acknowledges: "drained, ready to stop"

PHASE D — SWAP (downtime window: ~3-5 seconds)
  12. Stop running binary (SIGTERM → graceful shutdown)
  13. Archive old binary: ~/.temm1e/cambium/rollback/temm1e-<version>
  14. Move staging binary to production path
  15. Start new binary on port 8080
  16. New binary loads state from SQLite
  17. Channels reconnect (Telegram long-poll resumes, Discord WS reconnects)
  18. Perpetuum resumes (Pulse rebuilds schedule from SQLite)

PHASE E — MONITOR (5-minute watchdog window)
  19. Watchdog process monitors new binary
  20. Check every 10s: process alive? port responding? error rate normal?
  21. If ANY check fails within 5 minutes:
      - Kill new binary
      - Restore old binary from rollback/
      - Start old binary
      - Mark change as FAILED
      - Notify user
  22. After 5 minutes with all checks passing:
      - Mark change as DEPLOYED
      - Clean up staging/
      - Update cambium history

TOTAL DOWNTIME: ~3-5 seconds (Phase D only)
```

### The Watchdog

The watchdog is a separate, minimal process. It is part of the **immutable kernel** — Tem cannot modify it.

```
temm1e-watchdog
  - Single-purpose: monitors temm1e process health
  - No AI, no network, no complexity
  - Reads: process table, port check, error log
  - Actions: restart previous binary on failure
  - ~100 lines of Rust
  - Lives in Level 0 (immutable)
```

### State That Survives Swap

| State | Storage | Survives? | Notes |
|-------|---------|-----------|-------|
| Conversation history | SQLite memory.db | Yes | Already external |
| Credentials | credentials.toml | Yes | Already external |
| Config | config.toml | Yes | Already external |
| Perpetuum concerns | SQLite | Yes | Pulse rebuilds on start |
| Active conversations | Checkpoint table | Yes | Serialized during drain |
| Budget usage | SQLite | Yes | Loaded on start |
| Self-grow history | SQLite | Yes | Append-only |
| Channel connections | In-memory | No | Reconnect on start |
| Timers/intervals | In-memory | No | Rebuilt from SQLite |

---

## X. Risk Analysis

### Critical Risks (could cause data loss or security breach)

**R1: Guardrail circumvention**
- *Scenario:* LLM generates code that modifies Level 0 files
- *Mitigation:* Zone compliance check (Stage 3) uses SHA-256 checksums, not file path matching. Even if the LLM renames or symlinks files, the checksums catch it.
- *Residual risk:* Near zero. Checksum validation is deterministic.

**R2: Security vulnerability introduction**
- *Scenario:* Generated code has SQL injection, command injection, or auth bypass
- *Mitigation:* SecurityAuditor core (Stage 9) + clippy lints + test suite. Security findings above "info" → hard abort, human review required.
- *Residual risk:* Low. Novel vulnerability classes that no tool catches are possible but unlikely in the Green Zone scope.

**R3: Data corruption during swap**
- *Scenario:* Binary swap fails mid-process, SQLite in inconsistent state
- *Mitigation:* SQLite WAL mode provides crash recovery. Drain phase ensures all writes complete before shutdown. Checkpoint table is append-only.
- *Residual risk:* Very low. SQLite WAL is battle-tested.

### High Risks (could degrade service quality)

**R4: Regression not caught by tests**
- *Scenario:* New code passes all tests but degrades real-world behavior
- *Mitigation:* Integration test (Stage 10) + post-deploy monitoring (Stage 12). 5-minute watchdog catches runtime issues. Automatic rollback.
- *Residual risk:* Medium. Test coverage is the limiting factor. Mitigation: increase test coverage over time.

**R5: Runaway growth loop**
- *Scenario:* A growth session introduces a bug → triggers another growth session → introduces another bug → loop
- *Mitigation:* Rate limit (max N sessions/day). Cooldown after failure (24h). After 3 failures in 7 days, all growth requires approval.
- *Residual risk:* Low with rate limiting.

**R6: Resource exhaustion**
- *Scenario:* Compilation uses all CPU/disk/memory
- *Mitigation:* Build in isolated worktree. Build timeout (10 min). Max 3 compilations per session. `cargo clean` after session.
- *Residual risk:* Medium on resource-constrained systems. Mitigation: detect available resources before starting.

### Medium Risks (inconvenience, not harm)

**R7: Growth quality below expectations**
- *Scenario:* LLM writes code that technically passes but is poorly designed
- *Mitigation:* CodeReviewer core (Stage 8) evaluates quality. Minimum score threshold.
- *Residual risk:* Medium. Code review quality depends on the reviewing LLM's capability.

**R8: Unnecessary growth**
- *Scenario:* Trigger validation is too permissive, leading to changes that aren't needed
- *Mitigation:* Trigger validation (Stage 0) requires a specific, measurable gap. Cost-benefit check.
- *Residual risk:* Low-medium. Costs are bounded by budget ceiling.

**R9: User confusion**
- *Scenario:* Tem changes its own behavior, user doesn't understand why
- *Mitigation:* Transparency log. User notification on every change. Clear changelog with before/after.
- *Residual risk:* Low with good communication.

### Theoretical Risks (unlikely but worth tracking)

**R10: Emergent capability through accumulated changes**
- *Scenario:* Individual changes are each safe, but 100 changes together create an unintended emergent capability
- *Mitigation:* Periodic "diff from baseline" audit. Architecture rules are immutable (trait contracts, crate structure). The system's fundamental shape cannot drift because the skeleton is locked.
- *Residual risk:* Very low. Immutable kernel prevents structural drift.

---

## XI. The Codebase Self-Model

### Purpose

Tem reads this directory before every growth session. It provides:
1. **Architecture awareness** — what each crate does, how they connect
2. **Contract knowledge** — what traits define, what invariants must hold
3. **Zone awareness** — what can be modified, what requires approval, what is forbidden
4. **Historical context** — what was changed before, what worked, what failed

### Structure

```
docs/lab/cambium/
├── THEORY.md              ← you are here (foundational principles)
├── ARCHITECTURE.md        ← crate map, message flow, dependency graph
├── PROTECTED_ZONES.md     ← Level 0/1/2/3 file lists with checksums
├── TRAIT_CONTRACTS.md     ← every trait, its methods, its invariants
├── CODING_STANDARDS.md    ← rules cambiumn code must follow
├── PIPELINE.md            ← verification stages with pass/fail criteria
├── CAPABILITY_REGISTRY.md ← what Tem can currently do (auto-updated)
├── HISTORY.md             ← log of all growth sessions and outcomes
├── schemas/
│   ├── crate_graph.json   ← machine-readable dependency graph
│   ├── tool_registry.json ← all tools with schemas and zones
│   └── trait_map.json     ← trait → implementor mapping
└── sessions/              ← detailed records of each growth session
```

### Auto-Sync Requirement

The schemas/ directory must be **regenerated from actual code** on every commit. This prevents drift between the self-model and reality. A build script or CI step extracts:
- Crate names and dependencies from `Cargo.toml` workspace
- Tool names and schemas from tool registration code
- Trait definitions from `temm1e-core/src/traits/`
- Test counts per crate from `cargo test` output

If the self-model is stale, Tem is operating on outdated information. Auto-sync eliminates this risk.

---

## XII. Implementation Roadmap

### Phase 0: Theory + Self-Model (this document)

Create the theoretical foundation and codebase self-model. Zero code changes. Zero risk.

Deliverables:
- `docs/lab/cambium/THEORY.md` (this file)
- `docs/lab/cambium/ARCHITECTURE.md`
- `docs/lab/cambium/PROTECTED_ZONES.md`
- `docs/lab/cambium/TRAIT_CONTRACTS.md`
- `docs/lab/cambium/CODING_STANDARDS.md`

### Phase 1: Skill-Layer Growth

Tem creates and modifies its own **Markdown skills** — no Rust compilation needed. Skills are interpreted text, loaded from disk, zero deployment risk. This proves the growth loop without the hardest engineering challenges.

Deliverables:
- Growth trigger detection (Perpetuum concern)
- Skill authoring workflow (file write + registry reload)
- Skill testing (load, execute, verify output)
- Growth history tracking

Risk: Near zero. Bad skills are text files — delete and retry.

### Phase 2: The Cambium Pipeline

Build `temm1e-cambium` crate. Implement Stages 0-9 of the verification pipeline. Growth sessions produce committed code on a branch but do NOT deploy.

Deliverables:
- `temm1e-cambium` crate (pipeline orchestration)
- CodeReviewer and SecurityAuditor core definitions
- Protected zone enforcement with checksums
- Trust level state machine
- Session budget tracking

Risk: Low. Code is committed to branches, not deployed. Human reviews and merges.

### Phase 3: Blue-Green Deploy

Implement Stages 10-12. Growth sessions can autonomously deploy verified code.

Deliverables:
- Binary build in isolated worktree
- Blue-green swap protocol
- Watchdog process
- State serialization during drain
- Automatic rollback
- Post-deploy monitoring

Risk: Medium. This is where real autonomy begins. Extensive testing required.

### Phase 4: Autonomous Triggers

Enable user-driven and proactive growth triggers. Volition integration.

Deliverables:
- User need detection (Anima patterns → growth triggers)
- Quality degradation → growth triggers (Distill CUSUM)
- Bug self-detection → growth triggers (Vigil integration)
- Multi-step growth plans (dependent changes)

Risk: Medium-High. Autonomous trigger requires careful tuning to avoid unnecessary growth.

### Phase 5: WASM Plugin System (Future)

For most growth scenarios, avoid full recompilation. New tools and skills compiled as WASM modules, loaded at runtime. Aligns with SkyHub vision.

Deliverables:
- WASM compilation pipeline
- Runtime plugin loader
- WASM sandbox (memory + filesystem isolation)
- Signed plugin verification (SkyHub)

Risk: Medium. New runtime dependency (wasmtime), but sandboxed.

---

## XIII. Open Questions

These require further thinking before implementation:

1. **How does cambium interact with multiple users?** If User A needs feature X and User B doesn't, should the growth apply globally? What about conflicting preferences?

2. **How does cambium handle multi-crate changes?** If a new tool requires changes to both `temm1e-tools` and `temm1e-agent`, and one is Level 2 (autonomous) while the other is Level 1 (approval required), what happens? Current answer: the entire change inherits the highest trust level required.

3. **Should cambium changes be merged into main?** Or should they live on a separate branch that the production binary runs from? If separate branch, how do we prevent divergence from upstream development?

4. **What is the right session budget?** Max lines, max files, max tokens — these need empirical tuning. Start conservative, adjust based on observed failure rates.

5. **Should Tem explain its growth decisions in natural language?** Before growing, should it tell the user "I noticed you asked for X and I couldn't do it. I'm going to add a tool for that. Here's my plan: ..." This adds cost (LLM call) but dramatically improves trust and transparency.

6. **How does cambium interact with manual development?** When a human developer pushes code, the codebase changes. Does cambium need to re-read the self-model? How does it handle merge conflicts between its changes and human changes?

7. **What happens when the LLM provider is down?** Self-grow requires LLM calls. If the provider is unavailable, growth is impossible. Should there be a fallback provider? Or simply queue the trigger and retry later?

---

*This document is the foundation. It will evolve as we learn from early growth sessions. The principles (Section II) are stable. The implementation details will be refined.*
