//! Shared types for the Tem Social Intelligence system.

use serde::{Deserialize, Serialize};

// ── Config ────────────────────────────────────────────────────────────

/// Configuration for the social intelligence subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialConfig {
    /// Whether social intelligence is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Number of turns between LLM evaluations.
    #[serde(default = "default_turn_interval")]
    pub turn_interval: u32,

    /// Minimum seconds between evaluations (cooldown).
    #[serde(default = "default_min_interval_seconds")]
    pub min_interval_seconds: u64,

    /// Maximum turns to buffer before forcing an evaluation.
    #[serde(default = "default_max_buffer_turns")]
    pub max_buffer_turns: u32,
}

fn default_enabled() -> bool {
    true
}
fn default_turn_interval() -> u32 {
    5
}
fn default_min_interval_seconds() -> u64 {
    120
}
fn default_max_buffer_turns() -> u32 {
    30
}
fn default_n_next() -> u32 {
    5
}

impl Default for SocialConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            turn_interval: default_turn_interval(),
            min_interval_seconds: default_min_interval_seconds(),
            max_buffer_turns: default_max_buffer_turns(),
        }
    }
}

// ── Per-message facts ─────────────────────────────────────────────────

/// Raw facts extracted from a single message's text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageFacts {
    pub char_count: usize,
    pub word_count: usize,
    pub sentence_count: usize,
    pub question_count: usize,
    pub exclamation_count: usize,
    pub emoji_count: usize,
    pub code_block_count: usize,
    pub uppercase_ratio: f32,
    pub punctuation_density: f32,
    pub avg_sentence_length: f32,
    pub language_detected: String,
    pub contains_greeting: bool,
    pub contains_thanks: bool,
    pub contains_apology: bool,
    pub contains_question: bool,
    pub contains_command: bool,
}

/// Interaction-level facts for a single turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionFacts {
    pub seconds_since_last_message: u64,
    pub session_turn_number: u32,
    pub topic_shifted: bool,
    pub task_completed: bool,
    pub task_failed: bool,
    pub tool_calls_count: u32,
}

/// Combined facts for one conversation turn (user message + Tem response).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnFacts {
    pub turn_number: u32,
    pub timestamp: u64,
    pub user_message: MessageFacts,
    pub tem_response: MessageFacts,
    pub interaction: InteractionFacts,
}

// ── Trait scores ──────────────────────────────────────────────────────

/// A single scored dimension with confidence and provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitScore {
    /// The dimension value (0.0 = low, 1.0 = high).
    pub value: f32,
    /// How confident the system is in this score (0.0-1.0).
    pub confidence: f32,
    /// Number of observations contributing to this score.
    pub observations: u32,
    /// Epoch timestamp of the last update.
    pub last_updated: u64,
    /// Brief reasoning for the current value.
    pub reasoning: String,
}

// ── User profile ──────────────────────────────────────────────────────

/// Complete social profile for a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: String,
    #[serde(default)]
    pub communication_style: CommunicationStyle,
    #[serde(default)]
    pub personality_traits: PersonalityTraits,
    #[serde(default)]
    pub emotional_state: UserEmotionalState,
    #[serde(default)]
    pub trust: TrustModel,
    #[serde(default)]
    pub relationship_phase: RelationshipPhase,
    #[serde(default)]
    pub evaluation_count: u32,
    #[serde(default)]
    pub total_turns_analyzed: u32,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub last_evaluated_at: u64,
    #[serde(default)]
    pub last_message_at: u64,
    #[serde(default)]
    pub observations: Vec<String>,
    #[serde(default)]
    pub recommendations: Recommendations,
    /// Adaptive evaluation interval -- computed after each evaluation.
    #[serde(default = "default_n_next")]
    pub n_next: u32,
    /// Profile delta from last evaluation (average |change| across dimensions).
    #[serde(default)]
    pub last_delta: f32,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            user_id: String::new(),
            communication_style: CommunicationStyle::default(),
            personality_traits: PersonalityTraits::default(),
            emotional_state: UserEmotionalState::default(),
            trust: TrustModel::default(),
            relationship_phase: RelationshipPhase::default(),
            evaluation_count: 0,
            total_turns_analyzed: 0,
            created_at: 0,
            last_evaluated_at: 0,
            last_message_at: 0,
            observations: Vec::new(),
            recommendations: Recommendations::default(),
            n_next: 5,
            last_delta: 0.0,
        }
    }
}

// ── Communication style ───────────────────────────────────────────────

/// How a user communicates — scored dimensions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommunicationStyle {
    pub directness: Option<TraitScore>,
    pub formality: Option<TraitScore>,
    pub analytical_vs_emotional: Option<TraitScore>,
    pub verbosity: Option<TraitScore>,
    pub pace: Option<TraitScore>,
    pub technical_depth: Option<TraitScore>,
}

// ── Personality traits (Big Five) ─────────────────────────────────────

/// Big Five personality dimensions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersonalityTraits {
    pub openness: Option<TraitScore>,
    pub conscientiousness: Option<TraitScore>,
    pub extraversion: Option<TraitScore>,
    pub agreeableness: Option<TraitScore>,
    pub neuroticism: Option<TraitScore>,
}

// ── Emotional state ───────────────────────────────────────────────────

/// Current emotional state of the user (ephemeral, updated each evaluation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEmotionalState {
    pub current_mood: Option<String>,
    #[serde(default)]
    pub confidence: f32,
    #[serde(default)]
    pub reasoning: String,
    #[serde(default)]
    pub stress_level: f32,
    #[serde(default)]
    pub energy_level: f32,
}

impl Default for UserEmotionalState {
    fn default() -> Self {
        Self {
            current_mood: None,
            confidence: 0.0,
            reasoning: String::new(),
            stress_level: 0.0,
            energy_level: 0.5,
        }
    }
}

// ── Trust model ───────────────────────────────────────────────────────

/// How much the user trusts Tem (and vice versa).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustModel {
    /// Current trust level (0.0-1.0).
    #[serde(default = "default_trust_level")]
    pub current_level: f32,
    #[serde(default)]
    pub confidence: f32,
    #[serde(default)]
    pub reasoning: String,
}

fn default_trust_level() -> f32 {
    0.5
}

impl Default for TrustModel {
    fn default() -> Self {
        Self {
            current_level: default_trust_level(),
            confidence: 0.0,
            reasoning: String::new(),
        }
    }
}

// ── Relationship phase ────────────────────────────────────────────────

/// The phase of the relationship between Tem and the user.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum RelationshipPhase {
    /// First interactions — gathering initial signal.
    #[default]
    Discovery,
    /// Actively calibrating communication style.
    Calibration,
    /// Stable, productive relationship.
    Partnership,
    /// High trust, deep understanding.
    DeepPartnership,
}

impl std::fmt::Display for RelationshipPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelationshipPhase::Discovery => write!(f, "Discovery"),
            RelationshipPhase::Calibration => write!(f, "Calibration"),
            RelationshipPhase::Partnership => write!(f, "Partnership"),
            RelationshipPhase::DeepPartnership => write!(f, "Deep Partnership"),
        }
    }
}

// ── Recommendations ───────────────────────────────────────────────────

/// Actionable communication recommendations for Tem.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Recommendations {
    #[serde(default)]
    pub tone: String,
    #[serde(default)]
    pub adapt: String,
    #[serde(default)]
    pub avoid: String,
}

// ── Evaluation output ─────────────────────────────────────────────────

/// Structured output from an LLM evaluation pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationOutput {
    #[serde(default)]
    pub evaluation_id: String,
    #[serde(default)]
    pub turns_analyzed: Vec<u32>,
    #[serde(default)]
    pub communication_style: serde_json::Value,
    #[serde(default)]
    pub emotional_state: serde_json::Value,
    #[serde(default)]
    pub personality_traits: serde_json::Value,
    #[serde(default)]
    pub trust_assessment: serde_json::Value,
    #[serde(default)]
    pub relationship_phase: serde_json::Value,
    #[serde(default)]
    pub tem_self_update: serde_json::Value,
    #[serde(default)]
    pub observations: Vec<String>,
    #[serde(default)]
    pub recommendations: Recommendations,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn social_config_defaults() {
        let config = SocialConfig::default();
        assert!(config.enabled);
        assert_eq!(config.turn_interval, 5);
        assert_eq!(config.min_interval_seconds, 120);
        assert_eq!(config.max_buffer_turns, 30);
    }

    #[test]
    fn social_config_from_toml() {
        let toml_str = r#"
            enabled = false
            turn_interval = 10
        "#;
        let config: SocialConfig = toml::from_str(toml_str).unwrap();
        assert!(!config.enabled);
        assert_eq!(config.turn_interval, 10);
        // defaults for missing fields
        assert_eq!(config.min_interval_seconds, 120);
        assert_eq!(config.max_buffer_turns, 30);
    }

    #[test]
    fn user_profile_default() {
        let profile = UserProfile::default();
        assert!(profile.user_id.is_empty());
        assert_eq!(profile.evaluation_count, 0);
        assert_eq!(profile.relationship_phase, RelationshipPhase::Discovery);
        assert!(profile.observations.is_empty());
        assert_eq!(profile.n_next, 5);
        assert!((profile.last_delta - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn user_profile_serde_roundtrip() {
        let mut profile = UserProfile {
            user_id: "user_123".to_string(),
            relationship_phase: RelationshipPhase::Calibration,
            ..UserProfile::default()
        };
        profile.communication_style.directness = Some(TraitScore {
            value: 0.8,
            confidence: 0.6,
            observations: 3,
            last_updated: 1000,
            reasoning: "Short messages, imperative tone".to_string(),
        });

        let json = serde_json::to_string(&profile).unwrap();
        let restored: UserProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.user_id, "user_123");
        assert!(restored.communication_style.directness.is_some());
        let d = restored.communication_style.directness.unwrap();
        assert!((d.value - 0.8).abs() < f32::EPSILON);
        assert_eq!(restored.relationship_phase, RelationshipPhase::Calibration);
    }

    #[test]
    fn relationship_phase_display() {
        assert_eq!(RelationshipPhase::Discovery.to_string(), "Discovery");
        assert_eq!(
            RelationshipPhase::DeepPartnership.to_string(),
            "Deep Partnership"
        );
    }

    #[test]
    fn evaluation_output_deserialize_partial() {
        let json = r#"{
            "evaluation_id": "eval_001",
            "observations": ["User prefers concise answers"],
            "recommendations": {
                "tone": "direct",
                "adapt": "skip preamble",
                "avoid": "lengthy explanations"
            }
        }"#;
        let eval: EvaluationOutput = serde_json::from_str(json).unwrap();
        assert_eq!(eval.evaluation_id, "eval_001");
        assert_eq!(eval.observations.len(), 1);
        assert_eq!(eval.recommendations.tone, "direct");
        // Missing fields default
        assert!(eval.turns_analyzed.is_empty());
        assert!(eval.communication_style.is_null());
    }

    #[test]
    fn emotional_state_default() {
        let state = UserEmotionalState::default();
        assert!(state.current_mood.is_none());
        assert!((state.confidence - 0.0).abs() < f32::EPSILON);
        assert!((state.energy_level - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn trust_model_default() {
        let trust = TrustModel::default();
        assert!((trust.current_level - 0.5).abs() < f32::EPSILON);
        assert!((trust.confidence - 0.0).abs() < f32::EPSILON);
    }
}
