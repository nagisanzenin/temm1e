# TUI Classifier Hang — Root-Cause Report (2026-04-18)

## Symptom

TUI `process_message` hung indefinitely on short chat-style prompts
("hi", "briefly describe X", "what is 17*23"). Same binary, same Gemini
key, same Gemini model — CLI chat (`temm1e chat`) completed in ~12 s,
TUI smoke harness never returned.

## Investigation (seven instrumented probes + A/B provider test + body
bisection)

1. **Provider probes** — reqwest `.send().await` hangs specifically on
   the classifier request. Seven isolated probes (main task, spawned
   task, raw provider, worker task, second call, post-inbound) all
   returned in 1–5 s. Only the actual `classify_message` call from
   inside `process_message` stalled.
2. **Provider A/B** — swapped the active provider to Anthropic
   (`claude-haiku-4-5`) with no other changes. All five UX scenarios
   completed in 1–6 s, classifier returned correctly every time. Ruled
   out runtime, reqwest, tokio, TUI wiring — the stall is Gemini-side.
3. **Direct curl** — POSTed the *exact* JSON body our code emits to
   `/v1beta/models/gemini-3-flash-preview:generateContent`. Curl also
   hung past 60 s with zero bytes received. Our Rust code was innocent
   — the body itself stalls Gemini.
4. **Body bisection** — started from a minimal 773 B classifier payload
   (worked in 5.5 s), re-added pieces one at a time:
   - Full `## FIELD` definitions alone → works.
   - Full field definitions + two examples (`hello`, `fix main.rs`) →
     works.
   - Full field definitions + three examples (adds `build 5 independent
     Python modules with tests for each` → `difficulty=complex`) →
     **hangs past 25 s timeout with zero bytes.**
   - Same three examples with shorter surrounding field definitions →
     works. The trigger is the *combination* of a long, markdown-styled
     field-definition block (`##` headers, em-dashes, CAPITALIZED
     emphasis) with a `complex`-tagged parallel-work example.

## Root cause

Gemini 3 Flash Preview silently drops (never responds to) requests
whose system instruction matches a specific shape: long markdown-styled
definitions followed by "parallel/complex" classifier examples. The
server neither returns content nor rejects with an error code — the
connection stays open, yielding an infinite stall on the client.

This appears to be an internal Gemini content-policy gate misfiring on
the classifier prompt pattern. No public documentation we can cite.
Reproducible with a single `curl` command (see section 3 above).

## Fix (shipped in this change)

**`crates/temm1e-agent/src/llm_classifier.rs`** — rewrote
`CLASSIFY_BASE_PROMPT` to avoid the pattern that trips Gemini:
- Removed markdown `##` section headers.
- Removed em-dash (`—`) delimiters from field descriptions.
- Removed CAPITALIZED emphasis words in the `complex` definition.
- Shortened examples and moved them to an inline `"input" -> {json}`
  form.
- Kept all behavioral semantics (chat / order / stop; simple / standard
  / complex) and added extra chat examples so the classifier still
  resolves simple greetings, math questions, and fact questions as
  `chat` (not `order`).

Verified on both providers with five scenarios each:

| # | Prompt | Gemini wall | Anthropic wall | Classifier |
|---|--------|-------------|----------------|------------|
| 1 | "hi" | 11.1 s | 5.3 s | Chat/Simple ✓ |
| 2 | "what is 17 times 23" | 9.7 s | 4.8 s | Chat/Simple ✓ |
| 3 | "list files in /tmp" | 7.3 s | 1.2 s | Order/Standard ✓ |
| 4 | "briefly describe tokio" | 10.0 s | — | Chat/Simple ✓ |
| 5 | "read Cargo.toml …" | 7.3 s | — | Order/Standard ✓ |

All classifications correct. No timeouts. No rule-based fallback path
fired anywhere — the classifier LLM call returns normally on both
providers.

**`crates/temm1e-agent/src/runtime.rs`** — added a 30 s safety-net
timeout around `classify_message`. Under normal operation the classifier
returns in 1–6 s; this is a defensive ceiling so a future provider
incident never blocks the session indefinitely. On timeout we fall
through to the existing rule-based classifier and the main agentic LLM
call proceeds normally.

**`crates/temm1e-providers/src/gemini.rs`** — added reqwest client
`timeout(180)` + `connect_timeout(15)` so future Gemini hangs surface as
explicit `Provider` errors rather than stalls. Matches the defensive
timeout already present in Anthropic and OpenAI-compat providers.

## How to regression-test

Run the TUI smoke harness against Gemini 3 Flash Preview:
```
cargo run -p temm1e-tui --release --example tui_smoke -- --prompt "hi"
```
Expected: `SMOKE] wall<15s`, classifier logs `category=Chat`, no
`timeout` warnings.
