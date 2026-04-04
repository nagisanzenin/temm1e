# Tem Debug: Self-Diagnosing Bug Reporting for Autonomous AI Agents

> **Authors:** Quan Duong, Tem (TEMM1E Labs)
> **Date:** April 2026
> **Status:** Design complete. Pre-implementation.
> **Branch:** `tem-debug`

---

## Abstract

We present Tem Debug, a two-layer self-diagnosis system for TEMM1E — an autonomous AI agent runtime. Layer 0 provides centralized structured logging with rotation to `~/.temm1e/logs/`, solving the baseline observability gap where all tracing output was lost unless manually redirected. Layer 1 introduces autonomous bug reporting: during Perpetuum Sleep phase, the agent reviews its own error logs via LLM triage, sanitizes sensitive data, deduplicates against existing GitHub issues, and — with explicit user consent — creates structured bug reports on the project repository.

The system addresses three problems simultaneously: (1) non-developer users cannot self-debug, (2) the development team discovers bugs only through manual Discord/Telegram reports with insufficient context, and (3) no existing system enables an AI agent to self-diagnose failures and file bug reports about itself.

Prior art exists for each component individually — crash reporting (Sentry, Bugsnag), LLM-powered triage (Microsoft Triangle, openSUSE), and infrastructure self-healing (Azure VMware, AWS DevOps Agent). The integration of self-diagnosis + self-reporting + lifecycle-aware scheduling within a single AI agent is, to our knowledge, novel.

---

## 1. Introduction

### 1.1 The Observability Gap

TEMM1E is a cloud-native Rust AI agent runtime deployed by users ranging from developers to non-technical consumers. The current tracing architecture outputs structured JSON to stdout via `tracing_subscriber::fmt().json()` (main.rs:1222-1230). Unless the user manually redirects output (`> /tmp/temm1e.log 2>&1`), all diagnostic information is lost when the terminal closes.

This creates a support paradox: the users most likely to encounter bugs (non-developers using Telegram/WhatsApp) are the least equipped to collect diagnostic data. Bug reports arrive as "it stopped responding" with no stack traces, no error context, and no reproduction path.

### 1.2 The Self-Diagnosis Opportunity

TEMM1E already has the infrastructure for self-diagnosis:

1. **Panic recovery** — `catch_unwind` at 3 locations (Gateway worker main.rs:3821, outer handler main.rs:2501, CLI main.rs:5698) catches panics and converts them to error replies. The global panic hook (main.rs:1243-1262) routes all panics through `tracing::error!` with file:line location.

2. **Perpetuum lifecycle** — The Sleep phase (entered after 15 minutes idle) already runs self-work tasks: `FailureAnalysis` and `LogIntrospection` make LLM calls to review recent activity (self_work.rs:10-110). The concern system (`Cortex.create_concern()`) handles scheduling, firing, and one-shot cleanup.

3. **Credential scrubbing** — `credential_scrub.rs` already strips API keys (sk-ant-*, sk-or-*, AIzaSy*), auth headers, and known secret values from outbound text. This is the "last-line-of-defense filter" applied before every message to the user.

4. **Vault encryption** — ChaCha20-Poly1305 encryption (local.rs:127-152) with per-secret nonces, 32-byte random key at `~/.temm1e/vault.key` (0o600 permissions). The `/addkey` flow already supports secure credential ingestion via one-time-token encryption.

The missing piece is connecting these: route tracing output to a persistent file, scan that file during Sleep, triage via the existing LLM provider, scrub via the existing credential filter, and create a structured report.

### 1.3 Prior Art

**Crash reporting platforms (Sentry, Bugsnag, Crashlytics):** Client SDK captures events, POSTs to ingestion server, server groups by stack trace fingerprint. Sentry DSNs are write-only by design — safe to expose publicly. Self-hosted Sentry requires 20+ containers (Kafka, ClickHouse, PostgreSQL, Redis). The Rust ecosystem has `crashreport` and `crashlog` crates that hook `std::panic::set_hook` to generate GitHub issue URLs.

**GitHub automation (Dependabot, Renovate, CodeQL):** All use GitHub Apps with short-lived installation tokens. Fine-grained permissions: `Issues: write` + `Metadata: read`. Rate limit: 15K req/hr for Apps, 5K for PATs. Apps require server-side key management — not viable for self-hosted binaries.

**LLM-powered triage:** Microsoft Triangle System (ISSRE 2024) uses LLM agents for Azure incident triage. Log parsing accuracy reaches 0.96 with modern approaches. Key finding: LLMs excel at severity classification and summary generation but hallucinate root causes. Use for triage, not diagnosis.

**Privacy in automated reporting:** ChromeOS requires explicit opt-in during setup. Firefox crash reporter was flagged for GDPR violations (collecting before consent). Industry standard: proactive opt-in, preview before send, never include credentials or PII.

**Client-side tokens in open source:** The DogWifTool incident demonstrated that embedded tokens are extractable from distributed binaries regardless of obfuscation. Sentry's DSN model works because DSNs are write-only. GitHub PATs — even fine-grained — carry broader capabilities. **Recommendation: never ship tokens in open-source binaries.**

**Self-healing AI:** Azure VMware uses closed-loop control (detect → diagnose → act → verify). AWS DevOps Agent handles autonomous incident response. An AI agent filing bug reports about itself is a novel extension of this pattern.

---

## 2. Architecture

### 2.1 Layer 0: Centralized Log File

**Always on. Local only. Zero privacy risk.**

```
tracing_subscriber::fmt()
  ├── stdout layer (existing: JSON, env filter)
  └── file layer (NEW: ~/.temm1e/logs/temm1e.log)
       ├── Rolling: daily rotation
       ├── Retention: 7 days (configurable)
       └── Format: JSON (same as stdout)
```

Integration point: `src/main.rs:1222-1230` (CLI mode) and `src/main.rs:1205-1221` (TUI mode). Replace `tracing_subscriber::fmt().json().init()` with a layered subscriber that writes to both stdout and file.

The TUI mode already writes to `~/.temm1e/tui.log` (main.rs:1208-1213). Layer 0 generalizes this to all modes.

### 2.2 Layer 1: Auto Bug Reporter

**Opt-in. User-controlled. Runs during Perpetuum Sleep.**

```
Perpetuum Sleep Phase
  └── SelfWorkKind::BugReview
       ├── Read: ~/.temm1e/logs/temm1e.log (last 6 hours)
       ├── Filter: ERROR + WARN + panic lines
       ├── Group: by error signature (file:line + message prefix)
       ├── Triage: LLM call — "is this a real bug?"
       │    └── Categories: bug / user-error / config / transient
       ├── Scrub: credential_scrub::scrub() + path redaction
       ├── Dedup: GitHub Issues API search for same signature
       ├── Preview: show user the report before sending
       └── Create: GitHub Issues API POST
```

### 2.3 Authentication

Users provide a GitHub PAT via the existing `/addkey` flow:

```
User: /addkey github
Tem: Send me your GitHub Personal Access Token.
     Create one at github.com/settings/tokens with "public_repo" scope.
User: ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
Tem: [detects ghp_ prefix → "github"]
     [validates via GET /user → 200 OK]
     [saves to credentials.toml]
     GitHub connected! I can now report bugs I find in myself.
```

No bot token shipped. No server infrastructure. User controls their own auth.

---

## 3. Privacy and Security

### 3.1 What Gets Sent

| Field | Source | Example |
|---|---|---|
| Error message | Log line | `panic: byte index 200 is not a char boundary` |
| Location | Log line | `crates/temm1e-agent/src/context.rs:407` |
| Occurrence count | Log scanner | `3 times in last 6 hours` |
| TEMM1E version | Binary | `4.1.2` |
| OS + arch | System | `Darwin 23.6.0 (aarch64)` |
| Rust version | Build | `1.82` |
| Active provider | Config | `gemini` (name only, no keys) |
| Active channel | Config | `telegram` (name only, no tokens) |
| LLM triage summary | LLM call | `Category: Panic, Severity: High` |

### 3.2 What NEVER Gets Sent

| Data | Why | How enforced |
|---|---|---|
| API keys | Credential theft | `credential_scrub::scrub()` strips all key patterns |
| User messages | Privacy | Never included in log scan window |
| Conversation history | Privacy | Never included |
| Vault contents | Security | Vault is never opened by reporter |
| File paths with usernames | PII | Regex: `/Users/<name>/` → `~/` |
| Auth headers | Credential theft | Stripped by scrub patterns |
| .env values | Credential theft | Cross-referenced and redacted |
| Session IDs | Tracking | Stripped |

### 3.3 Consent Model

1. **Layer 0 (log file):** Always on. No consent needed — data stays local.
2. **Layer 1 (GitHub reporting):**
   - Requires GitHub PAT (user must explicitly add via `/addkey github`)
   - First bug detected: Tem sends preview + asks "Can I report this?"
   - User responds yes/no
   - Config: `[bug_reporter] enabled = true, consent_given = false`
   - User can disable permanently: `[bug_reporter] enabled = false`
   - Every report is shown to the user before submission

### 3.4 Rate Limiting

- Max 1 issue per 6 hours per installation
- Same error signature: only reported once (dedup against open issues)
- GitHub API rate: 5000 req/hr per PAT (we use ~3 per report: search + create + label)

---

## 4. Perpetuum Integration

### 4.1 New SelfWorkKind

```rust
// conscience.rs — add variant
pub enum SelfWorkKind {
    MemoryConsolidation,
    FailureAnalysis,
    LogIntrospection,
    SessionCleanup,
    BlueprintRefinement,
    BugReview,              // NEW
}

impl SelfWorkKind {
    pub fn uses_llm(&self) -> bool {
        matches!(self, Self::FailureAnalysis | Self::LogIntrospection | Self::BugReview)
    }
}
```

### 4.2 Sleep Phase Trigger

The existing Sleep transition in `cortex.rs` cycles through self-work kinds. BugReview is added to the rotation. When Sleep is entered and BugReview is selected:

1. `Cortex::fire_self_work()` matches `"bug_review"` → calls `self_work::review_bugs()`
2. `review_bugs()` reads log file, filters errors, groups by signature
3. For each new error group: LLM triage → scrub → dedup → preview → create
4. Concern is deleted after completion (one-shot)
5. `conscience.complete_self_work()` transitions back

### 4.3 LLM Triage Prompt

```
You are reviewing error logs from TEMM1E, an AI agent runtime.
Classify each error into exactly one category:

- BUG: A defect in TEMM1E code (panic, logic error, unhandled case)
- USER_ERROR: User misconfiguration or invalid input
- TRANSIENT: Temporary issue (network timeout, API rate limit, 500 error)
- CONFIG: Missing or invalid configuration

Error:
{error_text}

Location: {file}:{line}
Occurrences: {count} in last 6 hours

Respond with ONLY the category name and a one-sentence explanation.
```

Temperature: 0.2 (deterministic). No root cause analysis requested — raw facts only.

---

## 5. GitHub Integration

### 5.1 PAT Detection

Add to `crates/temm1e-vault/src/detector.rs`:

```rust
// GitHub PAT patterns
("ghp_[A-Za-z0-9]{36}", "github")    // Personal Access Token
("github_pat_[A-Za-z0-9_]{82}", "github") // Fine-grained PAT
```

Add to `crates/temm1e-core/src/config/credentials.rs`:

```rust
// In detect_api_key() — before generic sk- pattern
if trimmed.starts_with("ghp_") || trimmed.starts_with("github_pat_") {
    return Some(DetectedCredential {
        provider: "github",
        api_key: trimmed.to_string(),
        base_url: None,
    });
}

// In normalize_provider_name()
"github" | "gh" => Some("github"),
```

### 5.2 PAT Validation

```rust
// GET https://api.github.com/user
// Authorization: Bearer ghp_xxx
// Expected: 200 OK with { login: "username" }
```

No new provider crate needed. GitHub is not an LLM provider — it's a credential stored in credentials.toml, used only by the bug reporter.

### 5.3 Issue Creation

```rust
// POST https://api.github.com/repos/temm1e-labs/temm1e/issues
// Authorization: Bearer ghp_xxx
// Body: { title, body, labels: ["bug", "auto-reported"] }
```

### 5.4 Deduplication

```rust
// GET https://api.github.com/search/issues?q=repo:temm1e-labs/temm1e+is:open+label:auto-reported+"error_signature"
// If results > 0: skip (already reported)
```

Error signature: `{file}:{line}:{first_40_chars_of_message}`. Stable across runs, specific enough to avoid false dedup.

---

## 6. Risk Assessment

| Risk | Severity | Mitigation | Residual |
|---|---|---|---|
| Sensitive data in report | High | credential_scrub + path redaction + user preview | Low — multiple layers |
| Reporter bug causes crash loop | High | Reporter wrapped in catch_unwind. Failures logged locally, never trigger another report | Near-zero |
| Spam issues on GitHub | Medium | Rate limit (1/6hr) + dedup + user consent | Low |
| LLM hallucinates severity | Medium | LLM only classifies, never diagnoses. Raw facts in report body | Low |
| User doesn't have GitHub | Low | Layer 0 always works. Layer 1 is additive | Zero |
| GitHub API changes | Low | Standard REST API, stable since 2012 | Low |
| PAT scope too broad | Low | `public_repo` is minimum for issue creation on public repos. Cannot access private data | Low |
| GDPR compliance | Medium | Explicit opt-in, preview before send, disable config, no tracking | Low |

**Overall risk: LOW.** Layer 0 is zero-risk. Layer 1 has multiple safety gates (consent, scrubbing, preview, rate limit, dedup).

---

## 7. Novelty Assessment

| Capability | Prior art | Novel? |
|---|---|---|
| Centralized log file with rotation | Every production system | No |
| Structured crash capture | Sentry, Bugsnag, crashreport-rs | No |
| LLM-powered bug triage | Microsoft Triangle, openSUSE | Emerging |
| Auto-create GitHub issues | crashreport-rs (URL only) | Partially |
| AI agent self-diagnosing own failures | None found | **Yes** |
| AI agent filing bugs about itself | None found | **Yes** |
| Lifecycle-aware scheduling (Sleep → diagnose) | None found | **Yes** |
| Credential-scrubbed self-reporting | None found | **Yes** |

The individual components are established. The integration — an AI agent that detects its own failures during idle time, triages them with its own LLM, sanitizes them with its own credential scrubber, and files structured reports about itself — is genuinely new.

---

## 8. Success Criteria

| Criterion | Threshold | How to measure |
|---|---|---|
| Log file created on startup | 100% | `ls ~/.temm1e/logs/temm1e.log` after `temm1e start` |
| Log rotation works | Daily | Check log directory after 2+ days |
| Credential scrubbing completeness | 0 leaks | Inject known keys into test logs, verify scrubbed output |
| GitHub PAT /addkey flow | Works | Manual test: `/addkey github` → paste PAT → verify stored |
| Bug detection accuracy | ≥80% | Inject 10 known errors + 10 non-errors, check classification |
| Dedup effectiveness | 0 duplicates | Create same error 3x, verify only 1 issue created |
| User preview before send | 100% | Verify consent prompt appears on every report |
| Rate limiting | Max 1/6hr | Trigger 5 bugs in 1 hour, verify only 1 issue created |
| Reporter crash isolation | No cascade | Inject a panic in the reporter, verify system continues |

---

## 9. References

1. Sentry Architecture — develop.sentry.dev/application-architecture/overview/
2. Sentry DSN Security — sentry.zendesk.com/hc/en-us/articles/26741783759899
3. GitHub App Permissions — docs.github.com/en/rest/authentication/permissions-required-for-github-apps
4. Microsoft Triangle (Azure AIOps, ISSRE 2024) — microsoft.com/en-us/research/wp-content/uploads/2024/08/ISSRE24_LLM4triage.pdf
5. openSUSE LLM Bug Triage — news.opensuse.org/2025/11/19/hw-project-targets-bug-triage/
6. AWS Agentic DevOps Agent — aws.amazon.com/blogs/devops/leverage-agentic-ai-for-autonomous-incident-response/
7. ChromeOS Crash Reporter — chromium.googlesource.com/chromiumos/platform2/crash-reporter/README.md
8. Rust crashreport crate — github.com/ewpratten/crashreport-rs
9. Self-Healing Agent Pattern — dev.to/the_bookmaster/the-self-healing-agent-pattern
10. DogWifTool Token Compromise — (supply chain risk for embedded secrets)
