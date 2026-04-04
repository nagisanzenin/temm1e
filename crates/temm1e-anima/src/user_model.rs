//! Profile management — creation and evaluation scheduling.

use crate::types::{SocialConfig, UserProfile};
use std::time::{SystemTime, UNIX_EPOCH};

/// Create a blank user profile with default values.
pub fn new_profile(user_id: &str) -> UserProfile {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    UserProfile {
        user_id: user_id.to_string(),
        created_at: now,
        last_message_at: now,
        ..UserProfile::default()
    }
}

/// Determine whether it's time to run an LLM evaluation.
///
/// Returns true if enough turns have accumulated AND enough time has
/// elapsed since the last evaluation.
pub fn should_evaluate(
    turn_count: u32,
    last_eval_time: u64,
    now: u64,
    config: &SocialConfig,
) -> bool {
    should_evaluate_raw(
        turn_count,
        last_eval_time,
        now,
        config.turn_interval,
        config.min_interval_seconds,
    )
}

/// Raw version of `should_evaluate` that accepts field values directly.
///
/// Useful when the caller has a different SocialConfig type (e.g., from temm1e-core)
/// with the same fields.
pub fn should_evaluate_raw(
    turn_count: u32,
    last_eval_time: u64,
    now: u64,
    turn_interval: u32,
    min_interval_seconds: u64,
) -> bool {
    turn_count >= turn_interval && (now.saturating_sub(last_eval_time)) >= min_interval_seconds
}

/// Adaptive version that uses profile's computed `n_next` instead of config default.
///
/// Falls back to 5 if `profile_n_next` is 0.
pub fn should_evaluate_adaptive(
    turn_count: u32,
    last_eval_time: u64,
    now: u64,
    profile_n_next: u32,
    min_interval_seconds: u64,
) -> bool {
    let effective_n = if profile_n_next > 0 {
        profile_n_next
    } else {
        5
    };
    turn_count >= effective_n && (now.saturating_sub(last_eval_time)) >= min_interval_seconds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_profile_has_defaults() {
        let profile = new_profile("user_42");
        assert_eq!(profile.user_id, "user_42");
        assert_eq!(profile.evaluation_count, 0);
        assert_eq!(profile.total_turns_analyzed, 0);
        assert!(profile.created_at > 0);
        assert!(profile.observations.is_empty());
        assert!(profile.communication_style.directness.is_none());
    }

    #[test]
    fn should_evaluate_not_enough_turns() {
        let config = SocialConfig::default(); // turn_interval = 5
        assert!(!should_evaluate(3, 0, 300, &config));
    }

    #[test]
    fn should_evaluate_not_enough_time() {
        let config = SocialConfig::default(); // min_interval_seconds = 120
        assert!(!should_evaluate(5, 100, 200, &config)); // only 100s elapsed
    }

    #[test]
    fn should_evaluate_both_conditions_met() {
        let config = SocialConfig::default();
        assert!(should_evaluate(5, 0, 120, &config));
    }

    #[test]
    fn should_evaluate_exact_boundaries() {
        let config = SocialConfig::default();
        // Exactly at turn_interval and exactly at min_interval_seconds
        assert!(should_evaluate(5, 0, 120, &config));
    }

    #[test]
    fn should_evaluate_custom_config() {
        let config = SocialConfig {
            enabled: true,
            turn_interval: 10,
            min_interval_seconds: 300,
            max_buffer_turns: 30,
        };
        assert!(!should_evaluate(9, 0, 300, &config)); // not enough turns
        assert!(!should_evaluate(10, 0, 299, &config)); // not enough time
        assert!(should_evaluate(10, 0, 300, &config)); // both met
    }

    #[test]
    fn should_evaluate_handles_zero_last_eval() {
        let config = SocialConfig::default();
        // First evaluation ever — last_eval_time = 0
        assert!(should_evaluate(5, 0, 120, &config));
    }

    #[test]
    fn should_evaluate_adaptive_uses_profile_n() {
        // Profile says n_next=10, so 9 turns is not enough
        assert!(!should_evaluate_adaptive(9, 0, 300, 10, 120));
        // 10 turns is enough
        assert!(should_evaluate_adaptive(10, 0, 300, 10, 120));
    }

    #[test]
    fn should_evaluate_adaptive_fallback_on_zero() {
        // n_next=0 falls back to 5
        assert!(!should_evaluate_adaptive(4, 0, 300, 0, 120));
        assert!(should_evaluate_adaptive(5, 0, 300, 0, 120));
    }

    #[test]
    fn should_evaluate_adaptive_respects_cooldown() {
        // Enough turns, but not enough time
        assert!(!should_evaluate_adaptive(10, 100, 200, 10, 120));
        // Enough turns AND enough time
        assert!(should_evaluate_adaptive(10, 100, 220, 10, 120));
    }
}
