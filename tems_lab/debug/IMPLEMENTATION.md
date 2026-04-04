# Tem Debug — Implementation Guide

> Every file path, function signature, type definition, and integration point
> needed to implement Tem Debug without ambiguity.

---

## Phase 1: Layer 0 — Centralized Log File (~50 LOC)

### What changes

**File: `src/main.rs` (lines 1205-1230)**

Currently two subscriber paths (TUI and CLI). Both use `tracing_subscriber::fmt()` → stdout. Change both to multiplex stdout + file via `tracing_subscriber::layer`.

### New dependency

**File: `Cargo.toml` (root)**

```toml
[workspace.dependencies]
tracing-appender = "0.2"
```

**File: `crates/temm1e-observable/Cargo.toml`**

```toml
[dependencies]
tracing-appender = { workspace = true }
```

### New file

**File: `crates/temm1e-observable/src/file_logger.rs`**

```rust
//! Centralized file logger — daily rotating log at ~/.temm1e/logs/

use std::path::PathBuf;
use tracing_appender::rolling::{RollingFileAppender, Rotation};

/// Log directory: ~/.temm1e/logs/
pub fn log_dir() -> PathBuf {
    let base = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".temm1e")
        .join("logs");
    std::fs::create_dir_all(&base).ok();
    base
}

/// Create a rolling file appender.
/// Rotates daily. Files: temm1e.log, temm1e.log.2026-04-03, etc.
pub fn create_file_appender() -> RollingFileAppender {
    tracing_appender::rolling::daily(log_dir(), "temm1e.log")
}

/// Log file path for the current day (for user-facing messages).
pub fn current_log_path() -> PathBuf {
    log_dir().join("temm1e.log")
}
```

### Integration in main.rs

**Replace lines 1222-1230 (CLI mode):**

```rust
// BEFORE:
tracing_subscriber::fmt()
    .with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
    )
    .json()
    .init();

// AFTER:
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

let file_appender = temm1e_observable::file_logger::create_file_appender();
let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);

tracing_subscriber::registry()
    .with(env_filter)
    .with(
        tracing_subscriber::fmt::layer()
            .json()
    )
    .with(
        tracing_subscriber::fmt::layer()
            .json()
            .with_ansi(false)
            .with_writer(file_writer)
    )
    .init();

// IMPORTANT: _guard must be kept alive for the duration of the program.
// Move it to a scope that lives until shutdown (e.g., store in a variable
// that persists through main()).
```

**Replace lines 1205-1221 (TUI mode) similarly**, but keep the existing `tui.log` behavior and ADD the centralized log alongside it.

### Log retention

Add a cleanup function called on startup:

```rust
/// Delete log files older than `max_days` from ~/.temm1e/logs/
pub fn cleanup_old_logs(max_days: u32) {
    let dir = log_dir();
    let cutoff = chrono::Utc::now() - chrono::Duration::days(max_days as i64);
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    let modified: chrono::DateTime<chrono::Utc> = modified.into();
                    if modified < cutoff {
                        std::fs::remove_file(entry.path()).ok();
                    }
                }
            }
        }
    }
}
```

Call `cleanup_old_logs(7)` at startup in main.rs, after subscriber init.

### Config

```toml
# temm1e.toml
[observability]
log_level = "info"
log_file = true          # NEW: default true
log_retention_days = 7   # NEW: default 7
```

### Tests

**File: `crates/temm1e-observable/src/file_logger.rs` (at bottom)**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_dir_is_under_temm1e() {
        let dir = log_dir();
        assert!(dir.to_string_lossy().contains(".temm1e"));
        assert!(dir.to_string_lossy().ends_with("logs"));
    }

    #[test]
    fn current_log_path_ends_with_temm1e_log() {
        let path = current_log_path();
        assert!(path.to_string_lossy().ends_with("temm1e.log"));
    }

    #[test]
    fn create_file_appender_succeeds() {
        // Should not panic
        let _appender = create_file_appender();
    }
}
```

---

## Phase 2: GitHub PAT Support via /addkey (~30 LOC)

### Credential detection

**File: `crates/temm1e-vault/src/detector.rs` (lines 35-84, add to pattern list)**

```rust
// GitHub Personal Access Token (classic)
(r"ghp_[A-Za-z0-9]{36}", "github"),
// GitHub Fine-grained PAT
(r"github_pat_[A-Za-z0-9_]{82}", "github"),
```

**File: `crates/temm1e-core/src/config/credentials.rs`**

In `detect_api_key()`, add BEFORE the generic `sk-` pattern (around line 170):

```rust
if trimmed.starts_with("ghp_") || trimmed.starts_with("github_pat_") {
    return Some(DetectedCredential {
        provider: "github",
        api_key: trimmed.to_string(),
        base_url: None,
    });
}
```

In `normalize_provider_name()`, add:

```rust
"github" | "gh" => Some("github"),
```

### PAT validation

**File: `src/main.rs` (in the credential validation section, ~line 3560)**

GitHub is not an LLM provider — it doesn't go through `create_provider()`. Add a special case:

```rust
if cred.provider == "github" {
    // Validate GitHub PAT with GET /user
    let client = reqwest::Client::new();
    match client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", cred.api_key))
        .header("User-Agent", "TEMM1E")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            save_credentials("github", &cred.api_key, "github", None).await?;
            reply("GitHub connected! I can now report bugs I find in myself.");
        }
        Ok(resp) => {
            reply(&format!("GitHub PAT validation failed: {}", resp.status()));
        }
        Err(e) => {
            reply(&format!("GitHub API error: {}", e));
        }
    }
    continue; // Skip LLM provider reload
}
```

### Credential scrub update

**File: `crates/temm1e-tools/src/credential_scrub.rs` (lines 26-31)**

Already handles `ghp_` pattern:
```rust
// Existing: ghp_[a-zA-Z0-9]{36} → [REDACTED_KEY]
```

Verify `github_pat_` is also covered. If not, add:
```rust
(r"github_pat_[a-zA-Z0-9_]{20,}", "[REDACTED_KEY]"),
```

---

## Phase 3: Log Scanner + Error Grouping (~80 LOC)

### New file

**File: `crates/temm1e-perpetuum/src/log_scanner.rs`**

```rust
//! Log scanner — reads temm1e.log and groups errors by signature.

use std::collections::HashMap;
use std::path::Path;

/// A group of identical errors found in the log.
#[derive(Debug, Clone)]
pub struct ErrorGroup {
    /// Error signature: "{file}:{line}:{first_60_chars}"
    pub signature: String,
    /// The full first occurrence of the error message.
    pub message: String,
    /// Source file:line if available.
    pub location: Option<String>,
    /// Number of occurrences in the scan window.
    pub count: u32,
    /// Timestamps of occurrences (ISO 8601).
    pub timestamps: Vec<String>,
    /// The raw log lines (max 5, for context).
    pub sample_lines: Vec<String>,
}

/// Scan log file for ERROR/WARN/panic entries within the last `hours`.
pub fn scan_recent_errors(log_path: &Path, hours: u32) -> Vec<ErrorGroup> {
    let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours as i64);
    let mut groups: HashMap<String, ErrorGroup> = HashMap::new();

    let content = match std::fs::read_to_string(log_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    for line in content.lines() {
        // Parse JSON log line
        let parsed: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Check level
        let level = parsed.get("level").and_then(|l| l.as_str()).unwrap_or("");
        if level != "ERROR" && level != "WARN" {
            continue;
        }

        // Check timestamp within window
        let ts = parsed.get("timestamp").and_then(|t| t.as_str()).unwrap_or("");
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
            if dt < cutoff {
                continue; // Too old
            }
        }

        // Extract error info
        let message = parsed.get("fields")
            .and_then(|f| f.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_string();

        let location = parsed.get("fields")
            .and_then(|f| f.get("panic.location"))
            .and_then(|l| l.as_str())
            .map(String::from);

        // Build signature
        let loc_str = location.as_deref().unwrap_or("unknown");
        let msg_prefix: String = message.chars().take(60).collect();
        let signature = format!("{}:{}", loc_str, msg_prefix);

        // Group
        let group = groups.entry(signature.clone()).or_insert_with(|| ErrorGroup {
            signature,
            message: message.clone(),
            location: location.clone(),
            count: 0,
            timestamps: Vec::new(),
            sample_lines: Vec::new(),
        });
        group.count += 1;
        group.timestamps.push(ts.to_string());
        if group.sample_lines.len() < 5 {
            group.sample_lines.push(line.to_string());
        }
    }

    // Return groups sorted by count (most frequent first), filter count >= 2
    let mut result: Vec<ErrorGroup> = groups.into_values()
        .filter(|g| g.count >= 2)
        .collect();
    result.sort_by(|a, b| b.count.cmp(&a.count));
    result
}
```

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn scan_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        std::fs::write(&path, "").unwrap();
        let groups = scan_recent_errors(&path, 6);
        assert!(groups.is_empty());
    }

    #[test]
    fn scan_groups_by_signature() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        let now = chrono::Utc::now().to_rfc3339();
        let mut f = std::fs::File::create(&path).unwrap();
        for _ in 0..3 {
            writeln!(f, r#"{{"timestamp":"{}","level":"ERROR","fields":{{"message":"test error","panic.location":"src/foo.rs:42:1"}}}}"#, now).unwrap();
        }
        let groups = scan_recent_errors(&path, 6);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].count, 3);
    }

    #[test]
    fn scan_ignores_old_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        let old = (chrono::Utc::now() - chrono::Duration::hours(24)).to_rfc3339();
        let mut f = std::fs::File::create(&path).unwrap();
        for _ in 0..3 {
            writeln!(f, r#"{{"timestamp":"{}","level":"ERROR","fields":{{"message":"old error"}}}}"#, old).unwrap();
        }
        let groups = scan_recent_errors(&path, 6);
        assert!(groups.is_empty());
    }

    #[test]
    fn scan_filters_below_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        let now = chrono::Utc::now().to_rfc3339();
        let mut f = std::fs::File::create(&path).unwrap();
        // Only 1 occurrence — below threshold of 2
        writeln!(f, r#"{{"timestamp":"{}","level":"ERROR","fields":{{"message":"single error"}}}}"#, now).unwrap();
        let groups = scan_recent_errors(&path, 6);
        assert!(groups.is_empty());
    }
}
```

---

## Phase 4: LLM Triage (~40 LOC)

### Add to self_work.rs

**File: `crates/temm1e-perpetuum/src/self_work.rs`**

```rust
use crate::log_scanner::{self, ErrorGroup};

const BUG_TRIAGE_SYSTEM: &str = "You are reviewing error logs from TEMM1E, an AI agent runtime. \
    Classify each error into exactly one category: BUG (defect in TEMM1E code), \
    USER_ERROR (user misconfiguration), TRANSIENT (network/API temporary failure), \
    CONFIG (missing/invalid config). Respond with ONLY the category and one sentence.";

/// Triage result from LLM.
#[derive(Debug, Clone)]
pub struct TriageResult {
    pub error_group: ErrorGroup,
    pub category: String,      // "BUG", "USER_ERROR", "TRANSIENT", "CONFIG"
    pub explanation: String,
}

pub async fn triage_errors(
    errors: &[ErrorGroup],
    caller: &dyn LlmCaller,
) -> Vec<TriageResult> {
    let mut results = Vec::new();

    for error in errors {
        let prompt = format!(
            "Error: {}\nLocation: {}\nOccurrences: {} in last 6 hours",
            error.message,
            error.location.as_deref().unwrap_or("unknown"),
            error.count,
        );

        match caller.call(Some(BUG_TRIAGE_SYSTEM), &prompt).await {
            Ok(response) => {
                let text = response.trim().to_string();
                let category = if text.starts_with("BUG") {
                    "BUG"
                } else if text.starts_with("USER_ERROR") {
                    "USER_ERROR"
                } else if text.starts_with("TRANSIENT") {
                    "TRANSIENT"
                } else if text.starts_with("CONFIG") {
                    "CONFIG"
                } else {
                    "UNKNOWN"
                };
                results.push(TriageResult {
                    error_group: error.clone(),
                    category: category.to_string(),
                    explanation: text,
                });
            }
            Err(e) => {
                tracing::warn!(error = %e, "Bug triage LLM call failed");
            }
        }
    }

    results
}
```

---

## Phase 5: Credential Scrub Extension (~30 LOC)

### Path redaction

**File: `crates/temm1e-tools/src/credential_scrub.rs`**

Add a new function (does NOT modify existing `scrub()`):

```rust
/// Extended scrubbing for bug reports — strips paths with usernames.
pub fn scrub_for_report(text: &str, known_values: &[&str]) -> String {
    let mut result = scrub(text, known_values);

    // Redact home directory paths
    // /Users/<name>/... → ~/...
    // /home/<name>/... → ~/...
    let home_re = regex::Regex::new(r"(?i)/(?:Users|home)/[^/\s]+/").unwrap();
    result = home_re.replace_all(&result, "~/").to_string();

    // Redact Windows user paths
    // C:\Users\<name>\... → ~\...
    let win_re = regex::Regex::new(r"(?i)[A-Z]:\\Users\\[^\\]+\\").unwrap();
    result = win_re.replace_all(&result, r"~\").to_string();

    // Redact IP addresses
    let ip_re = regex::Regex::new(r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}").unwrap();
    result = ip_re.replace_all(&result, "[REDACTED_IP]").to_string();

    result
}
```

### Tests

```rust
#[test]
fn scrub_for_report_redacts_home_paths() {
    let text = "/Users/john/Documents/Github/skyclaw/src/main.rs:42";
    let result = scrub_for_report(text, &[]);
    assert_eq!(result, "~/Documents/Github/skyclaw/src/main.rs:42");
}

#[test]
fn scrub_for_report_redacts_ip() {
    let text = "Connected to 192.168.1.100:8080";
    let result = scrub_for_report(text, &[]);
    assert!(result.contains("[REDACTED_IP]"));
}
```

---

## Phase 6: GitHub Issue Creation + Dedup (~100 LOC)

### New file

**File: `crates/temm1e-perpetuum/src/bug_reporter.rs`**

```rust
//! Bug reporter — creates GitHub issues from triaged error groups.

use crate::self_work::TriageResult;
use temm1e_core::types::error::Temm1eError;

const GITHUB_API: &str = "https://api.github.com";
const REPO: &str = "temm1e-labs/temm1e";
const USER_AGENT: &str = "TEMM1E-BugReporter";

/// Check if an issue with this error signature already exists.
pub async fn is_duplicate(
    client: &reqwest::Client,
    token: &str,
    signature: &str,
) -> Result<bool, Temm1eError> {
    let query = format!(
        "repo:{} is:open label:auto-reported \"{}\"",
        REPO,
        &signature[..signature.len().min(60)]
    );
    let url = format!("{}/search/issues?q={}", GITHUB_API, urlencoding::encode(&query));

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| Temm1eError::Tool(format!("GitHub search failed: {}", e)))?;

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| Temm1eError::Tool(format!("GitHub search parse error: {}", e)))?;

    let count = body.get("total_count").and_then(|c| c.as_u64()).unwrap_or(0);
    Ok(count > 0)
}

/// Format an issue body from a triage result.
pub fn format_issue_body(triage: &TriageResult, version: &str, os_info: &str) -> String {
    let error = &triage.error_group;
    format!(
        "## [BUG] {}\n\n\
         **Auto-reported by Tem {} on {}**\n\n\
         ### Error\n```\n{}\n```\n\n\
         ### Location\n`{}`\n\n\
         ### Context\n\
         - Version: {}\n\
         - OS: {}\n\
         - Occurrences: {} in last 6 hours\n\n\
         ### Triage\n{}\n\n\
         ---\n\
         *This issue was automatically generated by Tem's self-diagnosis system.*\n\
         *User reviewed and approved before submission.*",
        error.message,
        version,
        chrono::Utc::now().format("%Y-%m-%d"),
        error.message,
        error.location.as_deref().unwrap_or("unknown"),
        version,
        os_info,
        error.count,
        triage.explanation,
    )
}

/// Create a GitHub issue.
pub async fn create_issue(
    client: &reqwest::Client,
    token: &str,
    title: &str,
    body: &str,
) -> Result<String, Temm1eError> {
    let url = format!("{}/repos/{}/issues", GITHUB_API, REPO);

    let payload = serde_json::json!({
        "title": title,
        "body": body,
        "labels": ["bug", "auto-reported"]
    });

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| Temm1eError::Tool(format!("GitHub issue creation failed: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(Temm1eError::Tool(format!(
            "GitHub issue creation failed ({}): {}",
            status, text
        )));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| Temm1eError::Tool(format!("GitHub response parse error: {}", e)))?;

    let url = body
        .get("html_url")
        .and_then(|u| u.as_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(url)
}
```

---

## Phase 7: Perpetuum Sleep Concern (~40 LOC)

### Update conscience.rs

**File: `crates/temm1e-perpetuum/src/conscience.rs` (line 44)**

```rust
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
        matches!(
            self,
            Self::FailureAnalysis | Self::LogIntrospection | Self::BugReview
        )
    }
}
```

### Update self_work.rs

**File: `crates/temm1e-perpetuum/src/self_work.rs`**

Add match arm in `execute_self_work()`:

```rust
SelfWorkKind::BugReview => review_bugs(store, caller).await,
```

Implement:

```rust
async fn review_bugs(
    store: &Arc<Store>,
    caller: Option<&Arc<dyn LlmCaller>>,
) -> Result<String, Temm1eError> {
    let caller = caller.ok_or_else(|| {
        Temm1eError::Config("BugReview requires LLM caller".to_string())
    })?;

    // Check if bug reporting is configured (GitHub PAT exists)
    let github_token = match load_github_token() {
        Some(t) => t,
        None => return Ok("BugReview: no GitHub token configured, skipping".to_string()),
    };

    // Check consent
    if !is_bug_report_consented() {
        return Ok("BugReview: user has not consented, skipping".to_string());
    }

    // Check rate limit (last report timestamp in store)
    if let Some(last) = store.get_note("bug_review_last_report").await? {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&last) {
            let elapsed = chrono::Utc::now() - dt;
            if elapsed < chrono::Duration::hours(6) {
                return Ok("BugReview: rate limited, skipping".to_string());
            }
        }
    }

    // Scan logs
    let log_path = temm1e_observable::file_logger::current_log_path();
    let errors = log_scanner::scan_recent_errors(&log_path, 6);

    if errors.is_empty() {
        return Ok("BugReview: no recurring errors found".to_string());
    }

    // Triage
    let triaged = triage_errors(&errors, caller.as_ref()).await;
    let bugs: Vec<_> = triaged.iter().filter(|t| t.category == "BUG").collect();

    if bugs.is_empty() {
        return Ok(format!("BugReview: {} errors found, 0 classified as bugs", triaged.len()));
    }

    // Process bugs (max 1 per review cycle)
    let bug = &bugs[0];
    let client = reqwest::Client::new();

    // Dedup
    if is_duplicate(&client, &github_token, &bug.error_group.signature).await? {
        return Ok("BugReview: bug already reported, skipping".to_string());
    }

    // Scrub
    let version = env!("CARGO_PKG_VERSION");
    let os_info = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
    let body = format_issue_body(bug, version, &os_info);
    let scrubbed_body = temm1e_tools::credential_scrub::scrub_for_report(&body, &[]);

    let title = format!("[BUG] {}", &bug.error_group.message[..bug.error_group.message.len().min(70)]);

    // Create issue
    let issue_url = create_issue(&client, &github_token, &title, &scrubbed_body).await?;

    // Record timestamp
    store.save_note("bug_review_last_report", &chrono::Utc::now().to_rfc3339()).await?;

    tracing::info!(url = %issue_url, "Bug reported to GitHub");
    Ok(format!("BugReview: reported {}", issue_url))
}
```

### Update cortex.rs

**File: `crates/temm1e-perpetuum/src/cortex.rs` (line 399)**

Add `BugReview` to the self-work rotation. The existing code cycles through kinds; add:

```rust
"bug_review" => SelfWorkKind::BugReview,
```

In `fire_self_work()` match arm.

---

## Phase 8: User Consent Flow (~50 LOC)

### Config

**File: `crates/temm1e-core/src/types/config.rs`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugReporterConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub consent_given: bool,
}

impl Default for BugReporterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            consent_given: false,
        }
    }
}
```

Add to `Temm1eConfig`:

```rust
#[serde(default)]
pub bug_reporter: BugReporterConfig,
```

### Consent check

```rust
fn is_bug_report_consented() -> bool {
    // Read from ~/.temm1e/temm1e.toml
    let config = load_config();
    config.bug_reporter.enabled && config.bug_reporter.consent_given
}
```

### First-time consent (in the channel message handler)

When BugReview finds a bug but consent is not given, it sends a message to the user's active chat:

```
I found a bug in myself during routine self-diagnosis.

Error: panic: byte index 200 is not a char boundary
Location: crates/temm1e-agent/src/context.rs:407
Occurred: 3 times in the last 6 hours

I can report this to my developers so they can fix it.
Here's exactly what I would send: [preview]

No API keys, messages, or personal data will be included.

Reply /bugreport yes to enable auto-reporting, or /bugreport no to disable.
```

The `/bugreport yes` command sets `consent_given = true` in config and retriggers the report.

---

## Dependency Summary

| Crate | New dependency | Why |
|---|---|---|
| temm1e-observable | `tracing-appender = "0.2"` | Rolling file appender |
| temm1e-perpetuum | `urlencoding = "2"` | URL-encode GitHub search queries |
| temm1e-perpetuum | `reqwest` (already exists) | GitHub API calls |

No new crates. No new binaries. Two new dependencies (both small, well-maintained).

---

## File Summary

| File | Action | LOC |
|---|---|---|
| `crates/temm1e-observable/src/file_logger.rs` | NEW | ~40 |
| `crates/temm1e-observable/src/lib.rs` | EDIT (add mod) | +2 |
| `crates/temm1e-perpetuum/src/log_scanner.rs` | NEW | ~80 |
| `crates/temm1e-perpetuum/src/bug_reporter.rs` | NEW | ~100 |
| `crates/temm1e-perpetuum/src/self_work.rs` | EDIT (add BugReview) | ~60 |
| `crates/temm1e-perpetuum/src/conscience.rs` | EDIT (add variant) | +5 |
| `crates/temm1e-perpetuum/src/cortex.rs` | EDIT (add match arm) | +3 |
| `crates/temm1e-perpetuum/src/lib.rs` | EDIT (add mod) | +2 |
| `crates/temm1e-tools/src/credential_scrub.rs` | EDIT (add scrub_for_report) | ~25 |
| `crates/temm1e-core/src/types/config.rs` | EDIT (add BugReporterConfig) | ~15 |
| `crates/temm1e-core/src/config/credentials.rs` | EDIT (add github detection) | ~10 |
| `crates/temm1e-vault/src/detector.rs` | EDIT (add github pattern) | +2 |
| `src/main.rs` | EDIT (file logger init + /addkey github + /bugreport) | ~40 |
| **Total** | **3 new + 10 edit** | **~420** |
