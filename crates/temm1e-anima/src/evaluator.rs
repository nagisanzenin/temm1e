//! LLM evaluation prompt builder and output parser.
//!
//! Builds structured prompts for the LLM to evaluate user interaction patterns,
//! then parses the structured JSON response and merges it into the user profile.

use crate::types::{CommunicationStyle, EvaluationOutput, TraitScore, UserProfile};
use std::time::{SystemTime, UNIX_EPOCH};
use temm1e_core::error::Temm1eError;

use crate::types::TurnFacts;

/// Build the system prompt and user prompt for an LLM evaluation pass.
///
/// Returns `(system_prompt, user_prompt)`.
pub fn build_evaluation_prompt(
    current_profile: &UserProfile,
    facts_buffer: &[(TurnFacts, String)],
    personality_name: &str,
) -> (String, String) {
    let system_prompt = format!(
        r#"You are {personality_name}'s Social Intelligence module. Your task is to analyze recent user interactions and update the user's behavioral profile.

RULES:
1. Return ONLY dimensions where you have MEANINGFUL evidence from the provided turns. Do not guess.
2. Include a confidence score (0.0-1.0) for EVERY assessment you make.
3. Include brief reasoning (1-2 sentences) for every update.
4. Do NOT speculate beyond what the evidence supports.
5. Do NOT pathologize — you are reading communication preferences, not diagnosing conditions.
6. User mood should NEVER influence work quality — it shapes COMMUNICATION STYLE ONLY.
7. If a dimension has insufficient evidence, OMIT it entirely from your response.

Return a JSON object matching this schema:
{{
  "evaluation_id": "<unique string>",
  "turns_analyzed": [<turn numbers>],
  "communication_style": {{
    "directness": {{"value": 0.0-1.0, "confidence": 0.0-1.0, "reasoning": "..."}},
    "formality": {{"value": 0.0-1.0, "confidence": 0.0-1.0, "reasoning": "..."}},
    "analytical_vs_emotional": {{"value": 0.0-1.0, "confidence": 0.0-1.0, "reasoning": "..."}},
    "verbosity": {{"value": 0.0-1.0, "confidence": 0.0-1.0, "reasoning": "..."}},
    "pace": {{"value": 0.0-1.0, "confidence": 0.0-1.0, "reasoning": "..."}},
    "technical_depth": {{"value": 0.0-1.0, "confidence": 0.0-1.0, "reasoning": "..."}}
  }},
  "emotional_state": {{
    "current_mood": "...",
    "confidence": 0.0-1.0,
    "reasoning": "...",
    "stress_level": 0.0-1.0,
    "energy_level": 0.0-1.0
  }},
  "personality_traits": {{
    "openness": {{"value": 0.0-1.0, "confidence": 0.0-1.0, "reasoning": "..."}},
    "conscientiousness": {{"value": 0.0-1.0, "confidence": 0.0-1.0, "reasoning": "..."}},
    "extraversion": {{"value": 0.0-1.0, "confidence": 0.0-1.0, "reasoning": "..."}},
    "agreeableness": {{"value": 0.0-1.0, "confidence": 0.0-1.0, "reasoning": "..."}},
    "neuroticism": {{"value": 0.0-1.0, "confidence": 0.0-1.0, "reasoning": "..."}}
  }},
  "trust_assessment": {{
    "current_level": 0.0-1.0,
    "confidence": 0.0-1.0,
    "reasoning": "..."
  }},
  "relationship_phase": "Discovery|Calibration|Partnership|DeepPartnership",
  "observations": ["observation 1", "observation 2"],
  "recommendations": {{
    "tone": "...",
    "adapt": "...",
    "avoid": "..."
  }}
}}

Only include dimensions where you have evidence. Omit any dimension object entirely if insufficient data."#,
    );

    // Build user prompt with current state + facts
    let profile_json = serde_json::to_string_pretty(current_profile).unwrap_or_default();

    let mut turns_section = String::new();
    for (facts, message) in facts_buffer {
        turns_section.push_str(&format!(
            "\n--- Turn {} (t={}) ---\nUser message: {}\nUser facts: word_count={}, question_count={}, exclamation_count={}, uppercase_ratio={:.2}, contains_greeting={}, contains_thanks={}, contains_command={}\nTem response facts: word_count={}, sentence_count={}\nInteraction: seconds_since_last={}, tool_calls={}, task_completed={}, task_failed={}\n",
            facts.turn_number,
            facts.timestamp,
            message,
            facts.user_message.word_count,
            facts.user_message.question_count,
            facts.user_message.exclamation_count,
            facts.user_message.uppercase_ratio,
            facts.user_message.contains_greeting,
            facts.user_message.contains_thanks,
            facts.user_message.contains_command,
            facts.tem_response.word_count,
            facts.tem_response.sentence_count,
            facts.interaction.seconds_since_last_message,
            facts.interaction.tool_calls_count,
            facts.interaction.task_completed,
            facts.interaction.task_failed,
        ));
    }

    let user_prompt = format!(
        r#"CURRENT PROFILE STATE:
{profile_json}

RECENT INTERACTIONS ({count} turns):
{turns_section}

Analyze these interactions and return your evaluation as JSON."#,
        count = facts_buffer.len(),
    );

    (system_prompt, user_prompt)
}

/// Parse the LLM's JSON response into an `EvaluationOutput`.
///
/// Handles responses wrapped in markdown code fences (```json ... ```).
pub fn parse_evaluation_output(response: &str) -> Result<EvaluationOutput, Temm1eError> {
    // Strip markdown code fences if present
    let json_str = extract_json_from_response(response);

    serde_json::from_str::<EvaluationOutput>(json_str)
        .map_err(|e| Temm1eError::Tool(format!("Failed to parse evaluation output: {e}")))
}

/// Extract JSON from a response that may be wrapped in markdown code fences.
fn extract_json_from_response(response: &str) -> &str {
    let trimmed = response.trim();

    // Try to find ```json ... ``` or ``` ... ```
    if let Some(start_idx) = trimmed.find("```") {
        let after_fence = &trimmed[start_idx + 3..];
        // Skip the optional language tag (e.g., "json")
        let content_start = after_fence.find('\n').map(|i| i + 1).unwrap_or(0);
        let content = &after_fence[content_start..];

        if let Some(end_idx) = content.find("```") {
            return content[..end_idx].trim();
        }
    }

    // No fences — assume the entire response is JSON
    trimmed
}

/// Merge an evaluation result into an existing user profile.
///
/// - Dimensions with values are merged using weighted averaging.
/// - Null/missing values are skipped; unobserved dimensions decay 5% confidence.
/// - Observations are appended (capped at 50).
/// - Recommendations replace previous values.
/// - Computes profile delta and adaptive N for next evaluation interval.
pub fn apply_evaluation(profile: &mut UserProfile, eval: &EvaluationOutput) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let turns_analyzed = eval.turns_analyzed.len() as u32;
    let eval_count = profile.evaluation_count;

    // Snapshot key values before merge for delta computation
    let old_directness = profile
        .communication_style
        .directness
        .as_ref()
        .map(|t| t.value)
        .unwrap_or(0.5);
    let old_formality = profile
        .communication_style
        .formality
        .as_ref()
        .map(|t| t.value)
        .unwrap_or(0.5);
    let old_verbosity = profile
        .communication_style
        .verbosity
        .as_ref()
        .map(|t| t.value)
        .unwrap_or(0.5);
    let old_pace = profile
        .communication_style
        .pace
        .as_ref()
        .map(|t| t.value)
        .unwrap_or(0.5);
    let old_tech = profile
        .communication_style
        .technical_depth
        .as_ref()
        .map(|t| t.value)
        .unwrap_or(0.5);
    let old_analytical = profile
        .communication_style
        .analytical_vs_emotional
        .as_ref()
        .map(|t| t.value)
        .unwrap_or(0.5);
    let old_trust = profile.trust.current_level;

    // Merge communication style (returns which dimensions were updated)
    let comm_updated = merge_comm_style_from_json(
        &mut profile.communication_style,
        &eval.communication_style,
        turns_analyzed,
        eval_count,
    );

    // Merge emotional state
    if !eval.emotional_state.is_null() {
        if let Ok(state) = serde_json::from_value(eval.emotional_state.clone()) {
            profile.emotional_state = state;
        }
    }

    // Merge personality traits (returns which dimensions were updated)
    let personality_updated = merge_personality_from_json(
        &mut profile.personality_traits,
        &eval.personality_traits,
        turns_analyzed,
        eval_count,
    );

    // Merge trust assessment
    let mut trust_updated = false;
    if !eval.trust_assessment.is_null() {
        if let Some(level) = eval
            .trust_assessment
            .get("current_level")
            .and_then(|v| v.as_f64())
        {
            let eval_confidence = eval
                .trust_assessment
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5) as f32;

            profile.trust.current_level = weighted_merge_v2(
                profile.trust.current_level,
                profile.trust.confidence > 0.0,
                level as f32,
                eval_confidence,
                turns_analyzed,
                eval_count,
            );
            profile.trust.confidence = eval_confidence;
            trust_updated = true;

            if let Some(reasoning) = eval
                .trust_assessment
                .get("reasoning")
                .and_then(|v| v.as_str())
            {
                profile.trust.reasoning = reasoning.to_string();
            }
        }
    }

    // Merge relationship phase
    if !eval.relationship_phase.is_null() {
        if let Some(phase_str) = eval.relationship_phase.as_str() {
            if let Ok(phase) =
                serde_json::from_value(serde_json::Value::String(phase_str.to_string()))
            {
                profile.relationship_phase = phase;
            }
        }
    }

    // Append observations (cap at 50, drop oldest)
    for obs in &eval.observations {
        profile.observations.push(obs.clone());
    }
    if profile.observations.len() > 50 {
        let excess = profile.observations.len() - 50;
        profile.observations.drain(..excess);
    }

    // Replace recommendations
    if !eval.recommendations.tone.is_empty()
        || !eval.recommendations.adapt.is_empty()
        || !eval.recommendations.avoid.is_empty()
    {
        profile.recommendations = eval.recommendations.clone();
    }

    // Update metadata
    profile.evaluation_count += 1;
    profile.last_evaluated_at = now;
    profile.total_turns_analyzed += turns_analyzed;

    // ── Compute profile delta (average |change| across updated dimensions) ──
    let new_directness = profile
        .communication_style
        .directness
        .as_ref()
        .map(|t| t.value)
        .unwrap_or(0.5);
    let new_formality = profile
        .communication_style
        .formality
        .as_ref()
        .map(|t| t.value)
        .unwrap_or(0.5);
    let new_verbosity = profile
        .communication_style
        .verbosity
        .as_ref()
        .map(|t| t.value)
        .unwrap_or(0.5);
    let new_pace = profile
        .communication_style
        .pace
        .as_ref()
        .map(|t| t.value)
        .unwrap_or(0.5);
    let new_tech = profile
        .communication_style
        .technical_depth
        .as_ref()
        .map(|t| t.value)
        .unwrap_or(0.5);
    let new_analytical = profile
        .communication_style
        .analytical_vs_emotional
        .as_ref()
        .map(|t| t.value)
        .unwrap_or(0.5);

    let deltas = [
        (old_directness - new_directness).abs(),
        (old_formality - new_formality).abs(),
        (old_verbosity - new_verbosity).abs(),
        (old_pace - new_pace).abs(),
        (old_tech - new_tech).abs(),
        (old_analytical - new_analytical).abs(),
        (old_trust - profile.trust.current_level).abs(),
    ];
    let active_deltas: Vec<f32> = deltas.iter().filter(|d| **d > 0.001).copied().collect();
    profile.last_delta = if active_deltas.is_empty() {
        0.0
    } else {
        active_deltas.iter().sum::<f32>() / active_deltas.len() as f32
    };

    // ── Adaptive N: convergence-driven ──────────────────────────────────
    const N_MIN: u32 = 5;
    const N_MAX: u32 = 30;
    const RESET_THRESHOLD: f32 = 0.15;

    if profile.last_delta > RESET_THRESHOLD {
        profile.n_next = N_MIN; // Behavioral shift detected -- evaluate frequently
    } else {
        let stability = 1.0 + (1.0 + profile.evaluation_count as f32).ln();
        profile.n_next = ((N_MIN as f32 * stability) as u32).clamp(N_MIN, N_MAX);
    }

    // ── Confidence decay: dimensions NOT updated lose 5% confidence ─────
    decay_unobserved_confidence(
        &mut profile.communication_style.directness,
        comm_updated.directness,
    );
    decay_unobserved_confidence(
        &mut profile.communication_style.formality,
        comm_updated.formality,
    );
    decay_unobserved_confidence(
        &mut profile.communication_style.analytical_vs_emotional,
        comm_updated.analytical_vs_emotional,
    );
    decay_unobserved_confidence(
        &mut profile.communication_style.verbosity,
        comm_updated.verbosity,
    );
    decay_unobserved_confidence(&mut profile.communication_style.pace, comm_updated.pace);
    decay_unobserved_confidence(
        &mut profile.communication_style.technical_depth,
        comm_updated.technical_depth,
    );
    decay_unobserved_confidence(
        &mut profile.personality_traits.openness,
        personality_updated.openness,
    );
    decay_unobserved_confidence(
        &mut profile.personality_traits.conscientiousness,
        personality_updated.conscientiousness,
    );
    decay_unobserved_confidence(
        &mut profile.personality_traits.extraversion,
        personality_updated.extraversion,
    );
    decay_unobserved_confidence(
        &mut profile.personality_traits.agreeableness,
        personality_updated.agreeableness,
    );
    decay_unobserved_confidence(
        &mut profile.personality_traits.neuroticism,
        personality_updated.neuroticism,
    );
    // Trust confidence decay (if trust was not updated this eval)
    if !trust_updated && profile.trust.confidence > 0.0 {
        profile.trust.confidence *= 0.95;
        if profile.trust.confidence < 0.1 {
            profile.trust.confidence = 0.0;
        }
    }
}

/// Decay confidence on a trait dimension that was not observed in the current evaluation.
/// Reduces confidence by 5%. Zeros out if below 0.1.
fn decay_unobserved_confidence(dim: &mut Option<TraitScore>, was_updated: bool) {
    if !was_updated {
        if let Some(ref mut score) = dim {
            score.confidence *= 0.95;
            if score.confidence < 0.1 {
                score.confidence = 0.0;
            }
        }
    }
}

/// Improved merge that weights by evidence strength and adapts with maturity.
///
/// `evidence_strength = confidence * min(1.0, turns_analyzed / 10.0)`
/// `merge_rate = 0.4 / (1 + 0.1 * eval_count)`
/// `weight = evidence_strength * merge_rate`
fn weighted_merge_v2(
    old_value: f32,
    has_prior: bool,
    eval_value: f32,
    eval_confidence: f32,
    turns_analyzed: u32,
    eval_count: u32,
) -> f32 {
    if !has_prior {
        return eval_value;
    }
    let evidence_strength = eval_confidence * (turns_analyzed as f32 / 10.0).min(1.0);
    let merge_rate = 0.4 / (1.0 + 0.1 * eval_count as f32);
    let weight = evidence_strength * merge_rate;
    old_value * (1.0 - weight) + eval_value * weight
}

/// Merge a TraitScore from a JSON value into an existing Option<TraitScore>.
///
/// Returns `true` if the dimension was updated (i.e. JSON had a valid value).
fn merge_trait_score(
    existing: &mut Option<TraitScore>,
    json_value: &serde_json::Value,
    turns_analyzed: u32,
    eval_count: u32,
) -> bool {
    if json_value.is_null() {
        return false;
    }

    let eval_value = match json_value.get("value").and_then(|v| v.as_f64()) {
        Some(v) => v as f32,
        None => return false,
    };

    let eval_confidence = json_value
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5) as f32;

    let reasoning = json_value
        .get("reasoning")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    match existing {
        Some(ref mut score) => {
            score.value = weighted_merge_v2(
                score.value,
                true,
                eval_value,
                eval_confidence,
                turns_analyzed,
                eval_count,
            );
            score.confidence = eval_confidence;
            score.observations += 1;
            score.last_updated = now;
            if !reasoning.is_empty() {
                score.reasoning = reasoning;
            }
        }
        None => {
            *existing = Some(TraitScore {
                value: eval_value,
                confidence: eval_confidence,
                observations: 1,
                last_updated: now,
                reasoning,
            });
        }
    }
    true
}

/// Tracks which communication style dimensions were updated during a merge.
#[derive(Debug, Default)]
struct CommStyleUpdated {
    directness: bool,
    formality: bool,
    analytical_vs_emotional: bool,
    verbosity: bool,
    pace: bool,
    technical_depth: bool,
}

/// Merge communication style dimensions from a JSON value.
/// Returns which dimensions were updated for confidence decay tracking.
fn merge_comm_style_from_json(
    style: &mut CommunicationStyle,
    json: &serde_json::Value,
    turns_analyzed: u32,
    eval_count: u32,
) -> CommStyleUpdated {
    let mut updated = CommStyleUpdated::default();
    if json.is_null() {
        return updated;
    }
    if let Some(v) = json.get("directness") {
        updated.directness =
            merge_trait_score(&mut style.directness, v, turns_analyzed, eval_count);
    }
    if let Some(v) = json.get("formality") {
        updated.formality = merge_trait_score(&mut style.formality, v, turns_analyzed, eval_count);
    }
    if let Some(v) = json.get("analytical_vs_emotional") {
        updated.analytical_vs_emotional = merge_trait_score(
            &mut style.analytical_vs_emotional,
            v,
            turns_analyzed,
            eval_count,
        );
    }
    if let Some(v) = json.get("verbosity") {
        updated.verbosity = merge_trait_score(&mut style.verbosity, v, turns_analyzed, eval_count);
    }
    if let Some(v) = json.get("pace") {
        updated.pace = merge_trait_score(&mut style.pace, v, turns_analyzed, eval_count);
    }
    if let Some(v) = json.get("technical_depth") {
        updated.technical_depth =
            merge_trait_score(&mut style.technical_depth, v, turns_analyzed, eval_count);
    }
    updated
}

/// Tracks which personality trait dimensions were updated during a merge.
#[derive(Debug, Default)]
struct PersonalityUpdated {
    openness: bool,
    conscientiousness: bool,
    extraversion: bool,
    agreeableness: bool,
    neuroticism: bool,
}

/// Merge personality trait dimensions from a JSON value.
/// Returns which dimensions were updated for confidence decay tracking.
fn merge_personality_from_json(
    traits: &mut crate::types::PersonalityTraits,
    json: &serde_json::Value,
    turns_analyzed: u32,
    eval_count: u32,
) -> PersonalityUpdated {
    let mut updated = PersonalityUpdated::default();
    if json.is_null() {
        return updated;
    }
    if let Some(v) = json.get("openness") {
        updated.openness = merge_trait_score(&mut traits.openness, v, turns_analyzed, eval_count);
    }
    if let Some(v) = json.get("conscientiousness") {
        updated.conscientiousness =
            merge_trait_score(&mut traits.conscientiousness, v, turns_analyzed, eval_count);
    }
    if let Some(v) = json.get("extraversion") {
        updated.extraversion =
            merge_trait_score(&mut traits.extraversion, v, turns_analyzed, eval_count);
    }
    if let Some(v) = json.get("agreeableness") {
        updated.agreeableness =
            merge_trait_score(&mut traits.agreeableness, v, turns_analyzed, eval_count);
    }
    if let Some(v) = json.get("neuroticism") {
        updated.neuroticism =
            merge_trait_score(&mut traits.neuroticism, v, turns_analyzed, eval_count);
    }
    updated
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Recommendations, RelationshipPhase, UserProfile};

    #[test]
    fn parse_clean_json() {
        let json = r#"{
            "evaluation_id": "eval_001",
            "turns_analyzed": [1, 2, 3],
            "communication_style": {
                "directness": {"value": 0.8, "confidence": 0.6, "reasoning": "Short messages"},
                "formality": {"value": 0.3, "confidence": 0.5, "reasoning": "Casual language"}
            },
            "emotional_state": {
                "current_mood": "focused",
                "confidence": 0.7,
                "reasoning": "Task-oriented messages",
                "stress_level": 0.2,
                "energy_level": 0.8
            },
            "personality_traits": {},
            "trust_assessment": {
                "current_level": 0.6,
                "confidence": 0.4,
                "reasoning": "Early interactions"
            },
            "relationship_phase": "Calibration",
            "observations": ["User prefers concise answers", "Technical background"],
            "recommendations": {
                "tone": "direct and technical",
                "adapt": "skip preamble, lead with answers",
                "avoid": "lengthy explanations"
            }
        }"#;

        let eval = parse_evaluation_output(json).unwrap();
        assert_eq!(eval.evaluation_id, "eval_001");
        assert_eq!(eval.turns_analyzed, vec![1, 2, 3]);
        assert_eq!(eval.observations.len(), 2);
        assert_eq!(eval.recommendations.tone, "direct and technical");
    }

    #[test]
    fn parse_json_with_code_fences() {
        let response = r#"Here is my analysis:

```json
{
    "evaluation_id": "eval_002",
    "turns_analyzed": [4, 5],
    "communication_style": {},
    "emotional_state": {},
    "personality_traits": {},
    "trust_assessment": {},
    "relationship_phase": "Discovery",
    "observations": ["New user"],
    "recommendations": {
        "tone": "warm",
        "adapt": "be patient",
        "avoid": "jargon"
    }
}
```

That's my evaluation."#;

        let eval = parse_evaluation_output(response).unwrap();
        assert_eq!(eval.evaluation_id, "eval_002");
        assert_eq!(eval.observations, vec!["New user"]);
    }

    #[test]
    fn parse_invalid_json() {
        let result = parse_evaluation_output("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn parse_partial_json() {
        // Missing many fields — should still work due to serde defaults
        let json = r#"{"evaluation_id": "partial"}"#;
        let eval = parse_evaluation_output(json).unwrap();
        assert_eq!(eval.evaluation_id, "partial");
        assert!(eval.turns_analyzed.is_empty());
        assert!(eval.observations.is_empty());
    }

    #[test]
    fn apply_evaluation_first_time() {
        let mut profile = UserProfile {
            user_id: "user_1".to_string(),
            ..UserProfile::default()
        };

        let eval = EvaluationOutput {
            evaluation_id: "eval_001".to_string(),
            turns_analyzed: vec![1, 2, 3],
            communication_style: serde_json::json!({
                "directness": {"value": 0.8, "confidence": 0.6, "reasoning": "Short msgs"}
            }),
            emotional_state: serde_json::json!({
                "current_mood": "focused",
                "confidence": 0.7,
                "reasoning": "Task-oriented",
                "stress_level": 0.2,
                "energy_level": 0.8
            }),
            personality_traits: serde_json::Value::Null,
            trust_assessment: serde_json::json!({
                "current_level": 0.6,
                "confidence": 0.4,
                "reasoning": "Early interactions"
            }),
            relationship_phase: serde_json::json!("Calibration"),
            tem_self_update: serde_json::Value::Null,
            observations: vec!["User is direct".to_string()],
            recommendations: Recommendations {
                tone: "direct".to_string(),
                adapt: "skip preamble".to_string(),
                avoid: "fluff".to_string(),
            },
        };

        apply_evaluation(&mut profile, &eval);

        // First time — value adopted directly
        assert!(profile.communication_style.directness.is_some());
        let d = profile.communication_style.directness.as_ref().unwrap();
        assert!((d.value - 0.8).abs() < f32::EPSILON);
        assert!((d.confidence - 0.6).abs() < f32::EPSILON);

        assert_eq!(
            profile.emotional_state.current_mood.as_deref(),
            Some("focused")
        );
        assert_eq!(profile.relationship_phase, RelationshipPhase::Calibration);
        assert_eq!(profile.evaluation_count, 1);
        assert_eq!(profile.total_turns_analyzed, 3);
        assert_eq!(profile.observations, vec!["User is direct"]);
        assert_eq!(profile.recommendations.tone, "direct");
    }

    #[test]
    fn apply_evaluation_weighted_merge_v2() {
        let mut profile = UserProfile {
            user_id: "user_1".to_string(),
            ..UserProfile::default()
        };

        // Set initial value
        profile.communication_style.directness = Some(TraitScore {
            value: 0.5,
            confidence: 0.4,
            observations: 2,
            last_updated: 1000,
            reasoning: "Initial".to_string(),
        });

        let eval = EvaluationOutput {
            evaluation_id: "eval_002".to_string(),
            turns_analyzed: vec![4, 5],
            communication_style: serde_json::json!({
                "directness": {"value": 0.9, "confidence": 0.7, "reasoning": "Very direct"}
            }),
            emotional_state: serde_json::Value::Null,
            personality_traits: serde_json::Value::Null,
            trust_assessment: serde_json::Value::Null,
            relationship_phase: serde_json::Value::Null,
            tem_self_update: serde_json::Value::Null,
            observations: vec![],
            recommendations: Recommendations::default(),
        };

        apply_evaluation(&mut profile, &eval);

        // weighted_merge_v2 with turns_analyzed=2, eval_count=0 (first eval):
        // evidence_strength = 0.7 * min(1.0, 2/10) = 0.7 * 0.2 = 0.14
        // merge_rate = 0.4 / (1.0 + 0.1 * 0) = 0.4
        // weight = 0.14 * 0.4 = 0.056
        // merged = 0.5 * (1 - 0.056) + 0.9 * 0.056 = 0.5 * 0.944 + 0.0504
        //        = 0.472 + 0.0504 = 0.5224
        let d = profile.communication_style.directness.as_ref().unwrap();
        assert!(
            (d.value - 0.5224).abs() < 0.01,
            "Expected ~0.5224, got {}",
            d.value
        );
        assert_eq!(d.observations, 3);
    }

    #[test]
    fn observations_capped_at_50() {
        let mut profile = UserProfile {
            user_id: "user_1".to_string(),
            observations: (0..48).map(|i| format!("obs_{i}")).collect(),
            ..UserProfile::default()
        };

        let eval = EvaluationOutput {
            evaluation_id: "eval".to_string(),
            turns_analyzed: vec![1],
            communication_style: serde_json::Value::Null,
            emotional_state: serde_json::Value::Null,
            personality_traits: serde_json::Value::Null,
            trust_assessment: serde_json::Value::Null,
            relationship_phase: serde_json::Value::Null,
            tem_self_update: serde_json::Value::Null,
            observations: vec![
                "new_1".to_string(),
                "new_2".to_string(),
                "new_3".to_string(),
            ],
            recommendations: Recommendations::default(),
        };

        apply_evaluation(&mut profile, &eval);

        // 48 + 3 = 51, capped to 50, oldest dropped
        assert_eq!(profile.observations.len(), 50);
        // First observation (obs_0) should have been dropped
        assert_eq!(profile.observations[0], "obs_1");
        assert_eq!(profile.observations[49], "new_3");
    }

    #[test]
    fn null_values_skipped_value_unchanged() {
        let mut profile = UserProfile {
            user_id: "user_1".to_string(),
            ..UserProfile::default()
        };
        profile.communication_style.directness = Some(TraitScore {
            value: 0.5,
            confidence: 0.4,
            observations: 1,
            last_updated: 1000,
            reasoning: "Initial".to_string(),
        });

        let eval = EvaluationOutput {
            evaluation_id: "eval".to_string(),
            turns_analyzed: vec![1],
            communication_style: serde_json::Value::Null,
            emotional_state: serde_json::Value::Null,
            personality_traits: serde_json::Value::Null,
            trust_assessment: serde_json::Value::Null,
            relationship_phase: serde_json::Value::Null,
            tem_self_update: serde_json::Value::Null,
            observations: vec![],
            recommendations: Recommendations::default(),
        };

        apply_evaluation(&mut profile, &eval);

        // Directness value should be unchanged (not updated in this eval)
        let d = profile.communication_style.directness.as_ref().unwrap();
        assert!((d.value - 0.5).abs() < f32::EPSILON);
        // But confidence should decay 5% since it was not observed
        assert!(
            (d.confidence - 0.38).abs() < 0.01,
            "Expected ~0.38, got {}",
            d.confidence
        );
    }

    #[test]
    fn build_prompt_contains_key_sections() {
        let profile = UserProfile {
            user_id: "user_1".to_string(),
            ..UserProfile::default()
        };

        let facts = vec![];
        let (system, user) = build_evaluation_prompt(&profile, &facts, "Tem");

        assert!(system.contains("Social Intelligence"));
        assert!(system.contains("confidence"));
        assert!(system.contains("NEVER influence work quality"));
        assert!(user.contains("CURRENT PROFILE STATE"));
        assert!(user.contains("RECENT INTERACTIONS"));
    }

    #[test]
    fn adaptive_n_reset_on_high_delta() {
        let mut profile = UserProfile {
            user_id: "user_1".to_string(),
            evaluation_count: 10, // Mature profile
            ..UserProfile::default()
        };
        // Set initial directness high
        profile.communication_style.directness = Some(TraitScore {
            value: 0.9,
            confidence: 0.8,
            observations: 10,
            last_updated: 1000,
            reasoning: "Very direct".to_string(),
        });

        // Eval says directness suddenly dropped to 0.1 -- big shift
        let eval = EvaluationOutput {
            evaluation_id: "eval_shift".to_string(),
            turns_analyzed: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            communication_style: serde_json::json!({
                "directness": {"value": 0.1, "confidence": 0.9, "reasoning": "Suddenly indirect"}
            }),
            emotional_state: serde_json::Value::Null,
            personality_traits: serde_json::Value::Null,
            trust_assessment: serde_json::Value::Null,
            relationship_phase: serde_json::Value::Null,
            tem_self_update: serde_json::Value::Null,
            observations: vec![],
            recommendations: Recommendations::default(),
        };

        apply_evaluation(&mut profile, &eval);

        // The delta should be significant (directness moved from 0.9 toward 0.1)
        assert!(profile.last_delta > 0.0, "Delta should be > 0");
        // If delta > 0.15, n_next resets to N_MIN=5
        if profile.last_delta > 0.15 {
            assert_eq!(profile.n_next, 5, "Should reset to N_MIN on high delta");
        }
    }

    #[test]
    fn adaptive_n_grows_logarithmically_on_stability() {
        let mut profile = UserProfile {
            user_id: "user_1".to_string(),
            ..UserProfile::default()
        };

        // Simulate many evaluations with null updates (no changes = stable)
        for _ in 0..20 {
            let eval = EvaluationOutput {
                evaluation_id: "eval_stable".to_string(),
                turns_analyzed: vec![1],
                communication_style: serde_json::Value::Null,
                emotional_state: serde_json::Value::Null,
                personality_traits: serde_json::Value::Null,
                trust_assessment: serde_json::Value::Null,
                relationship_phase: serde_json::Value::Null,
                tem_self_update: serde_json::Value::Null,
                observations: vec![],
                recommendations: Recommendations::default(),
            };
            apply_evaluation(&mut profile, &eval);
        }

        // After many stable evaluations, n_next should be larger than N_MIN
        assert!(
            profile.n_next > 5,
            "n_next should grow beyond N_MIN, got {}",
            profile.n_next
        );
        assert!(profile.n_next <= 30, "n_next should not exceed N_MAX");
    }

    #[test]
    fn confidence_decay_on_unobserved_dimensions() {
        let mut profile = UserProfile {
            user_id: "user_1".to_string(),
            ..UserProfile::default()
        };
        // Set initial values on two dimensions
        profile.communication_style.directness = Some(TraitScore {
            value: 0.5,
            confidence: 0.8,
            observations: 5,
            last_updated: 1000,
            reasoning: "Established".to_string(),
        });
        profile.communication_style.formality = Some(TraitScore {
            value: 0.6,
            confidence: 0.7,
            observations: 3,
            last_updated: 1000,
            reasoning: "Established".to_string(),
        });

        // Eval updates ONLY directness, not formality
        let eval = EvaluationOutput {
            evaluation_id: "eval_partial".to_string(),
            turns_analyzed: vec![1, 2, 3, 4, 5],
            communication_style: serde_json::json!({
                "directness": {"value": 0.6, "confidence": 0.7, "reasoning": "Updated"}
            }),
            emotional_state: serde_json::Value::Null,
            personality_traits: serde_json::Value::Null,
            trust_assessment: serde_json::Value::Null,
            relationship_phase: serde_json::Value::Null,
            tem_self_update: serde_json::Value::Null,
            observations: vec![],
            recommendations: Recommendations::default(),
        };

        apply_evaluation(&mut profile, &eval);

        // Directness was updated -- confidence should be fresh (0.7 from eval)
        let d = profile.communication_style.directness.as_ref().unwrap();
        assert!((d.confidence - 0.7).abs() < f32::EPSILON);

        // Formality was NOT updated -- confidence should decay 5%: 0.7 * 0.95 = 0.665
        let f = profile.communication_style.formality.as_ref().unwrap();
        assert!(
            (f.confidence - 0.665).abs() < 0.01,
            "Expected ~0.665, got {}",
            f.confidence
        );
        // Value should be unchanged
        assert!((f.value - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn confidence_decay_zeros_out_below_threshold() {
        let mut profile = UserProfile {
            user_id: "user_1".to_string(),
            ..UserProfile::default()
        };
        profile.communication_style.pace = Some(TraitScore {
            value: 0.5,
            confidence: 0.1, // Just at the boundary
            observations: 1,
            last_updated: 1000,
            reasoning: "Low confidence".to_string(),
        });

        // No updates to pace
        let eval = EvaluationOutput {
            evaluation_id: "eval".to_string(),
            turns_analyzed: vec![1],
            communication_style: serde_json::Value::Null,
            emotional_state: serde_json::Value::Null,
            personality_traits: serde_json::Value::Null,
            trust_assessment: serde_json::Value::Null,
            relationship_phase: serde_json::Value::Null,
            tem_self_update: serde_json::Value::Null,
            observations: vec![],
            recommendations: Recommendations::default(),
        };

        apply_evaluation(&mut profile, &eval);

        // 0.1 * 0.95 = 0.095 < 0.1 threshold -> zeroed out
        let p = profile.communication_style.pace.as_ref().unwrap();
        assert!(
            (p.confidence - 0.0).abs() < f32::EPSILON,
            "Expected 0.0, got {}",
            p.confidence
        );
    }

    #[test]
    fn weighted_merge_v2_no_prior() {
        // No prior value: adopt directly
        let result = weighted_merge_v2(0.0, false, 0.8, 0.9, 5, 0);
        assert!((result - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn weighted_merge_v2_decaying_merge_rate() {
        // Same inputs, but higher eval_count -> lower merge rate -> less movement
        let result_early = weighted_merge_v2(0.5, true, 0.9, 0.8, 10, 0);
        let result_late = weighted_merge_v2(0.5, true, 0.9, 0.8, 10, 20);
        assert!(
            result_early > result_late,
            "Early merge should move more: {result_early} vs {result_late}"
        );
        // Both should still be between 0.5 and 0.9
        assert!(result_early > 0.5 && result_early < 0.9);
        assert!(result_late > 0.5 && result_late < 0.9);
    }

    #[test]
    fn delta_is_zero_on_null_eval() {
        let mut profile = UserProfile {
            user_id: "user_1".to_string(),
            ..UserProfile::default()
        };

        let eval = EvaluationOutput {
            evaluation_id: "eval".to_string(),
            turns_analyzed: vec![1],
            communication_style: serde_json::Value::Null,
            emotional_state: serde_json::Value::Null,
            personality_traits: serde_json::Value::Null,
            trust_assessment: serde_json::Value::Null,
            relationship_phase: serde_json::Value::Null,
            tem_self_update: serde_json::Value::Null,
            observations: vec![],
            recommendations: Recommendations::default(),
        };

        apply_evaluation(&mut profile, &eval);
        assert!(
            (profile.last_delta - 0.0).abs() < f32::EPSILON,
            "Delta should be 0 when nothing changes"
        );
    }
}
