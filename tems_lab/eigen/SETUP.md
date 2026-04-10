# Eigen-Tune Setup Guide

> **Status: production beta (v4.9.0+).** Eigen-Tune is wired into the runtime as of branch `eigen-revisit`. Earlier versions had this guide describing a feature that produced no observable effect — that gap is now closed. See `tems_lab/eigen/INTEGRATION_PLAN.md` and `tems_lab/eigen/LOCAL_ROUTING_SAFETY.md` for the technical details.

Get self-tuning running on your machine. The data-collection path works on every supported platform; the training and serving path is platform-gated (see Compatibility below).

---

## What you're opting into

Eigen-Tune is **double opt-in** by design. You flip two switches:

1. **`enabled = true`** — turn the engine on. Conversations get captured to a local SQLite database, the state machine starts running, training cycles fire when conditions are met. **No user-facing change** — the cloud provider still serves every reply.
2. **`enable_local_routing = true`** — only after your tier passes the seven-gate safety chain (`LOCAL_ROUTING_SAFETY.md`), the agent actually serves you with the distilled local model. Until you flip this switch, the system runs in observation-only mode.

The two-switch design lets you watch the system work for several training cycles before letting it touch the user-facing reply path.

---

## Prerequisites

### 1. Ollama (required for training and serving)

Ollama runs your fine-tuned model locally. Without it, Eigen-Tune still collects training data and runs the state machine, but cannot train or serve models.

**macOS:**
```bash
brew install ollama
ollama serve
```

**Linux:**
```bash
curl -fsSL https://ollama.com/install.sh | sh
ollama serve
```

**Windows:**
Download from https://ollama.com/download

**Verify:**
```bash
curl http://localhost:11434/
# Should print: "Ollama is running"
```

### 2. A training backend (one of MLX or Unsloth)

You need ONE of these, depending on your hardware. Without one, training never runs but collection still works.

**Apple Silicon (M1/M2/M3/M4) → MLX:**
```bash
python3 -m pip install mlx-lm
python3 -c "import mlx_lm; print('MLX ready')"
```

**NVIDIA GPU (Linux, Windows-via-WSL2) → Unsloth:**
```bash
pip install unsloth trl datasets
python3 -c "import unsloth, trl, datasets; print('Unsloth ready')"
```

The Unsloth path uses a vendored Python wrapper at `scripts/eigentune_unsloth.py` that ships with the repo.

---

## Compatibility matrix

| Component | macOS-arm64 | macOS-x86 | Linux x86_64 | Windows x86_64 |
|---|---|---|---|---|
| Collection (always works) | ✓ | ✓ | ✓ | ✓ |
| Periodic state machine tick | ✓ | ✓ | ✓ | ✓ |
| `temm1e eigentune status` | ✓ | ✓ | ✓ | ✓ |
| MLX backend | ✓ (with `mlx-lm`) | ✗ | ✗ | ✗ |
| Unsloth backend | ⚠ slow (CPU/MPS only) | ⚠ slow (CPU only) | ✓ (CUDA) | ⚠ flaky — use WSL2 |
| Local routing | ✓ | ✓ | ✓ | ✓ |

If no training backend is detected, the trainer logs a clear "no backend" warning and the tier reverts to Collecting. **You never get a crash from a missing backend.**

---

## Enable Eigen-Tune (step 1: collection only)

Add to your `temm1e.toml`:

```toml
[eigentune]
enabled = true
# enable_local_routing = false   # default — leave it off until you've observed the pipeline work
```

Restart TEMM1E. The system now:
- Captures every (request, response) pair to `~/.temm1e/eigentune.db`
- Scores each pair via the Beta-Binomial quality model
- Runs the state machine tick every 60 seconds
- Triggers training when a tier accumulates ≥`min_pairs` (default 200) high-quality pairs with diversity entropy J ≥ 0.7
- Evaluates trained models against the eval holdout set
- Transitions through Collecting → Training → Evaluating → Shadowing

**You will not see any change in your replies.** Cloud still serves every request. To observe what's happening:

```bash
temm1e eigentune status
sqlite3 ~/.temm1e/eigentune.db "SELECT tier, state, pair_count FROM eigentune_tiers;"
tail -f /tmp/temm1e.log | grep Eigen-Tune
```

---

## Enable local routing (step 2: actually serve from the distilled model)

After you've observed at least one tier reach the `Graduated` state (with `temm1e eigentune status`), flip the second switch:

```toml
[eigentune]
enabled = true
enable_local_routing = true   # <-- second opt-in
```

Restart TEMM1E. The agent now:
- Calls `engine.route(complexity)` before each provider call
- For Graduated tiers, serves from the local model via Ollama's OpenAI-compat endpoint
- 30-second timeout on every local call with **automatic cloud fallback** on any failure
- Tool-bearing requests **always** route to cloud (small local models lack function calling — Gate 2)
- 5% of graduated calls are also sent to cloud for CUSUM drift detection
- If CUSUM detects drift, the tier auto-demotes back to Collecting

**Verify it's serving:**
```bash
tail -f /tmp/temm1e.log | grep "served from local model"
```

**Emergency stop a single tier:**
```bash
temm1e eigentune demote simple
# or via slash command:
/eigentune demote simple
```

**Stop everything immediately:** set `enable_local_routing = false` and restart. Cloud serves all requests immediately. (Set `enabled = false` to also stop collection.)

---

## Choose a base model

Eigen-Tune fine-tunes a base model. For MVP, the recommended models are restricted to families that support Ollama's `ADAPTER` directive (Llama, Mistral, Gemma) — this skips the GGUF conversion step.

### Recommended defaults (used when `base_model = "auto"`)

| Hardware | Simple tier | Standard tier | Complex tier |
|---|---|---|---|
| Apple Silicon ≤8 GB | `mlx-community/Llama-3.2-1B-Instruct-4bit` | (skip) | (skip) |
| Apple Silicon ≤16 GB | `mlx-community/Llama-3.2-1B-Instruct-4bit` | `mlx-community/Llama-3.2-3B-Instruct-4bit` | (skip) |
| Apple Silicon ≥16 GB | `mlx-community/Llama-3.2-1B-Instruct-4bit` | `mlx-community/Llama-3.2-3B-Instruct-4bit` | `mlx-community/Mistral-7B-Instruct-v0.3-4bit` |
| NVIDIA CUDA | `unsloth/Llama-3.2-1B-Instruct-bnb-4bit` | `unsloth/Llama-3.2-3B-Instruct-bnb-4bit` | `unsloth/Mistral-7B-Instruct-v0.3-bnb-4bit` |

### Override the model

```toml
[eigentune]
enabled = true
base_model = "unsloth/Mistral-7B-Instruct-v0.3-bnb-4bit"   # any HuggingFace repo ID
```

Or via CLI:
```bash
temm1e eigentune model mlx-community/Llama-3.2-3B-Instruct-4bit
```

(The CLI prints the suggested edit; you must restart the daemon for the change to take effect.)

---

## CLI subcommands

```bash
temm1e eigentune status     # status report + both opt-in switches + tier metrics
temm1e eigentune setup      # prerequisite check + install hints
temm1e eigentune model      # show base model + Ollama models + recommendations
temm1e eigentune model auto # show what auto would pick
temm1e eigentune tick       # manually advance the state machine
temm1e eigentune demote simple   # Gate 7 — emergency demote a tier
```

## Slash commands (in-chat)

```
/eigentune                  # = /eigentune status
/eigentune setup            # prerequisite check
/eigentune model            # show base model
/eigentune tick             # manually tick
/eigentune demote simple    # Gate 7 emergency stop
```

---

## What Happens Next (after collection is enabled)

1. **Collecting** — every conversation produces training pairs, automatically scored. State: `○ collecting`.
2. **Training** — when ≥`min_pairs` quality pairs accumulate AND diversity J ≥ 0.7, training fires automatically. The trainer subprocess runs in a child task so it doesn't block other operations.
3. **Evaluating** — the trained model is run against the eval holdout set; Wilson 99% lower bound on accuracy is computed.
4. **Shadowing** — if Wilson lower ≥ `graduation_accuracy` (default 0.95), the tier transitions. SPRT begins accumulating evidence. State: `◐ shadowing`.
5. **Graduated** — when SPRT accepts H₁ (default p₁=0.95), the tier graduates. CUSUM monitoring starts. State: `● graduated`.
6. **Local routing** — if `enable_local_routing = true`, the next request for that tier is served by the local model. Cloud fallback on any failure.
7. **Continuous monitoring** — CUSUM watches for drift. On alarm, the tier auto-demotes to Collecting and the system retrains later.

---

## Inspecting state directly

```bash
# Total pairs collected per tier
sqlite3 ~/.temm1e/eigentune.db \
  "SELECT complexity, COUNT(*), AVG(quality_score) FROM eigentune_pairs GROUP BY complexity;"

# Training run history
sqlite3 ~/.temm1e/eigentune.db \
  "SELECT id, status, base_model, train_loss, started_at, completed_at FROM eigentune_runs ORDER BY started_at DESC;"

# Tier states
sqlite3 ~/.temm1e/eigentune.db \
  "SELECT tier, state, pair_count, eval_accuracy, eval_n, sprt_lambda, cusum_s, serving_run_id FROM eigentune_tiers;"
```

Captured data lives in `~/.temm1e/eigentune.db` and never leaves your machine. To delete and reset: `rm ~/.temm1e/eigentune.db` (after setting `enabled = false` and restarting).

---

## Troubleshooting

**"No training backend available"**
- Install MLX (Apple Silicon) or Unsloth (CUDA Linux)
- Eigen-Tune still collects data without a training backend

**"Ollama not running"**
- Run `ollama serve` in a separate terminal
- Or set up as a system service: `brew services start ollama` (macOS)

**Tier stuck in Training for >1 hour**
- The trainer probably crashed. The state machine has a recovery path that auto-reverts the tier to Collecting after 1 hour. Check `/tmp/temm1e.log` for the trainer error.

**`eval_accuracy` keeps failing the Wilson gate**
- The base model may be too small for the complexity tier. Try a larger model.
- The training data may be too sparse. Lower `min_pairs` is NOT the fix — let more data accumulate.
- Check the eval holdout: `sqlite3 ~/.temm1e/eigentune.db "SELECT COUNT(*) FROM eigentune_pairs WHERE is_eval_holdout = 1;"`

**Local routing fires but the answers are bad**
- The CUSUM monitor will catch this within ~50 calls and auto-demote. If you can't wait: `temm1e eigentune demote <tier>`.
- Set `enable_local_routing = false` and restart to halt all local routing immediately.
- File an issue with the conversation transcript for tuning.

---

## What Eigen-Tune Does NOT Do

- Does NOT send your data anywhere — all training and serving is local
- Does NOT require GPU for data collection — only for training
- Does NOT modify existing conversations or provider behavior unless `enable_local_routing = true`
- Does NOT cost any LLM API money (the `teacher_enabled` opt-in is the only exception)
- Does NOT replace your cloud provider — it serves only Graduated tiers, and only after passing the seven-gate safety chain
- Does NOT route tool-bearing requests to local (Gate 2 — small local models lack function calling)
- Does NOT support model families outside Llama/Mistral/Gemma in MVP (Ollama's ADAPTER directive support — see INTEGRATION_PLAN §A4)

---

## Further reading

- `tems_lab/eigen/LOCAL_ROUTING_SAFETY.md` — the seven-gate safety chain protecting local serving
- `tems_lab/eigen/INTEGRATION_PLAN.md` — full implementation plan with risk analysis
- `tems_lab/eigen/CODE_ANCHORS.md` — verified file:line citations for the wiring
- `tems_lab/eigen/DESIGN.md` — original architectural design
- `tems_lab/eigen/RESEARCH_PAPER.md` — statistical machinery (Wilson, SPRT, CUSUM)
