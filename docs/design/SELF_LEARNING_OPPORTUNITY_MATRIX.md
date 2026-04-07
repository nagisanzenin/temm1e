# Self-Learning Opportunity Matrix — v4.6.0 Planning

> Deep scan of all TEMM1E subsystems for residual data that could feed back into future execution.
> Every row is an opportunity to close a learning loop using the V(a,t) = Q x R x U framework.

---

## Rating Scale

| Dimension | Low | Medium | High |
|-----------|-----|--------|------|
| **Timeproof** | Hardcoded ceiling — won't improve with better models | Partially timeproof — LLM decides some parts, code decides others | Fully timeproof — smarter model = smarter decisions, zero code changes |
| **User Value** | Developer convenience, marginal improvement | Noticeable quality or cost improvement for active users | Transformative — changes what the agent can do or how reliably it operates |
| **Complexity** | < 1 day, few files, additive only | 2-5 days, multiple crates, some refactoring | 1-2 weeks, architectural changes, new subsystems |
| **Availability** | Brand new — no existing infrastructure | Partial — structs/tables exist but unused or incomplete | Ready — infrastructure exists, just needs wiring |

---

## The Matrix

### Tier 1 — High Value, Infrastructure Ready

| # | Opportunity | Subsystem | Timeproof | User Value | Complexity | Availability | Description |
|---|------------|-----------|:---------:|:----------:|:----------:|:------------:|-------------|
| **1** | **Core stats activation** | temm1e-cores | High | High | Low | **Ready** — `CoreStats` struct exists with `record_success()`, `record_failure()`, tests passing, exported but never called | Wire CoreStats into invoke_tool.rs after execution. Track success/failure/rounds/cost per core. Inject stats into core selection: "architecture core: 85% success, avg 4 rounds". Retire cores below 25% success. |
| **2** | **Skill usage tracking** | temm1e-skills | High | High | Low | **Partial** — `Skill` struct has no usage fields, but `use_skill` tool in skill_invoke.rs has the invocation point ready | Add `invocation_count`, `last_invoked_at` to skill metadata. Track in skill_invoke.rs on each invocation. Boost high-adoption skills in matching. Auto-archive unused skills after 90 days. |
| **3** | **MCP tool quality** | temm1e-mcp | High | Medium | Low | **Partial** — `McpToolResult` has `is_error` flag, client tracks server info, but no aggregation | Aggregate success/failure per (server, tool). Compute reliability score. Implement circuit breaker: >30% error rate for 10 min → temp blacklist. Prefer reliable tools when duplicates exist across servers. |
| **4** | **Classification accuracy feedback** | temm1e-agent | High | High | Medium | **Partial** — classifier returns category/difficulty, runtime tracks rounds/tools/cost, but no validation loop | After task completion, compare predicted difficulty vs actual resource usage. If "Simple" task used 10+ rounds → misclassification. Store accuracy per (category, difficulty) pair. Feed back to classifier prompt: "Messages about X are usually Complex, not Simple." |
| **5** | **Tool reliability per task type** | temm1e-agent | High | High | Medium | **Partial** — `FailureTracker` exists per-session, `ToolOutput.is_error` captured, but no cross-session persistence | Persist tool success/failure by (tool_name, task_type) across sessions. Build reliability scores. Inject into system prompt: "shell: 95% success on file ops, 70% on network ops". Agent naturally avoids unreliable tool paths. |

### Tier 2 — High Value, Needs Some Building

| # | Opportunity | Subsystem | Timeproof | User Value | Complexity | Availability | Description |
|---|------------|-----------|:---------:|:----------:|:----------:|:------------:|-------------|
| **6** | **Consciousness injection efficacy** | temm1e-agent | High | High | Medium | **Partial** — consciousness engine captures rich `TurnObservation` (21 fields), `session_notes` accumulate, but insights are single-use (consumed after 1 turn) and never validated | Tag each consciousness intervention with unique ID. After next turn, compute delta: fewer tokens? fewer retries? better outcome? Build Beta posterior for intervention quality. Consciousness learns which types of whispers actually help. |
| **7** | **Cost prediction per task type** | temm1e-agent | Medium | High | Medium | **Partial** — `BudgetTracker` has cumulative totals, `SqliteUsageStore` persists per-turn costs with provider+model, but no per-task-type breakdown | Extend usage_log with `task_type` and `difficulty` columns. After N tasks, compute median cost per (task_type, difficulty, model). Before new task, predict: "This Complex deployment will cost ~$0.15". Enable cost-aware provider selection. |
| **8** | **Hive decomposition feedback** | temm1e-hive | High | High | High | **Partial** — blackboard stores `estimated_tokens` and `actual_tokens` per task, order tracks `completed_count`, but no post-order analysis compares estimates to actuals | After swarm order completes, compute: actual speedup vs estimated, token accuracy per subtask, DAG constraint quality (over/under-constrained). Feed back to Queen: "Your deploy-app decompositions overestimate tokens by 40%". Build task_type priors. |
| **9** | **Adaptive skull allocation** | temm1e-agent | Medium | Medium | Medium | **Partial** — context.rs logs allocation at debug level (system, tools, blueprint, lambda, history tokens), but fractions are hardcoded (MEMORY=15%, LEARNING=5%, BLUEPRINT=10%) | Track allocation + task outcome. Correlate: "Tasks with 20% lambda_memory succeed 95% vs 10% lambda at 85%". Shift fractions based on historical performance. Per-model adaptation (small models need different allocations than large). |
| **10** | **Prompt tier effectiveness** | temm1e-agent | High | Medium | Low | **Partial** — prompt_optimizer.rs selects tiers (Minimal/Basic/Standard/Full), logs token costs, but no outcome tracking | Track tier + task outcome. Build: "Standard tier: 92% success, Basic: 78%, Minimal: 65%". Inform tier selection: prefer Standard for Complex tasks even though it costs more tokens upfront. The total cost (prompt + rounds) is what matters. |

### Tier 3 — Medium Value or High Complexity

| # | Opportunity | Subsystem | Timeproof | User Value | Complexity | Availability | Description |
|---|------------|-----------|:---------:|:----------:|:----------:|:------------:|-------------|
| **11** | **Perpetuum schedule learning** | temm1e-perpetuum | High | Medium | High | **Partial** — `cognitive.review_schedule()` returns recommendations (keep/adjust, new_interval_secs) but they are generated and DISCARDED. Store exists with error_count, consecutive_errors. | Route schedule review recommendations back to cortex. Apply suggested interval changes. Measure: did the new interval catch events better? Build concern-type priors: "deploy monitors benefit from 5-min checks". |
| **12** | **Provider quality ranking** | temm1e-providers | Medium | High | Medium | **Partial** — usage_log has provider+model per turn, budget.rs has pricing, but no quality/latency comparison | Track per-model: latency percentiles, error rate, retry rate, cost efficiency. Build ranking: "For Simple tasks, Gemini Flash is 3x cheaper and 90% as good as Claude Sonnet". Inform provider selection or suggest to user. |
| **13** | **Memory retrieval quality** | temm1e-memory | High | Medium | Medium | **Partial** — `access_count` and `recall_boost` track recall frequency, FTS returns BM25 rank, but no utility signal (was the recalled memory actually used?) | After task completion, check if any recalled lambda memories appeared in the LLM's response. If yes → utility signal (+). If recalled but ignored → noise signal (-). Weight importance by actual utility, not just recall frequency. |
| **14** | **Skill authoring from patterns** | temm1e-skills | High | High | High | **New** — no infrastructure for runtime skill creation. Skills are currently filesystem-only markdown files. Would need: skill template, LLM authoring call, filesystem write, registry reload. | After a successful complex task that used a novel multi-tool procedure, author a skill (lighter than blueprint — just name + description + capabilities + instructions). Store to `~/.temm1e/skills/`. Requires: detection of "novel procedure", authoring LLM call, dedup against existing skills. |
| **15** | **Core prompt refinement** | temm1e-cores | High | Medium | High | **Partial** — CoreStats ready, CoreResult captures output quality, but no mechanism to modify core .md files or maintain version history | After N core executions, analyze: which prompts led to success? What instructions were followed vs ignored? Generate refined prompt via LLM. Store as new version alongside original. A/B test: serve both, compare outcomes. |
| **16** | **Gaze click accuracy** | temm1e-gaze | Medium | Medium | Medium | **Partial** — desktop_controller captures screenshots, click coordinates logged, but no before/after comparison | Post-click: wait N ms, screenshot, compare to pre-click state. Did target element change? Build per-element-type accuracy model. Learn optimal post-click wait times. |
| **17** | **Session pattern learning** | temm1e-gateway | Medium | Low | Medium | **New** — SessionManager has LRU eviction, SessionInfo has `last_active` and `message_count`, but no analytics or pattern extraction | Track: session duration, messages per session, tool usage patterns, time-of-day activity. Identify user archetypes: "follow-up users" (long sessions, many turns) vs "one-shot users". Adapt response verbosity and context loading. |
| **18** | **Tool co-occurrence patterns** | temm1e-agent | Medium | Low | Low | **Partial** — learning.rs already extracts tool sequences from history (`collect_tools_used`), task_type inferred from tool combo, but no co-occurrence analysis | Build co-occurrence matrix: P(tool_B | tool_A used). When agent uses shell, what tool comes next 70% of the time? Inject as hint: "After shell, consider file_read to verify". Lightweight — just a frequency table. |

---

## Priority Recommendations for v4.6.0

### Must-Do (Tier 1, items 1-3: Low complexity, Ready infrastructure)

These are the lowest-hanging fruit. Infrastructure already exists — just needs wiring.

| # | Opportunity | Est. Effort | Key Files |
|---|------------|-------------|-----------|
| 1 | Core stats activation | ~2 hours | types.rs (ready), invoke_tool.rs, runtime.rs |
| 2 | Skill usage tracking | ~3 hours | lib.rs (Skill struct), skill_invoke.rs |
| 3 | MCP tool quality | ~4 hours | client.rs, manager.rs, new metrics table |

### Should-Do (Tier 1-2, items 4-6: Medium complexity, High value)

These require some building but have the highest user-facing impact.

| # | Opportunity | Est. Effort | Key Files |
|---|------------|-------------|-----------|
| 4 | Classification accuracy feedback | ~1 day | llm_classifier.rs, runtime.rs, new persistence |
| 5 | Tool reliability per task type | ~1 day | self_correction.rs, runtime.rs, new persistence |
| 6 | Consciousness injection efficacy | ~1-2 days | consciousness_engine.rs, runtime.rs |

### Could-Do (Tier 2-3, items 7-12: Higher complexity, still valuable)

These are the strategic bets — harder to build but compound over time.

| # | Opportunity | Est. Effort | Key Files |
|---|------------|-------------|-----------|
| 7 | Cost prediction | ~1 day | budget.rs, sqlite_usage.rs |
| 8 | Hive decomposition feedback | ~2-3 days | queen.rs, blackboard.rs, new analysis |
| 10 | Prompt tier effectiveness | ~4 hours | prompt_optimizer.rs, runtime.rs |
| 14 | Skill authoring from patterns | ~3-5 days | New subsystem |

---

## How Each Opportunity Maps to V(a,t)

Every opportunity above produces artifacts that need the value function framework:

| Opportunity | Artifact Type | Q (Quality) | R (Recency) | U (Utility) | Drain |
|-------------|--------------|-------------|-------------|-------------|-------|
| Core stats | CoreStats record | success_rate | exp(-0.01 * days) | invocation_count | Retire <25% success |
| Skill usage | SkillUsage record | adoption_rate | exp(-0.01 * days) | invocation_count | Archive unused 90d |
| MCP tool quality | ToolQuality record | success_rate | exp(-0.02 * days) | times_called | Circuit breaker |
| Classification feedback | AccuracyRecord | match_rate | exp(-0.02 * days) | classifications_counted | Decay old records |
| Tool reliability | ToolReliability record | success_rate per type | exp(-0.01 * days) | task_count | Decay old records |
| Consciousness efficacy | InterventionRecord | turn_delta quality | exp(-0.02 * days) | intervention_count | Prune ineffective |
| Cost prediction | CostHistogram | prediction_accuracy | exp(-0.02 * days) | predictions_made | Replace stale models |
| Hive feedback | DecompositionRecord | speedup_accuracy | exp(-0.01 * days) | orders_completed | Decay old priors |

**The pattern is the same every time:** capture the residual, score it with V(a,t), inject the highest-value artifacts back into context, and drain the rest. The unified framework scales.
