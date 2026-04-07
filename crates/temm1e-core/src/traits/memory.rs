use crate::types::error::Temm1eError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── λ-Memory Types ─────────────────────────────────────────────

/// A single λ-memory entry with three fidelity layers.
///
/// Created with full/summary/essence at write time.
/// Decay score is computed lazily at read time — never stored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LambdaMemoryEntry {
    /// Unique hash identifier (first 12 hex chars of SHA-256).
    pub hash: String,
    /// Unix epoch seconds when created.
    pub created_at: u64,
    /// Unix epoch seconds when last accessed (recalled or created).
    pub last_accessed: u64,
    /// Number of times recalled via lambda_recall tool.
    pub access_count: u32,
    /// Importance score assigned by LLM at creation (1.0–5.0).
    pub importance: f32,
    /// Whether the user explicitly asked to remember this.
    pub explicit_save: bool,
    /// Full-fidelity content (user message + assistant core response).
    pub full_text: String,
    /// One-sentence summary (LLM-generated at creation).
    pub summary_text: String,
    /// Five-word-max essence (LLM-generated at creation).
    pub essence_text: String,
    /// Up to 5 tags (LLM-generated at creation).
    pub tags: Vec<String>,
    /// Whether this is a conversation memory, knowledge, or learning.
    pub memory_type: LambdaMemoryType,
    /// Session that created this memory.
    pub session_id: String,
    /// Additive importance boost from recalls (+0.3 per recall, capped at 2.0).
    /// GC applies -0.1 penalty for entries with no access since last sweep.
    #[serde(default)]
    pub recall_boost: f32,
}

/// Classification of λ-memory entries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LambdaMemoryType {
    /// Normal conversation turn memory.
    Conversation,
    /// Persistent knowledge (replaces old MemoryEntryType::Knowledge in context).
    Knowledge,
    /// Cross-task learning (replaces old learnings in context).
    Learning,
}

/// A single memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub metadata: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub session_id: Option<String>,
    pub entry_type: MemoryEntryType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryEntryType {
    Conversation,
    LongTerm,
    DailyLog,
    Skill,
    Knowledge,
    Blueprint,
}

/// Search options for memory queries
#[derive(Debug, Clone)]
pub struct SearchOpts {
    pub limit: usize,
    pub vector_weight: f32,
    pub keyword_weight: f32,
    pub session_filter: Option<String>,
    pub entry_type_filter: Option<MemoryEntryType>,
}

impl Default for SearchOpts {
    fn default() -> Self {
        Self {
            limit: 10,
            vector_weight: 0.7,
            keyword_weight: 0.3,
            session_filter: None,
            entry_type_filter: None,
        }
    }
}

/// Memory backend trait — persistence for conversations, long-term memory, and skills
#[async_trait]
pub trait Memory: Send + Sync {
    /// Store a memory entry
    async fn store(&self, entry: MemoryEntry) -> Result<(), Temm1eError>;

    /// Hybrid search: vector similarity + keyword matching
    async fn search(&self, query: &str, opts: SearchOpts) -> Result<Vec<MemoryEntry>, Temm1eError>;

    /// Get a specific memory entry by ID
    async fn get(&self, id: &str) -> Result<Option<MemoryEntry>, Temm1eError>;

    /// Delete a memory entry
    async fn delete(&self, id: &str) -> Result<(), Temm1eError>;

    /// List all sessions
    async fn list_sessions(&self) -> Result<Vec<String>, Temm1eError>;

    /// Get conversation history for a session
    async fn get_session_history(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, Temm1eError>;

    /// Backend name (e.g., "sqlite", "postgres", "markdown")
    fn backend_name(&self) -> &str;

    // ── λ-Memory methods (default no-op for backends that don't support it) ──

    /// Store a λ-memory entry.
    async fn lambda_store(&self, _entry: LambdaMemoryEntry) -> Result<(), Temm1eError> {
        Ok(())
    }

    /// Query λ-memory candidates ordered by importance DESC, limited to `limit`.
    async fn lambda_query_candidates(
        &self,
        _limit: usize,
    ) -> Result<Vec<LambdaMemoryEntry>, Temm1eError> {
        Ok(Vec::new())
    }

    /// Look up a λ-memory by hash prefix.
    async fn lambda_recall(
        &self,
        _hash_prefix: &str,
    ) -> Result<Option<LambdaMemoryEntry>, Temm1eError> {
        Ok(None)
    }

    /// Update last_accessed and increment access_count for a recalled memory.
    async fn lambda_touch(&self, _hash: &str) -> Result<(), Temm1eError> {
        Ok(())
    }

    /// FTS5 search returning (hash, bm25_rank) pairs.
    async fn lambda_fts_search(
        &self,
        _query: &str,
        _limit: usize,
    ) -> Result<Vec<(String, f64)>, Temm1eError> {
        Ok(Vec::new())
    }

    /// Garbage collect expired λ-memories. Returns count of deleted entries.
    async fn lambda_gc(&self, _now_epoch: u64, _max_age_secs: u64) -> Result<usize, Temm1eError> {
        Ok(0)
    }

    /// Update a λ-memory entry in place (for dedup merge).
    async fn lambda_update_entry(&self, _entry: &LambdaMemoryEntry) -> Result<(), Temm1eError> {
        Ok(())
    }

    /// Delete a λ-memory entry by hash.
    async fn lambda_delete(&self, _hash: &str) -> Result<(), Temm1eError> {
        Ok(())
    }

    // ── Tool reliability (v4.6.0) ─────────────────────────────────

    /// Record a tool execution outcome for cross-session reliability tracking.
    async fn record_tool_outcome(
        &self,
        _tool_name: &str,
        _task_type: &str,
        _success: bool,
    ) -> Result<(), Temm1eError> {
        Ok(())
    }

    /// Get tool reliability records (last 30 days, top 50 by sample size).
    async fn get_tool_reliability(&self) -> Result<Vec<ToolReliabilityRecord>, Temm1eError> {
        Ok(Vec::new())
    }

    // ── Classification outcomes (v4.6.0) ──────────────────────────

    /// Record a classification outcome for empirical prior tracking.
    #[allow(clippy::too_many_arguments)]
    async fn record_classification_outcome(
        &self,
        _category: &str,
        _difficulty: &str,
        _rounds: u32,
        _tools_used: u32,
        _cost_usd: f64,
        _success: bool,
        _prompt_tier: &str,
        _had_whisper: bool,
    ) -> Result<(), Temm1eError> {
        Ok(())
    }

    /// Get aggregated classification priors (last 30 days).
    async fn get_classification_priors(&self) -> Result<Vec<ClassificationPrior>, Temm1eError> {
        Ok(Vec::new())
    }

    // ── Skill usage (v4.6.0) ──────────────────────────────────────

    /// Record a skill invocation.
    async fn record_skill_usage(&self, _skill_name: &str) -> Result<(), Temm1eError> {
        Ok(())
    }

    /// Get skill usage records ordered by invocations DESC.
    async fn get_skill_usage(&self) -> Result<Vec<SkillUsageRecord>, Temm1eError> {
        Ok(Vec::new())
    }
}

// ── Self-learning record types (v4.6.0) ───────────────────────────

/// Tool reliability record — success/failure rates per (tool, task_type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolReliabilityRecord {
    pub tool_name: String,
    pub task_type: String,
    pub successes: u32,
    pub failures: u32,
    pub last_updated: u64,
}

impl ToolReliabilityRecord {
    pub fn success_rate(&self) -> f64 {
        let total = self.successes + self.failures;
        if total == 0 {
            return 0.5;
        }
        self.successes as f64 / total as f64
    }
}

/// Classification empirical prior — aggregated resource usage per (category, difficulty).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationPrior {
    pub category: String,
    pub difficulty: String,
    pub avg_rounds: f32,
    pub avg_tools: f32,
    pub avg_cost: f64,
    pub count: u32,
}

/// Skill usage record — invocation tracking per skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUsageRecord {
    pub skill_name: String,
    pub invocations: u32,
    pub last_invoked_at: u64,
}
