use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// What triggered this growth session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GrowthTrigger {
    /// Tem encountered an error/panic in its own code.
    BugDetected {
        error_signature: String,
        occurrences: usize,
    },
    /// User explicitly requested a capability.
    UserRequest {
        description: String,
        chat_id: String,
    },
    /// Distill quality score dropped below threshold.
    QualityDegradation {
        metric: String,
        current: f64,
        threshold: f64,
    },
    /// User correction pattern detected by Anima.
    UserCorrection { pattern: String, frequency: usize },
    /// Manual trigger (developer testing).
    Manual { description: String },
}

/// What kind of change the session aims to produce.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GrowthKind {
    NewTool,
    BugFix,
    Optimization,
    NewSkill,
    NewIntegration,
    NewCore,
}

/// Trust level for a specific modification scope.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TrustLevel {
    /// Level 0: Immutable. Cannot be modified by cambium.
    Immutable = 0,
    /// Level 1: Requires explicit human approval before commit.
    ApprovalRequired = 1,
    /// Level 2: Autonomous with full pipeline (compile+test+review).
    AutonomousFull = 2,
    /// Level 3: Autonomous with basic pipeline (compile only).
    AutonomousBasic = 3,
}

/// Result of a single pipeline stage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StageResult {
    Passed,
    Failed { reason: String, retry_count: usize },
    Skipped { reason: String },
}

/// Stage identifiers for the verification pipeline.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PipelineStage {
    TriggerValidation,
    SelfBriefing,
    CodeGeneration,
    ZoneCompliance,
    Compilation,
    Linting,
    Formatting,
    TestSuite,
    CodeReview,
    SecurityAudit,
    IntegrationTest,
    Deployment,
    PostDeployMonitoring,
}

impl std::fmt::Display for PipelineStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TriggerValidation => write!(f, "trigger_validation"),
            Self::SelfBriefing => write!(f, "self_briefing"),
            Self::CodeGeneration => write!(f, "code_generation"),
            Self::ZoneCompliance => write!(f, "zone_compliance"),
            Self::Compilation => write!(f, "compilation"),
            Self::Linting => write!(f, "linting"),
            Self::Formatting => write!(f, "formatting"),
            Self::TestSuite => write!(f, "test_suite"),
            Self::CodeReview => write!(f, "code_review"),
            Self::SecurityAudit => write!(f, "security_audit"),
            Self::IntegrationTest => write!(f, "integration_test"),
            Self::Deployment => write!(f, "deployment"),
            Self::PostDeployMonitoring => write!(f, "post_deploy_monitoring"),
        }
    }
}

/// Final outcome of a growth session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GrowthOutcome {
    /// Session has not finished yet.
    InProgress,
    /// All stages passed, change committed/deployed.
    Success,
    /// A stage failed after max retries.
    Failed {
        stage: PipelineStage,
        reason: String,
    },
    /// Zone compliance violation -- hard abort.
    ZoneViolation { files: Vec<String> },
    /// Session budget exceeded.
    BudgetExceeded,
    /// Human rejected the change (Level 1 approval denied).
    Rejected,
    /// Rolled back after deployment (post-deploy monitoring failed).
    RolledBack { reason: String },
}

/// A complete record of one cambium session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthSession {
    pub id: String,
    pub trigger: GrowthTrigger,
    pub kind: GrowthKind,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub stages: Vec<(PipelineStage, StageResult)>,
    pub files_changed: Vec<String>,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub git_branch: Option<String>,
    pub git_commit: Option<String>,
    pub outcome: GrowthOutcome,
    pub cost_usd: f64,
}

/// Trust state machine -- tracks earned trust from track record.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TrustState {
    /// Consecutive successful Level 3 changes.
    pub level3_streak: usize,
    /// Consecutive successful Level 2 changes.
    pub level2_streak: usize,
    /// Whether Level 2 has graduated to autonomous.
    pub level2_autonomous: bool,
    /// Whether Level 3 has graduated to autonomous.
    pub level3_autonomous: bool,
    /// Total rollbacks in the last 7 days.
    pub recent_rollbacks: usize,
    /// Whether all levels have been reset to approval-required.
    pub all_approval_required: bool,
    /// Last session timestamp (for cooldown enforcement).
    pub last_session_at: Option<DateTime<Utc>>,
    /// Last failure timestamp (for failure cooldown).
    pub last_failure_at: Option<DateTime<Utc>>,
    /// Total sessions today (for daily limit).
    pub sessions_today: usize,
    /// Date of sessions_today counter (resets daily).
    pub sessions_today_date: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growth_trigger_roundtrip() {
        let trigger = GrowthTrigger::BugDetected {
            error_signature: "utf8_boundary".into(),
            occurrences: 5,
        };
        let json = serde_json::to_string(&trigger).unwrap();
        let back: GrowthTrigger = serde_json::from_str(&json).unwrap();
        assert_eq!(trigger, back);
    }

    #[test]
    fn growth_trigger_user_request_roundtrip() {
        let trigger = GrowthTrigger::UserRequest {
            description: "Monitor Kubernetes pods".into(),
            chat_id: "12345".into(),
        };
        let json = serde_json::to_string(&trigger).unwrap();
        let back: GrowthTrigger = serde_json::from_str(&json).unwrap();
        assert_eq!(trigger, back);
    }

    #[test]
    fn growth_kind_roundtrip() {
        for kind in [
            GrowthKind::NewTool,
            GrowthKind::BugFix,
            GrowthKind::Optimization,
            GrowthKind::NewSkill,
            GrowthKind::NewIntegration,
            GrowthKind::NewCore,
        ] {
            let json = serde_json::to_string(&kind).unwrap();
            let back: GrowthKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, back);
        }
    }

    #[test]
    fn trust_level_ordering() {
        assert!(TrustLevel::Immutable < TrustLevel::ApprovalRequired);
        assert!(TrustLevel::ApprovalRequired < TrustLevel::AutonomousFull);
        assert!(TrustLevel::AutonomousFull < TrustLevel::AutonomousBasic);
    }

    #[test]
    fn trust_level_roundtrip() {
        for level in [
            TrustLevel::Immutable,
            TrustLevel::ApprovalRequired,
            TrustLevel::AutonomousFull,
            TrustLevel::AutonomousBasic,
        ] {
            let json = serde_json::to_string(&level).unwrap();
            let back: TrustLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(level, back);
        }
    }

    #[test]
    fn pipeline_stage_display() {
        assert_eq!(
            PipelineStage::TriggerValidation.to_string(),
            "trigger_validation"
        );
        assert_eq!(PipelineStage::Compilation.to_string(), "compilation");
        assert_eq!(
            PipelineStage::PostDeployMonitoring.to_string(),
            "post_deploy_monitoring"
        );
    }

    #[test]
    fn stage_result_roundtrip() {
        let results = vec![
            StageResult::Passed,
            StageResult::Failed {
                reason: "compilation error".into(),
                retry_count: 2,
            },
            StageResult::Skipped {
                reason: "not applicable".into(),
            },
        ];
        for result in results {
            let json = serde_json::to_string(&result).unwrap();
            let back: StageResult = serde_json::from_str(&json).unwrap();
            assert_eq!(result, back);
        }
    }

    #[test]
    fn growth_outcome_roundtrip() {
        let outcomes = vec![
            GrowthOutcome::InProgress,
            GrowthOutcome::Success,
            GrowthOutcome::Failed {
                stage: PipelineStage::Compilation,
                reason: "type error".into(),
            },
            GrowthOutcome::ZoneViolation {
                files: vec!["crates/temm1e-vault/src/lib.rs".into()],
            },
            GrowthOutcome::BudgetExceeded,
            GrowthOutcome::Rejected,
            GrowthOutcome::RolledBack {
                reason: "health check failed".into(),
            },
        ];
        for outcome in outcomes {
            let json = serde_json::to_string(&outcome).unwrap();
            let back: GrowthOutcome = serde_json::from_str(&json).unwrap();
            assert_eq!(outcome, back);
        }
    }

    #[test]
    fn trust_state_default() {
        let state = TrustState::default();
        assert_eq!(state.level3_streak, 0);
        assert_eq!(state.level2_streak, 0);
        assert!(!state.level2_autonomous);
        assert!(!state.level3_autonomous);
        assert_eq!(state.recent_rollbacks, 0);
        assert!(!state.all_approval_required);
        assert!(state.last_session_at.is_none());
        assert!(state.last_failure_at.is_none());
        assert_eq!(state.sessions_today, 0);
        assert!(state.sessions_today_date.is_none());
    }

    #[test]
    fn trust_state_roundtrip() {
        let state = TrustState {
            level3_streak: 10,
            level2_streak: 5,
            level2_autonomous: false,
            level3_autonomous: true,
            recent_rollbacks: 1,
            all_approval_required: false,
            last_session_at: Some(Utc::now()),
            last_failure_at: None,
            sessions_today: 2,
            sessions_today_date: Some("2026-04-08".into()),
        };
        let json = serde_json::to_string(&state).unwrap();
        let back: TrustState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, back);
    }

    #[test]
    fn growth_session_serializes() {
        let session = GrowthSession {
            id: "test-001".into(),
            trigger: GrowthTrigger::Manual {
                description: "test session".into(),
            },
            kind: GrowthKind::NewSkill,
            started_at: Utc::now(),
            completed_at: None,
            stages: vec![
                (PipelineStage::TriggerValidation, StageResult::Passed),
                (PipelineStage::SelfBriefing, StageResult::Passed),
            ],
            files_changed: vec!["skills/test.md".into()],
            lines_added: 10,
            lines_removed: 0,
            git_branch: Some("cambium/test".into()),
            git_commit: None,
            outcome: GrowthOutcome::InProgress,
            cost_usd: 0.003,
        };
        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains("test-001"));
        assert!(json.contains("NewSkill"));
    }
}
