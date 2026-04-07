# Self-Learning v4.6.0 — Core Stats, Tool Reliability, Classification Feedback

> Three closed-loop self-learning mechanisms that capture residual data already flowing
> through the system and feed it back into future decisions.

---

## Design Principle

All three mechanisms follow the same pattern:

```
Execution → Residual data (already computed, currently discarded)
  → Persist to DB
  → Aggregate into compact summary
  → Inject into LLM context as empirical priors
  → LLM makes better-informed decisions
  → Better execution → Better residuals → ...
```

**Cold start:** With zero data, all three behave identically to v4.5.1. They can only improve.

**No hardcoded decisions.** Data is presented to the LLM as facts. The LLM decides what to
do with it. Smarter model = smarter use of the same data.

---

## 1. Core Stats Activation

### Current State

`CoreStats` in `crates/temm1e-cores/src/types.rs:22-51`:
```rust
pub struct CoreStats {
    pub invocations: u32,
    pub successes: u32,
    pub failures: u32,
    pub avg_rounds: f32,
    pub total_cost_usd: f64,
}
```

Methods `record_success(rounds, cost)` and `record_failure(rounds, cost)` exist, tested, exported.
**Never called in production.**

### Implementation

#### Storage

One `MemoryEntry` per core:
- `id`: `"core_stats:{core_name}"`
- `content`: JSON of CoreStats
- `entry_type`: `MemoryEntryType::Knowledge`
- `metadata`: `{"type": "core_stats", "core_name": "{name}"}`

#### Wiring (invoke_tool.rs)

After `CoreRuntime::run()` returns:

```rust
// Load existing stats (or default)
let stats_id = format!("core_stats:{}", core_name);
let mut stats: CoreStats = memory.search(&stats_id, SearchOpts { limit: 1, .. })
    .await.ok()
    .and_then(|entries| entries.first().and_then(|e| serde_json::from_str(&e.content).ok()))
    .unwrap_or_default();

// Record outcome
match &result {
    Ok(r) => stats.record_success(r.rounds as u32, r.cost_usd),
    Err(_) => stats.record_failure(0, 0.0),
}

// Persist
let entry = MemoryEntry {
    id: stats_id,
    content: serde_json::to_string(&stats).unwrap_or_default(),
    entry_type: MemoryEntryType::Knowledge,
    metadata: json!({"type": "core_stats", "core_name": core_name}),
    timestamp: Utc::now(),
    session_id: None,
};
let _ = memory.store(entry).await;
```

#### Context Injection (invoke_tool.rs)

Before core selection, format available cores with stats:

```rust
fn format_core_stats_context(cores: &[(String, CoreStats)]) -> String {
    // "Available cores (recent performance):
    //   architecture — 87% success, avg 4 rounds, $0.03/call (N=23)
    //   debug — 62% success, avg 9 rounds, $0.08/call (N=8)
    //   code-review — 94% success, avg 3 rounds, $0.02/call (N=31)"
}
```

#### Files Changed

| File | Change |
|------|--------|
| `crates/temm1e-cores/src/invoke_tool.rs` | Load stats before run, record after, inject into output |
| `crates/temm1e-cores/src/types.rs` | Add `Default` impl, add `success_rate()` method, add `Serialize`/`Deserialize` |
| `crates/temm1e-cores/src/invoke_tool.rs` | Add `memory: Arc<dyn Memory>` parameter to InvokeCoreTool |

#### Problem: InvokeCoreTool Doesn't Have Memory Access

Looking at `invoke_tool.rs`, the `InvokeCoreTool` struct holds:
- `core_registry: Arc<RwLock<CoreRegistry>>`
- `provider: Arc<dyn Provider>`
- `tools: Vec<Arc<dyn Tool>>`
- `budget: BudgetTracker`
- `model_pricing: ...`
- `model: String`
- `max_context_tokens: usize`

It does NOT have `memory: Arc<dyn Memory>`. Need to add it.

In `src/main.rs` where `InvokeCoreTool::new()` is called (~line 2024-2043), `memory` is
already in scope as `Arc<dyn Memory>`. Just pass it through.

#### New Constructor

```rust
pub struct InvokeCoreTool {
    // ... existing fields ...
    memory: Arc<dyn Memory>,  // NEW
}

impl InvokeCoreTool {
    pub fn new(
        // ... existing params ...
        memory: Arc<dyn Memory>,  // NEW
    ) -> Self { ... }
}
```

---

## 2. Tool Reliability Per Task Type

### Current State

`FailureTracker` in `crates/temm1e-agent/src/self_correction.rs:10-16`:
- Tracks consecutive failures per tool name per session
- Created fresh every message, destroyed after
- Zero cross-session persistence

Tool execution results (`ToolOutput.is_error`) are available in the runtime loop but
never aggregated.

### Implementation

#### Schema

```sql
CREATE TABLE IF NOT EXISTS tool_reliability (
    tool_name TEXT NOT NULL,
    task_type TEXT NOT NULL,
    successes INTEGER NOT NULL DEFAULT 0,
    failures INTEGER NOT NULL DEFAULT 0,
    last_updated INTEGER NOT NULL,
    PRIMARY KEY (tool_name, task_type)
);
```

Add to `SqliteMemory::new()` init alongside other table creations.

#### New Memory Trait Methods

```rust
// In crates/temm1e-core/src/traits/memory.rs:
async fn record_tool_outcome(
    &self,
    _tool_name: &str,
    _task_type: &str,
    _success: bool,
) -> Result<(), Temm1eError> { Ok(()) }

async fn get_tool_reliability(&self) -> Result<Vec<ToolReliabilityRecord>, Temm1eError> {
    Ok(Vec::new())
}
```

Default no-ops — MarkdownMemory/FailoverMemory unaffected.

#### ToolReliabilityRecord Type

```rust
// In crates/temm1e-core/src/traits/memory.rs:
pub struct ToolReliabilityRecord {
    pub tool_name: String,
    pub task_type: String,
    pub successes: u32,
    pub failures: u32,
    pub last_updated: u64,
}

impl ToolReliabilityRecord {
    pub fn success_rate(&self) -> f64 {
        let total = self.successes + self.failures;
        if total == 0 { return 0.5; }
        self.successes as f64 / total as f64
    }
}
```

#### SQLite Implementation

```rust
// In crates/temm1e-memory/src/sqlite.rs:
async fn record_tool_outcome(&self, tool_name: &str, task_type: &str, success: bool) -> Result<(), Temm1eError> {
    let now = epoch_now();
    if success {
        sqlx::query(
            "INSERT INTO tool_reliability (tool_name, task_type, successes, failures, last_updated) \
             VALUES (?1, ?2, 1, 0, ?3) \
             ON CONFLICT(tool_name, task_type) DO UPDATE SET \
             successes = successes + 1, last_updated = ?3"
        ).bind(tool_name).bind(task_type).bind(now)
        .execute(&self.pool).await?;
    } else {
        sqlx::query(
            "INSERT INTO tool_reliability (tool_name, task_type, successes, failures, last_updated) \
             VALUES (?1, ?2, 0, 1, ?3) \
             ON CONFLICT(tool_name, task_type) DO UPDATE SET \
             failures = failures + 1, last_updated = ?3"
        ).bind(tool_name).bind(task_type).bind(now)
        .execute(&self.pool).await?;
    }
    Ok(())
}

async fn get_tool_reliability(&self) -> Result<Vec<ToolReliabilityRecord>, Temm1eError> {
    let cutoff = epoch_now() - 30 * 86400; // last 30 days
    let rows = sqlx::query_as::<_, ToolReliabilityRow>(
        "SELECT tool_name, task_type, successes, failures, last_updated \
         FROM tool_reliability WHERE last_updated > ?1 \
         ORDER BY (successes + failures) DESC LIMIT 50"
    ).bind(cutoff).fetch_all(&self.pool).await?;
    Ok(rows.into_iter().map(row_to_reliability).collect())
}
```

#### Recording in Runtime (runtime.rs)

After each tool execution in the tool-use loop, the runtime has:
- `tool_name` from `ContentPart::ToolUse { name, .. }`
- `is_error` from `ToolOutput.is_error`
- `task_type` from classifier output (already in scope as `classification.category`)

Add after tool result processing:

```rust
// After tool result is received:
let task_type_label = format!("{}:{}", classification.category, classification.difficulty);
let _ = self.memory.record_tool_outcome(&tool_name, &task_type_label, !output.is_error).await;
```

Fire-and-forget (`let _`) — never blocks the response path.

#### Context Injection (context.rs)

In `build_context()`, after learnings injection:

```rust
// Tool reliability (if available)
if let Ok(records) = memory.get_tool_reliability().await {
    if !records.is_empty() {
        let reliability_text = format_tool_reliability(&records);
        let tokens = estimate_tokens(&reliability_text);
        if tokens <= 100 { // hard cap: never more than 100 tokens
            lambda_messages.push(ChatMessage {
                role: Role::System,
                content: MessageContent::Text(reliability_text),
            });
        }
    }
}
```

Format function:
```rust
fn format_tool_reliability(records: &[ToolReliabilityRecord]) -> String {
    let mut lines = vec!["Tool reliability (last 30 days):".to_string()];
    for r in records.iter().take(10) {
        let total = r.successes + r.failures;
        if total < 3 { continue; } // skip low-sample records
        lines.push(format!(
            "  {}: {} {:.0}% (N={})",
            r.tool_name, r.task_type,
            r.success_rate() * 100.0, total
        ));
    }
    lines.join("\n")
}
```

#### Files Changed

| File | Change |
|------|--------|
| `crates/temm1e-core/src/traits/memory.rs` | Add `ToolReliabilityRecord`, `record_tool_outcome()`, `get_tool_reliability()` |
| `crates/temm1e-memory/src/sqlite.rs` | CREATE TABLE, implement both methods |
| `crates/temm1e-agent/src/runtime.rs` | Record tool outcome after each tool execution |
| `crates/temm1e-agent/src/context.rs` | Inject reliability summary into context |

---

## 3. Classification Accuracy Feedback

### Current State

`llm_classifier.rs` predicts (category, difficulty, blueprint_hint) for every message.
The runtime then uses actual resources (rounds, tools, cost) to process it.
**No comparison ever happens.**

### Implementation

#### Schema

```sql
CREATE TABLE IF NOT EXISTS classification_outcomes (
    category TEXT NOT NULL,
    difficulty TEXT NOT NULL,
    rounds INTEGER NOT NULL,
    tools_used INTEGER NOT NULL,
    cost_usd REAL NOT NULL,
    success INTEGER NOT NULL,
    timestamp INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_co_timestamp ON classification_outcomes(timestamp);
```

#### New Memory Trait Methods

```rust
async fn record_classification_outcome(
    &self,
    _category: &str,
    _difficulty: &str,
    _rounds: u32,
    _tools_used: u32,
    _cost_usd: f64,
    _success: bool,
) -> Result<(), Temm1eError> { Ok(()) }

async fn get_classification_priors(&self) -> Result<Vec<ClassificationPrior>, Temm1eError> {
    Ok(Vec::new())
}
```

#### ClassificationPrior Type

```rust
pub struct ClassificationPrior {
    pub category: String,
    pub difficulty: String,
    pub avg_rounds: f32,
    pub avg_tools: f32,
    pub avg_cost: f64,
    pub count: u32,
}
```

#### SQLite Implementation

```rust
async fn record_classification_outcome(
    &self, category: &str, difficulty: &str,
    rounds: u32, tools_used: u32, cost_usd: f64, success: bool,
) -> Result<(), Temm1eError> {
    let now = epoch_now();
    sqlx::query(
        "INSERT INTO classification_outcomes \
         (category, difficulty, rounds, tools_used, cost_usd, success, timestamp) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
    )
    .bind(category).bind(difficulty).bind(rounds as i32)
    .bind(tools_used as i32).bind(cost_usd).bind(success as i32).bind(now)
    .execute(&self.pool).await?;

    // Retention: keep last 500 records
    sqlx::query(
        "DELETE FROM classification_outcomes WHERE rowid NOT IN \
         (SELECT rowid FROM classification_outcomes ORDER BY timestamp DESC LIMIT 500)"
    ).execute(&self.pool).await.ok();

    Ok(())
}

async fn get_classification_priors(&self) -> Result<Vec<ClassificationPrior>, Temm1eError> {
    let cutoff = epoch_now() - 30 * 86400;
    let rows = sqlx::query(
        "SELECT category, difficulty, \
         AVG(rounds) as avg_rounds, AVG(tools_used) as avg_tools, \
         AVG(cost_usd) as avg_cost, COUNT(*) as cnt \
         FROM classification_outcomes WHERE timestamp > ?1 \
         GROUP BY category, difficulty \
         ORDER BY cnt DESC"
    ).bind(cutoff).fetch_all(&self.pool).await?;
    // ... map to ClassificationPrior
}
```

#### Recording in Runtime (runtime.rs)

At task completion (where learnings are extracted), add:

```rust
// After task finishes:
let _ = self.memory.record_classification_outcome(
    &classification_label,
    &difficulty_label,
    rounds as u32,
    tools_used_count as u32,
    turn_cost_usd,
    !interrupted, // success = completed without interruption
).await;
```

Fire-and-forget — never blocks response.

#### Context Injection (llm_classifier.rs or context.rs)

Inject empirical priors into the classifier's prompt:

```rust
// Before classifier call:
if let Ok(priors) = memory.get_classification_priors().await {
    if !priors.is_empty() {
        let priors_text = format_classification_priors(&priors);
        // Append to classifier system prompt
    }
}
```

Format:
```
Historical task profiles (last 30 days):
  Chat: avg 1.1 rounds, 0 tools, $0.002 (N=340)
  Order/Simple: avg 2.4 rounds, 1.2 tools, $0.03 (N=89)
  Order/Standard: avg 5.8 rounds, 3.4 tools, $0.09 (N=45)
  Order/Complex: avg 11.2 rounds, 6.1 tools, $0.18 (N=12)
```

~40-60 tokens. The classifier sees what each category actually costs and can calibrate.

#### Files Changed

| File | Change |
|------|--------|
| `crates/temm1e-core/src/traits/memory.rs` | Add `ClassificationPrior`, `record_classification_outcome()`, `get_classification_priors()` |
| `crates/temm1e-memory/src/sqlite.rs` | CREATE TABLE, implement both methods |
| `crates/temm1e-agent/src/runtime.rs` | Record outcome at task completion |
| `crates/temm1e-agent/src/llm_classifier.rs` | Load priors, inject into classifier prompt |

---

## Drain Mechanisms

All three produce artifacts that grow. All three need drains.

| Mechanism | Artifact | Growth Rate | Drain |
|-----------|----------|-------------|-------|
| Core stats | 1 record per core | Bounded (N cores) | No drain needed — fixed-size records, updated in place |
| Tool reliability | 1 row per (tool, task_type) | Bounded (N tools x M types) | `last_updated` filter — only show last 30 days. Old rows naturally ignored. |
| Classification outcomes | 1 row per task | Linear growth | Retention cap: 500 rows. DELETE oldest beyond 500. |

Core stats and tool reliability are **bounded by construction** — they update in place
(UPSERT), not append. Classification outcomes append and need the retention cap.

---

## Test Plan

### Core Stats
1. Unit: `CoreStats::record_success()` updates avg_rounds correctly
2. Unit: `CoreStats::record_failure()` increments failure count
3. Unit: `success_rate()` returns correct ratio
4. Integration: invoke core, check stats persisted, invoke again, check stats updated
5. Cold start: invoke core with no prior stats — defaults to zero, works normally

### Tool Reliability
1. Unit: `record_tool_outcome(success=true)` increments successes
2. Unit: `record_tool_outcome(success=false)` increments failures
3. Unit: `success_rate()` correct with mixed outcomes
4. Unit: `get_tool_reliability()` filters by 30-day cutoff
5. Unit: `format_tool_reliability()` skips low-N records
6. Integration: execute tool, check row created, execute again, check updated

### Classification Outcomes
1. Unit: `record_classification_outcome()` inserts row
2. Unit: retention cap at 500 rows
3. Unit: `get_classification_priors()` aggregates correctly
4. Unit: `format_classification_priors()` produces compact output
5. Integration: process message, check outcome recorded with correct labels
