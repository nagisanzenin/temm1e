# Cambium: Implementation Plan

> **Status:** Pre-implementation — research complete, awaiting confidence gate  
> **Date:** 2026-04-08  
> **Branch:** `cambiuming`  
> **Prerequisite:** THEORY.md (approved)  
> **Rule:** NO CODE until all research items for a phase show GREEN

---

## Guiding Constraints

1. **Zero risk to current Tem users.** Every phase is either pure documentation, additive-only code behind `enabled: false`, or isolated in a new crate with no callers.
2. **Each phase is independently shippable.** If we stop after Phase 2, the system is strictly better (has types, has a library), never worse.
3. **Success metrics are mechanical.** Not "feels right" — a script or command produces a pass/fail.
4. **Rollback for every phase is trivial.** Revert the commits. No migrations, no state changes, no user-visible impact.

---

## Phase Dependency Graph

```
Phase 0 ─── docs only, zero code
  │
  v
Phase 1 ─── types + config (disabled by default)
  │
  v
Phase 2 ─── temm1e-cambium crate (library, no callers)
  │
  v
Phase 3 ─── skill-layer growth (first integration)
  │
  v
Phase 4 ─── code pipeline (branch-only, no deploy)
  │
  v
Phase 5 ─── blue-green deploy (binary swap + watchdog)
```

Each phase gates on the previous phase's success metrics being **fully met**.

---

## Phase 0: Codebase Self-Model (Documentation Only)

### What Changes
Pure documentation. Zero code files modified.

### Deliverables

| File | Purpose |
|------|---------|
| `docs/lab/cambium/THEORY.md` | Foundational theory (DONE) |
| `docs/lab/cambium/IMPLEMENTATION_PLAN.md` | This file |
| `docs/lab/cambium/ARCHITECTURE.md` | Crate map, dependency graph, message flow |
| `docs/lab/cambium/PROTECTED_ZONES.md` | Level 0/1/2/3 file lists with SHA-256 checksums |
| `docs/lab/cambium/CODING_STANDARDS.md` | Rules cambiumn code must follow |

### Risk Assessment
**ZERO.** No code changes. No compilation. No behavior change.

### Success Metrics

| # | Metric | Verification Command |
|---|--------|---------------------|
| 0.1 | All 5 docs exist | `ls docs/lab/cambium/*.md \| wc -l` = 5 |
| 0.2 | PROTECTED_ZONES.md lists every Level 0 file with SHA-256 | Manual review |
| 0.3 | ARCHITECTURE.md crate list matches `Cargo.toml` workspace members | Diff check |
| 0.4 | No code files modified | `git diff --name-only \| grep -v docs/` = empty |

### Rollback
`git revert` the docs commit. Zero impact.

---

## Phase 1: Foundation Types & Config

### What Changes
Add `CambiumConfig` struct to config and cambium types to `temm1e-core`. All behind `#[serde(default)]` with `enabled: false`. **Zero behavior change** — the system parses the config section if present, ignores it otherwise.

### Exact File Changes

**File 1: `crates/temm1e-core/src/types/config.rs`**

Add after the existing `SocialConfig` block (~line 500+):

```rust
/// Configuration for cambium capability.
/// Disabled by default. When disabled, cambium has zero runtime cost.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CambiumConfig {
    /// Master switch. When false, no cambium activity occurs.
    #[serde(default)]
    pub enabled: bool,

    /// Maximum lines of code changed per cambium session.
    #[serde(default = "default_max_lines_per_session")]
    pub max_lines_per_session: usize,

    /// Maximum files touched per cambium session.
    #[serde(default = "default_max_files_per_session")]
    pub max_files_per_session: usize,

    /// Maximum cambium sessions per 24-hour period.
    #[serde(default = "default_max_sessions_per_day")]
    pub max_sessions_per_day: usize,

    /// Cooldown in seconds between cambium sessions.
    #[serde(default = "default_cooldown_secs")]
    pub cooldown_secs: u64,

    /// Cooldown in seconds after a failed session.
    #[serde(default = "default_failure_cooldown_secs")]
    pub failure_cooldown_secs: u64,

    /// Trust level override. None = use earned trust state machine.
    /// Some("approval_required") forces human approval for all changes.
    #[serde(default)]
    pub trust_level_override: Option<String>,

    /// Path to codebase self-model docs. Tem reads these before every session.
    #[serde(default = "default_self_model_path")]
    pub self_model_path: String,
}

fn default_max_lines_per_session() -> usize { 500 }
fn default_max_files_per_session() -> usize { 5 }
fn default_max_sessions_per_day() -> usize { 3 }
fn default_cooldown_secs() -> u64 { 3600 }        // 1 hour
fn default_failure_cooldown_secs() -> u64 { 86400 } // 24 hours
fn default_self_model_path() -> String { "docs/lab/cambium".to_string() }

impl Default for CambiumConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_lines_per_session: default_max_lines_per_session(),
            max_files_per_session: default_max_files_per_session(),
            max_sessions_per_day: default_max_sessions_per_day(),
            cooldown_secs: default_cooldown_secs(),
            failure_cooldown_secs: default_failure_cooldown_secs(),
            trust_level_override: None,
            self_model_path: default_self_model_path(),
        }
    }
}
```

Add field to `Temm1eConfig` struct (after `social: SocialConfig`):

```rust
#[serde(default)]
pub cambium: CambiumConfig,
```

**File 2: `crates/temm1e-core/src/types/mod.rs`**

No change needed — config.rs is already exported.

### New File: `crates/temm1e-core/src/types/cambium.rs`

Core types for the cambium system (used by the pipeline crate later):

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// What triggered this growth session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GrowthTrigger {
    /// Tem encountered an error/panic in its own code.
    BugDetected { error_signature: String, occurrences: usize },
    /// User explicitly requested a capability.
    UserRequest { description: String, chat_id: String },
    /// Distill quality score dropped below threshold.
    QualityDegradation { metric: String, current: f64, threshold: f64 },
    /// User correction pattern detected by Anima.
    UserCorrection { pattern: String, frequency: usize },
    /// Manual trigger (developer testing).
    Manual { description: String },
}

/// What kind of change the session aims to produce.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GrowthKind {
    NewTool,
    BugFix,
    Optimization,
    NewSkill,
    NewIntegration,
    NewCore,
}

/// Trust level for a specific modification scope.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrustLevel {
    /// Level 0: Immutable. Cannot be modified by cambium.
    Immutable = 0,
    /// Level 1: Requires explicit human approval before commit.
    ApprovalRequired = 1,
    /// Level 2: Autonomous with full pipeline (compile+test+review).
    AutonomousFull = 2,
    /// Level 3: Autonomous with basic pipeline (compile only).
    AutonomousBasic = 3,
}

/// Result of a single pipeline stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StageResult {
    Passed,
    Failed { reason: String, retry_count: usize },
    Skipped { reason: String },
}

/// Stage identifiers for the verification pipeline.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PipelineStage {
    TriggerValidation,
    SelfBriefing,
    CodeGeneration,
    ZoneCompliance,
    Compilation,
    Linting,
    Formatting,
    TestSuite,
    CodeReview,
    SecurityAudit,
    IntegrationTest,
    Deployment,
    PostDeployMonitoring,
}

/// A complete record of one cambium session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthSession {
    pub id: String,
    pub trigger: GrowthTrigger,
    pub kind: GrowthKind,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub stages: Vec<(PipelineStage, StageResult)>,
    pub files_changed: Vec<String>,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub git_branch: Option<String>,
    pub git_commit: Option<String>,
    pub outcome: GrowthOutcome,
    pub cost_usd: f64,
}

/// Final outcome of a growth session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GrowthOutcome {
    /// Session has not finished yet.
    InProgress,
    /// All stages passed, change committed/deployed.
    Success,
    /// A stage failed after max retries.
    Failed { stage: PipelineStage, reason: String },
    /// Zone compliance violation — hard abort.
    ZoneViolation { files: Vec<String> },
    /// Session budget exceeded.
    BudgetExceeded,
    /// Human rejected the change (Level 1 approval denied).
    Rejected,
    /// Rolled back after deployment (post-deploy monitoring failed).
    RolledBack { reason: String },
}

/// Trust state machine — tracks earned trust from track record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustState {
    /// Consecutive successful Level 3 changes.
    pub level3_streak: usize,
    /// Consecutive successful Level 2 changes.
    pub level2_streak: usize,
    /// Whether Level 2 has graduated to autonomous.
    pub level2_autonomous: bool,
    /// Whether Level 3 has graduated to autonomous.
    pub level3_autonomous: bool,
    /// Total rollbacks in the last 7 days.
    pub recent_rollbacks: usize,
    /// Whether all levels have been reset to approval-required.
    pub all_approval_required: bool,
    /// Last session timestamp (for cooldown enforcement).
    pub last_session_at: Option<DateTime<Utc>>,
    /// Last failure timestamp (for failure cooldown).
    pub last_failure_at: Option<DateTime<Utc>>,
    /// Total sessions today (for daily limit).
    pub sessions_today: usize,
    /// Date of sessions_today counter (resets daily).
    pub sessions_today_date: Option<String>,
}

impl Default for TrustState {
    fn default() -> Self {
        Self {
            level3_streak: 0,
            level2_streak: 0,
            level2_autonomous: false,
            level3_autonomous: false,
            recent_rollbacks: 0,
            all_approval_required: false,
            last_session_at: None,
            last_failure_at: None,
            sessions_today: 0,
            sessions_today_date: None,
        }
    }
}
```

### Why This Is Zero Risk

1. `CambiumConfig` has `#[serde(default)]` on the parent field AND `enabled: false` as default. If `[cambium]` is absent from TOML, the struct deserializes to disabled. No behavior change.
2. `cambium.rs` types are pure data definitions. They have no methods with side effects. They are imported by nothing until Phase 2.
3. No existing code paths are modified. No function signatures change.
4. No new dependencies added.

### Risk Assessment
**ZERO.** Additive types behind serde defaults. No runtime behavior change.

### Success Metrics

| # | Metric | Verification |
|---|--------|-------------|
| 1.1 | `cargo check --workspace` passes | Run command, exit code 0 |
| 1.2 | `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes | Run command, exit code 0 |
| 1.3 | `cargo test --workspace` — all existing tests pass, zero new failures | Run command, compare test count |
| 1.4 | `CambiumConfig::default().enabled == false` | Unit test |
| 1.5 | Existing `temm1e.toml` without `[cambium]` parses correctly | Unit test |
| 1.6 | `temm1e.toml` with `[cambium]\nenabled = true` parses correctly | Unit test |
| 1.7 | All `GrowthTrigger`, `GrowthKind`, `TrustLevel` variants serialize/deserialize roundtrip | Unit test |
| 1.8 | `TrustState::default()` has all streaks at 0, autonomous = false | Unit test |

### Tests to Write

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cambium_config_default_is_disabled() {
        let config = CambiumConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.max_lines_per_session, 500);
        assert_eq!(config.max_sessions_per_day, 3);
    }

    #[test]
    fn config_without_cambium_section_parses() {
        let toml_str = r#"
            [gateway]
            port = 8080
        "#;
        let config: Temm1eConfig = toml::from_str(toml_str).unwrap();
        assert!(!config.cambium.enabled);
    }

    #[test]
    fn config_with_cambium_section_parses() {
        let toml_str = r#"
            [cambium]
            enabled = true
            max_lines_per_session = 200
        "#;
        let config: Temm1eConfig = toml::from_str(toml_str).unwrap();
        assert!(config.cambium.enabled);
        assert_eq!(config.cambium.max_lines_per_session, 200);
        // Other fields use defaults
        assert_eq!(config.cambium.max_sessions_per_day, 3);
    }

    #[test]
    fn growth_trigger_roundtrip() {
        let trigger = GrowthTrigger::BugDetected {
            error_signature: "utf8_boundary".into(),
            occurrences: 5,
        };
        let json = serde_json::to_string(&trigger).unwrap();
        let back: GrowthTrigger = serde_json::from_str(&json).unwrap();
        assert_eq!(trigger, back);
    }

    #[test]
    fn trust_state_default() {
        let state = TrustState::default();
        assert_eq!(state.level3_streak, 0);
        assert!(!state.level2_autonomous);
        assert!(!state.all_approval_required);
    }

    #[test]
    fn trust_level_ordering() {
        assert!(TrustLevel::Immutable < TrustLevel::ApprovalRequired);
        assert!(TrustLevel::ApprovalRequired < TrustLevel::AutonomousFull);
        assert!(TrustLevel::AutonomousFull < TrustLevel::AutonomousBasic);
    }
}
```

### Rollback
`git revert` the commit. Config reverts to not having the field. Zero impact.

---

## Phase 2: `temm1e-cambium` Crate (Library Only)

### What Changes
Create a new crate `temm1e-cambium` with the pipeline logic, zone checker, trust state machine, and session history. **Not wired to any caller.** Pure library with unit tests.

### Why This Is Zero Risk
The crate exists in the workspace but nothing imports it. `cargo build` compiles it. `cargo test -p temm1e-cambium` tests it. No existing code references it. It could be deleted with zero impact.

### Crate Structure

```
crates/temm1e-cambium/
├── Cargo.toml
├── src/
│   ├── lib.rs              — Public API surface
│   ├── zone_checker.rs     — Protected zones enforcement (SHA-256 validation)
│   ├── trust.rs            — Trust state machine (streak tracking, level graduation)
│   ├── budget.rs           — Session budget enforcement (lines, files, daily limit, cooldown)
│   ├── pipeline.rs         — Pipeline stage orchestration (Stage 0-12 state machine)
│   ├── diff_analyzer.rs    — Analyze git diffs for zone compliance, test coverage
│   ├── history.rs          — SQLite persistence for growth sessions and trust state
│   └── skill_grower.rs     — Skill-layer growth (Phase 3 prereq, write .md files)
└── tests/
    ├── zone_checker_test.rs
    ├── trust_test.rs
    ├── budget_test.rs
    └── pipeline_test.rs
```

### `Cargo.toml`

```toml
[package]
name = "temm1e-cambium"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true
description = "Self-grow pipeline: gap-driven code evolution with verification harness"

[dependencies]
temm1e-core = { path = "../temm1e-core" }
serde = { workspace = true, features = ["derive"] }
serde_json.workspace = true
chrono = { workspace = true, features = ["serde"] }
sha2 = "0.10"
sqlx = { workspace = true, features = ["runtime-tokio", "sqlite"] }
tokio = { workspace = true, features = ["fs", "process", "time"] }
tracing.workspace = true
uuid = { version = "1", features = ["v4"] }

[dev-dependencies]
temm1e-test-utils = { path = "../temm1e-test-utils" }
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
tempfile = "3"
```

### Key Module Specifications

#### `zone_checker.rs` — Protected Zone Enforcement

```rust
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use temm1e_core::types::cambium::TrustLevel;

/// Loaded from PROTECTED_ZONES.md — maps file paths to trust levels.
pub struct ZoneManifest {
    /// SHA-256 checksums of Level 0 (immutable) files.
    pub immutable_checksums: HashMap<PathBuf, String>,
    /// File path patterns for each trust level.
    pub zones: HashMap<PathBuf, TrustLevel>,
}

impl ZoneManifest {
    /// Load from docs/lab/cambium/PROTECTED_ZONES.md
    pub fn load(self_model_path: &Path) -> Result<Self, ...>;

    /// Validate that no Level 0 files have been modified.
    /// Compares current file checksums against stored checksums.
    /// Returns Err if ANY Level 0 file's checksum differs.
    pub fn validate_immutable_kernel(&self, workspace: &Path) -> Result<(), Vec<String>>;

    /// Classify a list of changed file paths by trust level.
    /// Returns the HIGHEST (most restrictive) trust level required.
    pub fn classify_changes(&self, changed_files: &[PathBuf]) -> TrustLevel;

    /// Check if a specific file is in the immutable zone.
    pub fn is_immutable(&self, path: &Path) -> bool;
}
```

**Test cases:**
- Level 0 file modified → `validate_immutable_kernel` returns Err
- Level 0 file unchanged → returns Ok
- Mixed Level 1 + Level 2 changes → `classify_changes` returns Level 1
- Only Level 3 changes → returns Level 3
- Symlink to Level 0 file → detected and rejected
- Unknown file (not in manifest) → defaults to Level 1 (safe default)

#### `trust.rs` — Trust State Machine

```rust
use temm1e_core::types::cambium::{TrustLevel, TrustState};
use chrono::{DateTime, Utc};

pub struct TrustEngine {
    state: TrustState,
    config_override: Option<String>,
}

impl TrustEngine {
    pub fn new(state: TrustState, config_override: Option<String>) -> Self;

    /// Record a successful growth session at the given trust level.
    pub fn record_success(&mut self, level: TrustLevel);

    /// Record a failed growth session (rollback).
    pub fn record_failure(&mut self);

    /// Check if a change at the given trust level is currently allowed
    /// without human approval.
    pub fn is_autonomous(&self, level: TrustLevel) -> bool;

    /// Check if cooldown has elapsed since last session.
    pub fn cooldown_elapsed(&self, now: DateTime<Utc>, cooldown_secs: u64) -> bool;

    /// Check if failure cooldown has elapsed.
    pub fn failure_cooldown_elapsed(&self, now: DateTime<Utc>, cooldown_secs: u64) -> bool;

    /// Check if daily session limit has been reached.
    pub fn daily_limit_reached(&self, max_per_day: usize, today: &str) -> bool;

    /// Get current state (for persistence).
    pub fn state(&self) -> &TrustState;
}
```

**Test cases:**
- Initial state: Level 2 is NOT autonomous, Level 3 is autonomous
- After 10 Level 3 successes: Level 3 confirmed autonomous
- After 25 Level 2 successes: Level 2 becomes autonomous
- After 1 Level 2 failure: Level 2 streak resets, requires approval for 10 more
- After 3 failures in 7 days: ALL levels revert to approval required
- Config override "approval_required": all levels require approval regardless of streak
- Cooldown: session within cooldown period → denied
- Daily limit: 3 sessions today → 4th denied
- Date rollover: new day → counter resets

#### `budget.rs` — Session Budget Enforcement

```rust
pub struct SessionBudget {
    max_lines: usize,
    max_files: usize,
    lines_used: usize,
    files_used: Vec<String>,
}

impl SessionBudget {
    pub fn new(max_lines: usize, max_files: usize) -> Self;

    /// Check if adding a diff would exceed the budget.
    pub fn check(&self, lines_added: usize, lines_removed: usize, files: &[String]) -> Result<(), String>;

    /// Record usage from a diff.
    pub fn record(&mut self, lines_added: usize, lines_removed: usize, files: &[String]);

    /// Remaining capacity.
    pub fn remaining_lines(&self) -> usize;
    pub fn remaining_files(&self) -> usize;
}
```

**Test cases:**
- 500 line budget, 400 line diff → passes
- 500 line budget, 600 line diff → fails with descriptive message
- 5 file budget, 3 file diff → passes
- 5 file budget, 6 file diff → fails
- Multiple diffs accumulate correctly

#### `history.rs` — SQLite Persistence

```rust
pub struct GrowthHistory {
    pool: sqlx::SqlitePool,
}

impl GrowthHistory {
    pub async fn new(db_path: &str) -> Result<Self, ...>;

    /// Initialize tables if not exist.
    pub async fn init(&self) -> Result<(), ...>;

    /// Save a growth session record.
    pub async fn save_session(&self, session: &GrowthSession) -> Result<(), ...>;

    /// Load trust state.
    pub async fn load_trust_state(&self) -> Result<TrustState, ...>;

    /// Save trust state.
    pub async fn save_trust_state(&self, state: &TrustState) -> Result<(), ...>;

    /// Count sessions in the last N hours.
    pub async fn sessions_since(&self, since: DateTime<Utc>) -> Result<usize, ...>;

    /// Count rollbacks in the last N days.
    pub async fn rollbacks_since(&self, since: DateTime<Utc>) -> Result<usize, ...>;

    /// List recent sessions (for history display).
    pub async fn recent_sessions(&self, limit: usize) -> Result<Vec<GrowthSession>, ...>;
}
```

**SQLite schema:**

```sql
CREATE TABLE IF NOT EXISTS cambium_sessions (
    id TEXT PRIMARY KEY,
    trigger_json TEXT NOT NULL,
    kind TEXT NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    stages_json TEXT NOT NULL,
    files_changed_json TEXT NOT NULL,
    lines_added INTEGER NOT NULL DEFAULT 0,
    lines_removed INTEGER NOT NULL DEFAULT 0,
    git_branch TEXT,
    git_commit TEXT,
    outcome_json TEXT NOT NULL,
    cost_usd REAL NOT NULL DEFAULT 0.0
);

CREATE TABLE IF NOT EXISTS cambium_trust_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    state_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_started ON cambium_sessions(started_at);
CREATE INDEX IF NOT EXISTS idx_sessions_outcome ON cambium_sessions(outcome_json);
```

**Test cases:**
- Save session → load session → roundtrip matches
- Save trust state → load trust state → roundtrip matches
- sessions_since with various time ranges
- rollbacks_since counts correctly
- In-memory SQLite for all tests (`:memory:`)

### Risk Assessment
**ZERO.** New crate, no callers, no integration points. Could delete the entire crate directory and nothing breaks.

### Success Metrics

| # | Metric | Verification |
|---|--------|-------------|
| 2.1 | `cargo check --workspace` passes | Run command |
| 2.2 | `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes | Run command |
| 2.3 | `cargo test --workspace` — all existing tests still pass | Run command |
| 2.4 | `cargo test -p temm1e-cambium` — all new tests pass | Run command |
| 2.5 | Zone checker catches Level 0 modifications | Test case |
| 2.6 | Trust engine graduates after correct streaks | Test case |
| 2.7 | Trust engine demotes after failure | Test case |
| 2.8 | Budget enforcer rejects over-budget diffs | Test case |
| 2.9 | History roundtrips through SQLite | Test case |
| 2.10 | No existing crate imports `temm1e-cambium` | `grep -r "temm1e.self.grow" crates/ --include Cargo.toml` = only self |

### Rollback
Remove the crate directory and its entry from `Cargo.toml` workspace members. `git revert`. Zero impact.

---

## Phase 3: Skill-Layer Growth (First Live Integration)

### What Changes
This is the first phase where Tem actually DOES something new at runtime. But the "something" is writing `.md` skill files — text files that cannot break compilation, cannot introduce vulnerabilities, and can be deleted to undo.

Three integration points:
1. **SkillRegistry gets a `reload()` method** — so Tem can create a skill and load it without restart
2. **SelfWorkKind gets a `CambiumSkills` variant** — Perpetuum can schedule skill growth
3. **A new self_work handler** in `temm1e-perpetuum/src/self_work.rs` — the actual skill creation logic

### Exact File Changes

**File 1: `crates/temm1e-skills/src/lib.rs`**

Add public method to SkillRegistry (~after line 98):

```rust
/// Reload all skills from disk. Called by cambium after writing new skill files.
/// Safe to call at runtime — clears and re-scans both directories.
pub async fn reload(&mut self) -> Result<usize, Temm1eError> {
    self.load_skills().await?;
    Ok(self.skills.len())
}
```

This is trivially safe: `load_skills()` already clears the list and re-scans. We're just adding a public alias that returns the count.

**File 2: `crates/temm1e-perpetuum/src/conscience.rs`**

Add variant to SelfWorkKind enum (after `Vigil`):

```rust
pub enum SelfWorkKind {
    MemoryConsolidation,
    FailureAnalysis,
    LogIntrospection,
    SessionCleanup,
    BlueprintRefinement,
    Vigil,
    CambiumSkills,  // NEW — skill-layer cambium growth
}
```

Update `name()` match:
```rust
Self::CambiumSkills => "cambium_skills",
```

Update `uses_llm()`:
```rust
pub fn uses_llm(&self) -> bool {
    matches!(
        self,
        Self::FailureAnalysis | Self::LogIntrospection | Self::Vigil | Self::CambiumSkills
    )
}
```

**File 3: `crates/temm1e-perpetuum/src/cortex.rs`**

Add dispatch case (~line 435):
```rust
"cambium_skills" => SelfWorkKind::CambiumSkills,
```

**File 4: `crates/temm1e-perpetuum/src/self_work.rs`**

Add match arm in `execute_self_work()`:
```rust
SelfWorkKind::CambiumSkills => {
    if let Some(caller) = caller {
        grow_skills(store, caller).await
    } else {
        Ok("Skipped: no LLM caller available".to_string())
    }
}
```

Implement `grow_skills()` function (following Vigil pattern):

```rust
/// Skill-layer cambium growth: analyze recent interactions for unmet needs,
/// then generate skill files to address them.
async fn grow_skills(
    store: &Arc<Store>,
    caller: &Arc<dyn LlmCaller>,
) -> Result<String, Temm1eError> {
    // 1. Rate limit: max once per 24 hours
    let notes = store.get_volition_notes().await?;
    if let Some(last) = notes.iter().find(|n| n.note.starts_with("cambium_grow_last:")) {
        if let Some(ts) = n.note.strip_prefix("cambium_grow_last:") {
            if let Ok(last_dt) = DateTime::parse_from_rfc3339(ts) {
                let hours = (Utc::now() - last_dt.with_timezone(&Utc)).num_hours();
                if hours < 24 {
                    return Ok(format!("Skill growth: cooldown ({hours}h < 24h)"));
                }
            }
        }
    }

    // 2. Collect recent activity for gap analysis
    let activity = store.get_recent_activity(20).await.unwrap_or_default();
    if activity.is_empty() {
        return Ok("Skill growth: no recent activity to analyze".to_string());
    }

    // 3. Ask LLM to identify skill gaps
    let system = "You are analyzing recent user interactions to identify \
        capabilities that would be useful as reusable skills. \
        A skill is a markdown file with instructions for handling a specific task type. \
        Only suggest skills for patterns that appeared 3+ times. \
        Respond with JSON: {\"skills\": [{\"name\": \"...\", \"description\": \"...\", \
        \"capabilities\": [\"...\"], \"instructions\": \"...\"}]} \
        or {\"skills\": []} if no gaps found.";

    let prompt = format!(
        "Recent interactions:\n{}\n\nIdentify reusable skill opportunities.",
        activity.iter().map(|a| &a.summary).collect::<Vec<_>>().join("\n")
    );

    let response = caller.call(Some(system), &prompt).await?;

    // 4. Parse LLM response — if no skills suggested, done
    // 5. Write skill files to ~/.temm1e/skills/
    // 6. Record timestamp for rate limit

    store.save_volition_note(
        &format!("cambium_grow_last:{}", Utc::now().to_rfc3339()),
        "self_work",
    ).await?;

    Ok(format!("Skill growth: analyzed {} interactions, created {} skills", 
        activity.len(), skills_created))
}
```

### Activation Path
This code only executes when:
1. Perpetuum is enabled (config: `perpetuum.enabled = true`)
2. Conscience enters Sleep state
3. Sleep state's `work` field is `CambiumSkills`
4. Conscience only selects `CambiumSkills` when `cambium.enabled = true`

Since `cambium.enabled` defaults to `false`, this code is never reached unless explicitly enabled.

### Risk Assessment
**NEAR-ZERO.**
- Skill files are text. They cannot crash the runtime.
- A badly-written skill just produces bad instructions. The user says "that's wrong" and Tem learns.
- Rate limited to once per 24 hours.
- LLM call cost is bounded (single call per session).
- If anything goes wrong, delete `~/.temm1e/skills/cambium-*.md`.

### Success Metrics

| # | Metric | Verification |
|---|--------|-------------|
| 3.1 | All compilation gates pass | `cargo check && cargo clippy && cargo test` |
| 3.2 | `SelfWorkKind::CambiumSkills` serializes/deserializes | Unit test |
| 3.3 | `SkillRegistry::reload()` returns correct count after adding a skill file | Unit test |
| 3.4 | `grow_skills()` respects 24-hour rate limit | Unit test with mocked store |
| 3.5 | `grow_skills()` writes valid skill files parseable by SkillRegistry | Integration test |
| 3.6 | With `cambium.enabled = false`, CambiumSkills never triggers | Integration test |
| 3.7 | With `cambium.enabled = true`, skill created and loadable | Integration test |
| 3.8 | Created skill can be invoked via `use_skill` tool | End-to-end test |

### Rollback
Revert the commits. Delete any `~/.temm1e/skills/cambium-*.md` files. Zero state impact.

---

## Phase 4: Code Pipeline (Branch Only, No Deploy)

### What Changes
Wire `temm1e-cambium` pipeline to actually generate code changes. But changes are **committed to a git branch only** — never deployed, never merged automatically. The user reviews and merges manually.

This is where Tem starts writing Rust code into its own codebase. But the safety net is: the code lives on a branch. If it's bad, `git branch -D cambium/...`. Done.

### Integration Points

1. **New SelfWorkKind: `CambiumCode`** — Perpetuum triggers code growth
2. **Self-grow pipeline wired** — uses shell tool to run `cargo check/test/clippy`
3. **TemDOS cores created** — `CodeReviewer.core` and `SecurityAuditor.core` definition files
4. **Zone checker loaded** — reads PROTECTED_ZONES.md before every session
5. **Trust state machine active** — tracks success/failure streaks

### Key Design Decisions

**Code generation happens in a git worktree:**
```bash
git worktree add /tmp/temm1e-grow-<session-id> -b cambium/<date>-<description>
# ... make changes in worktree ...
# ... run cargo check/test in worktree ...
git worktree remove /tmp/temm1e-grow-<session-id>
```

This means:
- The running codebase is NEVER modified during code generation
- The worktree is disposable — if anything fails, just remove it
- The branch persists in git for human review
- No risk to the running binary

**The pipeline calls cargo as a subprocess:**
```rust
// In the git worktree (NOT the running codebase directory)
let check = Command::new("cargo")
    .args(["check", "--workspace"])
    .current_dir(&worktree_path)
    .output().await?;

let clippy = Command::new("cargo")
    .args(["clippy", "--workspace", "--all-targets", "--all-features", "--", "-D", "warnings"])
    .current_dir(&worktree_path)
    .output().await?;

let test = Command::new("cargo")
    .args(["test", "--workspace"])
    .current_dir(&worktree_path)
    .output().await?;
```

All compilation happens in `/tmp/`, not in the project directory. The running binary is unaffected.

### Risk Assessment
**ZERO to current system.** Code is generated in a temporary worktree, compiled in `/tmp/`, committed to a branch. The running binary, the main branch, and the user's experience are completely unaffected.

The only "risk" is wasted tokens if the LLM generates bad code. Bounded by session budget.

### Success Metrics

| # | Metric | Verification |
|---|--------|-------------|
| 4.1 | All compilation gates pass on main branch | Standard gate |
| 4.2 | Pipeline creates git worktree, makes changes, runs cargo check | Integration test |
| 4.3 | Pipeline aborts on Level 0 zone violation | Test with Level 0 file in diff |
| 4.4 | Pipeline aborts after 3 failed compilation retries | Test with intentionally bad code |
| 4.5 | Pipeline commits to branch on success | Verify git branch exists |
| 4.6 | Pipeline cleans up worktree on success and failure | Verify /tmp/ clean |
| 4.7 | CodeReviewer core produces review score | Test with sample diff |
| 4.8 | SecurityAuditor core catches `unsafe` block | Test with unsafe diff |
| 4.9 | Trust state machine updates correctly | Verify streak after success/failure |
| 4.10 | History records session with all stage results | Query SQLite after session |
| 4.11 | Running binary is completely unaffected during pipeline | Health check passes throughout |

### Rollback
Revert commits on main. Delete any `cambium/*` branches. Zero state impact.

---

## Phase 5: Blue-Green Deploy (Binary Swap + Watchdog)

### What Changes
The final phase: Tem can build a new binary and replace itself. This is the only phase with non-trivial risk, which is why it requires all previous phases to be proven.

### Components

1. **`temm1e-watchdog`** — Separate binary (~100 lines), Level 0 (immutable)
2. **Drain mode** — Signal handler in main binary for graceful connection handoff
3. **State checkpoint** — Serialize active conversations to SQLite before swap
4. **Blue-green swap** — Build → verify → drain → swap → monitor → promote/rollback

### Prerequisite: Phases 0-4 Success Metrics ALL Green

Before Phase 5 coding begins, ALL of the following must be true:
- Phase 4 has produced at least 5 successful code pipeline sessions
- Zero rollbacks in Phase 4
- Trust state machine shows Level 2 autonomous
- All 2127+ tests pass consistently
- User has explicitly approved Phase 5 development

### Risk Assessment
**LOW-MEDIUM.** The watchdog provides automatic rollback. The drain period preserves state. The worst case is a ~30-second service interruption followed by automatic recovery to the previous binary.

Mitigations:
- Watchdog is immutable and separate from Tem
- Old binary always kept as rollback target
- 5-minute monitoring window before declaring success
- Health checks every 10 seconds during monitoring
- SQLite WAL mode for crash-safe persistence

### Success Metrics

| # | Metric | Verification |
|---|--------|-------------|
| 5.1 | Watchdog starts, monitors health endpoint, detects crash | Integration test |
| 5.2 | Watchdog restarts old binary on crash detection | Integration test |
| 5.3 | Drain mode: no new messages accepted, in-flight complete | Load test |
| 5.4 | State checkpoint: conversations restored after swap | Integration test |
| 5.5 | Blue-green swap: new binary takes over port | End-to-end test |
| 5.6 | Rollback: bad binary detected, old binary restored automatically | Fault injection test |
| 5.7 | Zero data loss across swap | Before/after SQLite comparison |
| 5.8 | Channel reconnection: Telegram/Discord resume within 30s | Channel test |
| 5.9 | Perpetuum concerns survive swap | Verify concerns in SQLite |
| 5.10 | Total downtime < 10 seconds | Timing measurement |

### Rollback
The watchdog IS the rollback mechanism. If anything fails, the previous binary is restored automatically.

---

## Confidence Gate Checklist

Before coding ANY phase, ALL items must be GREEN:

### Phase 0 (Documentation)
- [x] THEORY.md written and approved
- [x] All crate structures researched
- [ ] ARCHITECTURE.md written (crate graph matches workspace)
- [ ] PROTECTED_ZONES.md written (all Level 0 files listed with checksums)
- [ ] CODING_STANDARDS.md written

### Phase 1 (Types & Config)
- [x] Config.rs structure researched (Temm1eConfig struct at config.rs:33-77)
- [x] Serde default behavior verified (existing patterns in codebase)
- [x] All type definitions designed (GrowthTrigger, TrustLevel, etc.)
- [x] Test cases enumerated
- [ ] Phase 0 success metrics ALL green

### Phase 2 (temm1e-cambium crate)
- [x] Cargo workspace member pattern researched
- [x] SQLite schema designed
- [x] Zone checker algorithm designed
- [x] Trust state machine transitions defined
- [x] All module APIs specified
- [ ] Phase 1 success metrics ALL green

### Phase 3 (Skill-Layer Growth)
- [x] SkillRegistry API researched (load_skills at lib.rs:62-88)
- [x] SelfWorkKind enum pattern researched (conscience.rs:43-72)
- [x] Vigil pattern studied as reference implementation
- [x] Cortex dispatch routing researched (cortex.rs:114-126)
- [x] Skill file format verified (YAML frontmatter + markdown body)
- [ ] Phase 2 success metrics ALL green

### Phase 4 (Code Pipeline)
- [x] Git worktree command verified
- [x] Shell tool execution pattern researched (sh -c with timeout)
- [x] TemDOS core definition format researched (.md with YAML frontmatter)
- [x] Core invocation pattern researched (invoke_tool.rs)
- [ ] Phase 3 success metrics ALL green
- [ ] At least 5 successful skill-layer growth sessions completed

### Phase 5 (Blue-Green Deploy)
- [x] Signal handling researched (ctrl_c at main.rs:5129-5148)
- [x] Health check endpoint verified (GET /health)
- [x] SQLite WAL behavior understood
- [x] State that survives restart catalogued
- [ ] Phase 4 success metrics ALL green
- [ ] At least 5 successful code pipeline sessions (branch only)
- [ ] Zero rollbacks in Phase 4
- [ ] User explicit approval for Phase 5

---

## Release Checklist (User-Ready Release)

After all phases complete:

1. [ ] All compilation gates pass: `cargo check && cargo clippy && cargo test && cargo fmt --check`
2. [ ] Test count increased (new crate tests + new integration tests)
3. [ ] README updated: cambium section with usage instructions
4. [ ] Config example: `[cambium]` section documented in example config
5. [ ] docs/lab/cambium/ complete: all schema files, session records
6. [ ] 10-turn CLI conversation test passes
7. [ ] Self-grow enabled, triggered, and produces a valid skill (Phase 3 demo)
8. [ ] Self-grow enabled, triggered, and produces a valid code branch (Phase 4 demo)
9. [ ] Binary swap tested end-to-end with watchdog (Phase 5 demo)
10. [ ] Version bumped in Cargo.toml
11. [ ] RELEASE_PROTOCOL.md followed
12. [ ] User approves the release

---

## Current Status

| Phase | Status | Blocking |
|-------|--------|----------|
| Phase 0 | IN PROGRESS | ARCHITECTURE.md, PROTECTED_ZONES.md, CODING_STANDARDS.md |
| Phase 1 | DESIGNED | Phase 0 completion |
| Phase 2 | DESIGNED | Phase 1 completion |
| Phase 3 | DESIGNED | Phase 2 completion |
| Phase 4 | DESIGNED | Phase 3 completion |
| Phase 5 | DESIGNED | Phase 4 completion + user approval |

---

*This plan is the contract. No code is written until the confidence gate for that phase shows all items GREEN. Every phase is independently verifiable and independently reversible.*
