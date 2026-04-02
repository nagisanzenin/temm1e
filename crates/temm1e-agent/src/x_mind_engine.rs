//! X-Mind v2 Engine — Consciousness-Orchestrated Subagent Architecture.
//!
//! The orchestrator manages artifacts, mind definitions, and provides
//! the interface between consciousness and the X-Mind subagents.
//! Consciousness calls the orchestrator to:
//! 1. Get the artifact manifest (for consciousness to skim)
//! 2. Load artifact content (for injection into worker)
//! 3. Invoke a mind subagent (to create/refresh artifacts)
//! 4. Execute artifact lifecycle actions (archive, delete, promote)

use crate::budget::{calculate_cost, get_pricing, BudgetTracker};
use crate::consciousness::safe_preview;
use crate::x_mind::{
    archive_artifact, delete_artifact, load_artifact_content, load_mind_definitions, record_access,
    save_artifact, Artifact, ArtifactActionKind, ArtifactManifest, ArtifactScope, ArtifactStatus,
    MindDefinition, MindInvocation, XMindsConfig,
};
use std::path::PathBuf;
use std::sync::Arc;
use temm1e_core::types::message::{
    ChatMessage, CompletionRequest, ContentPart, MessageContent, Role,
};
use temm1e_core::Provider;
use tokio::sync::Mutex;

/// The X-Mind v2 Orchestrator.
pub struct XMindOrchestrator {
    config: XMindsConfig,
    provider: Arc<dyn Provider>,
    model: String,
    /// Artifact directory on disk.
    artifact_dir: PathBuf,
    /// Loaded mind definitions (built-in + custom).
    mind_definitions: Vec<MindDefinition>,
    /// In-memory artifact manifest (refreshed from disk on demand).
    manifest: Mutex<ArtifactManifest>,
    /// Shared budget tracker.
    budget: Arc<BudgetTracker>,
    /// Read-only tools available to subagents (set via `with_tools`).
    read_only_tools: Vec<Arc<dyn temm1e_core::Tool>>,
}

impl XMindOrchestrator {
    /// Create a new orchestrator, loading artifacts and mind definitions.
    pub async fn new(
        config: XMindsConfig,
        provider: Arc<dyn Provider>,
        model: String,
        budget: Arc<BudgetTracker>,
    ) -> Self {
        let artifact_dir = config
            .artifact_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".temm1e")
                    .join("x_minds")
            });

        // Ensure directories exist
        let _ = std::fs::create_dir_all(artifact_dir.join("artifacts"));
        let _ = std::fs::create_dir_all(artifact_dir.join("custom"));

        let mind_definitions = load_mind_definitions(&artifact_dir);
        let manifest = ArtifactManifest::load(&artifact_dir);

        tracing::info!(
            enabled = config.enabled,
            minds = mind_definitions.len(),
            artifacts = manifest.artifacts.len(),
            artifact_dir = %artifact_dir.display(),
            "X-Mind v2 Orchestrator initialized"
        );

        Self {
            config,
            provider,
            model,
            artifact_dir,
            mind_definitions,
            manifest: Mutex::new(manifest),
            budget,
            read_only_tools: Vec::new(),
        }
    }

    /// Set the read-only tools available to X-Mind subagents.
    /// Filter from the full tool set: only grep, read_file, list_files, shell (read-only).
    pub fn with_tools(mut self, all_tools: &[Arc<dyn temm1e_core::Tool>]) -> Self {
        const READ_ONLY_TOOL_NAMES: &[&str] =
            &["grep", "read_file", "file_read", "list_files", "file_list"];
        self.read_only_tools = all_tools
            .iter()
            .filter(|t| READ_ONLY_TOOL_NAMES.contains(&t.name()))
            .cloned()
            .collect();
        tracing::debug!(
            tools = self.read_only_tools.len(),
            "X-Mind read-only tools configured"
        );
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    // ── Manifest Access (for consciousness) ─────────────────────────────

    /// Get the formatted manifest for consciousness input (Tier 1 + Tier 2 only).
    pub async fn manifest_for_consciousness(&self) -> String {
        let manifest = self.manifest.lock().await;
        manifest.format_for_consciousness()
    }

    /// Get available mind names for consciousness.
    pub fn available_minds(&self) -> Vec<(String, String)> {
        self.mind_definitions
            .iter()
            .map(|m| (m.name.clone(), m.description.clone()))
            .collect()
    }

    // ── Artifact Operations ─────────────────────────────────────────────

    /// Load full content of an artifact by ID (Tier 3).
    pub async fn load_artifact_full(&self, id: &str) -> Option<String> {
        let mut manifest = self.manifest.lock().await;
        if let Some(artifact) = manifest.get_mut(id) {
            record_access(&self.artifact_dir, artifact);
        }
        drop(manifest);
        load_artifact_content(&self.artifact_dir, id)
    }

    /// Build injection blocks for the given artifact IDs.
    /// Returns formatted `{x_mind:id}` blocks ready for system prompt.
    pub async fn build_injection(&self, artifact_ids: &[String]) -> String {
        let mut blocks = Vec::new();
        let mut manifest = self.manifest.lock().await;

        for id in artifact_ids {
            if let Some(content) = load_artifact_content(&self.artifact_dir, id) {
                // Record access
                if let Some(artifact) = manifest.get_mut(id) {
                    record_access(&self.artifact_dir, artifact);
                }
                blocks.push(format!(
                    "{{{{x_mind:{id}}}}}\n{content}\n{{{{/x_mind:{id}}}}}"
                ));
            } else {
                tracing::warn!(id = %id, "Artifact not found for injection");
            }
        }

        blocks.join("\n\n")
    }

    // ── Mind Invocation ─────────────────────────────────────────────────

    /// Invoke a mind subagent synchronously. Creates/updates an artifact.
    /// The mind runs its own LLM loop (with read-only tools in future).
    /// Returns the artifact ID on success.
    pub async fn invoke_mind(&self, invocation: &MindInvocation) -> Option<String> {
        let mind_def = self
            .mind_definitions
            .iter()
            .find(|m| m.name == invocation.mind)?;

        tracing::info!(
            mind = %invocation.mind,
            goal = %safe_preview(&invocation.goal, 100),
            artifact_id = %invocation.artifact_id,
            "Invoking X-Mind subagent"
        );

        let timeout = std::time::Duration::from_secs(mind_def.timeout_secs);

        // Run the mind's LLM call (single call for now — agentic loop in Phase 3)
        let result = match tokio::time::timeout(
            timeout,
            self.run_mind_llm(mind_def, &invocation.goal),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => {
                tracing::warn!(mind = %invocation.mind, "X-Mind subagent timed out");
                return None;
            }
        };

        let response_text = result?;

        // Parse 3-tier output from mind's response
        let (title, description, content) = parse_mind_output(&response_text);

        // Create/update the artifact
        let now = chrono::Utc::now().to_rfc3339();
        let artifact = Artifact {
            id: invocation.artifact_id.clone(),
            mind: invocation.mind.clone(),
            title,
            description,
            content,
            tags: vec![],
            scope: ArtifactScope::Task,
            status: ArtifactStatus::Active,
            created_at: now.clone(),
            updated_at: now.clone(),
            session_id: String::new(), // filled by caller
            turn_created: 0,
            turn_updated: 0,
            access_count: 0,
            last_accessed: now,
            token_estimate: Artifact::estimate_tokens(&response_text),
        };

        // Save to disk
        if let Err(e) = save_artifact(&self.artifact_dir, &artifact) {
            tracing::warn!(error = %e, "Failed to save mind artifact");
            return None;
        }

        // Update manifest
        {
            let mut manifest = self.manifest.lock().await;
            // Remove old version if exists
            manifest.artifacts.retain(|a| a.id != artifact.id);
            manifest.artifacts.push(artifact);
        }

        tracing::info!(
            mind = %invocation.mind,
            artifact_id = %invocation.artifact_id,
            "X-Mind subagent produced artifact"
        );

        Some(invocation.artifact_id.clone())
    }

    /// Invoke multiple minds concurrently. Returns list of produced artifact IDs.
    pub async fn invoke_minds(&self, invocations: &[MindInvocation]) -> Vec<String> {
        if invocations.is_empty() {
            return Vec::new();
        }

        // For single invocation, run directly
        if invocations.len() == 1 {
            return self
                .invoke_mind(&invocations[0])
                .await
                .into_iter()
                .collect();
        }

        // Multiple: run concurrently
        // We can't easily spawn tasks that borrow self, so run sequentially for now.
        // TODO: Phase 3 — proper concurrent subagent execution
        let mut results = Vec::new();
        for inv in invocations {
            if let Some(id) = self.invoke_mind(inv).await {
                results.push(id);
            }
        }
        results
    }

    // ── Artifact Lifecycle Actions ──────────────────────────────────────

    /// Execute artifact lifecycle actions from consciousness.
    pub async fn execute_actions(&self, actions: &[crate::x_mind::ArtifactAction]) {
        let mut manifest = self.manifest.lock().await;

        for action in actions {
            match action.action {
                ArtifactActionKind::Archive => {
                    let _ = archive_artifact(&self.artifact_dir, &action.id);
                    manifest.artifacts.retain(|a| a.id != action.id);
                    tracing::info!(id = %action.id, reason = %action.reason, "Consciousness archived artifact");
                }
                ArtifactActionKind::Delete => {
                    let _ = delete_artifact(&self.artifact_dir, &action.id);
                    manifest.artifacts.retain(|a| a.id != action.id);
                    tracing::info!(id = %action.id, reason = %action.reason, "Consciousness deleted artifact");
                }
                ArtifactActionKind::Promote => {
                    if let Some(artifact) = manifest.get_mut(&action.id) {
                        if let Some(new_scope) = action.new_scope {
                            artifact.scope = new_scope;
                            let _ = save_artifact(&self.artifact_dir, artifact);
                            tracing::info!(id = %action.id, scope = ?new_scope, "Consciousness promoted artifact");
                        }
                    }
                }
                ArtifactActionKind::Refresh => {
                    // Refresh is handled by consciousness invoking the mind again
                    tracing::debug!(id = %action.id, "Refresh requested — consciousness should invoke mind");
                }
            }
        }
    }

    /// Run pruning on the manifest.
    pub async fn prune(&self) {
        let mut manifest = self.manifest.lock().await;
        let actions = crate::x_mind::prune_manifest(&self.artifact_dir, &mut manifest, 0);
        if !actions.is_empty() {
            for action in &actions {
                tracing::debug!(action = %action, "Prune action");
            }
        }
    }

    /// Reload manifest from disk.
    pub async fn reload_manifest(&self) {
        let new_manifest = ArtifactManifest::load(&self.artifact_dir);
        let mut manifest = self.manifest.lock().await;
        *manifest = new_manifest;
    }

    // ── Internal LLM Execution ──────────────────────────────────────────

    /// Run a mind's agentic loop: LLM call → tool execution → repeat until done.
    /// Bounded by max_tool_calls from the mind definition.
    async fn run_mind_llm(&self, mind_def: &MindDefinition, goal: &str) -> Option<String> {
        let user_prompt = format!(
            "Goal: {}\n\n\
             You may use tools to examine files and code. When done, produce your analysis:\n\
             # [Title]\n\n\
             ## Summary\n[Brief description]\n\n\
             ## Analysis\n[Full detailed analysis]",
            goal
        );

        // Build tool declarations for read-only tools this mind can use
        let mind_tools: Vec<Arc<dyn temm1e_core::Tool>> = self
            .read_only_tools
            .iter()
            .filter(|t| mind_def.tools.iter().any(|allowed| allowed == t.name()))
            .cloned()
            .collect();

        let tool_definitions: Vec<temm1e_core::types::message::ToolDefinition> = mind_tools
            .iter()
            .map(|t| temm1e_core::types::message::ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters_schema(),
            })
            .collect();

        let mut messages = vec![ChatMessage {
            role: Role::User,
            content: MessageContent::Text(user_prompt),
        }];

        let mut tool_calls_used = 0;
        let max_rounds = mind_def.max_tool_calls + 1; // +1 for the final text response

        for round in 0..max_rounds {
            let request = CompletionRequest {
                model: self.model.clone(),
                messages: messages.clone(),
                tools: if tool_calls_used < mind_def.max_tool_calls {
                    tool_definitions.clone()
                } else {
                    vec![] // no more tools — force text response
                },
                max_tokens: None,
                temperature: Some(0.3),
                system: Some(mind_def.system_prompt.clone()),
            };

            let response = match self.provider.complete(request).await {
                Ok(resp) => resp,
                Err(e) => {
                    tracing::warn!(mind = %mind_def.name, round, error = %e, "Mind LLM call failed");
                    return None;
                }
            };

            // Track cost
            let pricing = get_pricing(&self.model);
            let cost = calculate_cost(
                response.usage.input_tokens,
                response.usage.output_tokens,
                &pricing,
            );
            self.budget.record_usage(
                response.usage.input_tokens,
                response.usage.output_tokens,
                cost,
            );

            // Extract text and tool calls
            let mut text_parts: Vec<String> = Vec::new();
            let mut tool_uses: Vec<(String, String, serde_json::Value)> = Vec::new();

            for part in &response.content {
                match part {
                    ContentPart::Text { text } => text_parts.push(text.clone()),
                    ContentPart::ToolUse {
                        id, name, input, ..
                    } => {
                        tool_uses.push((id.clone(), name.clone(), input.clone()));
                    }
                    _ => {}
                }
            }

            // If no tool calls, we have the final response
            if tool_uses.is_empty() {
                let final_text = text_parts.join("\n");
                tracing::debug!(
                    mind = %mind_def.name,
                    rounds = round + 1,
                    tool_calls = tool_calls_used,
                    "Mind subagent completed"
                );
                return Some(final_text.trim().to_string());
            }

            // Record assistant message with tool uses
            messages.push(ChatMessage {
                role: Role::Assistant,
                content: MessageContent::Parts(response.content.clone()),
            });

            // Execute tools and collect results
            let mut tool_result_parts: Vec<ContentPart> = Vec::new();
            for (tool_use_id, tool_name, input) in &tool_uses {
                tool_calls_used += 1;

                let tool_result =
                    if let Some(tool) = mind_tools.iter().find(|t| t.name() == tool_name) {
                        let tool_input = temm1e_core::ToolInput {
                            name: tool_name.clone(),
                            arguments: input.clone(),
                        };
                        let ctx = temm1e_core::ToolContext {
                            workspace_path: self.artifact_dir.clone(),
                            session_id: String::from("x-mind"),
                            chat_id: String::from("x-mind"),
                        };
                        match tool.execute(tool_input, &ctx).await {
                            Ok(output) => output.content,
                            Err(e) => format!("Error: {}", e),
                        }
                    } else {
                        format!("Tool '{}' not available to this mind", tool_name)
                    };

                tracing::debug!(
                    mind = %mind_def.name,
                    tool = %tool_name,
                    "Mind subagent tool call"
                );

                tool_result_parts.push(ContentPart::ToolResult {
                    tool_use_id: tool_use_id.clone(),
                    content: safe_preview(&tool_result, 2000),
                    is_error: false,
                });
            }

            // Add tool results to conversation
            messages.push(ChatMessage {
                role: Role::Tool,
                content: MessageContent::Parts(tool_result_parts),
            });
        }

        // Max rounds reached — return whatever text we have
        tracing::warn!(
            mind = %mind_def.name,
            "Mind subagent hit max rounds without final text response"
        );
        None
    }
}

// ── Output Parsing ──────────────────────────────────────────────────────

/// Parse a mind's response into 3 tiers: title, description (summary), content.
fn parse_mind_output(text: &str) -> (String, String, String) {
    let lines: Vec<&str> = text.lines().collect();

    let mut title = String::new();
    let mut summary = String::new();
    let mut analysis = String::new();

    #[derive(PartialEq)]
    enum Section {
        None,
        Title,
        Summary,
        Analysis,
    }
    let mut current = Section::None;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") && !trimmed.starts_with("## ") {
            title = trimmed.trim_start_matches("# ").to_string();
            current = Section::Title;
        } else if trimmed == "## Summary" {
            current = Section::Summary;
        } else if trimmed == "## Analysis" {
            current = Section::Analysis;
        } else {
            match current {
                Section::Summary => {
                    if !trimmed.is_empty() {
                        if !summary.is_empty() {
                            summary.push(' ');
                        }
                        summary.push_str(trimmed);
                    }
                }
                Section::Analysis => {
                    analysis.push_str(line);
                    analysis.push('\n');
                }
                _ => {
                    // Content before any section header goes to analysis
                    if !trimmed.is_empty() && title.is_empty() {
                        // First non-empty line without # is title fallback
                        title = safe_preview(trimmed, 80);
                    } else if !trimmed.is_empty() {
                        analysis.push_str(line);
                        analysis.push('\n');
                    }
                }
            }
        }
    }

    // Fallbacks
    if title.is_empty() {
        title = safe_preview(text, 60);
    }
    if summary.is_empty() {
        summary = safe_preview(text, 200);
    }
    if analysis.is_empty() {
        analysis = text.to_string();
    }

    (title, summary, analysis.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mind_output_structured() {
        let text = "# KV Store Architecture\n\n## Summary\nStack-based transaction layer.\n\n## Analysis\nDetailed module structure here.\nMore details.";
        let (title, summary, analysis) = parse_mind_output(text);
        assert_eq!(title, "KV Store Architecture");
        assert_eq!(summary, "Stack-based transaction layer.");
        assert!(analysis.contains("Detailed module structure"));
    }

    #[test]
    fn test_parse_mind_output_unstructured() {
        let text = "This is just plain text without headers.";
        let (title, summary, analysis) = parse_mind_output(text);
        assert!(!title.is_empty());
        assert!(!summary.is_empty());
        assert!(!analysis.is_empty());
    }

    #[test]
    fn test_parse_mind_output_partial() {
        let text = "# Title Only\n\nSome content without summary header.";
        let (title, summary, analysis) = parse_mind_output(text);
        assert_eq!(title, "Title Only");
        assert!(!summary.is_empty()); // fallback
        assert!(!analysis.is_empty());
    }
}
