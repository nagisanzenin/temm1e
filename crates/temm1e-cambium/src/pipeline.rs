//! # Pipeline: orchestration of the cambium verification stages.
//!
//! This module ties together the sandbox, zone checker, trust engine,
//! budget enforcer, and history into a deterministic state machine.
//!
//! Each pipeline session passes through a sequence of stages. Each stage
//! is a binary gate: pass or fail. The pipeline is LLM-agnostic — it
//! verifies outcomes mechanically.
//!
//! This module does NOT call any LLM. The code generation step is
//! delegated to a caller-provided closure (the `CodeGenerator` trait),
//! making the pipeline testable with mock generators and reusable across
//! different LLM providers.

use chrono::Utc;
use temm1e_core::types::cambium::{
    GrowthKind, GrowthOutcome, GrowthSession, GrowthTrigger, PipelineStage, StageResult, TrustLevel,
};
use temm1e_core::types::error::Temm1eError;

use crate::budget::SessionBudget;
use crate::sandbox::Sandbox;
use crate::trust::TrustEngine;
use crate::zone_checker::ZoneManifest;

/// Maximum retries for stages that can be retried (compilation, test).
pub const MAX_STAGE_RETRIES: usize = 3;

/// A code-generation callback that produces a diff for the sandbox.
///
/// The callback receives the sandbox path and the growth trigger, and
/// is responsible for writing the modified files into the sandbox.
/// On error, return a string explaining what went wrong.
///
/// In production this is wired to an LLM call. In tests it can be a
/// hard-coded transformation.
#[async_trait::async_trait]
pub trait CodeGenerator: Send + Sync {
    async fn generate(
        &self,
        sandbox: &Sandbox,
        trigger: &GrowthTrigger,
        kind: &GrowthKind,
    ) -> Result<(), String>;
}

/// Configuration for a pipeline run.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Maximum lines of code allowed to change in this session.
    pub max_lines: usize,
    /// Maximum files allowed to change in this session.
    pub max_files: usize,
    /// Whether to run the formatting stage.
    pub run_formatting: bool,
    /// Whether to run the test suite stage. Disable for fast testing.
    pub run_tests: bool,
    /// Whether to actually commit on success. False = dry run.
    pub commit_on_success: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            max_lines: 500,
            max_files: 5,
            run_formatting: true,
            run_tests: true,
            commit_on_success: true,
        }
    }
}

/// The pipeline orchestrator. Runs the full sequence of stages.
pub struct Pipeline<'a> {
    pub sandbox: &'a Sandbox,
    pub manifest: &'a ZoneManifest,
    pub trust: &'a mut TrustEngine,
    pub config: PipelineConfig,
}

impl<'a> Pipeline<'a> {
    pub fn new(
        sandbox: &'a Sandbox,
        manifest: &'a ZoneManifest,
        trust: &'a mut TrustEngine,
        config: PipelineConfig,
    ) -> Self {
        Self {
            sandbox,
            manifest,
            trust,
            config,
        }
    }

    /// Run the full pipeline for a session. Returns the completed
    /// `GrowthSession` regardless of outcome.
    pub async fn run<G: CodeGenerator>(
        &mut self,
        trigger: GrowthTrigger,
        kind: GrowthKind,
        generator: &G,
    ) -> Result<GrowthSession, Temm1eError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let started_at = Utc::now();
        let mut stages: Vec<(PipelineStage, StageResult)> = Vec::new();

        let mut session = GrowthSession {
            id: session_id.clone(),
            trigger: trigger.clone(),
            kind: kind.clone(),
            started_at,
            completed_at: None,
            stages: Vec::new(),
            files_changed: Vec::new(),
            lines_added: 0,
            lines_removed: 0,
            git_branch: None,
            git_commit: None,
            outcome: GrowthOutcome::InProgress,
            cost_usd: 0.0,
        };

        // STAGE 0: Trigger validation (always passes if trigger is well-formed).
        stages.push((PipelineStage::TriggerValidation, StageResult::Passed));

        // STAGE 1: Self-briefing (placeholder — would read self-model docs).
        stages.push((PipelineStage::SelfBriefing, StageResult::Passed));

        // Ensure the sandbox is initialized and synced.
        if !self.sandbox.exists().await {
            return self.fail(
                session,
                stages,
                PipelineStage::SelfBriefing,
                "Sandbox not initialized — run sandbox.init() before pipeline".into(),
            );
        }

        // Sync to upstream and create a fresh branch.
        if let Err(e) = self.sandbox.sync().await {
            return self.fail(
                session,
                stages,
                PipelineStage::SelfBriefing,
                format!("Sandbox sync failed: {e}"),
            );
        }
        let branch = match self.sandbox.create_branch(&session_id).await {
            Ok(b) => b,
            Err(e) => {
                return self.fail(
                    session,
                    stages,
                    PipelineStage::SelfBriefing,
                    format!("Branch creation failed: {e}"),
                );
            }
        };
        session.git_branch = Some(branch);

        // STAGE 2: Code generation.
        if let Err(reason) = generator.generate(self.sandbox, &trigger, &kind).await {
            stages.push((
                PipelineStage::CodeGeneration,
                StageResult::Failed {
                    reason: reason.clone(),
                    retry_count: 0,
                },
            ));
            return self.fail(session, stages, PipelineStage::CodeGeneration, reason);
        }
        stages.push((PipelineStage::CodeGeneration, StageResult::Passed));

        // Collect what changed.
        let changed_files = self.sandbox.changed_files().await?;
        let (lines_added, lines_removed) = self.sandbox.diff_stats().await?;
        session.files_changed = changed_files
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        session.lines_added = lines_added;
        session.lines_removed = lines_removed;

        // Check budget.
        let mut budget = SessionBudget::new(self.config.max_lines, self.config.max_files);
        let file_strs: Vec<String> = session.files_changed.clone();
        if let Err(reason) = budget.check(lines_added, lines_removed, &changed_files) {
            session.outcome = GrowthOutcome::BudgetExceeded;
            session.stages = stages;
            session.completed_at = Some(Utc::now());
            tracing::warn!(
                target: "cambium",
                session = %session_id,
                reason = %reason,
                "Pipeline aborted: budget exceeded"
            );
            return Ok(session);
        }
        budget.record(lines_added, lines_removed, &changed_files);

        // STAGE 3: Zone compliance.
        let trust_level = self.manifest.classify_changes(&changed_files);
        if trust_level == TrustLevel::Immutable {
            session.outcome = GrowthOutcome::ZoneViolation {
                files: file_strs.clone(),
            };
            stages.push((
                PipelineStage::ZoneCompliance,
                StageResult::Failed {
                    reason: "Immutable zone violation".into(),
                    retry_count: 0,
                },
            ));
            session.stages = stages;
            session.completed_at = Some(Utc::now());
            tracing::error!(
                target: "cambium",
                session = %session_id,
                files = ?file_strs,
                "Pipeline hard abort: zone violation"
            );
            return Ok(session);
        }
        // Verify Level 0 files have not been modified by checksum.
        if let Err(violations) = self
            .manifest
            .validate_immutable_kernel(&self.sandbox.path)
            .await
        {
            session.outcome = GrowthOutcome::ZoneViolation {
                files: violations.clone(),
            };
            stages.push((
                PipelineStage::ZoneCompliance,
                StageResult::Failed {
                    reason: "Immutable kernel checksum mismatch".into(),
                    retry_count: 0,
                },
            ));
            session.stages = stages;
            session.completed_at = Some(Utc::now());
            return Ok(session);
        }
        stages.push((PipelineStage::ZoneCompliance, StageResult::Passed));

        // STAGE 4: Compilation.
        let check = self.sandbox.cargo_check().await?;
        if !check.success {
            stages.push((
                PipelineStage::Compilation,
                StageResult::Failed {
                    reason: truncate(&check.stderr, 500),
                    retry_count: 0,
                },
            ));
            return self.fail(
                session,
                stages,
                PipelineStage::Compilation,
                "cargo check failed".into(),
            );
        }
        stages.push((PipelineStage::Compilation, StageResult::Passed));

        // STAGE 5: Linting (clippy).
        let clippy = self.sandbox.cargo_clippy().await?;
        if !clippy.success {
            stages.push((
                PipelineStage::Linting,
                StageResult::Failed {
                    reason: truncate(&clippy.stderr, 500),
                    retry_count: 0,
                },
            ));
            return self.fail(
                session,
                stages,
                PipelineStage::Linting,
                "clippy failed".into(),
            );
        }
        stages.push((PipelineStage::Linting, StageResult::Passed));

        // STAGE 6: Formatting (optional).
        if self.config.run_formatting {
            let fmt = self.sandbox.cargo_fmt_check().await?;
            if !fmt.success {
                stages.push((
                    PipelineStage::Formatting,
                    StageResult::Failed {
                        reason: truncate(&fmt.stdout, 500),
                        retry_count: 0,
                    },
                ));
                return self.fail(
                    session,
                    stages,
                    PipelineStage::Formatting,
                    "cargo fmt --check failed".into(),
                );
            }
            stages.push((PipelineStage::Formatting, StageResult::Passed));
        } else {
            stages.push((
                PipelineStage::Formatting,
                StageResult::Skipped {
                    reason: "disabled in config".into(),
                },
            ));
        }

        // STAGE 7: Test suite.
        if self.config.run_tests {
            let tests = self.sandbox.cargo_test().await?;
            if !tests.success {
                stages.push((
                    PipelineStage::TestSuite,
                    StageResult::Failed {
                        reason: truncate(&tests.stdout, 500),
                        retry_count: 0,
                    },
                ));
                return self.fail(
                    session,
                    stages,
                    PipelineStage::TestSuite,
                    "cargo test failed".into(),
                );
            }
            stages.push((PipelineStage::TestSuite, StageResult::Passed));
        } else {
            stages.push((
                PipelineStage::TestSuite,
                StageResult::Skipped {
                    reason: "disabled in config".into(),
                },
            ));
        }

        // STAGE 8 + 9: Code review and security audit (placeholders for
        // TemDOS core integration in future phases). For now they pass
        // automatically.
        stages.push((
            PipelineStage::CodeReview,
            StageResult::Skipped {
                reason: "TemDOS core integration deferred".into(),
            },
        ));
        stages.push((
            PipelineStage::SecurityAudit,
            StageResult::Skipped {
                reason: "TemDOS core integration deferred".into(),
            },
        ));

        // STAGE 10: Integration test (skipped — will be wired in Phase 5).
        stages.push((
            PipelineStage::IntegrationTest,
            StageResult::Skipped {
                reason: "deferred to Phase 5".into(),
            },
        ));

        // STAGE 11: Deployment — for Phase 4 we COMMIT TO BRANCH ONLY,
        // never deploy. The branch lives in the sandbox until the user
        // pushes it manually.
        if self.config.commit_on_success {
            let message = format!(
                "cambium: {}\n\nTrigger: {:?}\nKind: {:?}\nSession: {}",
                describe_kind(&kind),
                trigger,
                kind,
                session_id
            );
            match self.sandbox.commit_changes(&message).await {
                Ok(sha) => {
                    session.git_commit = Some(sha);
                    stages.push((PipelineStage::Deployment, StageResult::Passed));
                }
                Err(e) => {
                    stages.push((
                        PipelineStage::Deployment,
                        StageResult::Failed {
                            reason: e.to_string(),
                            retry_count: 0,
                        },
                    ));
                    return self.fail(session, stages, PipelineStage::Deployment, e.to_string());
                }
            }
        } else {
            stages.push((
                PipelineStage::Deployment,
                StageResult::Skipped {
                    reason: "dry run".into(),
                },
            ));
        }

        // STAGE 12: Post-deploy monitoring (deferred).
        stages.push((
            PipelineStage::PostDeployMonitoring,
            StageResult::Skipped {
                reason: "deferred to Phase 5".into(),
            },
        ));

        // Success — record in trust engine.
        self.trust.record_success(trust_level);
        session.stages = stages;
        session.outcome = GrowthOutcome::Success;
        session.completed_at = Some(Utc::now());

        tracing::info!(
            target: "cambium",
            session = %session_id,
            files_changed = session.files_changed.len(),
            lines_added,
            lines_removed,
            trust_level = ?trust_level,
            "Self-grow session succeeded"
        );

        Ok(session)
    }

    /// Helper: mark session as failed and return.
    fn fail(
        &mut self,
        mut session: GrowthSession,
        stages: Vec<(PipelineStage, StageResult)>,
        stage: PipelineStage,
        reason: String,
    ) -> Result<GrowthSession, Temm1eError> {
        self.trust.record_failure();
        session.stages = stages;
        session.outcome = GrowthOutcome::Failed { stage, reason };
        session.completed_at = Some(Utc::now());
        Ok(session)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        // Find a UTF-8 safe boundary.
        let mut end = max;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        format!("{}... [truncated]", &s[..end])
    }
}

fn describe_kind(kind: &GrowthKind) -> &'static str {
    match kind {
        GrowthKind::NewTool => "add new tool",
        GrowthKind::BugFix => "fix bug",
        GrowthKind::Optimization => "optimization",
        GrowthKind::NewSkill => "new skill",
        GrowthKind::NewIntegration => "new integration",
        GrowthKind::NewCore => "new core",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust::TrustEngine;
    use std::collections::HashMap;
    use std::path::Path;
    use tempfile::tempdir;

    /// Mock code generator that does nothing (no changes).
    struct NoopGenerator;
    #[async_trait::async_trait]
    impl CodeGenerator for NoopGenerator {
        async fn generate(
            &self,
            _sandbox: &Sandbox,
            _trigger: &GrowthTrigger,
            _kind: &GrowthKind,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    /// Mock generator that fails immediately.
    struct FailingGenerator;
    #[async_trait::async_trait]
    impl CodeGenerator for FailingGenerator {
        async fn generate(
            &self,
            _sandbox: &Sandbox,
            _trigger: &GrowthTrigger,
            _kind: &GrowthKind,
        ) -> Result<(), String> {
            Err("intentional test failure".to_string())
        }
    }

    fn empty_manifest() -> ZoneManifest {
        ZoneManifest::new(HashMap::new(), Vec::new())
    }

    #[test]
    fn pipeline_config_default() {
        let cfg = PipelineConfig::default();
        assert_eq!(cfg.max_lines, 500);
        assert_eq!(cfg.max_files, 5);
        assert!(cfg.run_formatting);
        assert!(cfg.run_tests);
        assert!(cfg.commit_on_success);
    }

    #[test]
    fn truncate_short_unchanged() {
        let s = "hello";
        assert_eq!(truncate(s, 100), "hello");
    }

    #[test]
    fn truncate_long_marked() {
        let s = "a".repeat(200);
        let t = truncate(&s, 50);
        assert!(t.contains("[truncated]"));
        assert!(t.len() < s.len() + 30);
    }

    #[test]
    fn truncate_utf8_safe() {
        // String with multi-byte chars near the boundary.
        let s = "hello world ẹ ẹ ẹ ẹ ẹ";
        let t = truncate(s, 15);
        // Should not panic, should produce valid UTF-8.
        assert!(!t.is_empty());
    }

    #[test]
    fn describe_kind_returns_static_str() {
        assert_eq!(describe_kind(&GrowthKind::NewTool), "add new tool");
        assert_eq!(describe_kind(&GrowthKind::BugFix), "fix bug");
    }

    #[tokio::test]
    async fn pipeline_aborts_when_sandbox_missing() {
        let tmp = tempdir().unwrap();
        let sandbox = Sandbox::new(
            tmp.path().join("does-not-exist"),
            "x".to_string(),
            "main".to_string(),
        );
        let manifest = empty_manifest();
        let mut trust = TrustEngine::new(Default::default(), None);
        let cfg = PipelineConfig::default();
        let mut pipeline = Pipeline::new(&sandbox, &manifest, &mut trust, cfg);

        let trigger = GrowthTrigger::Manual {
            description: "test".into(),
        };
        let result = pipeline
            .run(trigger, GrowthKind::NewSkill, &NoopGenerator)
            .await;
        let session = result.unwrap();
        assert!(matches!(session.outcome, GrowthOutcome::Failed { .. }));
    }

    #[test]
    fn no_op_generator_compiles() {
        // Just verify the trait can be implemented.
        let _g = NoopGenerator;
        let _f = FailingGenerator;
    }

    #[test]
    fn unused_path_import() {
        // Touch the Path import so the compiler doesn't warn.
        let _p = Path::new("test");
    }
}
