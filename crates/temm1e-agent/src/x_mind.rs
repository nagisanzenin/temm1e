//! X-Mind v2 — Consciousness-Orchestrated Subagent Architecture.
//!
//! X-Minds are goal-directed subagents invoked on demand by consciousness.
//! They produce 3-tier artifacts (title/description/content) that are persisted
//! to disk and selectively injected into the worker by consciousness.
//!
//! Consciousness is the boss. It reads the artifact manifest (Tier 1+2),
//! decides which artifacts to inject, and which minds to invoke for new analysis.

use serde::{Deserialize, Serialize};
use std::path::Path;

// ── Artifact (3-tier) ───────────────────────────────────────────────────

/// A 3-tier artifact produced by an X-Mind subagent.
///
/// - Tier 1 (title): 5-10 words, for manifest listing
/// - Tier 2 (description): 1-3 sentences, for consciousness to skim
/// - Tier 3 (content): full analysis, injected into worker when selected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// Unique identifier (e.g., "arch-kvstore-001").
    pub id: String,
    /// Which mind produced this.
    pub mind: String,
    /// Tier 1: short title (5-10 words).
    pub title: String,
    /// Tier 2: brief description (1-3 sentences, ~50 tokens).
    pub description: String,
    /// Tier 3: full analysis content (unbounded).
    pub content: String,
    /// Freeform tags for cross-cutting queries.
    pub tags: Vec<String>,
    /// Lifecycle scope.
    pub scope: ArtifactScope,
    /// Current lifecycle status.
    pub status: ArtifactStatus,
    /// When first created.
    pub created_at: String,
    /// When last updated.
    pub updated_at: String,
    /// Session that created this artifact.
    pub session_id: String,
    /// Turn number when created.
    pub turn_created: u32,
    /// Turn number when last updated.
    pub turn_updated: u32,
    /// How many times this artifact has been accessed (injected or read).
    pub access_count: u32,
    /// When last accessed.
    pub last_accessed: String,
    /// Estimated token count of Tier 3 content.
    pub token_estimate: usize,
}

/// Artifact lifecycle scope — determines retention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactScope {
    /// Relevant only to this tool loop. Auto-deleted when loop ends.
    Turn,
    /// Relevant to the current task. Kept until task changes.
    Task,
    /// Relevant to this conversation session.
    Session,
    /// Relevant across sessions. Persists indefinitely.
    Project,
}

/// Artifact lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactStatus {
    /// Available for injection.
    Active,
    /// Not updated recently — still usable but may need refresh.
    Stale,
    /// Moved to archive, not shown in default manifest.
    Archived,
}

impl Artifact {
    /// Estimate token count from content length (~4 chars per token).
    pub fn estimate_tokens(content: &str) -> usize {
        content.len() / 4 + 1
    }

    /// Format Tier 1 + Tier 2 for the manifest (what consciousness sees).
    pub fn manifest_entry(&self) -> String {
        format!(
            "[{}] {} | {}-scoped | {:?}\n  \"{}\"\n  {}\n  Tags: {} | ~{} tokens | accessed {}",
            self.id,
            self.mind,
            match self.scope {
                ArtifactScope::Turn => "turn",
                ArtifactScope::Task => "task",
                ArtifactScope::Session => "session",
                ArtifactScope::Project => "project",
            },
            self.status,
            self.title,
            self.description,
            if self.tags.is_empty() {
                "none".to_string()
            } else {
                self.tags.join(", ")
            },
            self.token_estimate,
            &self.last_accessed[..10], // date only
        )
    }

    /// Format Tier 3 for injection into worker prompt.
    pub fn injection_block(&self) -> String {
        format!(
            "{{{{x_mind:{}}}}}\n{}\n{{{{/x_mind:{}}}}}",
            self.id, self.content, self.id
        )
    }
}

// ── Manifest ────────────────────────────────────────────────────────────

/// The artifact manifest — a lightweight index that consciousness reads.
/// Contains only Tier 1 + Tier 2 of each active artifact.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactManifest {
    pub artifacts: Vec<Artifact>,
}

impl ArtifactManifest {
    /// Load manifest by scanning artifact directory.
    pub fn load(artifact_dir: &Path) -> Self {
        let artifacts_dir = artifact_dir.join("artifacts");
        let mut artifacts = Vec::new();

        let entries = match std::fs::read_dir(&artifacts_dir) {
            Ok(entries) => entries,
            Err(_) => return Self { artifacts },
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                match load_artifact_file(&path) {
                    Ok(artifact) => {
                        if artifact.status != ArtifactStatus::Archived {
                            artifacts.push(artifact);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            path = %path.display(),
                            error = %e,
                            "Failed to load artifact"
                        );
                    }
                }
            }
        }

        // Sort by access count (most accessed first)
        artifacts.sort_by(|a, b| b.access_count.cmp(&a.access_count));

        tracing::debug!(count = artifacts.len(), "Loaded artifact manifest");

        Self { artifacts }
    }

    /// Format the manifest for consciousness input (Tier 1 + Tier 2 only).
    pub fn format_for_consciousness(&self) -> String {
        if self.artifacts.is_empty() {
            return "Available artifacts: none".to_string();
        }

        let mut lines = vec![format!(
            "Available artifacts ({} active):",
            self.artifacts.len()
        )];
        for artifact in &self.artifacts {
            lines.push(format!("\n  {}", artifact.manifest_entry()));
        }
        lines.join("\n")
    }

    /// Get artifact by ID.
    pub fn get(&self, id: &str) -> Option<&Artifact> {
        self.artifacts.iter().find(|a| a.id == id)
    }

    /// Get mutable artifact by ID.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Artifact> {
        self.artifacts.iter_mut().find(|a| a.id == id)
    }

    /// Count active artifacts.
    pub fn active_count(&self) -> usize {
        self.artifacts
            .iter()
            .filter(|a| a.status == ArtifactStatus::Active)
            .count()
    }

    /// Count artifacts by mind.
    pub fn count_by_mind(&self, mind: &str) -> usize {
        self.artifacts.iter().filter(|a| a.mind == mind).count()
    }
}

// ── Artifact File I/O ───────────────────────────────────────────────────

/// Load an artifact from a markdown file with YAML frontmatter.
fn load_artifact_file(path: &Path) -> Result<Artifact, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;

    if let Some(rest) = content.strip_prefix("---\n") {
        if let Some(split_pos) = rest.find("\n---\n") {
            let frontmatter = &rest[..split_pos];
            let body = rest[split_pos + 5..].trim().to_string();

            let mut artifact: Artifact = serde_yaml::from_str(frontmatter)?;
            artifact.content = body;
            artifact.token_estimate = Artifact::estimate_tokens(&artifact.content);
            return Ok(artifact);
        }
    }

    Err("Invalid artifact format: missing YAML frontmatter".into())
}

/// Save an artifact to a markdown file with YAML frontmatter.
pub fn save_artifact(artifact_dir: &Path, artifact: &Artifact) -> Result<(), std::io::Error> {
    let dir = artifact_dir.join("artifacts");
    std::fs::create_dir_all(&dir)?;

    let filename = format!("{}.md", artifact.id);
    let path = dir.join(filename);

    // Build frontmatter (without content — content goes in body)
    let mut meta = artifact.clone();
    meta.content = String::new(); // Don't include content in frontmatter
    meta.token_estimate = Artifact::estimate_tokens(&artifact.content);

    let frontmatter = serde_yaml::to_string(&meta).map_err(std::io::Error::other)?;
    let file_content = format!("---\n{}---\n\n{}", frontmatter, artifact.content);

    std::fs::write(&path, file_content)?;
    tracing::debug!(
        id = %artifact.id,
        path = %path.display(),
        "Persisted artifact"
    );
    Ok(())
}

/// Archive an artifact (move to archive/ subdirectory).
pub fn archive_artifact(artifact_dir: &Path, id: &str) -> Result<(), std::io::Error> {
    let src = artifact_dir.join("artifacts").join(format!("{}.md", id));
    if !src.exists() {
        return Ok(());
    }
    let archive_dir = artifact_dir.join("archive");
    std::fs::create_dir_all(&archive_dir)?;
    let dst = archive_dir.join(format!("{}.md", id));
    std::fs::rename(&src, &dst)?;
    tracing::info!(id = %id, "Archived artifact");
    Ok(())
}

/// Delete an artifact permanently.
pub fn delete_artifact(artifact_dir: &Path, id: &str) -> Result<(), std::io::Error> {
    let path = artifact_dir.join("artifacts").join(format!("{}.md", id));
    if path.exists() {
        std::fs::remove_file(&path)?;
        tracing::info!(id = %id, "Deleted artifact");
    }
    Ok(())
}

/// Load full content of an artifact (Tier 3).
pub fn load_artifact_content(artifact_dir: &Path, id: &str) -> Option<String> {
    let path = artifact_dir.join("artifacts").join(format!("{}.md", id));
    load_artifact_file(&path).ok().map(|a| a.content)
}

/// Record an access (bump access_count and last_accessed).
pub fn record_access(artifact_dir: &Path, artifact: &mut Artifact) {
    artifact.access_count += 1;
    artifact.last_accessed = chrono::Utc::now().to_rfc3339();
    // Best-effort persist
    let _ = save_artifact(artifact_dir, artifact);
}

// ── Pruning ─────────────────────────────────────────────────────────────

/// Maximum active artifacts before pruning kicks in.
const MAX_ACTIVE_ARTIFACTS: usize = 30;
/// Maximum artifacts per mind.
const MAX_PER_MIND: usize = 10;
/// Sessions without access before auto-archive.
/// Sessions without access before auto-archive (used in future session tracking).
#[allow(dead_code)]
const STALE_SESSION_THRESHOLD: u32 = 5;

/// Prune the artifact manifest. Returns list of actions taken.
pub fn prune_manifest(
    artifact_dir: &Path,
    manifest: &mut ArtifactManifest,
    _current_session_count: u32,
) -> Vec<String> {
    let mut actions = Vec::new();

    // 1. Remove turn-scoped artifacts from previous turns
    let turn_scoped: Vec<String> = manifest
        .artifacts
        .iter()
        .filter(|a| a.scope == ArtifactScope::Turn)
        .map(|a| a.id.clone())
        .collect();
    for id in &turn_scoped {
        let _ = delete_artifact(artifact_dir, id);
        actions.push(format!("Deleted turn-scoped artifact: {}", id));
    }
    manifest
        .artifacts
        .retain(|a| a.scope != ArtifactScope::Turn);

    // 2. Mark stale artifacts (not accessed in STALE_SESSION_THRESHOLD sessions)
    // We approximate "sessions without access" by checking if the artifact
    // was last accessed more than STALE_SESSION_THRESHOLD sessions ago.
    // Since we don't track session counts globally, we use access_count as proxy.
    for artifact in &mut manifest.artifacts {
        if artifact.status == ArtifactStatus::Active && artifact.access_count == 0 {
            artifact.status = ArtifactStatus::Stale;
            let _ = save_artifact(artifact_dir, artifact);
            actions.push(format!("Marked stale: {}", artifact.id));
        }
    }

    // 3. Archive stale artifacts that have been stale for too long
    let to_archive: Vec<String> = manifest
        .artifacts
        .iter()
        .filter(|a| a.status == ArtifactStatus::Stale && a.access_count == 0)
        .map(|a| a.id.clone())
        .collect();
    for id in &to_archive {
        let _ = archive_artifact(artifact_dir, id);
        actions.push(format!("Archived stale artifact: {}", id));
    }
    manifest.artifacts.retain(|a| !to_archive.contains(&a.id));

    // 4. Enforce hard cap: max MAX_ACTIVE_ARTIFACTS
    while manifest.artifacts.len() > MAX_ACTIVE_ARTIFACTS {
        // Remove least-accessed artifact
        if let Some(pos) = manifest
            .artifacts
            .iter()
            .enumerate()
            .min_by_key(|(_, a)| a.access_count)
            .map(|(i, _)| i)
        {
            let removed = manifest.artifacts.remove(pos);
            let _ = archive_artifact(artifact_dir, &removed.id);
            actions.push(format!(
                "Archived (cap): {} (access_count={})",
                removed.id, removed.access_count
            ));
        }
    }

    // 5. Enforce per-mind cap: max MAX_PER_MIND per mind
    let mind_names: Vec<String> = manifest
        .artifacts
        .iter()
        .map(|a| a.mind.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    for mind in &mind_names {
        let count = manifest.count_by_mind(mind);
        if count > MAX_PER_MIND {
            let excess = count - MAX_PER_MIND;
            // Collect IDs to archive (least accessed for this mind)
            let mut mind_artifacts: Vec<(usize, u32)> = manifest
                .artifacts
                .iter()
                .enumerate()
                .filter(|(_, a)| &a.mind == mind)
                .map(|(i, a)| (i, a.access_count))
                .collect();
            mind_artifacts.sort_by_key(|(_, count)| *count);
            let to_remove: Vec<usize> = mind_artifacts
                .iter()
                .take(excess)
                .map(|(i, _)| *i)
                .collect();
            // Remove in reverse order to preserve indices
            for &idx in to_remove.iter().rev() {
                let removed = manifest.artifacts.remove(idx);
                let _ = archive_artifact(artifact_dir, &removed.id);
                actions.push(format!(
                    "Archived (per-mind cap): {} (mind={})",
                    removed.id, mind
                ));
            }
        }
    }

    if !actions.is_empty() {
        tracing::info!(count = actions.len(), "Artifact pruning completed");
    }

    actions
}

// ── Consciousness Decision Types ────────────────────────────────────────

/// What consciousness produces each round (structured output).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessDecision {
    /// Consciousness's own thoughts (injected as {consciousness} block).
    pub thoughts: String,
    /// Artifact IDs to inject into the worker prompt.
    #[serde(default)]
    pub inject_artifacts: Vec<String>,
    /// Minds to invoke for new/refreshed analysis.
    #[serde(default)]
    pub invoke_minds: Vec<MindInvocation>,
    /// Artifact lifecycle actions.
    #[serde(default)]
    pub artifact_actions: Vec<ArtifactAction>,
}

/// A request from consciousness to invoke an X-Mind subagent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MindInvocation {
    /// Which mind to invoke (e.g., "architect", "analyst", or custom name).
    pub mind: String,
    /// Goal for this invocation (what consciousness needs).
    pub goal: String,
    /// Artifact ID to create or update.
    pub artifact_id: String,
}

/// An artifact lifecycle action requested by consciousness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactAction {
    /// Action type.
    pub action: ArtifactActionKind,
    /// Target artifact ID.
    pub id: String,
    /// Reason for the action.
    #[serde(default)]
    pub reason: String,
    /// New scope (for promote/demote).
    #[serde(default)]
    pub new_scope: Option<ArtifactScope>,
}

/// Types of artifact lifecycle actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactActionKind {
    /// Move to archive.
    Archive,
    /// Permanently delete.
    Delete,
    /// Change scope.
    Promote,
    /// Invoke mind to update this artifact.
    Refresh,
}

// ── Mind Definition ─────────────────────────────────────────────────────

/// Definition of an X-Mind (built-in or custom, loaded from .md files).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MindDefinition {
    /// Unique name (e.g., "architect", "analyst", "sentinel").
    pub name: String,
    /// One-line description.
    pub description: String,
    /// System prompt for the mind's LLM calls.
    pub system_prompt: String,
    /// Read-only tools this mind can use.
    pub tools: Vec<String>,
    /// Max tool calls per invocation.
    pub max_tool_calls: usize,
    /// Timeout in seconds.
    pub timeout_secs: u64,
}

/// Load all mind definitions (built-in + custom from .md files).
pub fn load_mind_definitions(artifact_dir: &Path) -> Vec<MindDefinition> {
    let mut minds = vec![builtin_architect(), builtin_analyst(), builtin_sentinel()];

    // Load custom minds from custom/ subdirectory
    let custom_dir = artifact_dir.join("custom");
    if let Ok(entries) = std::fs::read_dir(&custom_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                match load_custom_mind(&path) {
                    Ok(mind) => {
                        tracing::info!(name = %mind.name, "Loaded custom X-Mind definition");
                        minds.push(mind);
                    }
                    Err(e) => {
                        tracing::warn!(path = %path.display(), error = %e, "Failed to load custom mind");
                    }
                }
            }
        }
    }

    minds
}

fn load_custom_mind(path: &Path) -> Result<MindDefinition, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    if let Some(rest) = content.strip_prefix("---\n") {
        if let Some(split_pos) = rest.find("\n---\n") {
            let frontmatter = &rest[..split_pos];
            let body = rest[split_pos + 5..].trim().to_string();

            #[derive(Deserialize)]
            struct FM {
                #[serde(default)]
                name: Option<String>,
                #[serde(default)]
                description: Option<String>,
                #[serde(default)]
                tools: Option<Vec<String>>,
                #[serde(default)]
                max_tool_calls: Option<usize>,
                #[serde(default)]
                timeout_secs: Option<u64>,
            }

            let fm: FM = serde_yaml::from_str(frontmatter)?;
            return Ok(MindDefinition {
                name: fm.name.unwrap_or(name),
                description: fm.description.unwrap_or_default(),
                system_prompt: body,
                tools: fm
                    .tools
                    .unwrap_or_else(|| vec!["grep".into(), "read_file".into()]),
                max_tool_calls: fm.max_tool_calls.unwrap_or(5),
                timeout_secs: fm.timeout_secs.unwrap_or(30),
            });
        }
    }

    // No frontmatter — use entire content as system prompt
    Ok(MindDefinition {
        name,
        description: String::new(),
        system_prompt: content.trim().to_string(),
        tools: vec!["grep".into(), "read_file".into()],
        max_tool_calls: 5,
        timeout_secs: 30,
    })
}

// ── Built-in Minds ──────────────────────────────────────────────────────

fn builtin_architect() -> MindDefinition {
    MindDefinition {
        name: "architect".into(),
        description: "Analyzes system and code architecture — modules, dependencies, patterns"
            .into(),
        system_prompt: "You are the ARCHITECT mind of an AI agent called Tem. When invoked, you \
            analyze the system, codebase, or problem structure.\n\n\
            Your output MUST follow this format:\n\
            # [Title — 5-10 word summary]\n\n\
            ## Summary\n[1-3 sentence description of your analysis]\n\n\
            ## Analysis\n[Full detailed analysis: module structure, dependencies, patterns, \
            design decisions, potential issues]\n\n\
            Use tools (grep, read_file) to examine actual code when available. \
            Build on previous artifacts — don't repeat what's already established."
            .into(),
        tools: vec!["grep".into(), "read_file".into(), "list_files".into()],
        max_tool_calls: 5,
        timeout_secs: 30,
    }
}

fn builtin_analyst() -> MindDefinition {
    MindDefinition {
        name: "analyst".into(),
        description:
            "Applies logical and mathematical reasoning — invariants, edge cases, correctness"
                .into(),
        system_prompt: "You are the ANALYST mind of an AI agent called Tem. When invoked, you \
            apply formal reasoning to the current work.\n\n\
            Your output MUST follow this format:\n\
            # [Title — 5-10 word summary]\n\n\
            ## Summary\n[1-3 sentence description of your findings]\n\n\
            ## Analysis\n[Full analysis: invariants that must hold, edge cases enumerated, \
            correctness verification, mathematical reasoning if applicable]\n\n\
            Be precise and exhaustive with edge cases. \
            Every claim must be justified with reasoning."
            .into(),
        tools: vec!["grep".into(), "read_file".into()],
        max_tool_calls: 3,
        timeout_secs: 20,
    }
}

fn builtin_sentinel() -> MindDefinition {
    MindDefinition {
        name: "sentinel".into(),
        description: "Monitors for security vulnerabilities, safety risks, dangerous operations"
            .into(),
        system_prompt: "You are the SENTINEL mind of an AI agent called Tem. When invoked, you \
            audit for security and safety.\n\n\
            Your output MUST follow this format:\n\
            # [Title — 5-10 word summary]\n\n\
            ## Summary\n[1-2 sentence overview: are there concerns or is everything safe?]\n\n\
            ## Analysis\n[Detailed findings: each vulnerability listed with severity, \
            explanation, and recommended fix. If nothing found, state explicitly.]\n\n\
            Check for: SQL injection, XSS, path traversal, command injection, \
            hardcoded credentials, insecure crypto, missing input validation."
            .into(),
        tools: vec!["grep".into(), "read_file".into()],
        max_tool_calls: 5,
        timeout_secs: 30,
    }
}

// ── Config ──────────────────────────────────────────────────────────────

/// Configuration for the X-Mind system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XMindsConfig {
    /// Master switch. OFF by default.
    #[serde(default)]
    pub enabled: bool,
    /// Maximum total tokens for artifact injection per round.
    #[serde(default = "default_token_budget")]
    pub token_budget: usize,
    /// Per-mind timeout in seconds for subagent invocations.
    #[serde(default = "default_mind_timeout_secs")]
    pub mind_timeout_secs: u64,
    /// Architect mind enabled.
    #[serde(default = "default_true")]
    pub architect_enabled: bool,
    /// Analyst mind enabled.
    #[serde(default = "default_true")]
    pub analyst_enabled: bool,
    /// Sentinel mind enabled.
    #[serde(default = "default_true")]
    pub sentinel_enabled: bool,
    /// Path for artifact persistence. Defaults to ~/.temm1e/x_minds/
    #[serde(default)]
    pub artifact_dir: Option<String>,
}

impl Default for XMindsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token_budget: default_token_budget(),
            mind_timeout_secs: default_mind_timeout_secs(),
            architect_enabled: true,
            analyst_enabled: true,
            sentinel_enabled: true,
            artifact_dir: None,
        }
    }
}

fn default_token_budget() -> usize {
    500
}
fn default_mind_timeout_secs() -> u64 {
    30
}
fn default_true() -> bool {
    true
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_3_tier() {
        let artifact = Artifact {
            id: "arch-kvstore-001".into(),
            mind: "architect".into(),
            title: "KV Store Transaction Architecture".into(),
            description: "Stack-based nested transaction layer with tombstone deletes.".into(),
            content: "## Module Structure\nTransactionalKVStore\n  - global_data\n  - txn_stack"
                .into(),
            tags: vec!["python".into(), "transactions".into()],
            scope: ArtifactScope::Task,
            status: ArtifactStatus::Active,
            created_at: "2026-04-02T10:00:00Z".into(),
            updated_at: "2026-04-02T10:15:00Z".into(),
            session_id: "cli-cli".into(),
            turn_created: 1,
            turn_updated: 5,
            access_count: 3,
            last_accessed: "2026-04-02T10:20:00Z".into(),
            token_estimate: 50,
        };

        let manifest = artifact.manifest_entry();
        assert!(manifest.contains("arch-kvstore-001"));
        assert!(manifest.contains("architect"));
        assert!(manifest.contains("KV Store Transaction Architecture"));
        assert!(manifest.contains("task-scoped"));

        let block = artifact.injection_block();
        assert!(block.contains("{{x_mind:arch-kvstore-001}}"));
        assert!(block.contains("Module Structure"));
        assert!(block.contains("{{/x_mind:arch-kvstore-001}}"));
    }

    #[test]
    fn test_artifact_file_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let artifact = Artifact {
            id: "test-001".into(),
            mind: "analyst".into(),
            title: "Test Artifact".into(),
            description: "A test artifact for roundtrip verification.".into(),
            content: "## Analysis\nThis is the full content.\nMultiple lines.".into(),
            tags: vec!["test".into()],
            scope: ArtifactScope::Task,
            status: ArtifactStatus::Active,
            created_at: "2026-04-02T00:00:00Z".into(),
            updated_at: "2026-04-02T00:00:00Z".into(),
            session_id: "test".into(),
            turn_created: 1,
            turn_updated: 1,
            access_count: 0,
            last_accessed: "2026-04-02T00:00:00Z".into(),
            token_estimate: 20,
        };

        save_artifact(dir.path(), &artifact).unwrap();
        let loaded = load_artifact_file(&dir.path().join("artifacts").join("test-001.md")).unwrap();

        assert_eq!(loaded.id, "test-001");
        assert_eq!(loaded.mind, "analyst");
        assert_eq!(loaded.title, "Test Artifact");
        assert!(loaded.content.contains("This is the full content."));
    }

    #[test]
    fn test_manifest_load_empty() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = ArtifactManifest::load(dir.path());
        assert!(manifest.artifacts.is_empty());
        assert_eq!(manifest.active_count(), 0);
    }

    #[test]
    fn test_manifest_load_with_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let a1 = Artifact {
            id: "a-001".into(),
            mind: "architect".into(),
            title: "Test".into(),
            description: "Test desc".into(),
            content: "Content".into(),
            tags: vec![],
            scope: ArtifactScope::Task,
            status: ArtifactStatus::Active,
            created_at: "2026-04-02T00:00:00Z".into(),
            updated_at: "2026-04-02T00:00:00Z".into(),
            session_id: "test".into(),
            turn_created: 1,
            turn_updated: 1,
            access_count: 5,
            last_accessed: "2026-04-02T00:00:00Z".into(),
            token_estimate: 2,
        };
        save_artifact(dir.path(), &a1).unwrap();

        let manifest = ArtifactManifest::load(dir.path());
        assert_eq!(manifest.artifacts.len(), 1);
        assert_eq!(manifest.artifacts[0].id, "a-001");
        assert_eq!(manifest.active_count(), 1);
    }

    #[test]
    fn test_manifest_format() {
        let manifest = ArtifactManifest {
            artifacts: vec![Artifact {
                id: "arch-001".into(),
                mind: "architect".into(),
                title: "Test Architecture".into(),
                description: "A test architecture description.".into(),
                content: "Full content here".into(),
                tags: vec!["rust".into()],
                scope: ArtifactScope::Project,
                status: ArtifactStatus::Active,
                created_at: "2026-04-02T00:00:00Z".into(),
                updated_at: "2026-04-02T00:00:00Z".into(),
                session_id: "test".into(),
                turn_created: 1,
                turn_updated: 1,
                access_count: 10,
                last_accessed: "2026-04-02T00:00:00Z".into(),
                token_estimate: 5,
            }],
        };

        let formatted = manifest.format_for_consciousness();
        assert!(formatted.contains("1 active"));
        assert!(formatted.contains("arch-001"));
        assert!(formatted.contains("Test Architecture"));
    }

    #[test]
    fn test_archive_artifact() {
        let dir = tempfile::tempdir().unwrap();
        let artifact = Artifact {
            id: "to-archive".into(),
            mind: "sentinel".into(),
            title: "Archive Me".into(),
            description: "Should be archived.".into(),
            content: "Content".into(),
            tags: vec![],
            scope: ArtifactScope::Task,
            status: ArtifactStatus::Active,
            created_at: "2026-04-02T00:00:00Z".into(),
            updated_at: "2026-04-02T00:00:00Z".into(),
            session_id: "test".into(),
            turn_created: 1,
            turn_updated: 1,
            access_count: 0,
            last_accessed: "2026-04-02T00:00:00Z".into(),
            token_estimate: 2,
        };
        save_artifact(dir.path(), &artifact).unwrap();

        archive_artifact(dir.path(), "to-archive").unwrap();

        // Should be in archive, not in artifacts
        assert!(!dir.path().join("artifacts/to-archive.md").exists());
        assert!(dir.path().join("archive/to-archive.md").exists());
    }

    #[test]
    fn test_delete_artifact() {
        let dir = tempfile::tempdir().unwrap();
        let artifact = Artifact {
            id: "to-delete".into(),
            mind: "analyst".into(),
            title: "Delete Me".into(),
            description: "Should be deleted.".into(),
            content: "Content".into(),
            tags: vec![],
            scope: ArtifactScope::Turn,
            status: ArtifactStatus::Active,
            created_at: "2026-04-02T00:00:00Z".into(),
            updated_at: "2026-04-02T00:00:00Z".into(),
            session_id: "test".into(),
            turn_created: 1,
            turn_updated: 1,
            access_count: 0,
            last_accessed: "2026-04-02T00:00:00Z".into(),
            token_estimate: 2,
        };
        save_artifact(dir.path(), &artifact).unwrap();
        assert!(dir.path().join("artifacts/to-delete.md").exists());

        delete_artifact(dir.path(), "to-delete").unwrap();
        assert!(!dir.path().join("artifacts/to-delete.md").exists());
    }

    #[test]
    fn test_pruning_turn_scoped() {
        let dir = tempfile::tempdir().unwrap();
        let a = Artifact {
            id: "turn-001".into(),
            mind: "analyst".into(),
            title: "Turn Scoped".into(),
            description: "Should auto-delete.".into(),
            content: "Content".into(),
            tags: vec![],
            scope: ArtifactScope::Turn,
            status: ArtifactStatus::Active,
            created_at: "2026-04-02T00:00:00Z".into(),
            updated_at: "2026-04-02T00:00:00Z".into(),
            session_id: "test".into(),
            turn_created: 1,
            turn_updated: 1,
            access_count: 0,
            last_accessed: "2026-04-02T00:00:00Z".into(),
            token_estimate: 2,
        };
        save_artifact(dir.path(), &a).unwrap();

        let mut manifest = ArtifactManifest { artifacts: vec![a] };
        let actions = prune_manifest(dir.path(), &mut manifest, 1);

        assert!(manifest.artifacts.is_empty());
        assert!(!actions.is_empty());
    }

    #[test]
    fn test_consciousness_decision_serde() {
        let decision = ConsciousnessDecision {
            thoughts: "Agent is on track.".into(),
            inject_artifacts: vec!["arch-001".into()],
            invoke_minds: vec![MindInvocation {
                mind: "analyst".into(),
                goal: "Check edge cases".into(),
                artifact_id: "analyst-001".into(),
            }],
            artifact_actions: vec![ArtifactAction {
                action: ArtifactActionKind::Archive,
                id: "old-001".into(),
                reason: "No longer relevant".into(),
                new_scope: None,
            }],
        };

        let json = serde_json::to_string(&decision).unwrap();
        let back: ConsciousnessDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(back.thoughts, "Agent is on track.");
        assert_eq!(back.inject_artifacts.len(), 1);
        assert_eq!(back.invoke_minds.len(), 1);
        assert_eq!(back.artifact_actions.len(), 1);
    }

    #[test]
    fn test_mind_definition_builtins() {
        let dir = tempfile::tempdir().unwrap();
        let minds = load_mind_definitions(dir.path());
        assert_eq!(minds.len(), 3);
        assert_eq!(minds[0].name, "architect");
        assert_eq!(minds[1].name, "analyst");
        assert_eq!(minds[2].name, "sentinel");
    }

    #[test]
    fn test_custom_mind_loading() {
        let dir = tempfile::tempdir().unwrap();
        let custom_dir = dir.path().join("custom");
        std::fs::create_dir_all(&custom_dir).unwrap();
        std::fs::write(
            custom_dir.join("creativity.md"),
            "---\nname: creativity\ndescription: Generates creative alternatives\ntools: [grep]\nmax_tool_calls: 3\ntimeout_secs: 15\n---\nYou are the CREATIVITY mind.\n",
        ).unwrap();

        let minds = load_mind_definitions(dir.path());
        assert_eq!(minds.len(), 4); // 3 built-in + 1 custom
        let creativity = minds.iter().find(|m| m.name == "creativity").unwrap();
        assert_eq!(creativity.max_tool_calls, 3);
        assert_eq!(creativity.timeout_secs, 15);
    }

    #[test]
    fn test_xminds_config_default() {
        let config = XMindsConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.token_budget, 500);
        assert_eq!(config.mind_timeout_secs, 30);
    }

    #[test]
    fn test_artifact_scope_serde() {
        for scope in [
            ArtifactScope::Turn,
            ArtifactScope::Task,
            ArtifactScope::Session,
            ArtifactScope::Project,
        ] {
            let json = serde_json::to_string(&scope).unwrap();
            let back: ArtifactScope = serde_json::from_str(&json).unwrap();
            assert_eq!(scope, back);
        }
    }

    #[test]
    fn test_artifact_token_estimation() {
        assert_eq!(Artifact::estimate_tokens(""), 1);
        assert_eq!(Artifact::estimate_tokens("hello world"), 3); // 11/4 + 1
        assert_eq!(Artifact::estimate_tokens(&"x".repeat(400)), 101); // 400/4 + 1
    }
}
