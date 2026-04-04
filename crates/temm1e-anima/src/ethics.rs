//! Confidence gating constants and helpers.
//!
//! These thresholds determine how much confidence is required before
//! the social intelligence system is allowed to influence different
//! layers of Tem's behavior.

use crate::types::TraitScore;

/// Minimum confidence to apply cosmetic adaptations (e.g., emoji usage).
pub const CONFIDENCE_COSMETIC: f32 = 0.3;

/// Minimum confidence to apply tonal adaptations (e.g., formality level).
pub const CONFIDENCE_TONAL: f32 = 0.5;

/// Minimum confidence to apply behavioral adaptations (e.g., proactive suggestions).
pub const CONFIDENCE_BEHAVIORAL: f32 = 0.7;

/// Minimum confidence to apply relational adaptations (e.g., trust-based shortcuts).
pub const CONFIDENCE_RELATIONAL: f32 = 0.8;

/// Minimum confidence to apply confrontational adaptations (e.g., pushing back on ideas).
pub const CONFIDENCE_CONFRONTATIONAL: f32 = 0.9;

/// Returns true if the trait score exists and its confidence meets or exceeds the threshold.
pub fn above_threshold(score: &Option<TraitScore>, threshold: f32) -> bool {
    score.as_ref().is_some_and(|s| s.confidence >= threshold)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn above_threshold_none() {
        assert!(!above_threshold(&None, CONFIDENCE_COSMETIC));
    }

    #[test]
    fn above_threshold_below() {
        let score = TraitScore {
            value: 0.8,
            confidence: 0.2,
            observations: 1,
            last_updated: 0,
            reasoning: String::new(),
        };
        assert!(!above_threshold(&Some(score), CONFIDENCE_COSMETIC));
    }

    #[test]
    fn above_threshold_exact() {
        let score = TraitScore {
            value: 0.8,
            confidence: 0.3,
            observations: 2,
            last_updated: 0,
            reasoning: String::new(),
        };
        assert!(above_threshold(&Some(score), CONFIDENCE_COSMETIC));
    }

    #[test]
    fn above_threshold_above() {
        let score = TraitScore {
            value: 0.8,
            confidence: 0.95,
            observations: 10,
            last_updated: 0,
            reasoning: String::new(),
        };
        assert!(above_threshold(&Some(score), CONFIDENCE_CONFRONTATIONAL));
    }
}
