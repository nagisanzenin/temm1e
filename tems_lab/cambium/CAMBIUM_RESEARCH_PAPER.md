# Cambium: Gap-Driven Runtime Evolution for a Rust AI Agent via a Deterministic Verification Harness

**Author:** The TEMM1E Project
**Draft:** v1.0
**Date:** 2026-04-08
**Branch of record:** `self-growing`
**Canonical implementation:** `crates/temm1e-cambium`, `crates/temm1e-watchdog`
**Canonical theory:** `docs/lab/cambium/THEORY.md`

---

## Abstract

Large language models are improving faster than the systems that host them. A model released in 2026 can reason about code more fluently than the orchestration layer it runs inside ever anticipated, yet the host binary itself is frozen at compile time. This paper describes Cambium, the self-growing subsystem of TEMM1E — a cloud-native Rust AI agent runtime — which closes this gap. Cambium lets the agent, named Tem, observe an empirical shortfall between what a user needed and what the system could deliver, write new Rust code to fill the shortfall, verify that code through a thirteen-stage deterministic pipeline, and deploy it through a blue-green binary swap supervised by an immutable watchdog process. The central architectural choice is to separate a pluggable LLM-backed code generator from a fixed mechanical verification harness; the quality of code depends on the model, but the safety of what ships depends only on the pipeline. We present the ten first principles that bound the system, a four-level trust hierarchy that gates what may grow autonomously, empirical validation against both a cheap-tier model (Gemini 3 Flash) and a medium-tier model (Claude Sonnet 4.6) that produced loadable Rust-adjacent skill artifacts at a total cost of less than two cents, and a description of the four production-class bugs that integration testing exposed in the deployment module before it ever touched a real binary. All 2274 tests in the workspace pass on the implementation branch. We argue that Cambium is what an AI agent runtime must look like in order to compound improvements rather than stagnate as frozen code against accelerating models.

---

## 1. Introduction

The dominant architectural pattern for LLM-based agents today is a frozen binary calling an improving model. The model behind the API gets better; the host does not. A 2030-era model reasoning inside a 2024-era runtime is a Formula 1 engine in a go-kart chassis. The model's additional capability has nowhere to go because the runtime does not know how to translate that capability into new tools, new integrations, or new orchestration strategies. The bottleneck migrates from the model to the code that calls the model.

There is an important distinction between two phrases that are often conflated: self-learning and self-growing. Self-learning updates weights, fine-tunes adapters, or accumulates retrieval corpora. It changes what the model knows. Self-growing changes what the runtime can do. In TEMM1E's terminology, self-learning lives in Lambda, Anima, Eigen-Tune, and the other distillation layers; self-growing is Cambium. Cambium does not make the underlying model smarter — the model is external and the provider's concern. Cambium makes the bridge between user intent and system capability wider, one verified Rust change at a time.

The runtime-evolution problem is old. Erlang/OTP solved a version of it in the 1990s with hot code reloading: processes can atomically swap their executing module while holding their state. This is the closest existing analogue to what Cambium does, but it differs in two important respects. First, Erlang's hot reload is invoked by humans, not by the system itself observing its own limitations. Second, Erlang does not include a mechanical verification harness that decides whether the new code is acceptable to ship; it assumes the operator knows what they are doing. Kubernetes rolling deployments share the downtime-minimising ethos but are again human-initiated and verify only that pods are responsive. Plugin systems (web browsers, IDEs, database extensions) allow third-party code to extend a host, but the decision to extend is made outside the system, and the plugin interface is typically narrower than the native codebase. None of these is a case of a program observing its own shortcomings and writing native code to fix them.

The Cambium thesis is that the right way to build an AI agent runtime is to factor it into two distinct pieces: an immutable kernel containing the verification harness, the deployment protocol, the watchdog, the vault, and the trait contracts; and a mutable surface where new tools, skills, integrations, and specialists can be added through a controlled pipeline. The kernel is engineered once, carefully, and never modified by the system itself. The surface grows gap-by-gap in response to observed evidence. The model is a pluggable code generator that fills the surface; better models produce better surface code; the kernel's safety guarantees are independent of which model is plugged in. This separation is what makes Cambium timeproof: it was designed for model capabilities that do not yet exist, and it does not need to be redesigned when they arrive.

---

## 2. The Cambium Metaphor

A tree grows outward from a thin living layer called the vascular cambium, a sheath of meristematic cells sandwiched between the inner bark (phloem) and the heartwood (xylem). Every year, the cambium divides: new cells to the inside become additional wood, new cells to the outside become additional bark. The annual ring that foresters count is the cambium's signature from one growing season. The heartwood of the tree — the dead, rigid, non-metabolising core that carries mechanical load — never changes once laid down. The cambium adds at the edge; the heartwood holds the line.

This is a near-perfect mapping for a self-growing runtime. The TEMM1E codebase divides naturally into the same three strata.

**Heartwood** is the immutable kernel: the `temm1e-vault` crate that encrypts credentials, the `temm1e-core` traits that define the architectural contracts every implementation must honour, the `temm1e-cambium` pipeline crate itself, the `temm1e-watchdog` supervisor binary, the security-enforcement code in the gateway, and `docs/lab/cambium/PROTECTED_ZONES.md` which catalogues all of the above with SHA-256 checksums. The heartwood can never be modified by Cambium. If a growth session somehow produces a diff that touches a heartwood file, the zone-compliance stage catches it via checksum comparison and hard-aborts the session with no retry allowed. This matches the botanical reality: damage to the heartwood of a large tree is survivable, but the tree cannot regenerate it; heartwood can only be protected.

**Cambium** is where growth actually happens: `temm1e-tools`, `temm1e-skills`, `temm1e-channels`, `temm1e-cores`, `temm1e-anima`, `temm1e-distill`, `temm1e-hive`, `temm1e-observable`. These are the crates where new tools can be added, new skills authored, new specialist cores created, new channel adapters written. The pipeline's job is to make sure everything added to this layer compiles, lints, tests, and integrates without regressing what already exists.

**Bark** is the runtime surface — the gateway, the agent loop, the main binary — where the system meets the outside world through channels and the command-line interface. Bark can be improved through Cambium but only under approval, because changes here affect every user on every message.

**Rings** are the growth sessions themselves. Every session, successful or failed, is recorded as an append-only `GrowthSession` entry with its trigger, kind, stages, diff statistics, git commit SHA, outcome, and cost in USD. These records form a permanent track record the trust engine reads to decide what Tem has earned the right to do autonomously. The rings are to Cambium what dendrochronology is to a forester: a complete, inspectable history of how the organism grew.

The metaphor is not decorative. It constrains the architecture. Asking "does this change touch the heartwood?" during design is the same act as running the zone-compliance stage at runtime. The mental model and the mechanical check are aligned, and alignment between the model builders carry in their heads and the checks the machine performs is the foundation of safe self-modification.

---

## 3. First Principles

The theory document `docs/lab/cambium/THEORY.md` states ten principles that together bound what Cambium can and cannot do. They are not independent; each exists because an obvious failure mode would otherwise be available.

**Principle 1: Growth is gap-driven, never speculative.** Every growth action closes a specific observable gap — a panic in the logs, a user request the agent could not fulfil, a measured quality degradation, a pattern of user corrections. There is no "this might be useful someday." Speculative growth leads to bloat; gap-driven growth ensures every change has a measurable before and after. The alternative — a system that tries to improve itself because its free time is otherwise unused — is indistinguishable from a busy process that accumulates entropy.

**Principle 2: The growth system is not growable.** The pipeline that enables growth is itself immutable. Tem cannot modify its own modification process. This is the foundational safety guarantee: if the growth system could modify itself, there would be no stable ground on which to reason about safety, because any safety proof would have to be recomputed after every edit to the prover.

**Principle 3: Verification is deterministic and LLM-agnostic.** The verification pipeline uses mechanical checks, not AI judgment. `cargo check`, `cargo clippy`, `cargo test`, SHA-256 comparison, budget arithmetic — these produce binary yes/no answers regardless of which model generated the code. A more persuasive model cannot talk the pipeline into shipping a broken patch, because the pipeline is not capable of being persuaded. This is an inversion of the common fear that smarter models will find verifier-bypass tricks; here, the verifier has no softness to exploit.

**Principle 4: Trust is earned through track record.** New growth capabilities start at maximum oversight. Successful changes build trust through consecutive-success streaks; failures reset streaks and, after three rollbacks in a rolling window, revert all levels to approval-required. This creates a natural ramp: cautious start, earned freedom, rapid correction on failure. It is a better scheme than asking a human to decide up front how much to trust a system that has not yet demonstrated anything.

**Principle 5: Capability expansion does not equal authority expansion.** If Tem grows a new `deploy-to-production` tool, it does not thereby gain the right to deploy without user authorisation. Capabilities (what tools exist) and authorities (what actions may be taken without asking) are orthogonal axes. This prevents growth from silently enlarging what Tem is allowed to do.

**Principle 6: The entity knows itself.** Tem maintains a current, accurate self-model of its architecture — crate graph, trait contracts, tool registry, message flow — rooted in `docs/lab/cambium/ARCHITECTURE.md` and machine-readable schemas. A surgeon who does not know anatomy is not a surgeon; an entity that modifies its own code without understanding its own architecture is performing blind mutation.

**Principle 7: Every growth action is reversible.** No Cambium change creates irreversible state. Every change is committed to git, every deployed binary has a timestamped backup, every SQLite database is append-only where the history lives. If a change causes harm, it can be undone completely. Reversibility turns every growth action from a permanent commitment into a bounded experiment.

**Principle 8: Growth has a cost and must be justified.** Every session consumes tokens, compute, and time, and every session carries some residual risk. The expected benefit must exceed the expected cost. The principle forces the trigger-validation stage to ask "how many users are affected and how severely?" rather than "is this idea interesting?"

**Principle 9: Transparency is non-negotiable.** Every action is logged with full context: what triggered it, what was changed, which stages passed and failed, what the outcome was, what it cost. The user can inspect every change and understand every decision. An opaque self-modifying system is structurally untrustworthy; transparency converts uncertainty into informed oversight.

**Principle 10: The human can always override.** Cambium can be disabled entirely via config or a slash command, any specific change can be rolled back, trust levels can be reset, protected zones can be added. The system serves the user; the user does not serve the system. Any architectural decision that would make this untrue is rejected on principle.

These ten principles are not arbitrary — each corresponds to a class of failure we can name. Remove any one of them and the system gains an attack surface that the others cannot patch.

---

## 4. Architecture

### 4.1 The trust hierarchy

Files and directories in the TEMM1E workspace are partitioned into four trust levels, catalogued in `docs/lab/cambium/PROTECTED_ZONES.md`.

**Level 0 — Immutable kernel.** The pipeline code itself (`crates/temm1e-cambium/`), the zone enforcer, `temm1e-vault`, the trait definitions in `temm1e-core/src/traits/`, the security enforcement in the gateway, the workspace `Cargo.toml` structure, `THEORY.md`, `PROTECTED_ZONES.md`, and the `temm1e-watchdog` binary. SHA-256 checksums are validated before every commit. If any Level 0 checksum changes, the entire session is aborted and the user is alerted. These files define what Cambium is and how it operates; if they could be modified, there would be no stable safety guarantee.

**Level 1 — Approval required.** The agent runtime (`temm1e-agent`), the gateway, provider integrations, Perpetuum scheduling, memory backends, `src/main.rs`, and any `Cargo.toml` dependency changes. The pipeline generates a diff and waits for explicit human approval before committing. A bug here affects every user and every message, so human judgment is required.

**Level 2 — Autonomous with full pipeline.** `temm1e-tools`, `temm1e-skills`, `temm1e-channels`, `temm1e-cores`, `temm1e-anima`, `temm1e-observable`, `temm1e-hive`, `temm1e-distill`. Full pipeline (compile, lint, test, code review, security audit) with no human gate, but every change is logged and the human can roll back at any time. These are extension points: adding a tool does not affect existing behaviour if all tests pass, so the blast radius is contained.

**Level 3 — Autonomous with basic pipeline.** `docs/`, new tests, `~/.temm1e/skills/*.md` runtime skill files, `~/.temm1e/cores/*.md` runtime core definitions. Compilation check only. Zero runtime risk because skills and core definitions are interpreted text, not compiled code.

Crucially, trust is earned, not granted at birth. Every level except Level 0 begins at approval-required. Ten consecutive successful Level 3 changes graduate Level 3 to autonomous. Twenty-five consecutive successful Level 2 changes graduate Level 2 to autonomous. Any rollback at a level resets its streak and imposes a cooldown. Three rollbacks in a seven-day window force all levels back to approval-required. The `TrustEngine` implementation in `crates/temm1e-cambium/src/trust.rs` encodes this state machine and persists it through the `history` module.

### 4.2 The verification pipeline

Every growth session passes through thirteen stages in strict order. Each stage is a binary gate: pass or fail. There is no partial pass. The orchestrator lives in `crates/temm1e-cambium/src/pipeline.rs`; the code-generation step is isolated behind a `CodeGenerator` trait so the pipeline is testable with mock generators and reusable across LLM providers.

Stage 0, **TriggerValidation**, checks that the growth trigger describes a real gap, is within scope, and is within the daily budget. If it fails, no cost has been incurred. Stage 1, **SelfBriefing**, reads the self-model docs and the relevant crate documentation and produces an architecture-aware modification plan. Stage 2, **CodeGeneration**, delegates to the pluggable generator which writes modified files into the sandbox. Stage 3, **ZoneCompliance**, classifies every changed file against the zone manifest, rejects any touch on a Level 0 file by hard abort, and verifies the checksums of the immutable kernel. Stage 4, **Compilation**, runs `cargo check --workspace` inside the sandbox and requires zero errors. Stage 5, **Linting**, runs `cargo clippy --workspace --all-targets --all-features -- -D warnings` and requires zero warnings. Stage 6, **Formatting**, runs `cargo fmt --all -- --check` and auto-fixes on failure because formatting is a solved mechanical problem. Stage 7, **TestSuite**, runs `cargo test --workspace` and requires every existing test to pass plus new tests to exist for new code. Stage 8, **CodeReview**, invokes the TemDOS CodeReviewer core for architecture compliance, code quality, error handling, and test coverage. Stage 9, **SecurityAudit**, invokes the TemDOS SecurityAuditor core for OWASP-style vulnerabilities, injection, credential exposure, and unsafe patterns. Stage 10, **IntegrationTest**, builds a release binary and runs a ten-turn conversation test against it. Stage 11, **Deployment**, is the blue-green binary swap. Stage 12, **PostDeployMonitoring**, watches the new binary for five minutes and reverts on any regression.

The pipeline state machine is enumerated in `temm1e-core/src/types/cambium.rs` as `PipelineStage` and `StageResult`. Stages 8, 9, 10, and 12 are currently stubbed in the Phase 4 implementation in `pipeline.rs` and will be wired in later phases; the pipeline records them as `Skipped` with a reason so the history is complete and auditable.

### 4.3 Sandbox isolation

All code generation, compilation, and testing happens inside a dedicated git clone at `~/.temm1e/cambium/sandbox/`. The production workspace is never touched. The sandbox is deliberately a full clone rather than a `git worktree`, because a clone has a separate `.git` database, a separate `target/` directory, and a separate working tree; even a catastrophic bug in the code generator cannot affect the production codebase. The sandbox also keeps a persistent build cache so incremental builds after the first cold compile take seconds rather than minutes, which matters because growth diffs are typically small.

Sandbox synchronisation uses `git fetch origin` followed by `git reset --hard origin/main` — read-only on the upstream, never pushes. When a session commits, it commits to a branch named `self-grow/<session-id>` which never touches `main` and never force-pushes. The sandbox lifecycle is implemented in `crates/temm1e-cambium/src/sandbox.rs`.

### 4.4 Deploy, watchdog, and blue-green swap

The deploy module in `crates/temm1e-cambium/src/deploy.rs` performs a six-stage binary swap: validate the new binary by running `--version` with a timeout; back up the current installed binary to a timestamped file in the rollback directory; optionally stop the running process via SIGTERM with SIGKILL fallback; install the new binary via atomic rename with remove-and-copy fallback; optionally start the new process and detect immediate crashes via `try_wait`; health-check the new process. Every failure path attempts automatic rollback. Every stage has a timeout. Nothing hangs indefinitely.

The watchdog is a separate minimal binary in `crates/temm1e-watchdog/src/main.rs`, under 250 lines of code, with no AI, no network, no complexity. It polls the supervised PID file every `interval` seconds, and if the process is dead it restarts from the configured binary path. It enforces a bounded restart window (default five restarts per five minutes) to prevent infinite restart loops. The watchdog is part of the immutable kernel: even if the `temm1e` binary is replaced with a broken version, the watchdog continues running from its own original code and can restore service. Simplicity is intentional; the smaller the watchdog's surface area, the smaller its bug surface, and the more trustworthy its role as last-resort supervisor.

### 4.5 Crate structure and position within TEMM1E

TEMM1E is a Cargo workspace of twenty-four crates plus a root binary. Cambium adds two: `temm1e-cambium` (the library crate containing `zone_checker`, `trust`, `budget`, `history`, `sandbox`, `pipeline`, and `deploy`) and `temm1e-watchdog` (the supervisor binary). Phase 1 added `CambiumConfig` to `temm1e-core/src/types/config.rs` and the type enumerations (`GrowthTrigger`, `GrowthKind`, `TrustLevel`, `PipelineStage`, `StageResult`, `GrowthSession`, `GrowthOutcome`, `TrustState`) to `temm1e-core/src/types/cambium.rs`. Phase 3 added the `CambiumSkills` variant to `SelfWorkKind` in `crates/temm1e-perpetuum/src/conscience.rs`, dispatched it in `cortex.rs`, and implemented `grow_skills()` in `crates/temm1e-perpetuum/src/self_work.rs`. This is the only place Cambium currently has a live caller inside the runtime; the library crate `temm1e-cambium` is otherwise dormant, waiting to be wired into code-level growth in Phase 6.

---

## 5. Timeproof Design

The central design decision in Cambium is the clean separation between a pluggable code generator and a deterministic verification harness. The code generator is an LLM behind a trait-object call; the harness is `cargo` and SHA-256 and diff arithmetic. The generator varies with time, provider, and prompt. The harness is constant.

This separation is what makes the system timeproof. A 2030-era model that writes perfect Rust will still go through the same thirteen-stage pipeline — it will just pass more quickly. The pipeline does not need to "keep up" with model capability because it verifies outcomes, not process. Compilation catches type errors with probability one regardless of who wrote the code. Tests catch behavioural regressions with probability proportional to coverage, and the coverage belongs to the host project, not the model. Clippy catches known anti-patterns with probability one. Security audits catch known vulnerability patterns. As test coverage increases, the pipeline's catch rate increases, which in turn means the model can be trusted with riskier changes because the safety net is stronger.

The table below, drawn from `docs/lab/cambium/THEORY.md`, projects how the system behaves across different capability eras. The pipeline is the constant; the generator is the variable.

| Era        | Code quality                                            | Pipeline behaviour                               | Growth rate               |
|------------|--------------------------------------------------------|-------------------------------------------------|---------------------------|
| 2026       | Good for simple tools; struggles with complex refactors | More retries; simpler changes; more oversight   | ~1–3 changes per week     |
| 2027–2028  | Reliable for most features; some architectural changes  | Fewer retries; moderate changes                 | ~1–2 changes per day      |
| 2029–2030  | Expert-level across full codebase                       | Near-zero retries; complex features             | ~5–10 changes per day     |
| 2030+      | Surpasses most human developers                         | Pipeline becomes the only bottleneck            | Rate-limited by policy    |

In the last row the rate limit is not a capability ceiling but an intentional policy — the daily session limit, the token budget, and the cooldown after failure. These are choices made to keep growth legible to the humans watching it, not compromises with the verifier. The verifier itself remains infinitely patient.

The argument for this architecture is simple and worth stating plainly. If the verifier depended on model judgment, a more capable model could in principle find arguments that fooled it. If the verifier is mechanical — compile, test, lint, checksum, diff-limit — there is nothing for a more capable model to fool. The pipeline's false-negative rate (letting a bug through) is independent of the generator's capability, and therefore the system's safety guarantees do not erode as the generator improves. This inversion of the usual concern — "smarter models are more dangerous" becomes "smarter models are more productive because the safety floor is flat" — is the whole reason the architecture is worth building.

---

## 6. Empirical Validation

### 6.1 Unit and integration coverage

The `temm1e-cambium` crate ships with unit tests for `zone_checker`, `trust`, `budget`, `history`, `sandbox`, `pipeline`, and `deploy`. The pipeline is tested with mock `CodeGenerator` implementations — a no-op generator that produces no diff, a failing generator that returns `Err`, and variants that simulate specific stage failures — allowing full state-machine coverage without invoking a real LLM. The integration test suite in `crates/temm1e-cambium/tests/deploy_integration_test.rs` uses shell-script "binaries" that respond to `--version` and `start` to exercise the full `Deployer::swap()` protocol against an isolated tempdir. A second integration test in `crates/temm1e-cambium/tests/real_temm1e_swap_test.rs` is gated by the `TEMM1E_CAMBIUM_REAL_TEST=1` environment variable and swaps an actual sixteen-megabyte `temm1e` release binary in an isolated `/tmp/temm1e-deploy-test/` directory, measuring per-stage timing. The real-binary swap completes in approximately 477 milliseconds total: 475 ms for the `cargo --version` cold-start validation, and essentially zero for the atomic rename, backup, and health check. At the workspace level, 2274 tests pass with zero failures after Phase 5.

### 6.2 The real-LLM proof

The decisive test is `crates/temm1e-perpetuum/tests/real_llm_grow_skills_test.rs`, which calls real provider APIs rather than mocks. The test seeds a SQLite store with seven synthetic activity notes describing a recurring user need (Docker container inspection, log reading, restart flow, CPU monitoring, `exec`-into patterns, Kubernetes development context), then invokes `execute_self_work(SelfWorkKind::CambiumSkills, ...)` against two providers in sequence:

- **Gemini 3 Flash** (`gemini-3-flash-preview`), representing the cheap tier.
- **Anthropic Claude Sonnet 4.6** (`claude-sonnet-4-6`), representing the medium tier.

API keys are read from environment variables first and fall back to `~/.temm1e/credentials.toml`, using the most recent key to tolerate rotated secrets. The test redirects skill output to a tempdir via the `TEMM1E_CAMBIUM_SKILLS_DIR` environment variable so that `~/.temm1e/skills/` is never touched. Each provider runs one session. After the session, the test loads the resulting `.md` files with a real `SkillRegistry::load_skills()` to verify they are usable in production.

The results are recorded in the commit body of `1f8c5d0` on the `self-growing` branch:

| Provider          | Model                      | Result | Files written | Elapsed (ms) |
|-------------------|----------------------------|--------|---------------|--------------|
| Gemini 3 Flash    | gemini-3-flash-preview     | OK     | 1             | 5,847        |
| Sonnet 4.6        | claude-sonnet-4-6          | OK     | 2             | 12,810       |

Both providers successfully analysed the activity notes, produced valid JSON skill suggestions, wrote parseable YAML-frontmatter-with-markdown-body skill files, and the files were loadable by the production `SkillRegistry`. Total cost across both providers was under two cents. Sonnet produced more granular output — two skills where Gemini produced one — which matches the intuition that a larger model finds more fine-grained distinctions in the same input. The cheap tier still worked, which is the more important observation: self-growth is not gated behind the frontier model of the moment. An agent deployed with a budget-conscious provider is still capable of filling gaps in its own capability surface.

### 6.3 Bugs found by integration testing

The deployment module was covered by shell-script integration tests before it was ever pointed at a real binary. That test harness found four production-class bugs that a synthetic unit test would not have caught.

**Bug 1: SIGTERM does not kill shell scripts that invoke `sleep`.** The initial test harness wrote fake binaries as `#!/bin/sh\n...\nsleep 60\n`. Sending SIGTERM to the script's PID terminated the shell but left the child `sleep` process running; the Deployer's stop-and-wait loop hung because the PID file referenced the shell and the actual process in the process table was the orphaned sleep. The fix was to invoke `exec sleep 60` so that the signal targets the intended process directly. This bug, caught during fake-binary testing, is a shell-scripting detail that would have remained invisible in synthetic unit tests.

**Bug 2: Spawned processes that exit immediately appear briefly alive.** `Deployer::start_new()` originally wrote the PID file and returned success without checking whether the spawned child had already exited. A broken new binary that crashed on startup would leave a valid-looking PID file pointing at a PID that was alive for zero milliseconds. The health check would then look for a zombie and sometimes find it. The fix was to `sleep(500ms)` and then call `try_wait()` on the `Child` handle; if the child has already reaped, the deploy reports immediate-exit failure and triggers rollback.

**Bug 3: macOS code-signing cache invalidation.** On macOS, copying a new Mach-O binary over an existing file at the same path triggers the kernel's code-signing cache to invalidate, and the next `exec()` of the path is killed by signal with exit code `None`. The original `install_new()` implementation used `tokio::fs::copy` which overwrites in place. The fix is two-fold: prefer `tokio::fs::rename` which atomically replaces the inode, and in the fallback remove-and-copy path, remove the target first to force a new inode. The same fix applies in `rollback()`. This bug existed only on macOS; Linux test runs were perfectly happy.

**Bug 4: Zombie processes report as alive on macOS.** `is_process_alive(pid)` originally used `kill -0 <pid>` which returns success on Linux for any PID with an owning process. On macOS, `kill -0` returns success even for zombies — processes that have exited but whose parent has not yet reaped them. The test would start a binary that crashed on `start`, watch `kill -0` report it alive, and then fail confusingly because the process did not respond. The fix adds a `ps -o state= -p <pid>` check that treats any state starting with `Z` as dead. On Linux the fast path uses `/proc/<pid>/status` directly.

The recurring theme across these four bugs is that the integration test harness — shell scripts, real PID management, real file operations, real `exec()` calls — exercised parts of the system that a clean in-process unit test would have smoothed over. They are not obscure corner cases; they are the sort of bugs that would surface in production as silent corruption if the module were trusted based on unit coverage alone. The lesson is general: for a deploy module, integration tests are not optional.

---

## 7. Real Sample Output

The most concrete evidence that Cambium works is the content it produces. During the real-LLM test run, Sonnet 4.6 wrote two files into the tempdir skills directory: `self-grow-docker-container-inspection.md` and `self-grow-docker-container-management.md`. Gemini 3 Flash wrote one: `self-grow-docker-container-management.md`.

Each file was a valid TEMM1E skill. The Gemini file had YAML frontmatter identifying the skill name, a human-readable description, a capabilities array listing `docker`, `containers`, `devops`, `monitoring`, and `troubleshooting`, a version string, and a markdown body with step-by-step instructions including shell command examples for listing containers, inspecting state, following logs, and executing a shell inside a running container. The Sonnet files were similar in structure but more granular, separating container inspection from container management into two distinct procedural skills, each with its own capability list. Both files loaded without error into the production `SkillRegistry::load_skills()` implementation in `temm1e-skills`, which means they were immediately usable by the agent's `use_skill` tool — no binary restart, no manual curation, no hand-editing.

The significance is not that these specific skills are production-worthy as shipped (they describe Docker, which is a topic two major models know well). The significance is that the closed loop runs end to end: synthetic activity notes in, LLM analysis, structured JSON output, filesystem write, registry reload, tool invocation. Every link in the chain held under real provider output. The earlier test had to be patched when Sonnet wrapped its JSON array in a markdown code fence — a new `extract_json_array` helper in `self_work.rs` locates the first `[` and last `]` in the response and extracts the enclosed substring, with six unit tests covering plain, fenced, prose-wrapped, empty, nested, and no-bracket cases. This is the kind of defensive parsing self-growing systems need: they must assume their upstream model output is messy and extract signal from it. After the fix, both providers produced loadable skills on the first try.

---

## 8. Discussion

### 8.1 What works today

The library crate `temm1e-cambium` is complete: zone checker with checksum validation, trust state machine with graduated autonomy, budget enforcement with line and file caps, JSON-file history persistence, git-cloned sandbox with synchronisation and branching, thirteen-stage pipeline orchestrator with pluggable code generator, deployer with validate-backup-stop-install-start-health-rollback protocol, per-stage timing capture, and a minimal supervisor binary `temm1e-watchdog` under 250 lines of Rust with cross-platform process-liveness checks. The skill-layer growth path is wired into the running agent through `SelfWorkKind::CambiumSkills` in Perpetuum; Tem can, today, analyse recent activity and author new skill files during its Sleep state without a binary restart. This has been proven against both a cheap-tier and a medium-tier real LLM. All 2274 tests in the workspace pass, clippy is clean, fmt is clean, and the cost of the full real-LLM validation is under two cents.

### 8.2 What is deferred

Five items are known-incomplete and tracked for Phase 6 and beyond.

**Drain mode.** The blue-green swap currently takes a brief downtime window of roughly three to five seconds between stopping the old process and starting the new one. A proper drain mode would stop accepting new channel messages, complete in-flight message processing within a 30-second timeout, persist in-flight state to a SQLite checkpoint table, and only then shut down. The Deployer has all the hooks for it; the drain signal across Gateway and channel adapters is not yet wired.

**Multi-instance coordination.** Cambium assumes a single running `temm1e` process per installation. For a cluster with multiple instances sharing a memory backend, the swap protocol would need leader election and staggered rollout. This is a natural extension of the existing design but has no current implementation.

**TemDOS review and audit cores.** Stages 8 (`CodeReview`) and 9 (`SecurityAudit`) are currently skipped by the pipeline with a "TemDOS core integration deferred" reason. The core definitions themselves live in `temm1e-cores` and exist; the wiring from pipeline to core is the missing piece.

**Integration test stage.** Stage 10 runs a ten-turn conversation against the staged binary and is currently skipped. The infrastructure for 10-turn CLI self-tests already exists (see the user-facing self-testing protocol in TEMM1E's project memory) and needs only to be invoked from within the pipeline with the staged binary path.

**Post-deploy monitoring.** Stage 12 runs for five minutes after deployment, watching error rates and quality scores, and auto-rolls-back on regression. This is wired as a skipped stage today. Implementation requires hooking the monitoring into the Distill quality scores and the observable error rate counters.

None of these blocks Phase 3 skill-layer growth, which is the currently-live capability. All are required before Phase 6 unlocks autonomous code-level growth.

### 8.3 Comparison with other approaches

Erlang/OTP hot code reloading is the closest prior art. The OTP runtime allows a running process to atomically swap its executing module while holding state, which is exactly the operational move Cambium's deploy module performs at the binary level. But Erlang hot reload is a developer tool, not a self-directed capability: humans write the new module, humans compile it, humans call `code:load_file/1` to swap it in. There is no mechanical verification harness to reject a module that breaks invariants, and there is no self-observation loop that identifies which gaps to fill. Erlang gives you the swap; Cambium gives you the swap plus the decision of when and why and the verification that it is safe.

Kubernetes rolling deployments share the same downtime-minimising ethos but operate on container images with readiness probes rather than source code. They assume the human operator has decided the new image is correct; the orchestrator only verifies that pods become healthy. The "is this change safe" question is externalised to CI pipelines that the cluster does not own.

Plugin systems (browser extensions, VSCode extensions, database extension frameworks like Postgres hooks) allow third-party code to extend a host, but the decision to extend comes from outside the system and the plugin interface is typically much narrower than the native codebase. A plugin-based architecture is by design unable to evolve its own core.

Modern AutoML and neural architecture search systems self-modify neural network parameters and structure, but operate in the weight-space of one model rather than in the runtime code around the model. They are what "self-learning" looks like in the Lambda/Anima sense, not what runtime evolution looks like.

Cambium is, to our knowledge, the first published design for a production Rust AI agent runtime that observes its own limitations, writes native code to address them, verifies the code through a fixed mechanical harness, and deploys it through blue-green swap supervised by an immutable watchdog. Every piece of this individually exists somewhere; the combination and the first-principles framing do not.

### 8.4 The path forward

The next concrete milestone is Phase 6: wiring `temm1e-cambium` into the agent runtime as a full code-level growth path, backed by a LLM provider, targeting Level 2 (tools, skills, channels, cores, anima, distill, hive, observable) with approval required until each zone earns its autonomy badge. Once this is live, the remaining deferred items — drain mode, TemDOS review cores, integration stage, post-deploy monitoring, multi-instance coordination — become required for responsible autonomous operation and will be built in that order.

---

## 9. Conclusion

We built Cambium to make a specific architectural claim: a cloud-native AI agent runtime should be factored into an immutable kernel and a growable surface, and the surface should grow under the control of a deterministic verification harness driven by a pluggable LLM-backed code generator. This factoring produces a system whose safety guarantees are independent of the generating model's capability — a 2030-era model with expert-level Rust fluency will ship changes through the same pipeline as a 2026 cheap-tier model, and the pipeline's false-negative rate is a property of the host codebase's test coverage, not of the model behind the curtain.

The proof that this is not merely a design document is the real-LLM empirical validation. Both a cheap-tier model (Gemini 3 Flash) and a medium-tier model (Claude Sonnet 4.6) successfully analysed synthetic activity data, identified capability gaps, wrote structured skill suggestions, and had their output land as loadable production-ready skill files in a TEMM1E registry — at a total cost under two cents and with zero hand-editing. The integration test harness for the deployment module exposed four production-class bugs (SIGTERM on shell scripts, zero-millisecond-lifetime processes, macOS code-signing cache, zombie detection) that unit tests would have missed; the module is stronger for them. The full workspace remains at 2274 passing tests, zero failures, clippy clean, fmt clean.

The broader claim is that this is what an AI agent runtime must look like to compound improvements over time rather than stagnate as frozen code against accelerating models. Every year the underlying LLMs get better. Every year the gap between what a user wants and what a static runtime can deliver widens. Cambium is the mechanism by which Tem closes that gap from its own side of the interface — gap-driven, verified, reversible, transparent, human-overridable. It is the cambium layer of a living system: thin, active, adding rings at the edge, while the heartwood holds the line.

---

## References

- TEMM1E Project. *Cambium: Theoretical Foundation* (`docs/lab/cambium/THEORY.md`), working theory v0.1, 2026-04-08.
- TEMM1E Project. *Cambium: Implementation Plan* (`docs/lab/cambium/IMPLEMENTATION_PLAN.md`), pre-implementation gate document, 2026-04-08.
- TEMM1E Project. *Cambium: Protected Zones* (`docs/lab/cambium/PROTECTED_ZONES.md`), SHA-256 catalogue of the immutable kernel.
- TEMM1E Project. *Cambium: Architecture* (`docs/lab/cambium/ARCHITECTURE.md`), crate map, dependency graph, message flow.
- TEMM1E Project. *Cambium: Coding Standards* (`docs/lab/cambium/CODING_STANDARDS.md`), rules self-grown code must follow.
- TEMM1E Project. *TEMM1E Vision* (`docs/TEMM1E_VISION.md`), the cloud-native headless agent runtime thesis.
- TEMM1E Project. OpenClaw architecture research (`docs/openclaw/ARCHITECTURE.md`), the TypeScript predecessor whose agent loop informed TEMM1E's initial trait decomposition.
- TEMM1E Project. ZeroClaw architecture research (`docs/zeroclaw/ARCHITECTURE.md`), the Rust predecessor whose trait-based modularity informed the twelve-trait extension that underpins Cambium's zone model.
- Ericsson / Erlang OTP Team. *OTP Design Principles: Release Handling and Hot Code Loading*. The OTP reference treatment of module-level hot swap, the closest prior art to Cambium's binary-level swap and the baseline against which the separation of "who decides" and "who verifies" is made.
- Aristotle. *Nicomachean Ethics*, Book VI. The distinction between *techne* (making) and *praxis* (doing) considered during naming — growth as embodied practice rather than mere output — and rejected in favour of the biological metaphor because the pipeline is explicitly mechanical, not practical in Aristotle's sense.
- Esau, K. *Anatomy of Seed Plants*, 2nd ed., Wiley, 1977. Reference botanical treatment of the vascular cambium, heartwood, and the growth-ring formation that gave the subsystem its name and its architectural metaphor.
- TEMM1E Project. Commit `f6804fb` (Phases 0–4 initial implementation), `6708dcb` (Phase 5 blue-green deploy and watchdog), `403e8d6` (Phase 3 live verification integration tests), `1f8c5d0` (real-LLM end-to-end proof with Gemini 3 Flash and Claude Sonnet 4.6) on branch `self-growing`.
