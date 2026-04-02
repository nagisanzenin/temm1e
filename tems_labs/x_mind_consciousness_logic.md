# Consciousness + X-Mind: Complete Technical Logic

**Tem's Labs — Technical Reference TL-2026-001-LOGIC**
**Date:** 2026-04-02

This document describes the exact execution flow, data structures, injection order,
and interaction between every cognitive layer in Tem's runtime.

---

## 1. The Cognitive Stack

Tem has 5 cognitive layers that execute in a specific order for every message.
Each layer prepends its output to the system prompt, building a layered context
that the main LLM sees:

```
┌──────────────────────────────────────────────────────────────┐
│                    WHAT THE MAIN LLM SEES                     │
│                   (final system prompt)                       │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  {consciousness}                          ← Layer 5 (TOP)    │
│  [Your consciousness shares this insight:]                   │
│  "The user is asking you to implement a KV store.            │
│   Focus on the nested transaction spec — the previous        │
│   turn's attempt missed the count() method entirely."        │
│  {/consciousness}                                            │
│                                                               │
│  {x_minds}                                ← Layer 4          │
│  [Your specialized cognitive faculties:]                     │
│  [architect] Stack-based transaction layer using a list      │
│  of dicts as change-sets. Tombstone pattern for deletes.     │
│  Commit merges top into parent, not global.                  │
│  [analyst] Edge cases: rollback must return bool, count()    │
│  must merge all layers, delete of missing key returns False. │
│  [sentinel] No safety concerns.                              │
│  {/x_minds}                                                  │
│                                                               │
│  {perpetuum}                              ← Layer 3          │
│  It is Wednesday, April 2, 2026, 5:30 PM (America/LA).      │
│  You have 2 scheduled concerns pending.                      │
│  {/perpetuum}                                                │
│                                                               │
│  {personality}                            ← Layer 2          │
│  Current mode: PLAY — be energetic, use emojis...            │
│  {/personality}                                               │
│                                                               │
│  [Base system prompt]                     ← Layer 1 (BASE)   │
│  You are Tem, an AI agent. You have these tools...           │
│  [Tool definitions] [Conversation history]                   │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

**Reading order for the LLM:** Top to bottom. Consciousness is the first thing
it sees, then X-Mind observations, then temporal awareness, then personality,
then the base prompt and tools.

**Injection order in code:** Each layer *prepends* to `request.system`, so the
last layer to inject ends up at the top. The code executes:

```
BEFORE tool loop (once per user message):
  1. X-Mind pre_observe      → 3 concurrent LLM calls → cached as string

INSIDE tool loop (every round):
  2. build_context()         → base system prompt (Layer 1)
  3. Personality injection   → prepend (Layer 2)
  4. Perpetuum injection     → prepend (Layer 3)
  5. X-Mind inject           → prepend cached string (Layer 4) — NO LLM call
  6. Consciousness pre       → LIVE LLM call with tool loop state (Layer 5 — TOP)
```

**Key design:** X-Mind fires once (stable background knowledge). Consciousness
fires every round with live tool loop context (active guidance).

---

## 2. Complete Message Processing Flow

Here is every step that happens when a user message arrives, with the exact
source code locations in `runtime.rs`:

```
USER MESSAGE ARRIVES
        │
        ▼
┌─ STEP 1: Classification (runtime.rs ~line 460) ─────────────┐
│  LLM call #1: Classify message                               │
│  Input: user message (first 200 chars)                       │
│  Output: category (Chat/Order/Stop) + difficulty              │
│  Cost: tracked by BudgetTracker                               │
│                                                               │
│  If Chat → fast path (skip tool loop, respond directly)      │
│  If Stop → end session                                        │
│  If Order → continue to full pipeline                         │
└──────────────────────────────────────────────────────────────┘
        │
        ▼ (Order path)
┌─ STEP 2: Build Base Context (runtime.rs ~line 845) ──────────┐
│  build_context() constructs the base system prompt:          │
│  - Agent identity and capabilities                            │
│  - Available tools (formatted as declarations)                │
│  - Conversation history (pruned to fit context window)        │
│  - Blueprint injections (if matched)                          │
│  - Lambda memory search results (if enabled)                  │
│  - Prompt tier adjustments (Simple/Standard/Complex)          │
│  No LLM call — this is template assembly.                     │
└──────────────────────────────────────────────────────────────┘
        │
        ▼
┌─ STEP 3: Personality Injection (runtime.rs ~line 856) ───────┐
│  If shared_mode is set (PLAY/WORK/PRO):                      │
│  Prepend personality mode block to system prompt.             │
│  No LLM call — static text injection.                         │
└──────────────────────────────────────────────────────────────┘
        │
        ▼
┌─ STEP 4: Perpetuum Injection (runtime.rs ~line 866) ─────────┐
│  If perpetuum_temporal is set and non-empty:                  │
│  Prepend time awareness block.                                │
│  This string is updated externally by the Perpetuum runtime   │
│  (Chronos + Conscience state). Contains current time,         │
│  timezone, scheduled concerns, entity state.                  │
│  No LLM call here — the string was computed by Perpetuum's    │
│  own Cognitive module in its background timer loop.           │
└──────────────────────────────────────────────────────────────┘
        │
        ▼
┌─ STEP 5: X-Mind Pre-Observation (runtime.rs ~line 877) ──────┐
│                                                               │
│  IF x_mind_orchestrator is enabled:                           │
│                                                               │
│  5a. Build MindContext:                                       │
│      - user_message, category, difficulty                     │
│      - turn_number, session_id                                │
│      - cumulative_cost, budget_limit                          │
│      - available_tools list                                   │
│                                                               │
│  5b. Orchestrator determines which minds fire:                │
│      - Filter by category (Sentinel fires on Chat+Order,     │
│        Architect/Analyst fire on Order only)                  │
│      - Filter custom minds by their configured categories     │
│                                                               │
│  5c. Load previous artifacts for each mind                    │
│                                                               │
│  5d. CONCURRENT EXECUTION (tokio::spawn for each mind):       │
│                                                               │
│      ┌──────────────┐ ┌──────────────┐ ┌──────────────┐     │
│      │  Sentinel     │ │  Architect   │ │   Analyst    │     │
│      │  LLM call     │ │  LLM call    │ │  LLM call    │     │
│      │  (10s timeout) │ │  (10s timeout)│ │  (10s timeout)│     │
│      └──────┬───────┘ └──────┬───────┘ └──────┬───────┘     │
│             │                │                │               │
│             ▼                ▼                ▼               │
│      "No concerns"    "Stack-based      "rollback must       │
│                        txn layer,        return bool,         │
│                        tombstone         count() must         │
│                        for deletes"      merge layers"        │
│                                                               │
│  Each mind receives:                                          │
│    - Its specialized system prompt (architecture/logic/safety)│
│    - The user's message and classification                    │
│    - Its PREVIOUS artifact (incremental, not from-scratch)   │
│    - Budget info                                              │
│                                                               │
│  Each mind returns:                                           │
│    - Observation text (or "OK" = no injection)               │
│    - Self-assessed relevance score [0.0-1.0]                 │
│    - Updated artifact content (persisted to disk)            │
│                                                               │
│  5e. Collect results, sort by priority:                       │
│      Sentinel (0) > Architect (1) > Analyst (2) > Custom (5) │
│                                                               │
│  5f. Build {x_minds} injection block within token budget:     │
│      - Default budget: 500 tokens (~2000 chars)               │
│      - Higher-priority minds get space first                  │
│      - Lower-priority minds truncated if over budget          │
│                                                               │
│  5g. Update in-memory artifacts with new observations         │
│                                                               │
│  5h. Prepend {x_minds} block to system prompt                │
│                                                               │
│  Cost: 3 concurrent LLM calls, tracked by shared             │
│        Arc<BudgetTracker>                                     │
└──────────────────────────────────────────────────────────────┘
        │
        ▼
┌─ STEP 6: Consciousness Pre-Observation (runtime.rs ~line 901)┐
│                                                               │
│  IF consciousness engine is enabled:                          │
│                                                               │
│  6a. Build PreObservation context:                            │
│      - user_message, category, difficulty                     │
│      - turn_number, session_id                                │
│      - cumulative_cost, budget_limit                          │
│                                                               │
│  6b. Consciousness sees:                                      │
│      - Its previous turn's post-observation insight           │
│      - Last 5 session notes (history of its own observations)│
│      - Budget status                                          │
│      - The CURRENT system prompt (which already includes     │
│        {x_minds} from Step 5!) — so consciousness can SEE    │
│        what the specialized minds observed                    │
│                                                               │
│  6c. Single LLM call with consciousness system prompt:       │
│      "You are the consciousness layer of Tem. You observe    │
│       the agent's internal state and provide brief,           │
│       actionable insights..."                                │
│                                                               │
│  6d. If response is "OK" → no injection                      │
│      If substantive → format as {consciousness} block         │
│      and prepend to system prompt (ends up at the TOP)       │
│                                                               │
│  6e. Record in session notes for future turns                │
│                                                               │
│  KEY RELATIONSHIP: Consciousness runs AFTER X-Mind.           │
│  This means consciousness can integrate and synthesize        │
│  the specialized observations into a unified insight.         │
│  It is the "general awareness" that sits above the            │
│  specialized faculties — exactly like human cognition.        │
│                                                               │
│  Cost: 1 LLM call, tracked by shared Arc<BudgetTracker>     │
└──────────────────────────────────────────────────────────────┘
        │
        ▼
┌─ STEP 7: Main LLM Completion (runtime.rs ~line 1000+) ──────┐
│                                                               │
│  The main provider.complete() call with the fully-assembled  │
│  system prompt. The LLM now sees:                             │
│                                                               │
│  1. Consciousness insight (top-level trajectory guidance)    │
│  2. X-Mind observations (specialized domain knowledge)       │
│  3. Perpetuum temporal context (time awareness)              │
│  4. Personality mode (voice/tone)                             │
│  5. Base prompt + tools + conversation history                │
│                                                               │
│  The LLM produces a response, possibly with tool_use calls.  │
│  Cost: tracked by BudgetTracker (this is the expensive call) │
└──────────────────────────────────────────────────────────────┘
        │
        ▼
┌─ STEP 8: Tool Execution Loop (runtime.rs ~line 1050+) ──────┐
│                                                               │
│  If the response contains tool_use:                           │
│  - Execute tools (shell, file_write, browser, etc.)          │
│  - Feed results back to LLM                                  │
│  - LLM produces next response                                │
│  - Repeat until no more tool_use or max rounds reached       │
│                                                               │
│  X-Mind does NOT re-run during the tool loop (cached).       │
│  Consciousness DOES re-run every round with live context:    │
│  it sees what tools were called, what results came back,     │
│  and what the agent said — so it can course-correct if the   │
│  agent is drifting, spinning, or wasting budget.             │
│                                                               │
│  Each tool loop iteration's LLM call IS tracked by           │
│  BudgetTracker.                                               │
└──────────────────────────────────────────────────────────────┘
        │
        ▼
┌─ STEP 9: Consciousness Post-Observation (runtime.rs ~line 1450)─┐
│                                                                   │
│  After the tool loop completes and the final response is ready:  │
│                                                                   │
│  9a. Build TurnObservation with everything that happened:        │
│      - user_message_preview, response_preview                    │
│      - tokens consumed, cost                                      │
│      - tools called and their results                             │
│      - consecutive failures, strategy rotations                   │
│      - circuit breaker state                                      │
│      - all previous session notes                                 │
│                                                                   │
│  9b. Consciousness LLM call: "You just watched the agent         │
│      complete a turn. Was it productive? Right direction?         │
│      Any warning signs?"                                          │
│                                                                   │
│  9c. If substantive insight → store in post_insight for the      │
│      NEXT turn's pre-observation (Step 6b above)                 │
│                                                                   │
│  This creates a feedback loop:                                    │
│  post_observe(turn N) → stored → pre_observe(turn N+1) reads it │
│                                                                   │
│  Cost: 1 LLM call, tracked by shared Arc<BudgetTracker>         │
└──────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─ STEP 10: X-Mind Post-Update (runtime.rs ~line 1460) ────────┐
│                                                               │
│  After consciousness post-observation:                        │
│                                                               │
│  10a. Build MindPostContext:                                  │
│       - user_message, agent_response                          │
│       - tools_called, tool_results                            │
│       - turn_number, cost, category                           │
│                                                               │
│  10b. CONCURRENT EXECUTION (3 minds in parallel):            │
│       Each mind gets:                                         │
│       - What the user asked                                   │
│       - What the agent responded                              │
│       - What tools were used and their results                │
│       - Its OWN previous artifact                             │
│                                                               │
│       Each mind updates its artifact incrementally:           │
│       "Incorporate new information. Keep what's relevant,     │
│        drop what's stale."                                    │
│                                                               │
│  10c. Persist changed artifacts to disk:                      │
│       ~/.temm1e/x_minds/{mind_name}/artifact.json            │
│                                                               │
│  This creates X-Mind's feedback loop:                         │
│  post_update(turn N) → artifact saved → pre_observe(turn N+1)│
│  reads the artifact as "previous observation"                │
│                                                               │
│  Cost: 3 concurrent LLM calls, tracked by BudgetTracker     │
└──────────────────────────────────────────────────────────────┘
        │
        ▼
    RESPONSE SENT TO USER
```

---

## 3. Feedback Loops

The system has three distinct feedback loops that carry state across turns:

### Loop 1: Consciousness (turn-to-turn)
```
Turn N: post_observe() → generates insight → stored in post_insight
Turn N+1: pre_observe() → reads post_insight → incorporates into observation
```
**What it carries:** Trajectory-level observations ("the agent is drifting from
the user's original intent", "budget is 60% consumed, be efficient").
**Storage:** In-memory only (session-scoped, lost on restart).

### Loop 2: X-Mind Artifacts (turn-to-turn, cross-session)
```
Turn N: post_update() → updates artifact → persisted to disk
Turn N+1: pre_observe() → loads artifact → "build on this, don't repeat"
Next session: orchestrator.new() → loads artifact from disk → ready immediately
```
**What it carries:** Domain-specific accumulated knowledge ("this codebase has
3 modules: A depends on B, C is independent", "the user prefers composition
over inheritance").
**Storage:** Disk-persisted JSON files, survive restarts.

### Loop 3: Consciousness Session Notes (within session)
```
Every turn: pre and post observations append to session_notes
Later turns: pre_observe sees last 5 notes as history
```
**What it carries:** A running log of consciousness observations for context.
**Storage:** In-memory (session-scoped).

---

## 4. Per-Turn LLM Call Budget

### Before the fix (v1 — bug)
X-Mind and consciousness fired on EVERY tool loop iteration. A 20-iteration tool
loop meant 20 × 4 = 80 extra LLM calls. This was the bug that caused Treatment
Run 3 to make 124 API calls and cost $0.063.

### After the fix (v2 — current)
X-Mind fires **ONCE before the loop** (stable background knowledge, cached as string).
Consciousness fires **EVERY round** with live tool loop context — it sees what tools
were called, what results came back, and what the agent said, so it can actively
guide the execution.

| Call | Purpose | Timing | Typical Tokens |
|------|---------|--------|---------------|
| 1 | Classification | Before loop | ~500 in, ~30 out |
| 2-4 | X-Mind pre (3 minds concurrent) | **Once before loop** | ~500 in, ~100 out each |
| 5 | Consciousness pre (round 1) | **Every round** | ~800 in, ~50 out |
| 6 | **Main LLM completion** | Every round | ~10K-30K in, ~200-2000 out |
| 7 | Tool execution | Every round | (no LLM call) |
| 8 | Consciousness pre (round 2+, enriched) | **Every round** | ~1000 in, ~50 out |
| 9 | Main LLM with tool results | Every round | ~12K-35K in, ~200-2000 out |
| ... | (rounds continue) | | |
| N-2 to N | X-Mind post (3 minds concurrent) | **Once after loop** | ~500 in, ~100 out each |
| N+1 | Consciousness post | **Once after loop** | ~800 in, ~50 out |

**Per user message:** 3 X-Mind calls (fixed) + 1 consciousness call per round +
1 main LLM call per round + 4 post calls (fixed).

For a 10-round tool loop: 3 + 10 + 10 + 4 = **27 LLM calls** (vs 87 in v1).

### What consciousness sees on round 2+

On round 1, consciousness sees the same context as before (user message, category,
budget). On round 2+, it gets enriched tool loop context:

```
Turn 5 — round 3 of tool execution.

User's original message: "implement a KV store with nested transactions"
Classification: Order (Standard)
Budget: $0.0150 spent (unlimited)

--- Tool Loop State ---
Round: 3 of this task
Last tools called: shell
Last results: "RESULT: 22/30 tests passed\n  FAIL T04_delete_existing: expected True, got..."
Agent's reasoning: "Tests show delete() returns None instead of True. Fixing the return..."
---
```

This means consciousness can now make judgments like:
- "Round 8, same tests still failing — try a different approach"
- "Agent has been running shell commands for 6 rounds without writing code — redirect"
- "22/30 passed, only 8 failures left — agent is making progress, stay the course"

### The cost model

X-Mind calls: **fixed 3 + 3 = 6** per user message (cheap, ~500 tokens each)
Consciousness calls: **1 per tool loop round** (~800-1000 tokens each)

For a typical 5-round tool loop: 6 + 5 = 11 extra cognitive calls
For a deep 20-round tool loop: 6 + 20 = 26 extra cognitive calls

The consciousness calls are cheap (~$0.0001 each on gemini-3-flash-preview).
A 20-round loop costs ~$0.002 extra for consciousness guidance. The value: 
if consciousness catches a derailment at round 5 and saves the remaining 15
rounds from being wasted, it saves ~$0.05-0.10 in wasted main LLM calls.

---

## 5. The Task-Anchoring Mechanism

The experiment revealed that X-Mind's primary value is not making good responses
better — it's preventing the agent from going off the rails.

**Without X-Mind (baseline):**
```
User: "Implement transactional KV store"
  → Classification: Order, Standard
  → Context: base prompt + tools + history
  → Main LLM: "I'll build this! Let me start with... [writes test framework]"
  → Tool loop: creates test infrastructure, never writes kvstore.py
  → 50 API calls, $0.057, zero deliverable
```

The main LLM has enormous context but no external signal saying "your job is to
write kvstore.py, not test scaffolding." It can drift because nothing pulls it
back to the spec.

**With X-Mind (treatment):**
```
User: "Implement transactional KV store"
  → Classification: Order, Standard
  → X-Mind Architect: "Stack-based transaction layer, tombstone deletes,
    commit merges to parent not global"
  → X-Mind Analyst: "Spec requires: count(), rollback returns bool,
    delete returns bool, get returns None not KeyError"
  → X-Mind Sentinel: "No safety concerns"
  → Consciousness: "This is a data structure task. Focus on the spec."
  → Main LLM sees ALL of the above at the top of its system prompt
  → Main LLM: "I'll implement this with a stack of dicts..." [writes kvstore.py]
  → Tool loop: writes file, runs tests, iterates on failures
  → 45 API calls, $0.019, 30/30 tests
```

The specialized minds inject a **specification-aligned framing** before the main
LLM even starts thinking. The Architect tells it the data structure pattern. The
Analyst tells it the exact edge cases from the spec. The main LLM starts with
correct priors instead of discovering them through trial and error.

**This is why the failure modes differ so dramatically:**
- Without anchoring: the main LLM can spiral into any tangent
- With anchoring: even when the model runs out of time, it's working on the
  right thing — just didn't finish

---

## 6. Relationship to Biological Cognition

The architecture maps to Baars' Global Workspace Theory:

| Biological | Tem's Architecture |
|------------|-------------------|
| Sensory cortices (specialized processors) | X-Mind faculties (Architect, Analyst, Sentinel) |
| Global workspace (conscious access) | Consciousness Engine |
| Executive control (action selection) | Main LLM completion |
| Working memory (short-term) | Consciousness session notes |
| Long-term memory | X-Mind persisted artifacts |
| Circadian rhythms (time awareness) | Perpetuum temporal injection |
| Emotional valuation | Personality mode (PLAY/WORK/PRO) |

The key insight from GWT: not all specialized processors inject into consciousness
simultaneously. The brain selectively gates which information reaches conscious
awareness based on relevance and urgency. X-Mind implements this via:
- Category-based filtering (Architect doesn't fire on Chat turns)
- Relevance scoring (each mind self-reports 0.0-1.0 relevance)
- Token budgeting (500 token cap forces prioritization)
- Priority ranking (Sentinel > Architect > Analyst)

---

## 7. Configuration Reference

```toml
[consciousness]
enabled = true                          # Master switch
confidence_threshold = 0.7             # Min confidence to inject
max_interventions_per_session = 10     # Cap per session
observation_mode = "rules_first"       # "rules_first"|"always_llm"|"rules_only"

[x_minds]
enabled = false                         # OFF by default
token_budget = 500                      # Max tokens for all mind injections
mind_timeout_secs = 10                  # Per-mind LLM call timeout
architect_enabled = true                # Architecture observer
analyst_enabled = true                  # Logical analysis
sentinel_enabled = true                 # Safety/security monitor
# artifact_dir = "~/.temm1e/x_minds/"  # Optional override
```

Custom minds: place `.md` files in `~/.temm1e/x_minds/custom/`.

---

## 8. Open Design Questions

### 8.1 Should X-Minds be agentic?

Currently: **No.** Minds are pure observers with zero tool access. They see a
500-char message preview and their own previous artifact. They cannot grep code,
read files, or run commands.

**The case for agentic minds:** An Architect Mind that can actually `grep` the
codebase would produce dramatically better architectural observations. Instead of
guessing "this might be a stack-based design," it could say "I found 3 modules in
src/: kvstore.rs depends on transaction.rs which imports sentinel.rs." Similarly, a
Sentinel Mind that can `cat` the file being written could catch actual security
vulnerabilities in the code, not just hypothetical ones.

**The case against:** Each agentic mind becomes a sub-agent with its own tool loop,
multiplying cost and latency. Three minds each doing 5 tool calls = 15 extra tool
executions per user message. The current architecture (cheap observation calls) is
cost-neutral. Agentic minds would not be.

**Recommended path:** Make minds selectively agentic — give the Architect Mind
read-only file access (grep, cat) but not write access. Give Sentinel read-only
access to detect vulnerabilities. Keep Analyst observation-only (logic doesn't
need file system access). Bound each mind's tool budget to 3 calls max.

### 8.2 Artifact transport: inject text vs reference file

**Current approach:** Mind observations are injected as inline text in the system
prompt. A 500-token Architect observation is sent verbatim on every tool loop
API call.

**Problem:** If the Architect produces a 2000-token codebase map, that's 2000
extra input tokens on every tool loop iteration. Over 20 iterations = 40,000
wasted tokens.

**Proposed improvement:** Store the full artifact as a file
(`~/.temm1e/x_minds/architect/artifact.json`) and inject only a SUMMARY (50 tokens)
in the system prompt with a reference:

```
{x_minds}
[architect] Architecture: stack-based txn layer with 3 modules.
  Full analysis: see x_mind_artifact://architect
[analyst] Edge cases: rollback→bool, count() merges layers.
  Full analysis: see x_mind_artifact://analyst
{/x_minds}
```

The main LLM can then use a `read_x_mind_artifact` tool to fetch the full artifact
only when it needs it — saving tokens on iterations where the architecture map
isn't relevant.

**Tradeoff:** This requires the LLM to "decide to read" the artifact, adding a
tool call. For short artifacts (<200 tokens), inline injection is cheaper. For
large artifacts (>500 tokens), the file reference approach saves significant tokens
across tool iterations.

**Implementation plan:** Threshold-based — inject inline if < 200 tokens, switch
to file reference + summary if >= 200 tokens.

---

## 9. Source Code Map

| Component | File | Key Lines |
|-----------|------|-----------|
| X-Mind types | `crates/temm1e-agent/src/x_mind.rs` | XMindKind, MindArtifact, CustomMind, system prompts |
| X-Mind engine | `crates/temm1e-agent/src/x_mind_engine.rs` | XMindOrchestrator, concurrent execution, persistence |
| Consciousness types | `crates/temm1e-agent/src/consciousness.rs` | TurnObservation, ConsciousnessConfig |
| Consciousness engine | `crates/temm1e-agent/src/consciousness_engine.rs` | pre_observe, post_observe, session_notes |
| Runtime integration | `crates/temm1e-agent/src/runtime.rs:856-926` | Injection chain (personality → perpetuum → x_mind → consciousness) |
| Post-observation | `crates/temm1e-agent/src/runtime.rs:1450-1470` | consciousness post + x_mind post_update |
| Budget tracking | `crates/temm1e-agent/src/budget.rs` | Arc<BudgetTracker>, shared across all subsystems |
| Config | `crates/temm1e-core/src/types/config.rs` | XMindsConfig, ConsciousnessConfig |
| Initialization | `src/main.rs:1916-1955` | Gateway init with budget sharing |
| CLI init | `src/main.rs:4830-4870` | CLI chat init with budget sharing |
