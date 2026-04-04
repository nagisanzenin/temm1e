//! SQLite persistence for the social intelligence system.
//!
//! Stores user profiles, evaluation logs, facts buffers, and observations.

use crate::types::{TurnFacts, UserProfile};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use temm1e_core::error::Temm1eError;
use tracing::{debug, info};

/// Maximum time allowed for any single database operation.
const DB_TIMEOUT: u64 = 5;

/// SQLite-backed storage for social intelligence data.
pub struct SocialStorage {
    pool: SqlitePool,
}

impl SocialStorage {
    /// Create a new SocialStorage and initialise the schema.
    ///
    /// `db_url` is a SQLite connection string, e.g. `"sqlite:social.db"` or
    /// `"sqlite::memory:"` for an in-memory database.
    pub async fn new(db_url: &str) -> Result<Self, Temm1eError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(db_url)
            .await
            .map_err(|e| Temm1eError::Memory(format!("Failed to connect to social SQLite: {e}")))?;

        // Best-effort WAL mode and busy timeout for concurrent access resilience
        sqlx::query("PRAGMA journal_mode=WAL;")
            .execute(&pool)
            .await
            .ok();
        sqlx::query("PRAGMA busy_timeout=5000;")
            .execute(&pool)
            .await
            .ok();

        let storage = Self { pool };
        storage.init_tables().await?;
        info!("Social intelligence storage initialised");
        Ok(storage)
    }

    /// Create all tables if they don't already exist.
    async fn init_tables(&self) -> Result<(), Temm1eError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS social_user_profile (
                user_id TEXT PRIMARY KEY,
                profile_json TEXT NOT NULL,
                evaluation_count INTEGER DEFAULT 0,
                total_turns INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL,
                last_evaluated_at INTEGER,
                last_message_at INTEGER
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Temm1eError::Memory(format!("Failed to create social_user_profile: {e}")))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS social_evaluation_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                evaluation_json TEXT NOT NULL,
                model_used TEXT NOT NULL,
                tokens_used INTEGER,
                created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Temm1eError::Memory(format!("Failed to create social_evaluation_log: {e}")))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS social_facts_buffer (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                turn_number INTEGER NOT NULL,
                facts_json TEXT NOT NULL,
                message_content TEXT,
                created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Temm1eError::Memory(format!("Failed to create social_facts_buffer: {e}")))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS social_observations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                observation TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Temm1eError::Memory(format!("Failed to create social_observations: {e}")))?;

        // Indexes for common queries
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_social_eval_user ON social_evaluation_log(user_id)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Temm1eError::Memory(format!("Failed to create index: {e}")))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_social_facts_user ON social_facts_buffer(user_id)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Temm1eError::Memory(format!("Failed to create index: {e}")))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_social_obs_user ON social_observations(user_id)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Temm1eError::Memory(format!("Failed to create index: {e}")))?;

        Ok(())
    }

    /// Retrieve a user's social profile, if one exists.
    pub async fn get_profile(&self, user_id: &str) -> Result<Option<UserProfile>, Temm1eError> {
        let row: Option<(String,)> = tokio::time::timeout(
            Duration::from_secs(DB_TIMEOUT),
            sqlx::query_as("SELECT profile_json FROM social_user_profile WHERE user_id = ?")
                .bind(user_id)
                .fetch_optional(&self.pool),
        )
        .await
        .map_err(|_| Temm1eError::Memory("get_profile timed out".to_string()))?
        .map_err(|e| Temm1eError::Memory(format!("get_profile query failed: {e}")))?;

        match row {
            Some((json,)) => match serde_json::from_str::<UserProfile>(&json) {
                Ok(profile) => {
                    debug!(user_id = %user_id, "Retrieved social profile");
                    Ok(Some(profile))
                }
                Err(e) => {
                    tracing::warn!(
                        user_id = %user_id,
                        error = %e,
                        "Failed to parse user profile, returning fresh"
                    );
                    Ok(Some(crate::user_model::new_profile(user_id)))
                }
            },
            None => Ok(None),
        }
    }

    /// Insert or replace a user's social profile.
    pub async fn upsert_profile(&self, profile: &UserProfile) -> Result<(), Temm1eError> {
        let json = serde_json::to_string(profile)
            .map_err(|e| Temm1eError::Memory(format!("Failed to serialize profile: {e}")))?;

        tokio::time::timeout(
            Duration::from_secs(DB_TIMEOUT),
            sqlx::query(
                r#"
                INSERT OR REPLACE INTO social_user_profile
                    (user_id, profile_json, evaluation_count, total_turns, created_at, last_evaluated_at, last_message_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&profile.user_id)
            .bind(&json)
            .bind(profile.evaluation_count as i64)
            .bind(profile.total_turns_analyzed as i64)
            .bind(profile.created_at as i64)
            .bind(profile.last_evaluated_at as i64)
            .bind(profile.last_message_at as i64)
            .execute(&self.pool),
        )
        .await
        .map_err(|_| Temm1eError::Memory("upsert_profile timed out".to_string()))?
        .map_err(|e| Temm1eError::Memory(format!("upsert_profile failed: {e}")))?;

        debug!(user_id = %profile.user_id, "Upserted social profile");
        Ok(())
    }

    /// Delete a user's social profile.
    pub async fn delete_profile(&self, user_id: &str) -> Result<(), Temm1eError> {
        tokio::time::timeout(
            Duration::from_secs(DB_TIMEOUT),
            sqlx::query("DELETE FROM social_user_profile WHERE user_id = ?")
                .bind(user_id)
                .execute(&self.pool),
        )
        .await
        .map_err(|_| Temm1eError::Memory("delete_profile timed out".to_string()))?
        .map_err(|e| Temm1eError::Memory(format!("delete_profile failed: {e}")))?;

        debug!(user_id = %user_id, "Deleted social profile");
        Ok(())
    }

    /// Buffer a set of turn facts for later evaluation.
    pub async fn buffer_facts(
        &self,
        user_id: &str,
        turn: u32,
        facts: &TurnFacts,
        message: &str,
    ) -> Result<(), Temm1eError> {
        let facts_json = serde_json::to_string(facts)
            .map_err(|e| Temm1eError::Memory(format!("Failed to serialize facts: {e}")))?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        tokio::time::timeout(
            Duration::from_secs(DB_TIMEOUT),
            sqlx::query(
                r#"
                INSERT INTO social_facts_buffer (user_id, turn_number, facts_json, message_content, created_at)
                VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(user_id)
            .bind(turn as i64)
            .bind(&facts_json)
            .bind(message)
            .bind(now as i64)
            .execute(&self.pool),
        )
        .await
        .map_err(|_| Temm1eError::Memory("buffer_facts timed out".to_string()))?
        .map_err(|e| Temm1eError::Memory(format!("buffer_facts failed: {e}")))?;

        // Enforce max buffer size (delete oldest if over limit)
        sqlx::query(
            "DELETE FROM social_facts_buffer WHERE user_id = ?1 AND id NOT IN (
                SELECT id FROM social_facts_buffer WHERE user_id = ?1 ORDER BY id DESC LIMIT 30
            )",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .ok(); // Best-effort cleanup

        debug!(user_id = %user_id, turn = turn, "Buffered turn facts");
        Ok(())
    }

    /// Retrieve all buffered facts for a user, ordered by turn number.
    ///
    /// Returns `(TurnFacts, message_content)` pairs.
    pub async fn get_buffered_facts(
        &self,
        user_id: &str,
    ) -> Result<Vec<(TurnFacts, String)>, Temm1eError> {
        let rows: Vec<(String, String)> = tokio::time::timeout(
            Duration::from_secs(DB_TIMEOUT),
            sqlx::query_as(
                r#"
                SELECT facts_json, COALESCE(message_content, '')
                FROM social_facts_buffer
                WHERE user_id = ?
                ORDER BY turn_number ASC
                "#,
            )
            .bind(user_id)
            .fetch_all(&self.pool),
        )
        .await
        .map_err(|_| Temm1eError::Memory("get_buffered_facts timed out".to_string()))?
        .map_err(|e| Temm1eError::Memory(format!("get_buffered_facts failed: {e}")))?;

        let mut result = Vec::with_capacity(rows.len());
        for (facts_json, message) in rows {
            let facts: TurnFacts = serde_json::from_str(&facts_json).map_err(|e| {
                Temm1eError::Memory(format!("Failed to deserialize buffered facts: {e}"))
            })?;
            result.push((facts, message));
        }

        debug!(user_id = %user_id, count = result.len(), "Retrieved buffered facts");
        Ok(result)
    }

    /// Clear all buffered facts for a user (after evaluation).
    pub async fn clear_buffer(&self, user_id: &str) -> Result<(), Temm1eError> {
        tokio::time::timeout(
            Duration::from_secs(DB_TIMEOUT),
            sqlx::query("DELETE FROM social_facts_buffer WHERE user_id = ?")
                .bind(user_id)
                .execute(&self.pool),
        )
        .await
        .map_err(|_| Temm1eError::Memory("clear_buffer timed out".to_string()))?
        .map_err(|e| Temm1eError::Memory(format!("clear_buffer failed: {e}")))?;

        debug!(user_id = %user_id, "Cleared facts buffer");
        Ok(())
    }

    /// Log an evaluation result for audit / debugging.
    pub async fn log_evaluation(
        &self,
        user_id: &str,
        eval_json: &str,
        model: &str,
        tokens: u32,
    ) -> Result<(), Temm1eError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        tokio::time::timeout(
            Duration::from_secs(DB_TIMEOUT),
            sqlx::query(
                r#"
                INSERT INTO social_evaluation_log (user_id, evaluation_json, model_used, tokens_used, created_at)
                VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(user_id)
            .bind(eval_json)
            .bind(model)
            .bind(tokens as i64)
            .bind(now as i64)
            .execute(&self.pool),
        )
        .await
        .map_err(|_| Temm1eError::Memory("log_evaluation timed out".to_string()))?
        .map_err(|e| Temm1eError::Memory(format!("log_evaluation failed: {e}")))?;

        // Keep only last 100 evaluations per user
        sqlx::query(
            "DELETE FROM social_evaluation_log WHERE user_id = ?1 AND id NOT IN (
                SELECT id FROM social_evaluation_log WHERE user_id = ?1 ORDER BY id DESC LIMIT 100
            )",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .ok(); // Best-effort GC

        debug!(user_id = %user_id, model = %model, tokens = tokens, "Logged evaluation");
        Ok(())
    }

    /// Add an observation for a user.
    pub async fn add_observation(
        &self,
        user_id: &str,
        observation: &str,
    ) -> Result<(), Temm1eError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        tokio::time::timeout(
            Duration::from_secs(DB_TIMEOUT),
            sqlx::query(
                r#"
                INSERT INTO social_observations (user_id, observation, created_at)
                VALUES (?, ?, ?)
                "#,
            )
            .bind(user_id)
            .bind(observation)
            .bind(now as i64)
            .execute(&self.pool),
        )
        .await
        .map_err(|_| Temm1eError::Memory("add_observation timed out".to_string()))?
        .map_err(|e| Temm1eError::Memory(format!("add_observation failed: {e}")))?;

        // Keep only last 200 observations per user
        sqlx::query(
            "DELETE FROM social_observations WHERE user_id = ?1 AND id NOT IN (
                SELECT id FROM social_observations WHERE user_id = ?1 ORDER BY id DESC LIMIT 200
            )",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .ok(); // Best-effort GC

        debug!(user_id = %user_id, "Added observation");
        Ok(())
    }

    /// Retrieve recent observations for a user, newest first.
    pub async fn get_observations(
        &self,
        user_id: &str,
        limit: usize,
    ) -> Result<Vec<String>, Temm1eError> {
        let rows: Vec<(String,)> = tokio::time::timeout(
            Duration::from_secs(DB_TIMEOUT),
            sqlx::query_as(
                r#"
                SELECT observation FROM social_observations
                WHERE user_id = ?
                ORDER BY id DESC
                LIMIT ?
                "#,
            )
            .bind(user_id)
            .bind(limit as i64)
            .fetch_all(&self.pool),
        )
        .await
        .map_err(|_| Temm1eError::Memory("get_observations timed out".to_string()))?
        .map_err(|e| Temm1eError::Memory(format!("get_observations failed: {e}")))?;

        let observations: Vec<String> = rows.into_iter().map(|(o,)| o).collect();
        debug!(user_id = %user_id, count = observations.len(), "Retrieved observations");
        Ok(observations)
    }

    /// Run SQLite VACUUM to reclaim disk space. Call periodically (e.g., weekly).
    pub async fn vacuum(&self) -> Result<(), Temm1eError> {
        sqlx::query("VACUUM;")
            .execute(&self.pool)
            .await
            .map_err(|e| Temm1eError::Memory(format!("VACUUM failed: {e}")))?;
        debug!("Social storage VACUUM complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::{collect_interaction_facts, collect_message_facts};
    use crate::types::{RelationshipPhase, TurnFacts};

    async fn test_storage() -> SocialStorage {
        SocialStorage::new("sqlite::memory:").await.unwrap()
    }

    fn sample_profile(user_id: &str) -> UserProfile {
        UserProfile {
            user_id: user_id.to_string(),
            created_at: 1000,
            last_message_at: 2000,
            ..UserProfile::default()
        }
    }

    fn sample_turn_facts(turn: u32) -> TurnFacts {
        TurnFacts {
            turn_number: turn,
            timestamp: 1000 + (turn as u64 * 60),
            user_message: collect_message_facts("Hello, how are you?"),
            tem_response: collect_message_facts("I'm doing great! How can I help?"),
            interaction: collect_interaction_facts(30, turn, false, false, 0),
        }
    }

    #[tokio::test]
    async fn profile_crud() {
        let storage = test_storage().await;

        // Initially no profile
        let result = storage.get_profile("user_1").await.unwrap();
        assert!(result.is_none());

        // Upsert
        let profile = sample_profile("user_1");
        storage.upsert_profile(&profile).await.unwrap();

        // Retrieve
        let retrieved = storage.get_profile("user_1").await.unwrap().unwrap();
        assert_eq!(retrieved.user_id, "user_1");
        assert_eq!(retrieved.created_at, 1000);

        // Update
        let mut updated = retrieved;
        updated.evaluation_count = 5;
        updated.relationship_phase = RelationshipPhase::Calibration;
        storage.upsert_profile(&updated).await.unwrap();

        let retrieved2 = storage.get_profile("user_1").await.unwrap().unwrap();
        assert_eq!(retrieved2.evaluation_count, 5);
        assert_eq!(
            retrieved2.relationship_phase,
            RelationshipPhase::Calibration
        );

        // Delete
        storage.delete_profile("user_1").await.unwrap();
        let result = storage.get_profile("user_1").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn facts_buffer_cycle() {
        let storage = test_storage().await;

        // Buffer some facts
        let facts1 = sample_turn_facts(1);
        let facts2 = sample_turn_facts(2);
        storage
            .buffer_facts("user_1", 1, &facts1, "Hello, how are you?")
            .await
            .unwrap();
        storage
            .buffer_facts("user_1", 2, &facts2, "What can you do?")
            .await
            .unwrap();

        // Retrieve
        let buffered = storage.get_buffered_facts("user_1").await.unwrap();
        assert_eq!(buffered.len(), 2);
        assert_eq!(buffered[0].0.turn_number, 1);
        assert_eq!(buffered[0].1, "Hello, how are you?");
        assert_eq!(buffered[1].0.turn_number, 2);
        assert_eq!(buffered[1].1, "What can you do?");

        // Clear
        storage.clear_buffer("user_1").await.unwrap();
        let buffered = storage.get_buffered_facts("user_1").await.unwrap();
        assert!(buffered.is_empty());
    }

    #[tokio::test]
    async fn facts_buffer_isolation() {
        let storage = test_storage().await;

        let facts = sample_turn_facts(1);
        storage
            .buffer_facts("user_1", 1, &facts, "msg1")
            .await
            .unwrap();
        storage
            .buffer_facts("user_2", 1, &facts, "msg2")
            .await
            .unwrap();

        let user1 = storage.get_buffered_facts("user_1").await.unwrap();
        let user2 = storage.get_buffered_facts("user_2").await.unwrap();
        assert_eq!(user1.len(), 1);
        assert_eq!(user2.len(), 1);
        assert_eq!(user1[0].1, "msg1");
        assert_eq!(user2[0].1, "msg2");
    }

    #[tokio::test]
    async fn evaluation_logging() {
        let storage = test_storage().await;

        storage
            .log_evaluation("user_1", r#"{"test": true}"#, "claude-3-haiku", 150)
            .await
            .unwrap();

        // No getter for eval logs yet — just verify it doesn't error
        storage
            .log_evaluation("user_1", r#"{"test": 2}"#, "claude-3-haiku", 200)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn observations_crud() {
        let storage = test_storage().await;

        storage
            .add_observation("user_1", "User prefers concise answers")
            .await
            .unwrap();
        storage
            .add_observation("user_1", "User is a Rust developer")
            .await
            .unwrap();
        storage
            .add_observation("user_1", "User works late at night")
            .await
            .unwrap();

        // Get with limit
        let obs = storage.get_observations("user_1", 2).await.unwrap();
        assert_eq!(obs.len(), 2);
        // Newest first
        assert_eq!(obs[0], "User works late at night");

        // Get all
        let obs_all = storage.get_observations("user_1", 100).await.unwrap();
        assert_eq!(obs_all.len(), 3);
    }

    #[tokio::test]
    async fn observations_isolation() {
        let storage = test_storage().await;

        storage.add_observation("user_1", "obs1").await.unwrap();
        storage.add_observation("user_2", "obs2").await.unwrap();

        let obs1 = storage.get_observations("user_1", 10).await.unwrap();
        let obs2 = storage.get_observations("user_2", 10).await.unwrap();
        assert_eq!(obs1.len(), 1);
        assert_eq!(obs2.len(), 1);
        assert_eq!(obs1[0], "obs1");
        assert_eq!(obs2[0], "obs2");
    }

    #[tokio::test]
    async fn facts_buffer_hard_limit() {
        let storage = test_storage().await;

        // Insert 35 facts (limit is 30)
        for i in 0..35u32 {
            let facts = sample_turn_facts(i);
            storage
                .buffer_facts("user_1", i, &facts, &format!("msg_{i}"))
                .await
                .unwrap();
        }

        // Should be capped at 30
        let buffered = storage.get_buffered_facts("user_1").await.unwrap();
        assert!(
            buffered.len() <= 30,
            "Expected <= 30, got {}",
            buffered.len()
        );
    }

    #[tokio::test]
    async fn profile_deserialization_resilience() {
        let storage = test_storage().await;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Insert deliberately malformed JSON
        sqlx::query(
            "INSERT INTO social_user_profile (user_id, profile_json, created_at) VALUES (?, ?, ?)",
        )
        .bind("broken_user")
        .bind("{not valid json at all")
        .bind(now as i64)
        .execute(&storage.pool)
        .await
        .unwrap();

        // get_profile should return a fresh profile instead of erroring
        let result = storage.get_profile("broken_user").await.unwrap();
        assert!(result.is_some());
        let profile = result.unwrap();
        assert_eq!(profile.user_id, "broken_user");
        assert_eq!(profile.evaluation_count, 0); // Fresh profile
    }

    #[tokio::test]
    async fn vacuum_succeeds() {
        let storage = test_storage().await;
        storage.vacuum().await.unwrap();
    }
}
