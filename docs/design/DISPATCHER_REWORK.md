# Dispatcher Rework — Chat Classification Removal & Future Direction

**Status**: Chat bypass removed in v5.3.5. Future rework proposed.
**Date**: 2026-04-17

## The Problem

Users reported that Tem "lies about its work" — responding with text claiming to have performed actions (read files, checked configs, ran commands) without ever calling any tools.

### Root cause

The V2 LLM classifier (`llm_classifier.rs`) categorizes every user message into one of three buckets:

| Category | What happens |
|----------|-------------|
| **Chat** | Returns immediately with the classifier's pre-generated text. **Zero tools.** |
| **Order** | Sends brief ack, enters the agentic tool loop with full tool access. |
| **Stop** | Returns ack, signals task cancellation. |

When the classifier misclassifies an actionable request as "Chat", the agent fabricates a response from the LLM's training data instead of using tools to get real answers. The user sees a confident, detailed reply — but none of it is grounded in actual execution.

### Why misclassification happens

The classifier prompt defines Chat as: *"the user is asking a question, greeting you, thanking you, or having a conversation."*

The phrase **"asking a question"** is too broad. Questions like:
- "What files are in this directory?"
- "How many tests does this project have?"
- "What does the config look like?"

...are questions that **require tool use** to answer honestly, but the classifier sees "asking a question" and routes them to the tool-less Chat path.

### The A/B test

| Prompt | Classifier said | Old behavior | New behavior |
|--------|----------------|-------------|-------------|
| "Hey, how are you?" | Chat | Early return, no tools | Entered loop, model chose no tools (correct) |
| "What files are in the current directory?" | Order | Used tools | Used tools (same) |
| "Can you check if there's a README.md here?" | Order | Used tools | Used tools (same) |
| "Read Cargo.toml and tell me the version" | Order | Used tools | Used tools (same) |
| "How many tests does this project have?" | **Chat** | **Fabricated answer** | Entered loop, tools available |

**Result**: Pure chat ("hey") still works — the model naturally responds without tools. Actionable questions no longer get fabricated answers.

**Cost tradeoff**: Pure chat messages now cost ~$0.015 instead of ~$0.001 (full system prompt + tool definitions are sent). Acceptable for correctness.

## The Fix (v5.3.5)

Removed the Chat early-return path in `runtime.rs`. Chat-classified messages now fall through to the agentic tool loop, identical to Order. The model decides whether to use tools — not the classifier.

```
// Before (v5.3.4 and earlier):
Chat → return immediately with classifier text (no tools)
Order → enter agentic loop (with tools)

// After (v5.3.5):
Chat → enter agentic loop (with tools, model decides)
Order → enter agentic loop (with tools)
```

## Redundancy in the Current Dispatcher

With Chat no longer short-circuiting, the three-way classification is effectively:

| Category | Current behavior | Unique value |
|----------|-----------------|-------------|
| **Chat** | Falls through to agentic loop (simple profile) | None — same as Order/Simple |
| **Order/Simple** | Falls through to agentic loop (simple profile) | None — same as Chat |
| **Order/Standard** | Falls through to agentic loop (standard profile) | Logged but max_iterations not enforced |
| **Order/Complex** | Routes to Hive swarm (if enabled) | **Only unique branch** |
| **Stop** | Returns immediately, signals cancellation | Unique |

The only classification that produces **materially different behavior** is:
1. **Complex + Hive enabled** → routes to swarm
2. **Stop** → cancels active task

Everything else enters the same agentic loop with the same tool access and the same `max_tool_rounds = 200` ceiling. The `ExecutionProfile.max_iterations` field (2/5/10) is logged but never enforced as a loop constraint.

## Future Rework Options

### Option 1: Simplify to two categories

Replace Chat/Order/Stop with:

| Category | Behavior |
|----------|---------|
| **Execute** | Enter agentic loop (default for everything) |
| **Stop** | Cancel active task |

Swarm routing moves from classifier to a separate heuristic (compound task detection already exists in the rule-based fallback). This eliminates the classifier as a correctness risk entirely.

**Pros**: Simplest, zero misclassification risk, classifier cost drops (simpler prompt).
**Cons**: Loses the "brief ack" UX for orders (user sees nothing until the first tool result).

### Option 2: Keep classifier for UX only

The classifier still runs, but its output is used **only for**:
- Generating the brief acknowledgment ("on it, reading the file")
- Difficulty estimation for telemetry/Eigen-Tune
- Swarm routing (Complex → Hive)

It **never** gates tool access. All messages enter the agentic loop regardless.

**Pros**: Best UX (immediate ack), zero correctness risk, telemetry preserved.
**Cons**: Classifier cost still paid on every message (~1K tokens).

### Option 3: Remove classifier entirely

No classification step. The first LLM call is the agentic call with full tools. The model generates its own acknowledgment as part of its response.

**Pros**: One fewer API call per message, simplest architecture.
**Cons**: No early ack (user waits for full response), no difficulty telemetry, swarm routing needs a different trigger.

### Recommendation

**Option 2** is the best path forward. It preserves the UX benefit (immediate ack) and telemetry, while eliminating the correctness risk that caused the lying behavior. The classifier becomes advisory — it can never prevent tools from being used.

The current v5.3.5 implementation is effectively Option 2 minus the early ack for Chat. A follow-up could add early ack back for Chat-classified messages (just like Order gets an ack) without re-introducing the early return.

## Appendix: ExecutionProfile dead code

The `ExecutionProfile` struct defines:
- `max_iterations: u32` (2/5/10) — **logged but never enforced**
- `skip_tool_loop: bool` — **never referenced**

These fields should either be wired into the runtime loop or removed to avoid confusion. The actual loop ceiling is `self.max_tool_rounds` (hardcoded 200).
