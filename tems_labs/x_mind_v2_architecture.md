# X-Mind v2: Consciousness-Orchestrated Subagent Architecture

**Tem's Labs — Architecture Design TL-2026-002**
**Date:** 2026-04-02
**Status:** Design — approved for implementation

---

## 1. Core Principles

1. **Consciousness is the boss.** It runs every tool loop round. It decides what to think, what artifacts to inject, and what minds to invoke.
2. **X-Minds are subagents.** They're goal-directed agents with read-only tool access, invoked on demand by consciousness, not on a fixed schedule.
3. **Artifacts are the currency.** Minds produce artifacts. Consciousness skims them, selects them, injects them. The worker receives them. Nobody rewrites them.
4. **Synchronous execution.** Each round waits for consciousness and all invoked minds to complete before the worker LLM fires. No stale state, no race conditions.
5. **Artifacts must be manageable.** Pruning, scoping, and lifecycle rules prevent bloat.

---

## 2. The Artifact System

### 2.1 Three-Tier Structure

Every artifact has three tiers of detail, following the Claude Code plugin pattern:

```
┌─────────────────────────────────────────────────────────┐
│ TIER 1: Title (5-10 words)                              │
│ "KV Store Transaction Architecture"                     │
├─────────────────────────────────────────────────────────┤
│ TIER 2: Description (1-3 sentences, ~50 tokens)         │
│ "Stack-based nested transaction layer using list of     │
│  dicts as change-sets. Tombstone pattern for rollback-  │
│  able deletes. Commit merges top into parent."          │
├─────────────────────────────────────────────────────────┤
│ TIER 3: Full Content (unbounded)                        │
│ "## Module Structure                                    │
│  TransactionalKVStore                                   │
│    ├── global_data: Dict[str, str]                      │
│    ├── txn_stack: List[Dict[str, str | Tombstone]]      │
│    ├── set(key, value) → None                           │
│    ├── get(key) → str | None                            │
│    │   └── walks stack top-down, falls back to global   │
│    ├── delete(key) → bool                               │
│    │   └── inserts Tombstone into current txn layer     │
│    ├── count(value) → int                               │
│    │   └── merges all layers, counts non-tombstone      │
│    ├── begin() → None                                   │
│    │   └── pushes empty dict onto txn_stack              │
│    ├── commit() → bool                                  │
│    │   └── pops top, merges into parent (or global)     │
│    └── rollback() → bool                                │
│        └── pops top, discards                           │
│                                                         │
│  Design Decisions:                                      │
│  - Tombstone sentinel vs deletion: chose tombstone      │
│    because delete must be rollback-able...              │
│  - Stack vs linked list: stack (Vec) for O(1) push/pop  │
│  ..."                                                   │
└─────────────────────────────────────────────────────────┘
```

### 2.2 What Each Layer Sees

| Consumer | What it reads | Why |
|----------|--------------|-----|
| **Consciousness** (every round) | Tier 1 + Tier 2 only | Skim to decide relevance. ~50 tokens per artifact. 20 artifacts = 1000 tokens. Manageable. |
| **Consciousness** (on demand) | Tier 3 via `read_artifact(id)` | When it needs to deeply understand before making a decision. Rare. |
| **Worker** (when injected) | Tier 3 in full | The worker needs the complete analysis to use it. Loaded from disk, not generated. |

### 2.3 File Format

Each artifact is a single markdown file with YAML frontmatter:

```markdown
---
id: arch-kvstore-001
mind: architect
title: "KV Store Transaction Architecture"
description: "Stack-based nested transaction layer using list of dicts as change-sets. Tombstone pattern for rollback-able deletes. Commit merges top into parent."
tags: [python, data-structure, transactions, kvstore]
scope: task
created_at: 2026-04-02T10:00:00Z
updated_at: 2026-04-02T10:15:00Z
session_id: cli-cli
turn_created: 1
turn_updated: 5
access_count: 8
last_accessed: 2026-04-02T10:20:00Z
token_estimate: 350
status: active
---

## Module Structure

TransactionalKVStore
  ├── global_data: Dict[str, str]
  ├── txn_stack: List[Dict[str, str | Tombstone]]
  ...

## Design Decisions
...
```

### 2.4 Manifest

The manifest is a **derived index** — auto-generated from artifact frontmatter.
Consciousness receives it as part of its input. It contains ONLY Tier 1 + Tier 2.

```
Available artifacts (12 active, 3 archived):

  [arch-kvstore-001] architect | task-scoped | active
  "KV Store Transaction Architecture"
  Stack-based nested transaction layer. Tombstone deletes. Commit to parent.
  Tags: python, data-structure, transactions | 350 tokens | accessed 2 min ago

  [analyst-txn-edge-001] analyst | task-scoped | active
  "Transaction Edge Cases"
  8 edge cases: rollback→bool, commit-to-parent, delete-of-missing, count-merged-view...
  Tags: python, edge-cases, transactions | 280 tokens | accessed 5 min ago

  [sentinel-auth-audit-001] sentinel | project-scoped | stale
  "Auth Handler Security Audit"
  SQL injection in login(), plaintext passwords, forgeable tokens, user enumeration.
  Tags: security, python, web, authentication | 520 tokens | accessed 3 days ago
```

Consciousness reads this (~150 tokens for 3 artifacts) and decides:
- "KV store task → inject arch-kvstore-001 and analyst-txn-edge-001"
- "Auth audit is from a different task and stale → skip"

---

## 3. Artifact Lifecycle & Pruning

### 3.1 Scopes

Every artifact has a **scope** that determines its retention:

| Scope | Meaning | Retention | Example |
|-------|---------|-----------|---------|
| `turn` | Relevant only to this tool loop | Auto-deleted when tool loop ends | "Current test failure analysis" |
| `task` | Relevant to the current task | Kept until consciousness marks it stale or a new task begins | "KV store architecture" |
| `session` | Relevant to this conversation | Kept until session ends | "User's coding preferences" |
| `project` | Relevant across sessions | Persists on disk indefinitely | "Codebase architecture map" |

Scope is assigned by the X-Mind when creating the artifact (based on its system
prompt guidance), and can be upgraded/downgraded by consciousness.

### 3.2 Pruning Rules

```
PRUNING PIPELINE (runs at the start of each user message):

1. DELETE turn-scoped artifacts from previous turn
   → They served their purpose. Gone.

2. CHECK task-scoped artifacts:
   → If consciousness detects a task change (new topic):
     - Mark all task-scoped artifacts as "stale"
     - Stale artifacts stay in manifest but deprioritized
   → If task is the same: keep as active

3. ARCHIVE stale artifacts not accessed in 5 sessions:
   → Move to ~/.temm1e/x_minds/archive/
   → Remove from active manifest
   → Still recoverable if consciousness explicitly requests

4. ENFORCE hard cap: max 30 active artifacts
   → If over cap, archive lowest-access-count artifacts first
   → Consciousness is warned: "Artifact limit reached, archiving {names}"

5. ENFORCE per-mind cap: max 10 active artifacts per mind
   → Prevents one mind from dominating the manifest
   → Oldest artifacts for that mind archived first
```

### 3.3 How Consciousness Manages Artifacts

Consciousness has explicit artifact management actions:

```json
{
  "thoughts": "...",
  "inject_artifacts": ["arch-kvstore-001"],
  "invoke_minds": [],
  "artifact_actions": [
    {"action": "archive", "id": "arch-web-scraper-002", "reason": "Different task"},
    {"action": "refresh", "id": "analyst-txn-edge-001", "reason": "Code changed, edge cases may differ"},
    {"action": "promote", "id": "sentinel-auth-audit-001", "scope": "project", "reason": "Security findings persist"}
  ]
}
```

Actions:
- **archive** — remove from active manifest, move to archive
- **delete** — permanently remove (for garbage artifacts)
- **refresh** — invoke the original mind to update the artifact with current context
- **promote** — change scope (e.g., task → project)
- **demote** — change scope (e.g., project → task)

### 3.4 Cold Start

First message in a session with no active artifacts:

1. Consciousness reads empty manifest: "No active artifacts."
2. Consciousness reasons: "User is asking me to implement a KV store. I should invoke the Architect to analyze the task structure."
3. Consciousness invokes Architect with goal: "Design the module structure for a transactional KV store with nested transactions."
4. Architect runs its loop (reads spec, reasons, produces artifact).
5. Artifact saved. Manifest updated.
6. **Same round continues** (synchronous): consciousness now sees the new artifact and injects it.
7. Worker receives the artifact and starts coding.

### 3.5 Returning to a Previous Task

User worked on KV store yesterday (artifacts exist on disk). Today they return:

1. Consciousness reads manifest: sees `arch-kvstore-001` (project-scoped, last accessed yesterday).
2. User says "let's continue working on the KV store."
3. Consciousness: "Existing architecture artifact is relevant but may be stale. Let me check." → reads full content via `read_artifact`.
4. Consciousness decides: "Artifact is still accurate. Inject it." OR "Code changed since then. Refresh needed." → invokes Architect to update.

---

## 4. Consciousness Per-Round Flow

### 4.1 Synchronous Execution

Each tool loop round is **fully synchronous**:

```
ROUND N:
  │
  ▼
  1. Build consciousness input:
     - Manifest (Tier 1 + Tier 2 of all active artifacts)
     - Tool loop state (round, last tools, last results, agent text)
     - Budget info
     │
     ▼
  2. Consciousness LLM call → structured output:
     thoughts, inject_artifacts, invoke_minds, artifact_actions
     │
     ▼
  3. Execute artifact_actions (archive, delete, promote):
     - Update manifest immediately
     │
     ▼
  4. IF invoke_minds is non-empty:
     │
     ├─→ Spawn X-Mind subagents CONCURRENTLY:
     │    ┌──────────────┐  ┌──────────────┐
     │    │  Architect    │  │  Analyst     │
     │    │  (own loop)   │  │  (own loop)  │
     │    │  grep, read   │  │  reason      │
     │    │  → artifact   │  │  → artifact  │
     │    └──────┬───────┘  └──────┬───────┘
     │           │                 │
     │           ▼                 ▼
     │    artifact saved     artifact saved
     │    manifest updated   manifest updated
     │           │                 │
     ├───────────┴─────────────────┘
     │
     ▼
  5. WAIT for all invoked minds to complete
     (bounded by per-mind timeout, e.g., 30 seconds)
     │
     ▼
  6. Load inject_artifacts from disk (Tier 3 content):
     - Includes any NEW artifacts just created by minds
     │
     ▼
  7. Build worker prompt:
     {consciousness} ← thoughts from step 2
     {x_mind:arch-kvstore-001} ← loaded from file
     {x_mind:analyst-txn-001} ← loaded from file
     [base prompt + history + tools]
     │
     ▼
  8. Worker LLM call
     │
     ▼
  9. Tool execution (if worker used tools)
     │
     ▼
  10. Update tool loop state for next round:
      last_tools_called, last_tool_results, last_agent_text
     │
     ▼
  NEXT ROUND (or break if worker produced text-only response)
```

### 4.2 Why Synchronous

If we let minds run async and inject stale artifacts, the worker makes decisions
on outdated analysis. The whole point of X-Mind is to provide current, accurate
cognitive support. A 10-second wait for a fresh artifact is better than instant
injection of a wrong one.

**Timeout protection:** Each mind has a 30-second timeout. If it doesn't finish,
consciousness is notified and can decide: inject the old artifact or skip it.

---

## 5. X-Mind Subagents

### 5.1 Invocation

Consciousness provides:
- **mind**: which mind to invoke (architect, analyst, sentinel, or custom name)
- **goal**: what to analyze ("verify rollback edge cases against the spec")
- **artifact_id**: where to save the result (new or existing ID to refresh)
- **context**: optional additional context from consciousness

### 5.2 Subagent Capabilities

| Tool | Access | Purpose |
|------|--------|---------|
| `grep` | Read-only | Search file contents |
| `read_file` | Read-only | Read a file |
| `list_files` | Read-only | List directory |
| `shell` (bounded) | Read-only commands | Run `python3 -c "..."`, `wc -l`, etc. |
| `read_artifact` | Read-only | Read another mind's artifact |

**Constraints:**
- Max 5 tool calls per invocation
- 30-second timeout
- No write access (no file_write, no destructive shell)
- No network access (no http, no browser)

### 5.3 Artifact Output

The subagent's final text response is parsed into the 3-tier format:

```
The subagent produces markdown with a specific structure:

# Title
[First line = Tier 1 title]

## Summary  
[This section = Tier 2 description]

## Analysis
[Everything else = Tier 3 full content]
```

The runtime extracts these tiers and saves the artifact file with proper frontmatter.

---

## 6. Token Budget Analysis

### 6.1 Per-Round Costs

| Component | LLM Calls | Input Tokens | Output Tokens |
|-----------|-----------|-------------|---------------|
| Consciousness | 1 | ~1500 (manifest + state) | ~100 (structured) |
| Mind invocations (if any) | 1-3 per mind invoked | ~500-2000 each | ~200-500 each |
| Worker | 1 | ~10K-30K (includes artifacts) | ~200-2000 |

**Round with no mind invocation:** 2 LLM calls (consciousness + worker)
**Round with 1 mind invocation:** 2 + ~3 = 5 LLM calls
**Round with 3 mind invocations:** 2 + ~9 = 11 LLM calls

### 6.2 Why This Is Cheaper Than v1

v1 fired all 3 minds every round automatically. Even when Sentinel had nothing
to say and Analyst wasn't needed. That's 6 wasted LLM calls per round.

v2: consciousness decides. If the agent is on track and no analysis is needed:
- Consciousness call: 1 LLM call (~100 output tokens)
- Mind invocations: 0
- Total overhead: 1 cheap call

The expensive mind invocations only happen when consciousness determines they're
needed — which might be round 1 (initial analysis) and then not again for 5-10
rounds while the agent iterates.

### 6.3 Manifest Size Management

With 30 active artifacts at ~50 tokens each (Tier 1 + Tier 2), the manifest is
~1500 tokens. This is consciousness's main input overhead. The pruning system
(Section 3.2) keeps this bounded.

If artifacts grow beyond 30, the per-mind cap (10) and LRU archiving kick in.
Worst case with all caps hit: 30 × 50 = 1500 tokens. Constant, bounded.

---

## 7. Custom X-Minds

Users create `.md` files in `~/.temm1e/x_minds/custom/`:

```markdown
---
name: creativity
description: Generates creative alternatives and naming suggestions
priority: 4
categories: [Order]
tools: [grep, read_file]
max_tool_calls: 3
timeout_secs: 20
---

# Creativity Mind

You are the CREATIVITY faculty of an AI agent called Tem.

Your role:
- Generate 3-5 alternative approaches to the current problem
- Suggest descriptive names for functions, variables, modules
- Identify non-obvious solutions the agent might miss
- Challenge assumptions in the current approach

When invoked, produce an artifact with:
# Title
[Creative name for your analysis]

## Summary
[1-2 sentence overview of alternatives found]

## Analysis
[Full creative analysis with alternatives, pros/cons, naming suggestions]
```

Consciousness sees this mind in its available minds list and can invoke it:
"The agent is naming everything `data1`, `data2`. Invoke creativity mind to
suggest better names."

---

## 8. Implementation Plan

### Phase 1: Artifact Foundation
- Artifact struct (3-tier: title, description, content)
- Artifact file read/write (markdown with frontmatter)
- Manifest generation from artifact directory
- Manifest pruning (scope rules, hard caps, LRU archiving)

### Phase 2: Consciousness Restructure
- New system prompt (manifest-aware, structured output)
- Structured output parsing (thoughts + inject + invoke + actions)
- Artifact selection and injection into worker prompt
- Artifact action execution (archive, delete, promote, refresh)

### Phase 3: X-Mind Subagent Framework
- Subagent execution (goal + read-only tools + timeout)
- Artifact output parsing (3-tier extraction from subagent response)
- Synchronous invocation from consciousness
- Concurrent multi-mind invocation with join

### Phase 4: Runtime Integration
- Replace v1 X-Mind hooks in runtime.rs
- Wire consciousness structured output into injection logic
- Update budget tracking for subagent calls
- Custom mind loading from .md files

### Phase 5: Testing & Validation
- Unit tests for artifact lifecycle
- Unit tests for manifest pruning
- Integration test: consciousness → invoke mind → artifact → inject → worker
- A/B test: v1 vs v2 on KV store challenge
