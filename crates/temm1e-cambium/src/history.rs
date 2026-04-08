//! Growth session history persistence.
//!
//! Stores cambium sessions and trust state as JSON files on disk. Sessions
//! are append-only in JSON-lines format (one session per line); trust state
//! is stored as a single JSON object in a sibling file.
//!
//! This avoids a `sqlx` dependency for the pipeline crate. For a production
//! deployment, the history file sits in `~/.temm1e/cambium/` alongside
//! `history.jsonl` (sessions) and `trust_state.json` (current trust state).

use std::path::{Path, PathBuf};
use temm1e_core::types::cambium::{GrowthSession, TrustState};
use temm1e_core::types::error::Temm1eError;

/// Persistent history of cambium sessions and trust state.
///
/// The `base_dir` contains two files:
/// - `history.jsonl` -- append-only JSON-lines log of growth sessions
/// - `trust_state.json` -- single JSON object with the current trust state
pub struct GrowthHistory {
    base_dir: PathBuf,
}

impl GrowthHistory {
    /// Create a new history store rooted at the given directory.
    ///
    /// The directory is created on first write if it does not exist.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: path.into(),
        }
    }

    /// Return the path to the sessions history file.
    fn history_file(&self) -> PathBuf {
        self.base_dir.join("history.jsonl")
    }

    /// Return the path to the trust state file.
    fn trust_state_file(&self) -> PathBuf {
        self.base_dir.join("trust_state.json")
    }

    /// Ensure the base directory exists.
    async fn ensure_base_dir(&self) -> Result<(), Temm1eError> {
        if !self.base_dir.as_path().exists() {
            tokio::fs::create_dir_all(&self.base_dir)
                .await
                .map_err(|e| {
                    Temm1eError::Config(format!(
                        "failed to create cambium history dir {}: {}",
                        self.base_dir.display(),
                        e
                    ))
                })?;
        }
        Ok(())
    }

    /// Append a session to the history log.
    ///
    /// Sessions are stored one per line as JSON. This allows append-only
    /// writes and streaming reads.
    pub async fn save_session(&self, session: &GrowthSession) -> Result<(), Temm1eError> {
        self.ensure_base_dir().await?;

        let mut line = serde_json::to_string(session)
            .map_err(|e| Temm1eError::Tool(format!("failed to serialize growth session: {}", e)))?;
        line.push('\n');

        let path = self.history_file();
        let existing = if path.exists() {
            tokio::fs::read(&path).await.map_err(|e| {
                Temm1eError::Tool(format!(
                    "failed to read cambium history at {}: {}",
                    path.display(),
                    e
                ))
            })?
        } else {
            Vec::new()
        };

        let mut combined = existing;
        combined.extend_from_slice(line.as_bytes());

        tokio::fs::write(&path, combined).await.map_err(|e| {
            Temm1eError::Tool(format!(
                "failed to write cambium history at {}: {}",
                path.display(),
                e
            ))
        })?;

        tracing::debug!(
            id = %session.id,
            path = %path.display(),
            "cambium: session saved"
        );

        Ok(())
    }

    /// Load all recorded sessions from the history log.
    ///
    /// Returns an empty vector if the history file does not exist.
    /// Malformed lines are logged and skipped, not propagated as errors,
    /// so a single corrupt entry does not prevent loading the rest.
    pub async fn load_sessions(&self) -> Result<Vec<GrowthSession>, Temm1eError> {
        let path = self.history_file();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let contents = tokio::fs::read_to_string(&path).await.map_err(|e| {
            Temm1eError::Tool(format!(
                "failed to read cambium history at {}: {}",
                path.display(),
                e
            ))
        })?;

        let mut sessions = Vec::new();
        for (idx, line) in contents.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<GrowthSession>(trimmed) {
                Ok(session) => sessions.push(session),
                Err(e) => {
                    tracing::warn!(
                        line_number = idx + 1,
                        error = %e,
                        "cambium: skipping malformed history line"
                    );
                }
            }
        }

        Ok(sessions)
    }

    /// Overwrite the trust state file with the given state.
    pub async fn save_trust_state(&self, state: &TrustState) -> Result<(), Temm1eError> {
        self.ensure_base_dir().await?;

        let json = serde_json::to_vec_pretty(state)
            .map_err(|e| Temm1eError::Tool(format!("failed to serialize trust state: {}", e)))?;

        let path = self.trust_state_file();
        tokio::fs::write(&path, json).await.map_err(|e| {
            Temm1eError::Tool(format!(
                "failed to write trust state at {}: {}",
                path.display(),
                e
            ))
        })?;

        tracing::debug!(path = %path.display(), "cambium: trust state saved");
        Ok(())
    }

    /// Load the trust state from disk. Returns `None` if the file does not exist.
    pub async fn load_trust_state(&self) -> Result<Option<TrustState>, Temm1eError> {
        let path = self.trust_state_file();
        if !path.exists() {
            return Ok(None);
        }

        let contents = tokio::fs::read_to_string(&path).await.map_err(|e| {
            Temm1eError::Tool(format!(
                "failed to read trust state at {}: {}",
                path.display(),
                e
            ))
        })?;

        let state: TrustState = serde_json::from_str(&contents).map_err(|e| {
            Temm1eError::Tool(format!(
                "failed to parse trust state at {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(Some(state))
    }

    /// Return the base directory path.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use temm1e_core::types::cambium::{
        GrowthKind, GrowthOutcome, GrowthTrigger, PipelineStage, StageResult,
    };
    use tempfile::tempdir;

    fn sample_session(id: &str) -> GrowthSession {
        GrowthSession {
            id: id.to_string(),
            trigger: GrowthTrigger::Manual {
                description: "test session".into(),
            },
            kind: GrowthKind::NewSkill,
            started_at: Utc::now(),
            completed_at: None,
            stages: vec![(PipelineStage::TriggerValidation, StageResult::Passed)],
            files_changed: vec!["skills/test.md".into()],
            lines_added: 5,
            lines_removed: 0,
            git_branch: Some("cambium/test".into()),
            git_commit: None,
            outcome: GrowthOutcome::InProgress,
            cost_usd: 0.001,
        }
    }

    #[tokio::test]
    async fn load_sessions_returns_empty_when_no_file() {
        let dir = tempdir().expect("tempdir");
        let history = GrowthHistory::new(dir.path().join("cambium"));
        let sessions = history.load_sessions().await.expect("load");
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn save_and_load_single_session_roundtrip() {
        let dir = tempdir().expect("tempdir");
        let history = GrowthHistory::new(dir.path().join("cambium"));

        let session = sample_session("s-001");
        history.save_session(&session).await.expect("save");

        let loaded = history.load_sessions().await.expect("load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "s-001");
    }

    #[tokio::test]
    async fn save_multiple_sessions_are_all_loaded() {
        let dir = tempdir().expect("tempdir");
        let history = GrowthHistory::new(dir.path().join("cambium"));

        for i in 0..5 {
            let session = sample_session(&format!("s-{:03}", i));
            history.save_session(&session).await.expect("save");
        }

        let loaded = history.load_sessions().await.expect("load");
        assert_eq!(loaded.len(), 5);
        for (i, session) in loaded.iter().enumerate() {
            assert_eq!(session.id, format!("s-{:03}", i));
        }
    }

    #[tokio::test]
    async fn save_session_creates_base_dir() {
        let dir = tempdir().expect("tempdir");
        let nested = dir.path().join("nested/deep/cambium");
        let history = GrowthHistory::new(&nested);

        assert!(!nested.exists());
        let session = sample_session("s-001");
        history.save_session(&session).await.expect("save");
        assert!(nested.exists());
    }

    #[tokio::test]
    async fn load_trust_state_returns_none_when_no_file() {
        let dir = tempdir().expect("tempdir");
        let history = GrowthHistory::new(dir.path().join("cambium"));
        let state = history.load_trust_state().await.expect("load");
        assert!(state.is_none());
    }

    #[tokio::test]
    async fn save_and_load_trust_state_roundtrip() {
        let dir = tempdir().expect("tempdir");
        let history = GrowthHistory::new(dir.path().join("cambium"));

        let state = TrustState {
            level3_streak: 5,
            level2_streak: 3,
            level2_autonomous: false,
            level3_autonomous: false,
            recent_rollbacks: 1,
            all_approval_required: false,
            last_session_at: Some(Utc::now()),
            last_failure_at: None,
            sessions_today: 2,
            sessions_today_date: Some("2026-04-08".into()),
        };

        history.save_trust_state(&state).await.expect("save");
        let loaded = history
            .load_trust_state()
            .await
            .expect("load")
            .expect("should be present");

        assert_eq!(loaded.level3_streak, 5);
        assert_eq!(loaded.level2_streak, 3);
        assert_eq!(loaded.recent_rollbacks, 1);
        assert_eq!(loaded.sessions_today, 2);
        assert_eq!(loaded.sessions_today_date.as_deref(), Some("2026-04-08"));
    }

    #[tokio::test]
    async fn save_trust_state_overwrites_existing() {
        let dir = tempdir().expect("tempdir");
        let history = GrowthHistory::new(dir.path().join("cambium"));

        let mut state = TrustState {
            level3_streak: 5,
            ..TrustState::default()
        };
        history.save_trust_state(&state).await.expect("save 1");

        state.level3_streak = 10;
        state.level3_autonomous = true;
        history.save_trust_state(&state).await.expect("save 2");

        let loaded = history
            .load_trust_state()
            .await
            .expect("load")
            .expect("present");
        assert_eq!(loaded.level3_streak, 10);
        assert!(loaded.level3_autonomous);
    }

    #[tokio::test]
    async fn malformed_history_line_is_skipped() {
        let dir = tempdir().expect("tempdir");
        let base = dir.path().join("cambium");
        tokio::fs::create_dir_all(&base).await.expect("mkdir");

        // Write two good lines and one malformed line.
        let session = sample_session("s-001");
        let good_line = serde_json::to_string(&session).expect("serialize");
        let contents = format!("{}\n{{not-json\n{}\n", good_line, good_line);
        tokio::fs::write(base.join("history.jsonl"), contents)
            .await
            .expect("write");

        let history = GrowthHistory::new(base);
        let loaded = history.load_sessions().await.expect("load");
        // The two good lines are loaded; the malformed one is skipped.
        assert_eq!(loaded.len(), 2);
    }

    #[tokio::test]
    async fn empty_lines_in_history_are_ignored() {
        let dir = tempdir().expect("tempdir");
        let base = dir.path().join("cambium");
        tokio::fs::create_dir_all(&base).await.expect("mkdir");

        let session = sample_session("s-001");
        let good_line = serde_json::to_string(&session).expect("serialize");
        let contents = format!("\n\n{}\n\n", good_line);
        tokio::fs::write(base.join("history.jsonl"), contents)
            .await
            .expect("write");

        let history = GrowthHistory::new(base);
        let loaded = history.load_sessions().await.expect("load");
        assert_eq!(loaded.len(), 1);
    }

    #[tokio::test]
    async fn base_dir_accessor_returns_path() {
        let dir = tempdir().expect("tempdir");
        let base = dir.path().join("cambium");
        let history = GrowthHistory::new(&base);
        assert_eq!(history.base_dir(), base.as_path());
    }

    #[tokio::test]
    async fn malformed_trust_state_file_returns_error() {
        let dir = tempdir().expect("tempdir");
        let base = dir.path().join("cambium");
        tokio::fs::create_dir_all(&base).await.expect("mkdir");
        tokio::fs::write(base.join("trust_state.json"), b"{not-valid-json")
            .await
            .expect("write");

        let history = GrowthHistory::new(base);
        let result = history.load_trust_state().await;
        assert!(result.is_err());
    }
}
