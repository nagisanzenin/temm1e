# Tem Anima — Implementation Spec

> Every file to create, every file to modify, exact line numbers, exact signatures.
> **100% confidence. Zero ambiguity. Ready to code.**

**Date:** 2026-04-04
**Branch:** `emotional-intelligence`
**Architecture:** `TEM_EMOTIONAL_INTELLIGENCE_ARCHITECTURE.md`
**Status:** ALL PHASES COMPLETE (shipped as `temm1e-anima` in v4.3.0)

---

## Table of Contents

1. [New Crate: temm1e-anima](#1-new-crate-temm1e-anima)
2. [Files to Create](#2-files-to-create)
3. [Files to Modify](#3-files-to-modify)
4. [Personality Centralization](#4-personality-centralization)
5. [Execution Order](#5-execution-order)

---

## 1. New Crate: temm1e-anima

### Workspace Registration

**File: `Cargo.toml` (workspace root, line ~20)**
Add to `members`:
```toml
"crates/temm1e-anima",
```

Add to `[workspace.dependencies]` (line ~130):
```toml
temm1e-anima = { path = "crates/temm1e-anima" }
```

### Crate Cargo.toml

```toml
[package]
name = "temm1e-anima"
version = "0.1.0"
edition = "2021"

[dependencies]
temm1e-core.workspace = true
async-trait.workspace = true
serde = { workspace = true, features = ["derive"] }
serde_json.workspace = true
toml.workspace = true
sqlx = { workspace = true, features = ["runtime-tokio", "sqlite"] }
tokio.workspace = true
tracing.workspace = true
sha2 = "0.10"
```

---

## 2. Files to Create

### 2.1 `crates/temm1e-anima/src/lib.rs`

Public API. Exports all modules and the initialization function.

```rust
pub mod types;
pub mod personality;
pub mod facts;
pub mod evaluator;
pub mod user_model;
pub mod communication;
pub mod storage;
pub mod ethics;

pub use personality::PersonalityConfig;
pub use types::*;
pub use storage::SocialStorage;

/// Initialize social intelligence system.
/// Called from main.rs after config load, before agent creation.
pub async fn initialize(
    social_config: &SocialConfig,
    personality: &PersonalityConfig,
    db_url: &str,
) -> Result<SocialEngine, temm1e_core::Temm1eError> {
    let storage = SocialStorage::new(db_url).await?;
    Ok(SocialEngine {
        config: social_config.clone(),
        personality: personality.clone(),
        storage: std::sync::Arc::new(storage),
    })
}
```

### 2.2 `crates/temm1e-anima/src/types.rs`

All shared types for the social intelligence system.

```rust
use serde::{Deserialize, Serialize};

/// Config section added to Temm1eConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_turn_interval")]
    pub turn_interval: u32,              // Default: 10
    #[serde(default = "default_min_interval")]
    pub min_interval_seconds: u64,       // Default: 300
    #[serde(default = "default_max_buffer")]
    pub max_buffer_turns: u32,           // Default: 30
}

/// Raw observable facts — NO interpretation, just numbers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnFacts {
    pub turn_number: u32,
    pub timestamp: u64,
    pub user_message: MessageFacts,
    pub tem_response: MessageFacts,
    pub interaction: InteractionFacts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageFacts {
    pub char_count: u32,
    pub word_count: u32,
    pub sentence_count: u32,
    pub question_count: u32,
    pub exclamation_count: u32,
    pub emoji_count: u32,
    pub code_block_count: u32,
    pub uppercase_ratio: f32,
    pub punctuation_density: f32,
    pub avg_sentence_length: f32,
    pub language_detected: String,
    pub contains_greeting: bool,
    pub contains_thanks: bool,
    pub contains_apology: bool,
    pub contains_question: bool,
    pub contains_command: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionFacts {
    pub seconds_since_last_message: u64,
    pub session_turn_number: u32,
    pub topic_shifted: bool,
    pub task_completed: bool,
    pub task_failed: bool,
    pub tool_calls_count: u32,
}

/// TraitScore — universal scoring unit, values set by LLM evaluator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitScore {
    pub value: f32,
    pub confidence: f32,
    pub observations: u32,
    pub last_updated: u64,
    pub reasoning: String,
}

/// Full user profile — one per user, iteratively updated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: String,
    pub communication_style: CommunicationStyle,
    pub personality_traits: PersonalityTraits,
    pub emotional_state: EmotionalState,
    pub trust: TrustModel,
    pub relationship_phase: RelationshipPhase,
    pub evaluation_count: u32,
    pub total_turns_analyzed: u32,
    pub created_at: u64,
    pub last_evaluated_at: u64,
    pub last_message_at: u64,
    pub observations: Vec<String>,
    pub recommendations: Recommendations,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationStyle {
    pub directness: Option<TraitScore>,
    pub formality: Option<TraitScore>,
    pub analytical_vs_emotional: Option<TraitScore>,
    pub verbosity: Option<TraitScore>,
    pub pace: Option<TraitScore>,
    pub technical_depth: Option<TraitScore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityTraits {
    pub openness: Option<TraitScore>,
    pub conscientiousness: Option<TraitScore>,
    pub extraversion: Option<TraitScore>,
    pub agreeableness: Option<TraitScore>,
    pub neuroticism: Option<TraitScore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalState {
    pub current_mood: Option<String>,
    pub confidence: f32,
    pub reasoning: String,
    pub stress_level: f32,
    pub energy_level: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustModel {
    pub current_level: f32,
    pub confidence: f32,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RelationshipPhase {
    #[default]
    Discovery,
    Calibration,
    Partnership,
    DeepPartnership,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Recommendations {
    pub tone: String,
    pub adapt: String,
    pub avoid: String,
}

/// The evaluation output schema — what the LLM returns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationOutput {
    pub evaluation_id: String,
    pub turns_analyzed: Vec<u32>,
    pub communication_style: serde_json::Value,   // Partial update
    pub emotional_state: serde_json::Value,
    pub personality_traits: serde_json::Value,
    pub trust_assessment: serde_json::Value,
    pub relationship_phase: serde_json::Value,
    pub tem_self_update: serde_json::Value,
    pub observations: Vec<String>,
    pub recommendations: Recommendations,
}

/// The social engine — main entry point passed as Arc to runtime
pub struct SocialEngine {
    pub config: SocialConfig,
    pub personality: crate::personality::PersonalityConfig,
    pub storage: std::sync::Arc<crate::storage::SocialStorage>,
}
```

### 2.3 `crates/temm1e-anima/src/personality.rs`

Loads `personality.toml` + `soul.md`. Provides `PersonalityConfig` used everywhere.

Key struct:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityConfig {
    pub identity: IdentityConfig,
    pub facets: BigFiveFacets,
    pub values: ValuesConfig,
    pub communication: CommunicationDefaults,
    pub boundaries: BoundaryConfig,
    pub modes: ModeConfigs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    pub name: String,                   // "Tem"
    pub full_name: String,              // "TEMM1E"
    pub tagline: String,                // "with a one, not an i"
    pub soul_document: Option<String>,  // Path to soul.md
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfigs {
    pub play: ModeConfig,
    pub work: ModeConfig,
    pub pro: ModeConfig,
    pub none: ModeConfig,
    pub default: String,                // "play"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfig {
    pub description: String,
    pub emoticon: String,
    pub tone: String,
    pub classifier_voice: String,       // Injected into classifier
    pub runtime_voice: String,          // Injected into runtime prompt
    pub switch_message: String,         // Tool output on mode switch
}
```

Functions:
- `load_personality(config_dir: &Path) -> PersonalityConfig` — loads TOML + soul.md
- `PersonalityConfig::default()` — returns stock Tem personality
- `generate_identity_section(&self) -> String` — replaces hardcoded `section_identity()`
- `generate_classifier_mode(&self, mode: Temm1eMode) -> String` — replaces `CLASSIFY_MODE_*`
- `generate_runtime_mode_block(&self, mode: Temm1eMode) -> String` — replaces `mode_prompt_block()`
- `generate_switch_message(&self, mode: Temm1eMode) -> String` — replaces hardcoded switch messages

### 2.4 `crates/temm1e-anima/src/facts.rs`

Per-message fact collection. Pure code, no LLM, ~1ms.

```rust
/// Extract raw observable facts from a message. No interpretation.
pub fn collect_message_facts(text: &str) -> MessageFacts { ... }

/// Extract interaction-level facts for this turn.
pub fn collect_interaction_facts(
    seconds_since_last: u64,
    session_turn: u32,
    task_completed: bool,
    task_failed: bool,
    tool_calls: u32,
) -> InteractionFacts { ... }
```

### 2.5 `crates/temm1e-anima/src/evaluator.rs`

Builds evaluation prompt, parses LLM output.

```rust
/// Build the evaluation input for the LLM.
pub fn build_evaluation_prompt(
    current_profile: &UserProfile,
    facts_buffer: &[TurnFacts],
    recent_messages: &[(String, String)],  // (user, tem) pairs
) -> (String, String) { ... }  // Returns (system_prompt, user_prompt)

/// Parse the LLM's JSON response into EvaluationOutput.
pub fn parse_evaluation_output(json_str: &str) -> Result<EvaluationOutput, ...> { ... }

/// Merge evaluation deltas into existing profile.
pub fn apply_evaluation(profile: &mut UserProfile, eval: &EvaluationOutput) { ... }
```

### 2.6 `crates/temm1e-anima/src/user_model.rs`

Profile management logic.

```rust
/// Create a blank profile for a new user.
pub fn new_profile(user_id: &str) -> UserProfile { ... }

/// Check if evaluation should trigger (turn count, time interval).
pub fn should_evaluate(
    turn_count: u32,
    last_eval_time: u64,
    config: &SocialConfig,
) -> bool { ... }
```

### 2.7 `crates/temm1e-anima/src/communication.rs`

Generates system prompt sections from profile data.

```rust
/// Generate the user profile section for system prompt injection.
/// ~100-200 tokens. Only includes dimensions above confidence threshold.
/// Shapes COMMUNICATION only — never work quality.
pub fn section_user_profile(profile: &UserProfile) -> String { ... }

/// Generate a lightweight profile summary for classifier injection.
/// ~50-100 tokens.
pub fn classifier_profile_summary(profile: &UserProfile) -> String { ... }
```

### 2.8 `crates/temm1e-anima/src/storage.rs`

SQLite persistence. Follows `temm1e-memory/src/sqlite.rs` patterns.

```rust
pub struct SocialStorage {
    pool: sqlx::SqlitePool,
}

impl SocialStorage {
    pub async fn new(db_url: &str) -> Result<Self, Temm1eError> { ... }
    async fn init_tables(&self) -> Result<(), Temm1eError> { ... }

    // Profile CRUD
    pub async fn get_profile(&self, user_id: &str) -> Result<Option<UserProfile>, ...> { ... }
    pub async fn upsert_profile(&self, profile: &UserProfile) -> Result<(), ...> { ... }
    pub async fn delete_profile(&self, user_id: &str) -> Result<(), ...> { ... }

    // Facts buffer
    pub async fn buffer_facts(&self, user_id: &str, facts: &TurnFacts, message: &str) -> Result<(), ...> { ... }
    pub async fn get_buffered_facts(&self, user_id: &str) -> Result<Vec<(TurnFacts, String)>, ...> { ... }
    pub async fn clear_buffer(&self, user_id: &str) -> Result<(), ...> { ... }

    // Evaluation log
    pub async fn log_evaluation(&self, user_id: &str, eval: &EvaluationOutput, model: &str, tokens: u32) -> Result<(), ...> { ... }

    // Relational memory
    pub async fn add_observation(&self, user_id: &str, observation: &str, eval_id: &str) -> Result<(), ...> { ... }
    pub async fn get_observations(&self, user_id: &str, limit: usize) -> Result<Vec<String>, ...> { ... }
}
```

SQLite tables (CREATE TABLE IF NOT EXISTS):
```sql
CREATE TABLE IF NOT EXISTS social_user_profile (
    user_id TEXT PRIMARY KEY,
    profile_json TEXT NOT NULL,
    evaluation_count INTEGER DEFAULT 0,
    total_turns INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL,
    last_evaluated_at INTEGER,
    last_message_at INTEGER
);

CREATE TABLE IF NOT EXISTS social_evaluation_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    evaluation_json TEXT NOT NULL,
    turns_analyzed TEXT NOT NULL,
    model_used TEXT NOT NULL,
    tokens_used INTEGER,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS social_facts_buffer (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    turn_number INTEGER NOT NULL,
    facts_json TEXT NOT NULL,
    message_content TEXT,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS social_observations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    observation TEXT NOT NULL,
    source_eval_id TEXT,
    created_at INTEGER NOT NULL
);
```

### 2.9 `crates/temm1e-anima/src/ethics.rs`

User controls and confidence gating.

```rust
/// Confidence thresholds for profile injection
pub const CONFIDENCE_COSMETIC: f32 = 0.3;
pub const CONFIDENCE_TONAL: f32 = 0.5;
pub const CONFIDENCE_BEHAVIORAL: f32 = 0.7;
pub const CONFIDENCE_RELATIONAL: f32 = 0.8;
pub const CONFIDENCE_CONFRONTATIONAL: f32 = 0.9;

/// Filter profile dimensions below required confidence for the action type.
pub fn confidence_gate(profile: &UserProfile, action_type: ActionType) -> FilteredProfile { ... }
```

---

## 3. Files to Modify

### 3.1 `crates/temm1e-core/src/types/config.rs`

**Add `SocialConfig` to `Temm1eConfig` (line ~75):**
```rust
pub social: SocialConfig,
```

**Add Default impl** for SocialConfig (serde defaults handle it).

**Temm1eMode Display impl (lines 22-31):** Leave as-is for now. Low priority — only affects logging.

### 3.2 `src/main.rs`

**After config load (line ~1380), before agent creation (line ~1925):**

```rust
// Load personality (~line 1455, after mode parsing)
let personality = temm1e_social::personality::load_personality(
    &dirs::home_dir().unwrap_or_default().join(".temm1e")
);

// Initialize social engine (~line 1600, after memory init)
let social_db_url = format!(
    "sqlite:{}/social.db?mode=rwc",
    dirs::home_dir().unwrap_or_default().join(".temm1e").display()
);
let social_engine = if config.social.enabled {
    Some(Arc::new(temm1e_social::initialize(&config.social, &personality, &social_db_url).await?))
} else {
    None
};

// Pass personality to agent runtime builder (~line 1925)
let mut runtime = temm1e_agent::AgentRuntime::with_limits(...)
    .with_personality(Arc::new(personality))      // NEW
    .with_social_engine(social_engine.clone());   // NEW
```

### 3.3 `crates/temm1e-agent/src/runtime.rs`

**Add fields to AgentRuntime struct (~line 91):**
```rust
personality: Option<Arc<PersonalityConfig>>,
social_engine: Option<Arc<SocialEngine>>,
social_turn_count: std::sync::atomic::AtomicU32,
```

**Add builder methods (~line 250):**
```rust
pub fn with_personality(mut self, p: Arc<PersonalityConfig>) -> Self {
    self.personality = Some(p);
    self
}
pub fn with_social_engine(mut self, e: Option<Arc<SocialEngine>>) -> Self {
    self.social_engine = e;
    self
}
```

**Hook 1: Facts collection (after line 464, before classification):**
```rust
// Collect raw facts for social intelligence
if let Some(engine) = &self.social_engine {
    let facts = temm1e_social::facts::collect_message_facts(&user_text);
    let interaction = temm1e_social::facts::collect_interaction_facts(...);
    let turn_facts = TurnFacts { turn_number, timestamp: now, user_message: facts, ... };
    let _ = engine.storage.buffer_facts(&msg.user_id, &turn_facts, &user_text).await;
    self.social_turn_count.fetch_add(1, Ordering::Relaxed);
}
```

**Hook 2: Profile injection into classifier (line ~491):**
```rust
// Pass personality + profile to classify_message
let profile_summary = if let Some(engine) = &self.social_engine {
    if let Ok(Some(profile)) = engine.storage.get_profile(&msg.user_id).await {
        Some(temm1e_social::communication::classifier_profile_summary(&profile))
    } else { None }
} else { None };

let (classification, classify_usage) = classify_message(
    ..., current_mode, profile_summary.as_deref()  // NEW param
).await?;
```

**Hook 3: Profile injection into system prompt (line ~789):**
```rust
// Add user profile section to system prompt
if let Some(engine) = &self.social_engine {
    if let Ok(Some(profile)) = engine.storage.get_profile(&msg.user_id).await {
        let profile_section = temm1e_social::communication::section_user_profile(&profile);
        request.system = Some(format!("{}\n\n{}", existing_system, profile_section));
    }
}
```

**Hook 4: Replace mode_prompt_block (lines 795-803):**
```rust
// Use personality-driven mode block instead of hardcoded
let mode_block = if let Some(p) = &self.personality {
    p.generate_runtime_mode_block(mode)
} else {
    mode_prompt_block(mode)  // Fallback to existing hardcode
};
```

**Hook 5: Background evaluation trigger (after line ~1343):**
```rust
// Trigger background evaluation if turn threshold reached
if let Some(engine) = &self.social_engine {
    let turn_count = self.social_turn_count.load(Ordering::Relaxed);
    if temm1e_social::user_model::should_evaluate(turn_count, ..., &engine.config) {
        let engine = engine.clone();
        let provider = self.provider.clone();
        let model = self.model.clone();
        let user_id = msg.user_id.clone();
        tokio::spawn(async move {
            // Background: read profile, read buffer, call LLM, merge, write back
            if let Err(e) = run_evaluation(&engine, &provider, &model, &user_id).await {
                tracing::warn!(error = %e, "Social evaluation failed");
            }
        });
        self.social_turn_count.store(0, Ordering::Relaxed);
    }
}
```

### 3.4 `crates/temm1e-agent/src/prompt_optimizer.rs`

**Modify `section_identity()` (lines 221-254):**
```rust
fn section_identity(&self) -> PromptSection {
    // Use personality config if available, otherwise fall back to hardcoded
    if let Some(personality) = &self.personality {
        PromptSection {
            name: "identity",
            text: personality.generate_identity_section(),
        }
    } else {
        PromptSection {
            name: "identity",
            text: /* existing hardcoded text */,
        }
    }
}
```

**Add `personality` field to SystemPromptBuilder struct (line 63):**
```rust
personality: Option<&'a PersonalityConfig>,
```

**Add builder method:**
```rust
pub fn personality(mut self, p: &'a PersonalityConfig) -> Self {
    self.personality = Some(p);
    self
}
```

### 3.5 `crates/temm1e-agent/src/llm_classifier.rs`

**Modify `classify_message()` signature (line 187):**
Add `profile_summary: Option<&str>` parameter.

**Modify `build_classify_prompt()` (line 149):**
```rust
fn build_classify_prompt(
    categories: &[String],
    mode: Temm1eMode,
    personality: Option<&PersonalityConfig>,  // NEW
    profile_summary: Option<&str>,            // NEW
) -> String {
    let mut prompt = CLASSIFY_BASE_PROMPT.to_string();

    // Mode injection — personality-driven or fallback
    if let Some(p) = personality {
        prompt.push_str(&p.generate_classifier_mode(mode));
    } else {
        prompt.push_str(match mode {
            Temm1eMode::Play => CLASSIFY_MODE_PLAY,
            // ... existing fallback
        });
    }

    // User profile injection — NEW
    if let Some(summary) = profile_summary {
        prompt.push_str(&format!("\n\nUSER CONTEXT:\n{}", summary));
    }

    // Blueprint categories (existing, unchanged)
    ...
}
```

### 3.6 `crates/temm1e-tools/src/mode_switch.rs`

**Add `personality` field to ModeSwitchTool (line 18):**
```rust
pub struct ModeSwitchTool {
    mode: SharedMode,
    personality: Option<Arc<PersonalityConfig>>,  // NEW
}
```

**Modify confirmation messages (lines 94-101):**
```rust
let message = if let Some(p) = &self.personality {
    p.generate_switch_message(new_mode)
} else {
    match new_mode {
        Temm1eMode::Play => "Mode switched to PLAY! Let's have some fun! :3".to_string(),
        // ... existing fallback
    }
};
```

### 3.7 `crates/temm1e-agent/Cargo.toml`

Add dependency:
```toml
temm1e-anima.workspace = true
```

### 3.8 `Cargo.toml` (root binary)

Add dependency:
```toml
temm1e-anima.workspace = true
```

---

## 4. Personality Centralization

### All Hardcoded Sites and Their Replacement

| # | File:Lines | Current | Replacement |
|---|-----------|---------|-------------|
| 1 | `prompt_optimizer.rs:221-254` | Hardcoded `section_identity()` | `personality.generate_identity_section()` |
| 2 | `llm_classifier.rs:116-121` | `CLASSIFY_MODE_PLAY` const | `personality.generate_classifier_mode(Play)` |
| 3 | `llm_classifier.rs:123-128` | `CLASSIFY_MODE_WORK` const | `personality.generate_classifier_mode(Work)` |
| 4 | `llm_classifier.rs:130-135` | `CLASSIFY_MODE_PRO` const | `personality.generate_classifier_mode(Pro)` |
| 5 | `llm_classifier.rs:137-141` | `CLASSIFY_MODE_NONE` const | `personality.generate_classifier_mode(None)` |
| 6 | `runtime.rs:1937-1951` | `mode_prompt_block()` PLAY | `personality.generate_runtime_mode_block(Play)` |
| 7 | `runtime.rs:1952-1966` | `mode_prompt_block()` WORK | `personality.generate_runtime_mode_block(Work)` |
| 8 | `runtime.rs:1967-1982` | `mode_prompt_block()` PRO | `personality.generate_runtime_mode_block(Pro)` |
| 9 | `runtime.rs:1983-1985` | `mode_prompt_block()` NONE | `personality.generate_runtime_mode_block(None)` |
| 10 | `mode_switch.rs:94-101` | Switch messages | `personality.generate_switch_message(mode)` |
| 11 | `config.rs:22-31` | `Display for Temm1eMode` | Low priority — only affects logs |
| 12 | `onboarding/steps.rs:29-56` | TUI mode descriptions | Read from personality config |
| 13 | `heartbeat.rs:326-342` | Heartbeat template | Low priority — autonomous mode text |

**All replacements fall back to existing hardcoded text if `personality` is None.** Zero breaking change.

---

## 5. Execution Order

### Phase 1: Crate skeleton + personality centralization (COMPLETE)

1. Create `crates/temm1e-anima/` with Cargo.toml
2. Create `types.rs` — all shared types
3. Create `personality.rs` — PersonalityConfig, loader, stock Tem defaults, all generate_* methods
4. Create `storage.rs` — SQLite tables, CRUD
5. Create `lib.rs` — public API
6. Register in workspace Cargo.toml
7. Modify `prompt_optimizer.rs` — personality-driven `section_identity()`
8. Modify `llm_classifier.rs` — personality-driven mode injection + profile summary param
9. Modify `runtime.rs` — personality-driven `mode_prompt_block()` + builder methods
10. Modify `mode_switch.rs` — personality-driven switch messages
11. Modify `main.rs` — load personality, pass to runtime
12. **Gate: `cargo check --workspace` + `cargo clippy` + `cargo test`** PASSED

### Phase 2: Facts collection + evaluation + injection (COMPLETE)

13. Create `facts.rs` — per-message fact extraction
14. Create `evaluator.rs` — LLM evaluation prompt + output parsing + improved merge with adaptive N
15. Create `user_model.rs` — profile management, should_evaluate() with adaptive turn interval
16. Create `communication.rs` — section_user_profile(), classifier_profile_summary()
17. Create `ethics.rs` — confidence gating at 5 tiers
18. Modify `runtime.rs` — wire facts collection (hook 1), profile injection (hooks 2-3), background evaluation (hook 5)
19. Modify `main.rs` — initialize social engine, pass to runtime
20. Modify `temm1e-core/config.rs` — add SocialConfig to Temm1eConfig
21. **Gate: `cargo check --workspace` + `cargo clippy` + `cargo test`** PASSED

### Phase 3: Stock personality files + setup flow (COMPLETE)

22. Create stock `personality.toml` and `TEM.md` (soul document) as embedded defaults
23. Add first-launch generation: if `~/.temm1e/personality.toml` missing, write stock
24. Add `/profile` commands (show, off, reset, export, delete)
25. **Gate: full compilation + 10-turn CLI test** PASSED

---

## Key Patterns to Follow

| Pattern | Source | Apply To |
|---------|--------|----------|
| SQLite init | `temm1e-memory/src/sqlite.rs:26-37` | `storage.rs` — same pool options, same CREATE IF NOT EXISTS |
| Config loading | `temm1e-core/src/config/loader.rs:25-63` | `personality.rs` — same TOML parse, same env expansion |
| Shared state | `runtime.rs:44` (`SharedMode = Arc<RwLock<>>`) | `Arc<SocialEngine>` passed to runtime |
| Builder pattern | `runtime.rs:209-278` (`.with_*()` methods) | `.with_personality()`, `.with_social_engine()` |
| Background task | `runtime.rs:1212-1343` (blueprint authoring spawn) | Background evaluation spawn |
| Tool with shared state | `mode_switch.rs:18-26` | ModeSwitchTool gets `Arc<PersonalityConfig>` |
| Fallback | N/A | Every personality-driven path has `else { existing_hardcode }` |

---

*This spec was the implementation blueprint for temm1e-anima. All phases complete and shipped in v4.3.0.*
