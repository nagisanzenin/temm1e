# TemDOS — Tem Delegated Operating Subsystem
## Research Paper & Implementation Plan

## Context

TEMM1E's main agent currently handles all cognitive work inline — when it needs architecture analysis, it reads 50 files itself, polluting its own context window. When it needs security review AND test generation, it does them sequentially. The skill system (static markdown injected into context) adds knowledge but not execution capability.

**TemDOS (Tem Delegated Operating Subsystem)** introduces specialist sub-agents — called Cores, inspired by GLaDOS's personality cores from Portal — that run as tools within the main agent's loop. Each Core is a pre-defined specialist with its own LLM loop, full tool access, and shared budget. The main agent invokes Cores like any other tool, gets structured output back, and continues.

This replaces ad-hoc subagent spawning with stable, versioned, tested specialists.

---

## Part 0: Formal Definitions

### The Main Agent (GLaDOS)

The **Main Agent** is TEMM1E's central consciousness — the single entity that owns the user conversation, holds conversation history, manages the budget, and makes all high-level decisions. It is the `AgentRuntime` in `crates/temm1e-agent/src/runtime.rs`.

**Responsibilities:**
- Receives user messages from channels (Telegram, Discord, CLI, etc.)
- Classifies intent (Chat / Order / Stop) via the LLM classifier
- Maintains the full conversation context (history, memory, blueprints)
- Executes the tool loop: calls LLM → parses tool_use → dispatches tools → feeds results back
- Makes strategic decisions: when to use tools directly, when to delegate to a Core
- Tracks budget across all operations (its own + all Core invocations)
- Returns the final response to the user

**What the Main Agent is NOT:**
- It is not a specialist. It is a generalist coordinator.
- It does not need to be an expert in architecture, security, or testing — it has Cores for that.
- It does not do deep research itself when a Core can do it in isolation without polluting its context.

**The Main Agent's judgment** is the most valuable resource in the system. Every token of context it carries should serve the current task. Cores exist to offload deep research so the Main Agent stays focused.

### Cores (Personality Cores)

A **Core** is a specialist sub-agent — a pre-defined, versioned, tested AI entity with a fixed domain of expertise. It runs as a tool within the Main Agent's loop.

**Properties:**
- **Identity**: Each Core has a name, description, and version defined in a `.md` file
- **Specialist system prompt**: A fixed prompt that focuses the Core on its domain
- **Full tool access**: Can read files, run shell commands, search code — everything the Main Agent can do, EXCEPT invoke other Cores
- **Own context**: Runs in an isolated LLM loop with its own SessionContext. Does not see or pollute the Main Agent's conversation
- **Shared budget**: Deducts from the same BudgetTracker as the Main Agent. No separate allocation
- **Stateless**: Each invocation starts fresh. No memory between invocations
- **Returns text**: The Core's final answer is returned as a ToolOutput to the Main Agent

**What a Core is NOT:**
- Not the Main Agent. It cannot talk to the user directly.
- Not recursive. It cannot invoke other Cores.
- Not persistent. It has no memory between invocations.
- Not a replacement for simple tool calls. The Main Agent should not invoke a Core to read one file.

### How They Work Together

```
User: "Refactor the provider system to support streaming for all backends"

Main Agent (GLaDOS):
  1. Classifies: Order (Complex)
  2. Thinks: "I need to understand the current architecture before refactoring"
  3. Invokes invoke_core(core="architecture", task="Map the provider crate:
     which backends support streaming, which don't, what trait methods exist,
     what types are involved")
  4. Architecture Core runs (own loop, ~8 tool rounds, reads 12 files)
  5. Returns: "3 of 5 backends support streaming. The Provider trait has
     stream() returning BoxStream. Anthropic and OpenAI implement it.
     Gemini has a stub. Grok and MiniMax don't implement it..."
  6. Main Agent now has a precise map WITHOUT reading those 12 files itself
  7. Plans the refactor with full understanding
  8. Writes the code, using its clean context for the actual implementation
```

**Maximum effectiveness** comes from the division of labor:
- The Main Agent decides WHAT to do (strategy)
- Cores figure out the DETAILS (research, analysis, auditing)
- The Main Agent acts on the Core's findings (implementation)

This mirrors GLaDOS: she makes decisions. The cores feed her information. The morality core doesn't steer the ship — it informs the captain.

### Authoring Custom Cores (User Guide)

Users can create their own Cores by writing a `.md` file in `~/.temm1e/cores/` (global) or `<workspace>/cores/` (project-local).

**Template:**

```markdown
---
name: my-core-name
description: "One sentence — what this core does"
version: "1.0.0"
---

You are the [Name] Core, a specialist attached to a TEMM1E agent.

<task> contains the specific request from the main agent.

## Your Expertise
[Describe what this core knows and does well]

## Your Protocol
1. [Step-by-step approach for this domain]
2. [How to use tools for this analysis]
3. [How to structure the output]

## Constraints
- You CANNOT invoke other cores
- Stay focused on the task
- Return your final answer as plain text
```

**Rules for effective Core definitions:**
1. **Be specific about the domain.** "You are a security auditor" is better than "You analyze things."
2. **Define a protocol.** Tell the Core how to approach problems in its domain — what to look for, what order to investigate, how to structure findings.
3. **Include output format guidance.** The Main Agent needs actionable information, not stream-of-consciousness.
4. **Keep the system prompt focused.** Long prompts consume tokens every round. Say what matters, cut what doesn't.
5. **Test with real tasks.** A Core that sounds good on paper but gives vague answers needs a better prompt.

**The `<task>` placeholder** is replaced at invocation time with whatever the Main Agent passes. Design the prompt so this substitution reads naturally.

---

## Part T: Theoretical Foundation — Strengths, Weaknesses, and Timeproofing

Before implementation, we must identify every structural weakness, understand every strength, and design preemptive solutions. Code can be refactored. Bad theory propagates forever.

### T.1 The Ten Weaknesses of TemDOS

**W1: Context Blindness (Cold Start)**

Every Core invocation starts from zero. The Core doesn't know what the user said, what the Main Agent already discovered, what other Cores found, or what was tried and failed. The `task` string is the only bridge between the Main Agent's rich context and the Core's empty one.

If the Main Agent writes a vague task ("analyze the providers"), the Core works blind. If the Main Agent writes a detailed task with relevant context, the Core is informed. This puts the burden on the Main Agent to be a good "briefer" — and LLMs are inconsistent at knowing what context is relevant to pass.

*Preemptive solution*: Add an optional `context` parameter alongside `task` in the `invoke_core` schema. The Main Agent can pass conversation excerpts, previous Core findings, or specific constraints. The Core's system prompt includes a `<context>` section that gets populated. This makes the bridge between Main Agent and Core explicit and structured, rather than hoping the Main Agent stuffs everything into the task string.

```json
{
  "core": "architecture",
  "task": "Map all consumers of CompletionResponse and assess blast radius of adding a field",
  "context": "User is refactoring the provider system. Previous security core found no credential issues in providers. The user prefers minimal changes."
}
```

**W2: Cost Amplification**

A Core is a full sub-agent loop. An 8-round invocation with a 100K context window could cost $1-3 in provider calls. If the Main Agent casually invokes Cores for tasks it could handle in 1-2 tool calls, costs explode. The Main Agent's judgment about "when to invoke a Core" is the single biggest cost lever.

System prompt guidelines are soft — the LLM might still over-invoke.

*Preemptive solution*: Add a `cost_hint` to CoreResult that reports actual cost back to the Main Agent. Inject a running "Core spend this session" counter into the Main Agent's system prompt, making cost visible to the decision-maker. The Main Agent sees: "Cores have spent $2.40 of your $10 budget this session" and calibrates naturally. This is not a hard limit — it's information that enables good judgment.

**W3: No Inter-Core Communication**

Cores cannot talk to each other. If Architecture Core finds something relevant to Security Core, there's no direct channel. The Main Agent must relay.

This means dependent analyses cannot be parallelized: invoke Architecture → get result → invoke Security with "Architecture found X." Two sequential Core invocations instead of one parallel batch.

*Preemptive solution*: Accept this as a structural feature, not a bug. Inter-Core communication would re-introduce recursion risk (Core A talks to Core B which talks to Core A). The Main Agent IS the communication bus. The pattern is: parallel for independent work, sequential for dependent work. The Main Agent's system prompt should explicitly teach this pattern:
- "Need architecture AND security independently? Invoke both in parallel."
- "Need security to build on architecture findings? Invoke architecture first, then pass its output as context to security."

**W4: No Progress Feedback (The Silent Wait)**

The Main Agent calls `invoke_core` and waits. For potentially minutes. The user sees nothing — no progress, no intermediate results, no "I'm reading file X." This is terrible UX for long-running Cores.

*Preemptive solution*: Pass the Main Agent's `StreamingNotifier` (or a similar channel) to the Core runtime. The Core emits status updates at key checkpoints:
- `"[Architecture Core] Reading project structure..."`
- `"[Architecture Core] Analyzing 12 files in temm1e-providers..."`
- `"[Architecture Core] Synthesizing findings..."`

These are sent to the user via the existing streaming infrastructure. The Core doesn't need full streaming (token-by-token) — just checkpoint notifications. Implementation: the Core runtime calls `notifier.notify_tool_start("architecture: reading project structure")` at the start of each tool round.

**W5: Output Quality Variance**

A Core's output quality is entirely determined by its system prompt. A poorly written Core gives consistently bad output. Since Cores are stateless, they cannot learn from feedback within a session.

*Preemptive solution*: Two mechanisms.

First, **Core performance tracking** — analogous to Blueprint stats (`times_executed`, `times_succeeded`, `times_failed`, `avg_tool_calls`). After each Core invocation, the Main Agent implicitly evaluates: did the Core's output contribute to task completion? Track success rates per Core. Low-performing Cores surface for prompt revision.

Second, **output structure requirements** — each Core's system prompt should mandate a structured output format (findings, confidence, evidence). Structured output is easier for the Main Agent to evaluate and synthesize than free-form text.

**W6: Redundant Work**

The Core has the same tools as the Main Agent. If the Main Agent already read file X and then invokes a Core that also needs file X, the Core reads it again. Duplicate work, duplicate tokens.

*Preemptive solution*: The `context` parameter (from W1) serves double duty. The Main Agent can pass relevant findings it already has: "I already read runtime.rs — here are lines 50-120 which are relevant." The Core doesn't re-read what the Main Agent already knows. This is opt-in, not automatic — the Main Agent decides what to pre-share vs. what the Core should discover independently.

Some redundancy is acceptable. The cost of a redundant file read (~500 tokens) is negligible compared to the context isolation benefit. Don't over-optimize this.

**W7: No Memory Between Invocations**

If Architecture Core analyzed the repo in message 1 and is invoked again in message 5, it starts from scratch. All previous findings are lost. This is by design (statelessness), but it means repeated similar questions cost the same every time.

*Preemptive solution*: **Core result caching.** After each Core invocation, store the CoreResult in memory (Lambda memory or a dedicated store) with the core name, task hash, and timestamp. Before invoking a Core, check the cache: if an identical or semantically similar task was answered recently (within the session or within a time window), inject the cached result as context.

This is NOT persistent Core state — it's the Main Agent caching results it received. The Core remains stateless. The Main Agent just avoids re-asking questions it already has answers to.

Implementation: Add `maybe_recall_core_result(core_name, task)` that searches recent Core outputs in memory. If found, the Main Agent can either skip the invocation entirely or pass the cached result as context for the Core to build on.

**W8: No Output Verification**

The Main Agent receives a Core's output and trusts it. But Cores can hallucinate, miss critical details, or give confidently wrong analysis. There's no verification step.

*Preemptive solution*: Build verification into the Core's own protocol, not as an external check. Each Core's system prompt should include a self-verification step:

```
## Before Reporting
- Verify each claim against the actual code you read
- If you referenced a file, confirm the line numbers are correct
- If you made a recommendation, confirm the affected code paths exist
- State your confidence level: HIGH (verified against code), MEDIUM (inferred), LOW (uncertain)
```

The Main Agent can also spot-check high-stakes Core findings with a quick tool call. For maximum rigor on critical tasks, invoke the code-review Core to review another Core's output — but this is sequential and costly, reserved for high-risk changes.

**W9: System Prompt Token Overhead**

Each Core's system prompt is sent with EVERY LLM call the Core makes. A 2,000-token system prompt across 8 rounds = 16,000 tokens of pure overhead. Verbose Core prompts directly multiply cost.

*Preemptive solution*: **Core prompt budget guideline**: recommend 300-600 tokens for the system prompt. This is enforced socially (documentation) not mechanically (no hard limit). The parser can emit a warning if a Core's prompt exceeds 800 tokens.

The Core prompt should be a protocol definition, not an encyclopedia. "Here's how to approach security analysis" (300 tokens) not "Here's everything about OWASP" (3,000 tokens). The Core can discover domain knowledge through tools — it doesn't need it pre-loaded.

**W10: Main Agent Invocation Quality**

The entire TemDOS system depends on the Main Agent making good decisions: when to invoke a Core, which Core to invoke, what task to write, what context to pass. A bad invocation produces a bad result regardless of Core quality.

*Preemptive solution*: The system prompt guidelines must be precise and example-driven. Include concrete examples of good vs. bad invocations:

```
GOOD: invoke_core(core="architecture", task="Map all files that import from
      temm1e-providers and list which specific types/functions they use.
      I need to know the blast radius of changing CompletionResponse.",
      context="Adding a 'cache_hit: bool' field to CompletionResponse")

BAD:  invoke_core(core="architecture", task="look at the code")
```

The difference is specificity. Good tasks give the Core a clear objective and enough context to work autonomously. Bad tasks leave the Core guessing.

---

### T.2 The Seven Strengths of TemDOS

**S1: Context Isolation — The Main Agent Stays Clean**

This is the killer advantage. The Main Agent's context window is its most valuable resource — every token should serve the current task. When the Main Agent does its own research (reading 50 files, running 30 greps), all that raw data fills its context and pushes out conversation history.

With TemDOS, research happens in the Core's isolated context. The Main Agent receives only the distilled answer. Its context stays clean, focused, and full of relevant conversation — not raw file dumps from 15 minutes ago.

*How to maximize*: Never leak Core internals to the Main Agent. The CoreResult contains only the final answer text + metadata (rounds, cost). The Core's internal history (tool calls, intermediate reasoning) is discarded after invocation.

**S2: Parallelism — Time Is Not Wasted**

The Main Agent can fire multiple Cores simultaneously. "Analyze architecture AND audit security AND generate tests" as three parallel invocations. Wall-clock time = max(T_architecture, T_security, T_test) instead of the sum.

*How to maximize*: The system prompt should STRONGLY encourage parallel invocation for independent analyses. The Main Agent should think in batches: "What independent questions do I have?" → invoke all at once → synthesize results.

**S3: Specialization Depth — Expert vs. Generalist**

A Core's entire context is dedicated to its domain. It doesn't balance "I'm writing code" with "I'm also analyzing architecture." The Architecture Core ONLY analyzes architecture. Its system prompt is optimized for that one task. It knows what to look for, what order to investigate, how to structure findings.

The Main Agent, by contrast, is a generalist following ad-hoc instructions. It's spread thin across many concerns.

*How to maximize*: Invest heavily in Core system prompts. Each foundational Core should be as good as a purpose-built expert system. Iterate on prompts based on real usage. Track which prompts produce the best outputs.

**S4: Predictable Interface — Tools Are Contracts**

A Core is invoked via `invoke_core(core, task, context)` and returns `ToolOutput { content, is_error }`. This is the same contract as every other tool. No special handling, no new protocol, no conversation management. The executor already knows how to dispatch it, wait for it, and feed the result back.

*How to maximize*: Keep the interface simple. Resist adding special return types, streaming modes, or callback mechanisms to the Core tool. It's a tool. It takes input. It returns output. The simplicity IS the feature.

**S5: Composability — The Main Agent Synthesizes**

The Main Agent's generalist perspective is most valuable when combining specialist outputs. Architecture Core says "these 3 files are affected." Security Core says "this file has a credential leak." Test Core says "no tests cover this path." The Main Agent synthesizes: "I need to fix the leak in file X, update the 3 affected files, and add tests for the uncovered path."

No individual Core could do this synthesis because none has the full picture. The Main Agent does.

*How to maximize*: Core outputs should be structured for easy synthesis — bullet points, file paths with line numbers, actionable recommendations. Not essays.

**S6: Versioned and Testable — Cores Evolve**

Core definitions are `.md` files with version numbers. They can be iterated, A/B tested, shared, and rolled back. A bad prompt is a version bump away from being fixed.

*How to maximize*: Track Core performance metrics. After N invocations, compare output quality across versions. Auto-authored Cores (v0.1.0) get promoted to v1.0.0 after the user validates them.

**S7: Timeproof — Infrastructure vs. Intelligence**

The Core runtime is infrastructure (Rust code). The Core definition is intelligence (system prompt). When LLMs improve, Core definitions can be updated without recompiling. When new models arrive, Cores automatically benefit through the Provider trait. When new tools are added to TEMM1E, Cores automatically get access.

The runtime is minimal — no classification, no blueprints, no consciousness. There's nothing to become stale. It's a tool loop that will work the same whether the LLM is Claude 4 or Claude 10.

*How to maximize*: Keep the runtime minimal. Resist adding features to the Core runtime that belong in Core definitions. The system prompt is the right place for domain intelligence. The runtime is the right place for tool dispatch and budget tracking. Don't mix them.

---

### T.3 Preemptive Design Decisions (Weakness → Countermeasure)

| Weakness | Countermeasure | Implementation |
|----------|---------------|----------------|
| W1: Context blindness | `context` parameter in invoke_core schema | Schema: `{ core, task, context? }` |
| W2: Cost amplification | Core spend counter in Main Agent's prompt | Track cumulative Core cost per session, inject into system prompt |
| W3: No inter-core comms | Explicit sequential/parallel guidance in system prompt | Guidelines with concrete examples |
| W4: Silent wait | Checkpoint notifications via StreamingNotifier | Core runtime emits status at tool round boundaries |
| W5: Output quality | Performance tracking + structured output mandate | CoreStats (invocations, success rate); output format in Core prompts |
| W6: Redundant work | `context` parameter for pre-sharing known info | Same as W1 — context serves double duty |
| W7: No memory | Core result caching in Lambda memory | `maybe_recall_core_result()` check before invocation |
| W8: No verification | Self-verification protocol in Core prompts + confidence levels | Mandatory "Before Reporting" section in all Core prompts |
| W9: Prompt overhead | 300-600 token guideline + parser warning at 800 | Documentation + warning log at load time |
| W10: Invocation quality | Example-driven system prompt guidelines | Good vs. bad invocation examples in Main Agent prompt |

### T.4 Timeproofing Guarantees

TemDOS must remain valid as the world changes. Here are the assumptions that could break and how the design survives each:

| Assumption | If It Breaks | TemDOS Survives Because |
|------------|-------------|------------------------|
| LLMs cost ~$1-15/M tokens | Prices drop 10x | Shared budget still works; Cores just become cheaper. No hardcoded cost thresholds. |
| LLMs cost ~$1-15/M tokens | Prices rise 10x | Budget tracking surfaces cost to Main Agent; it naturally invokes fewer Cores. System adapts without code change. |
| Context windows are ~200K | Windows grow to 2M+ | Core isolation still valuable for focus (not just token limits). A 2M-context agent is still distracted by 50 file reads. |
| Models sometimes hallucinate | Models become near-perfect | Self-verification protocol becomes redundant but harmless. No code to remove. |
| Tool calling is imperfect | Tool calling becomes native/perfect | Core runtime's tool loop works better. No fallback code needed. |
| Users want 5-10 Cores | Users want 100+ Cores | Registry is HashMap-based, O(1) lookup. No hardcoded limits on Core count. |
| One provider per agent | Multi-provider routing | Core's `model_preference` field already supports routing. Add provider field when needed. |
| Cores are stateless | Users want persistent Cores | Core result caching (W7 fix) provides pseudo-persistence without changing the Core runtime. If true persistence is needed later, it's an additive change (new field in CoreDefinition, optional state parameter). |

### T.5 The One Invariant

Every design decision in TemDOS must preserve this invariant:

> **The Main Agent is the sole decision-maker. Cores inform. Cores never steer.**

If a Core could invoke another Core → Cores steer (they make delegation decisions).
If a Core could talk to the user → Cores steer (they control the conversation).
If a Core had persistent state → Cores accumulate influence across invocations.
If a Core could modify the Main Agent's context → Cores directly affect decisions.

None of these are allowed. The Main Agent receives Core output as data, evaluates it with its own judgment, and decides what to do. This is what makes TemDOS safe, predictable, and trustworthy.

---

## Part I: Research — Why Cores Beat the Current Standard

### 1.1 The skill.md Problem

Current agentic systems (Claude Code, OpenClaw) use skill.md files — static markdown documents injected into the main agent's system prompt. Three structural limitations:

1. **Context pollution**: A 2,000-token skill injected across 10 tool rounds consumes 20,000 tokens from the main agent's context budget. The agent carries research instructions even when doing unrelated work.

2. **No execution isolation**: The main agent follows skill instructions using its own tool calls, interleaving research with the actual task. A 15-file architecture scan fills the context with raw file contents, pushing out conversation history.

3. **No parallelism**: Skills execute sequentially — the agent can follow one set of instructions at a time.

### 1.2 How Cores Solve Each Problem

| Problem | skill.md | TemDOS |
|---------|----------|-------------|
| Context pollution | Skill text (S tokens) x R rounds = S*R tokens in main context | Main agent sees only the invocation (~50 tokens) + answer (~500 tokens). Internal core work is invisible. |
| Execution isolation | Main agent does the research, interleaving with its task | Core runs in its own LLM loop with its own SessionContext. Main agent's history is untouched. |
| Parallelism | Sequential only | Multiple `invoke_core` calls in a single response → `execute_tools_parallel()` runs them concurrently |

### 1.3 How Cores Differ from Existing Multi-Agent Systems

| | skill.md (Claude/OpenClaw) | Agent tool (Claude Code) | Multi-agent (CrewAI/AutoGen) | **Cores (TEMM1E)** |
|---|---|---|---|---|
| Execution | Context injection | Ad-hoc subagent | Agents converse | **Pre-defined specialist, tool interface** |
| Recursion | N/A | Agents spawn agents | Agents call agents | **Flat — structurally impossible** |
| Context | Shared (pollutes main) | Isolated | Isolated | **Isolated** |
| Specialization | Prompt-dependent | Written per-invocation | Role-defined, mutable | **Fixed, versioned, tested** |
| Budget | Main agent's | Unbounded per-agent | Per-agent | **Shared atomic pool** |
| Parallelism | No | Yes but ad-hoc | Yes but recursive risk | **Yes, structurally safe** |

### 1.4 Honest Disadvantages

1. **Cost multiplier**: Each Core is a full sub-agent loop — a single invocation may make many LLM API calls (one per tool round). An 8-round Core invocation is 8 provider calls with their own context windows. Simple tasks that the Main Agent can handle in 1-2 tool calls are strictly cheaper done inline.
2. **Cold start**: Cores are stateless — they rebuild understanding from scratch each invocation. The main agent's accumulated context is not available.
3. **No conversation context**: The core doesn't know the user's preferences, prior conversation, or ongoing task details beyond the query string.
4. **Complexity**: New crate, new runtime, new tool registration. skill.md is literally a text file.

**Mitigation**: The main agent's system prompt includes guidelines for when to invoke cores (complex multi-step analysis) vs. doing work inline (simple 1-2 tool call tasks).

---

## Part II: Mathematical Rigor

### 2.1 Budget Boundedness

**Definitions:**
- B = user-configured budget limit (max_spend_usd)
- C_max = maximum cost of a single LLM API call
- N = maximum concurrent agents (1 main + K cores)
- For Claude Opus (200K context): C_max = (200,000 * $5 + 8,192 * $25) / 1,000,000 = $1.20

**Theorem**: Total spend never exceeds B + N * C_max.

**Proof**: BudgetTracker uses AtomicU64::fetch_add for recording and AtomicU64::load for checking. Each agent calls check_budget() before each provider call. Between check and record, exactly one call is made.

Worst case: all N agents pass check_budget() simultaneously (each sees cumulative < B), then each makes one call. Maximum overshoot = N * C_max.

After those calls, the next check_budget() call by any agent sees cumulative >= B and returns Err. No further calls are made.

With N=6 (1 main + 5 concurrent cores) and Opus pricing: overshoot <= 6 * $1.20 = $7.20. For a $10 budget, worst case total = $17.20.

**Note**: Relaxed ordering is correct because fetch_add is always atomic regardless of ordering. We do not need a compare-and-swap — the budget is a soft limit, and the bounded overshoot is acceptable.

### 2.2 Context Efficiency

**Skill approach**: S tokens of skill text consumed per round for R rounds = S * R tokens.

**Core approach**: Main agent context grows by ~50 tokens (tool_use block) + A tokens (tool_result/answer). Total: 50 + A per invocation, regardless of the core's internal round count.

**Savings ratio**: (50 + A) / (S * R)

For typical values (S=2,000 skill tokens, R=10 rounds, A=500 answer tokens):
- Savings = 550 / 20,000 = 2.75% of skill cost
- **97.25% context reduction**

### 2.3 Parallelism Speedup

For K independent cores with execution times T_1, T_2, ..., T_K:
- Serial: T_serial = T_1 + T_2 + ... + T_K
- Parallel: T_parallel = max(T_1, T_2, ..., T_K)
- Speedup = T_serial / T_parallel
- For K equal-complexity cores: speedup = K (linear)

Bounded by the executor's semaphore (default 5 concurrent) and provider rate limits.

### 2.4 Recursion Safety Proof

**Given**: Tool set T_main = {t_1, t_2, ..., t_n, invoke_core}. Core tool set T_core = T_main \ {invoke_core}.

**Claim**: No execution path exists where a core invokes another core.

**Proof by construction**:
1. `InvokeCoreTool.execute()` builds T_core by filtering: `self.tools.iter().filter(|t| t.name() != "invoke_core")`
2. CoreRuntime receives T_core and converts it to ToolDefinitions for the CompletionRequest
3. The LLM inside the core sees only T_core tools — invoke_core is absent from the schema
4. Even if the LLM hallucinated "invoke_core" as a tool name, `execute_tool()` searches T_core by name and returns `Temm1eError::Tool("Unknown tool: invoke_core")`

**Depth bound**: Let depth(main_agent) = 0. Invoking a core creates depth 1. Since T_core does not contain invoke_core, depth 2 is unreachable. Maximum depth = 1. QED.

---

## Part III: Architecture Design

### 3.1 Crate Structure

```
crates/temm1e-cores/
  Cargo.toml
  src/
    lib.rs              -- pub mod, re-exports, create_invoke_core_tool() factory
    definition.rs       -- CoreDefinition struct, YAML frontmatter parser
    registry.rs         -- CoreRegistry: load from ~/.temm1e/cores/ and <workspace>/cores/
    runtime.rs          -- CoreRuntime: simplified agent loop (the heart)
    invoke_tool.rs      -- InvokeCoreTool: Tool trait implementation
    types.rs            -- CoreResult, CoreInvocation structs
```

**Dependencies**: temm1e-core (traits, types), temm1e-agent (executor, budget). No circular dependency — temm1e-cores depends on both; main.rs depends on temm1e-cores for tool registration.

### 3.2 Core Definition Format

Location: `~/.temm1e/cores/` (global) and `<workspace>/cores/` (project-local)

```markdown
---
name: architecture
description: "Analyzes repository structure, dependency graphs, module coupling, and design patterns"
version: "1.0.0"
---

You are the Architecture Core, a specialist analyst attached to a TEMM1E agent.

<task> contains the specific analysis request from the main agent.

## Your Protocol
1. Read the task carefully
2. Use tools to explore relevant code (file_read, file_list, shell, git)
3. Synthesize findings into a structured answer
4. Be thorough but concise — the main agent needs actionable information

## Constraints
- You CANNOT invoke other cores
- Stay focused on the task. Do not wander into unrelated analysis
- Return your final answer as plain text
```

**Frontmatter schema**:
```rust
struct CoreFrontmatter {
    name: String,
    description: String,
    version: String,
    /// Optional temperature override. Defaults to 0.0 (deterministic).
    /// Creative cores use 0.7 for sampling variance.
    temperature: Option<f32>,
}
```

Body after closing `---` becomes the system prompt. `<task>` is replaced with the actual query at invocation time.

### 3.3 Core Runtime (The Heart)

A stripped-down agent loop with exactly the features it needs:

```
Budget check → Build request (system prompt + history + tool defs) → Provider call → Record usage → Parse response → If no tool calls: return answer → Execute tools → Append results → Loop
```

**What it has**: System prompt, tool loop, budget checking, tool execution, history management, simple context pruning.

**What it does NOT have**: Classification, blueprints, consciousness, social intelligence, streaming, prompted tool calling fallback, lambda memory, learning extraction, interrupt handling, task decomposition.

**Temperature**: 0.0 for deterministic specialist work.

**Context pruning**: Simple approach — keep system prompt + last N messages (where N is sized to fit model context). No priority-based budgeting needed because core conversations are short and focused.

### 3.4 InvokeCoreTool (Tool Trait Implementation)

```rust
struct InvokeCoreTool {
    registry: Arc<CoreRegistry>,
    provider: Arc<dyn Provider>,
    all_tools: Vec<Arc<dyn Tool>>,    // Full set from main agent
    budget: Arc<BudgetTracker>,
    model: String,
    model_pricing: ModelPricing,
    max_context_tokens: usize,
    workspace_path: PathBuf,
}
```

**execute() flow**:
1. Parse `core` name and `task` string from arguments
2. Look up core in registry → CoreDefinition
3. Filter tools: `all_tools.filter(|t| t.name() != "invoke_core")` → recursion prevention
4. Build system prompt: replace `<task>` placeholder with actual task
5. Construct CoreRuntime with filtered tools, shared provider, shared budget
6. `core_runtime.run(task, workspace_path).await` → CoreResult
7. Format output: include answer + cost/rounds metadata
8. Return `ToolOutput { content, is_error: false }`

**parameters_schema()**:
```json
{
  "type": "object",
  "properties": {
    "core": { "type": "string", "description": "Name of the core to invoke (e.g., 'architecture', 'security')" },
    "task": { "type": "string", "description": "The specific task or question for the core. Be detailed and specific." },
    "context": { "type": "string", "description": "Optional. Relevant context: conversation excerpts, previous Core findings, constraints, or pre-read file contents. Reduces Core's cold start." }
  },
  "required": ["core", "task"]
}
```

### 3.5 Budget Sharing — Arc-Wrap BudgetTracker

**Change in temm1e-agent/src/runtime.rs**:
```rust
// Before:
budget: BudgetTracker,

// After:
budget: Arc<BudgetTracker>,
```

All `self.budget.method()` call sites are unchanged — Arc<T> implements Deref<Target=T>, so method calls pass through. Construction changes from `BudgetTracker::new(max)` to `Arc::new(BudgetTracker::new(max))`.

Add accessor: `pub fn budget(&self) -> Arc<BudgetTracker>` for external consumers.

**Risk**: ZERO. BudgetTracker already uses AtomicU64 internally. All methods take `&self`. Arc just adds reference counting for sharing.

### 3.6 System Prompt Guidelines for Main Agent

Injected into the main agent's system prompt when cores are available:

```
## Specialist Cores

You have access to specialist cores via the `invoke_core` tool. Each core is an independent AI agent with full tool access that runs until completion.

### Available Cores
[dynamically generated from registry: name — description]

### When to Use Cores
- USE a core when a task requires deep, focused analysis that would take many tool rounds
- DO NOT use a core for simple tasks you can handle in 1-2 tool calls
- Invoke MULTIPLE cores in parallel when you need independent analyses

### How Cores Work
- Cores share your budget — they deduct from the same spending pool
- Cores have full tool access (file, shell, git, browser, etc.)
- Cores run in isolation — they have their own context, not yours
- Cores CANNOT call other cores
- Be specific in your task description — the core cannot ask you follow-up questions
```

### 3.7 Foundational Cores (8)

Aligned with benchmark targets: **Coding, Web Browsing, Full Computer Use, Deep Research, General Creativity.**

| Core | File | Benchmark Target | Purpose |
|------|------|-----------------|---------|
| **architecture** | `cores/architecture.md` | Coding | Maps repo structure, dependency graphs, module boundaries, crate coupling. Called before refactors, new features, or any code change that affects multiple files. Prevents the Main Agent from going in blind. |
| **code-review** | `cores/code-review.md` | Coding | Reviews code for correctness, performance, edge cases, error handling, idiomatic patterns. The Main Agent invokes this AFTER writing code to self-audit before presenting to the user. Catches bugs the Main Agent's generalist perspective misses. |
| **test** | `cores/test.md` | Coding | Generates comprehensive test suites. Reads existing test patterns and conventions, writes unit + integration + edge case tests. The Main Agent invokes this after implementation to verify correctness. |
| **debug** | `cores/debug.md` | Coding | Investigates bugs systematically. Given a failing test, error message, or unexpected behavior — traces execution paths, reads logs, instruments code, identifies root cause, proposes targeted fix. |
| **web** | `cores/web.md` | Web Browsing | Specialist for web tasks — navigating sites, extracting data, filling forms, comparing pages, monitoring changes. Knows browser tool patterns (Prowl blueprints, observation layers, OTK login). Focuses the Main Agent's browser usage into efficient, targeted operations. |
| **desktop** | `cores/desktop.md` | Full Computer Use | Specialist for desktop automation — screen reading via Gaze (SoM overlay + xcap), mouse/keyboard control via enigo, app interaction sequences. Knows how to build reliable click-type-verify loops, handle UI state transitions, and recover from visual mismatches. |
| **research** | `cores/research.md` | Deep Research | Deep investigation specialist. Multi-step research across codebase, documentation, and web. Synthesizes findings from many sources into structured, cited reports. The Main Agent invokes this for "understand X thoroughly" tasks instead of doing shallow searches itself. |
| **creative** | `cores/creative.md` | General Creativity | Ideation and creative problem-solving. Given constraints, generates novel approaches, alternative architectures, naming ideas, UX concepts, analogies, and unconventional solutions. Thinks laterally where the Main Agent thinks linearly. Temperature 0.7 (not 0.0 — creativity needs sampling variance). |

**Benchmark mapping:**
- **Coding**: architecture + code-review + test + debug (4 cores covering the full dev cycle: understand → implement → review → test → debug)
- **Web Browsing**: web (specialist in Prowl/browser tool patterns)
- **Full Computer Use**: desktop (specialist in Gaze/enigo desktop control)
- **Deep Research**: research (multi-source synthesis specialist)
- **General Creativity**: creative (lateral thinking with higher temperature)

### 3.8 Core Auto-Authoring (Emergent Cores)

Just as the Main Agent already authors new Blueprints after completing complex tasks (see `runtime.rs` lines 1414-1550, background spawn), it should also author new Cores when it detects a repeated specialist need.

**When to author a Core**: After completing a task where:
1. The Main Agent spent 5+ tool rounds doing research/analysis in a specific domain
2. That domain is not covered by an existing Core
3. The task was successful (stop_reason = end_turn, no errors)

This mirrors `should_create_blueprint()` logic — if the task was complex, used tools, and succeeded, the system captures the pattern.

**How it works**:
1. Post-task, the Main Agent evaluates: "Did I do specialist work that a Core could have done?"
2. If yes, it makes one LLM call with the task history, asking: "Write a Core definition (.md file) for a specialist that could handle this type of work"
3. The LLM generates frontmatter (name, description, version) + system prompt
4. The new Core is saved to `~/.temm1e/cores/` and hot-loaded into the registry

**The authoring prompt** extracts:
- What domain the work was in
- What tools were used and how
- What protocol the agent followed (the sequence of tool calls)
- What output format was most useful

**Versioning**: Auto-authored Cores start at version "0.1.0" (draft). The user can review, edit, and bump to "1.0.0" when satisfied. Draft cores are still invocable — they just have a version signal.

### 3.9 Parallel Post-Task Processing

Currently, Blueprint authoring runs as a single background `tokio::spawn` after task completion (runtime.rs ~line 1414). With Cores, we now have TWO post-task authoring processes plus learning extraction. These should all run in parallel:

**Current (sequential-ish):**
```
Task done → extract_learnings() → spawn(author_blueprint || refine_blueprint)
```

**New (fully parallel):**
```
Task done → tokio::join!(
    extract_learnings(history),
    maybe_author_blueprint(history, meta),
    maybe_author_core(history, meta, registry),
)
```

All three are independent:
- **Learning extraction**: Scans history for reusable insights → stores to memory
- **Blueprint authoring/refinement**: Evaluates if a blueprint should be created or updated
- **Core authoring**: Evaluates if a new specialist Core should be created

Each is a single LLM call (or skip if conditions not met). Running them in parallel via `tokio::join!` instead of sequentially saves 2-3 LLM call round-trips worth of latency.

**Implementation**: Refactor the current sequential post-task block in `runtime.rs` into a `post_task_processing()` async function that uses `tokio::join!` for all three operations. Each operation returns `Option<artifact>` — None if skipped, Some if produced. The caller logs what was created.

**Budget note**: Post-task LLM calls also deduct from the shared BudgetTracker. If the budget is exhausted, the authoring calls gracefully fail (return None) without affecting the user's response — the task is already complete.

---

## Part IV: Implementation Plan

### Step 1: Arc-Wrap BudgetTracker
**Files**: `crates/temm1e-agent/src/runtime.rs`, `crates/temm1e-agent/src/budget.rs`
- Change `budget: BudgetTracker` → `budget: Arc<BudgetTracker>` in AgentRuntime
- Update constructors: `new()`, `with_limits()`
- Add `pub fn budget(&self) -> Arc<BudgetTracker>` accessor
- All `self.budget.` call sites unchanged (Deref)
- Run full test suite to verify zero regression

### Step 2: Create `crates/temm1e-cores/` Skeleton
- Cargo.toml with workspace dependencies
- Add to workspace members in root Cargo.toml
- Add feature flag `cores` (default-enabled)
- src/lib.rs with mod declarations
- src/types.rs with CoreResult struct

### Step 3: CoreDefinition + Parser (`definition.rs`)
- Reuse YAML frontmatter parsing pattern from `temm1e-skills/src/lib.rs`
- CoreFrontmatter: name, description, version
- Parse body as system prompt text
- Unit tests: valid parse, missing fields, malformed YAML

### Step 4: CoreRegistry (`registry.rs`)
- Load from `~/.temm1e/cores/` and `<workspace>/cores/`
- Index by name (HashMap<String, CoreDefinition>)
- `get_core(name) -> Option<&CoreDefinition>`
- `list_cores() -> Vec<&CoreDefinition>`
- Unit tests: load from temp dir, duplicate names, empty dir

### Step 5: CoreRuntime (`runtime.rs`) — Critical Path
- Simplified agent loop (see 3.3 above)
- Budget check before each provider call
- Tool execution via temm1e-agent's execute_tool()
- Simple history pruning (keep last N messages within context limit)
- Return CoreResult { output, rounds, input_tokens, output_tokens, cost_usd }
- Unit tests with mock provider: single round, multi-round tool use, budget exhaustion

### Step 6: InvokeCoreTool (`invoke_tool.rs`)
- Implement Tool trait for InvokeCoreTool
- Tool filtering (exclude invoke_core from core's tool set)
- Task substitution in system prompt
- Error handling: unknown core name, budget exceeded, provider error
- Unit tests: recursion prevention (invoke_core not in filtered tools), unknown core error

### Step 7: Wire into main.rs
- Load CoreRegistry after tools are created
- Create InvokeCoreTool with shared provider, budget, tools
- Push to tools vec before constructing AgentRuntime
- Feature-gated behind `#[cfg(feature = "cores")]`

### Step 8: Write Foundational Core Definitions (8 cores)
- Create `cores/` directory with 8 `.md` files: architecture, code-review, test, debug, web, desktop, research, creative
- Each core has: focused system prompt, domain protocol, output format guidance, temperature override where needed (creative: 0.7)
- Ship in repo for all users; also copied to `~/.temm1e/cores/` on first run
- Benchmark-aligned: Coding (4), Web Browsing (1), Computer Use (1), Deep Research (1), Creativity (1)

### Step 9: System Prompt Integration
- Add core usage guidelines to system prompt builder in context.rs
- Dynamically list available cores from registry
- Only inject when cores are available (registry non-empty)

### Step 10: Weakness Countermeasures
- **W1/W6 context parameter**: Already in schema (Step 6). Core runtime substitutes `<context>` placeholder.
- **W2 cost visibility**: Track cumulative Core spend per session in a `CoreSpendTracker` (simple AtomicU64). Inject "Core spend: $X.XX" into Main Agent system prompt via context builder.
- **W4 progress notifications**: Pass `Option<StreamingNotifier>` to CoreRuntime. Emit checkpoint at each tool round: `"[{core_name}] Round {n}: {tool_name}..."`.
- **W5 performance tracking**: Add `CoreStats` struct (invocations, total_cost, avg_rounds) persisted to memory. Updated after each invocation.
- **W7 result caching**: After invocation, store `CoreCachedResult { core_name, task_hash, output, timestamp }` in memory. Before invocation, `maybe_recall_core_result()` checks cache — if hit, Main Agent can skip or pass as context.
- **W8 self-verification**: Mandatory "Before Reporting" section in all foundational Core prompts with confidence levels (HIGH/MEDIUM/LOW).
- **W9 prompt budget**: Parser warns at load time if Core prompt exceeds 800 tokens.
- **W10 invocation quality**: Good vs. bad invocation examples in Main Agent system prompt guidelines.

### Step 11: Core Auto-Authoring (`authoring.rs`)
- `should_author_core(history, meta, registry) -> bool` — true if 5+ research rounds in uncovered domain
- `build_core_authoring_prompt(history, meta) -> String` — extracts domain, protocol, output format
- `parse_authored_core(response) -> CoreDefinition` — parses LLM output into frontmatter + body
- Save to `~/.temm1e/cores/` with version "0.1.0"
- Hot-reload into registry via `registry.load_core(definition)`

### Step 11: Parallel Post-Task Processing
- Refactor post-task block in `runtime.rs` into `post_task_processing()` function
- Use `tokio::join!` for three parallel operations:
  1. `extract_learnings(history)` — existing, moved
  2. `maybe_author_blueprint(history, meta)` — existing, moved
  3. `maybe_author_core(history, meta, registry)` — new
- Each returns `Option<artifact>`, None if skipped
- All share the same BudgetTracker; graceful skip if budget exhausted
- Log what was produced

### Step 13: Tests
- Unit tests per module (steps 3-6, 10-11)
- Integration test: full core invocation through InvokeCoreTool
- Budget sharing test: concurrent cores deducting from same AtomicU64
- Recursion test: verify invoke_core absent from core tool set
- Context parameter test: verify `<context>` substitution in Core prompt
- Progress notification test: verify checkpoint emissions
- Result caching test: verify cache hit skips re-invocation
- Auto-authoring test: mock post-task authoring with threshold detection
- Parallel post-task test: verify all three operations run concurrently
- CLI chat test: manual multi-turn with core invocation

---

## Part V: Verification Plan

### Compilation Gates
```bash
cargo check --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo test --workspace
```

### Live CLI Chat Test (10 turns)
1. "Hello, what cores do you have available?"
2. "Use the architecture core to analyze the temm1e-providers crate structure"
3. "What did the architecture core find?"
4. "Use the security core to audit temm1e-vault for credential handling issues"
5. "Now use architecture AND security cores in parallel to analyze temm1e-agent"
6. "What was the cost of those core invocations?"
7. "Use the debug core to investigate why test X fails" (pick a real test)
8. "Use the test core to generate tests for temm1e-cores/src/definition.rs"
9. "Do you remember what the architecture core said about temm1e-providers?"
10. "Summarize all core findings from this session"

Validates: core invocation, parallel execution, budget tracking, context isolation, memory recall of core outputs.

---

## Critical Files to Modify

| File | Change | Risk |
|------|--------|------|
| `crates/temm1e-agent/src/runtime.rs` | Arc-wrap BudgetTracker, add accessor | ZERO — Deref passthrough |
| `Cargo.toml` (root) | Add temm1e-cores to workspace members + features | ZERO — additive |
| `src/main.rs` | Register InvokeCoreTool in tools vec | LOW — additive, feature-gated |
| `crates/temm1e-agent/src/context.rs` | Inject core guidelines into system prompt | LOW — conditional injection |
| `crates/temm1e-agent/src/runtime.rs` (post-task) | Refactor post-task into parallel tokio::join! | LOW — same operations, parallel execution |
| `crates/temm1e-cores/*` (NEW) | Entire new crate (6 source files + authoring) | N/A — new code |
| `cores/*.md` (NEW) | 8 core definition files | N/A — new files |
