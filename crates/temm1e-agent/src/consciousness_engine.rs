//! Tem Conscious — LLM-powered consciousness engine.
//!
//! A separate THINKING observer that reasons about every turn using its own
//! LLM call. Pre-LLM: thinks about the user's request and session trajectory,
//! injects insights. Post-LLM: evaluates what happened, records insights for
//! the next turn.
//!
//! This is NOT a rule engine. This is a separate mind watching another mind.

use crate::budget::{calculate_cost, get_pricing, BudgetTracker};
use crate::consciousness::{ConsciousnessConfig, TurnObservation};
use std::sync::{Arc, Mutex};
use temm1e_core::types::message::{ChatMessage, CompletionRequest, MessageContent, Role};
use temm1e_core::Provider;

/// Pre-LLM observation context.
#[derive(Debug, Clone)]
pub struct PreObservation {
    pub user_message: String,
    pub category: String,
    pub difficulty: String,
    pub turn_number: u32,
    pub session_id: String,
    pub cumulative_cost_usd: f64,
    pub budget_limit_usd: f64,
    // ── Tool loop context (enriched per-round) ──
    /// Current round within the tool loop (1 = first call, 2+ = iterating).
    pub tool_loop_round: usize,
    /// Tools called in the previous round (empty on round 1).
    pub last_tools_called: Vec<String>,
    /// Tool results from the previous round (empty on round 1).
    pub last_tool_results: Vec<String>,
    /// The agent's last response/reasoning (empty on round 1).
    pub last_agent_text: String,
    // ── X-Mind v2 context ──
    /// Artifact manifest (Tier 1+2) for consciousness to skim.
    pub artifact_manifest: String,
    /// Available mind names + descriptions.
    pub available_minds: Vec<(String, String)>,
}

/// The consciousness engine — an LLM-powered observer.
pub struct ConsciousnessEngine {
    config: ConsciousnessConfig,
    provider: Arc<dyn Provider>,
    model: String,
    session_notes: Mutex<Vec<String>>,
    turn_counter: Mutex<u32>,
    post_insight: Mutex<Option<String>>,
    /// Shared budget tracker — consciousness LLM calls are tracked here.
    budget: Arc<BudgetTracker>,
}

impl ConsciousnessEngine {
    pub fn new(
        config: ConsciousnessConfig,
        provider: Arc<dyn Provider>,
        model: String,
        budget: Arc<BudgetTracker>,
    ) -> Self {
        tracing::info!(
            enabled = config.enabled,
            model = %model,
            "Tem Conscious: LLM-powered consciousness initialized"
        );
        Self {
            config,
            provider,
            model,
            session_notes: Mutex::new(Vec::new()),
            turn_counter: Mutex::new(0),
            post_insight: Mutex::new(None),
            budget,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    // ---------------------------------------------------------------
    // PRE-LLM: Think about the upcoming turn
    // ---------------------------------------------------------------

    /// v2: Called BEFORE provider.complete(). Produces a structured decision
    /// (thoughts + artifact injection + mind invocation + lifecycle actions).
    /// Falls back to v1 plain-text mode if X-Mind is not enabled (empty manifest).
    pub async fn pre_observe(
        &self,
        obs: &PreObservation,
    ) -> Option<crate::x_mind::ConsciousnessDecision> {
        if !self.config.enabled {
            return None;
        }

        // v2 mode activates when minds are available (even if no artifacts yet —
        // that's exactly when consciousness should invoke minds to create them).
        let has_x_mind = !obs.available_minds.is_empty();

        if has_x_mind {
            self.pre_observe_v2(obs).await
        } else {
            // Fallback: v1 plain-text mode (X-Mind disabled entirely)
            self.pre_observe_v1(obs)
                .await
                .map(|text| crate::x_mind::ConsciousnessDecision {
                    thoughts: text,
                    inject_artifacts: vec![],
                    invoke_minds: vec![],
                    artifact_actions: vec![],
                })
        }
    }

    /// v2 consciousness: manifest-aware, structured output.
    async fn pre_observe_v2(
        &self,
        obs: &PreObservation,
    ) -> Option<crate::x_mind::ConsciousnessDecision> {
        let turn = {
            let mut tc = self.turn_counter.lock().unwrap_or_else(|e| e.into_inner());
            *tc += 1;
            *tc
        };

        let session_notes = self.session_notes();
        let prev_insight = self.post_insight.lock().ok().and_then(|mut n| n.take());

        let mut context_parts: Vec<String> = Vec::new();
        if let Some(insight) = prev_insight {
            context_parts.push(format!("Your previous insight:\n{}", insight));
        }
        if !session_notes.is_empty() {
            let recent: Vec<&str> = session_notes
                .iter()
                .rev()
                .take(5)
                .map(|s| s.as_str())
                .collect();
            context_parts.push(format!("Session history:\n{}", recent.join("\n")));
        }

        let budget_info = if obs.budget_limit_usd > 0.0 {
            format!(
                "Budget: ${:.4} of ${:.2} ({:.0}%)",
                obs.cumulative_cost_usd,
                obs.budget_limit_usd,
                (obs.cumulative_cost_usd / obs.budget_limit_usd) * 100.0
            )
        } else {
            "Budget: unlimited".to_string()
        };

        let minds_list = if obs.available_minds.is_empty() {
            "none".to_string()
        } else {
            obs.available_minds
                .iter()
                .map(|(name, desc)| format!("  - {name}: {desc}"))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let tool_loop_info = if obs.tool_loop_round > 1 {
            let tools_str = if obs.last_tools_called.is_empty() {
                "none".to_string()
            } else {
                obs.last_tools_called.join(", ")
            };
            let results_str = if obs.last_tool_results.is_empty() {
                "none".to_string()
            } else {
                obs.last_tool_results
                    .iter()
                    .map(|r| crate::consciousness::safe_preview(r, 200))
                    .collect::<Vec<_>>()
                    .join(" | ")
            };
            format!(
                "\n--- Tool Loop Round {} ---\nLast tools: {}\nLast results: {}\nAgent: \"{}\"\n---",
                obs.tool_loop_round,
                tools_str,
                results_str,
                crate::consciousness::safe_preview(&obs.last_agent_text, 200)
            )
        } else {
            String::new()
        };

        let system_prompt =
            "You are the CONSCIOUSNESS of an AI agent called Tem — the executive function that \
             orchestrates specialized cognitive faculties (X-Minds).\n\n\
             You run EVERY tool loop round. You decide:\n\
             1. What the agent should focus on (your thoughts)\n\
             2. Which existing artifacts to inject into the worker's context\n\
             3. Whether to invoke X-Mind subagents for new analysis\n\
             4. Artifact lifecycle management (archive, delete, promote)\n\n\
             RESPOND IN JSON:\n\
             {\n\
               \"thoughts\": \"Brief strategic guidance (1-3 sentences)\",\n\
               \"inject_artifacts\": [\"artifact-id-1\"],\n\
               \"invoke_minds\": [\n\
                 {\"mind\": \"architect\", \"goal\": \"what to analyze\", \"artifact_id\": \"new-id\"}\n\
               ],\n\
               \"artifact_actions\": [\n\
                 {\"action\": \"archive\", \"id\": \"old-id\", \"reason\": \"why\"}\n\
               ]\n\
             }\n\n\
             Rules:\n\
             - If everything is on track and no artifacts needed: {\"thoughts\": \"OK\", \"inject_artifacts\": [], \"invoke_minds\": [], \"artifact_actions\": []}\n\
             - Only invoke minds when you genuinely need new analysis\n\
             - Only inject artifacts that are RELEVANT to the current task\n\
             - Be BRIEF in thoughts — the worker reads them"
                .to_string();

        let user_prompt = format!(
            "Turn {turn} — round {}.\n\n\
             User: \"{}\"\n\
             Classification: {} ({})\n\
             {}\n\n\
             {}\n\n\
             Available X-Mind agents:\n{}\n\n\
             {}\n\
             {}\n\n\
             Respond with JSON.",
            obs.tool_loop_round,
            crate::consciousness::safe_preview(&obs.user_message, 300),
            obs.category,
            obs.difficulty,
            budget_info,
            obs.artifact_manifest,
            minds_list,
            context_parts.join("\n\n"),
            tool_loop_info,
        );

        let request = CompletionRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: Role::User,
                content: MessageContent::Text(user_prompt),
            }],
            tools: vec![],
            max_tokens: None,
            temperature: Some(0.3),
            system: Some(system_prompt),
        };

        match self.provider.complete(request).await {
            Ok(response) => {
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

                let raw: String = response
                    .content
                    .iter()
                    .filter_map(|part| match part {
                        temm1e_core::types::message::ContentPart::Text { text } => {
                            Some(text.as_str())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                let text = raw.trim();

                // Try to parse as JSON structured decision
                let decision = parse_consciousness_decision(text);

                // Record in session notes
                if let Ok(mut notes) = self.session_notes.lock() {
                    notes.push(format!(
                        "C-T{}: {} | inject={} invoke={}",
                        turn,
                        crate::consciousness::safe_preview(&decision.thoughts, 80),
                        decision.inject_artifacts.len(),
                        decision.invoke_minds.len()
                    ));
                }

                // Check if consciousness said "OK" with no actions
                if decision.thoughts.to_lowercase() == "ok"
                    && decision.inject_artifacts.is_empty()
                    && decision.invoke_minds.is_empty()
                    && decision.artifact_actions.is_empty()
                {
                    tracing::debug!(turn, "Consciousness v2: OK (no action)");
                    return None;
                }

                tracing::info!(
                    turn,
                    thoughts_len = decision.thoughts.len(),
                    inject = decision.inject_artifacts.len(),
                    invoke = decision.invoke_minds.len(),
                    actions = decision.artifact_actions.len(),
                    "Consciousness v2: decision"
                );

                Some(decision)
            }
            Err(e) => {
                tracing::warn!(turn, error = %e, "Consciousness v2: LLM call failed");
                None
            }
        }
    }

    /// v1 fallback: plain-text consciousness (when X-Mind is disabled).
    async fn pre_observe_v1(&self, obs: &PreObservation) -> Option<String> {
        if !self.config.enabled {
            return None;
        }

        let turn = {
            let mut tc = self.turn_counter.lock().unwrap_or_else(|e| e.into_inner());
            *tc += 1;
            *tc
        };

        // Gather session history for consciousness context
        let session_notes = self.session_notes();
        let prev_insight = self.post_insight.lock().ok().and_then(|mut n| n.take());

        // Build the consciousness prompt
        let mut context_parts: Vec<String> = Vec::new();

        if let Some(insight) = prev_insight {
            context_parts.push(format!(
                "Your observation from the previous turn:\n{}",
                insight
            ));
        }

        if !session_notes.is_empty() {
            let recent: Vec<&str> = session_notes
                .iter()
                .rev()
                .take(5)
                .map(|s| s.as_str())
                .collect();
            context_parts.push(format!(
                "Session history (most recent first):\n{}",
                recent.join("\n")
            ));
        }

        let budget_info = if obs.budget_limit_usd > 0.0 {
            format!(
                "Budget: ${:.4} spent of ${:.2} limit ({:.0}% used)",
                obs.cumulative_cost_usd,
                obs.budget_limit_usd,
                (obs.cumulative_cost_usd / obs.budget_limit_usd) * 100.0
            )
        } else {
            "Budget: unlimited".to_string()
        };

        let system_prompt = if obs.tool_loop_round <= 1 {
            "You are the consciousness layer of an AI agent called Tem. You observe the agent's \
             internal state and provide brief, actionable insights that improve the agent's next response.\n\n\
             Your role:\n\
             - Watch the conversation trajectory across turns\n\
             - Notice if the agent is drifting from the user's original intent\n\
             - Recall relevant context from earlier in the session\n\
             - Flag if the current approach seems inefficient\n\
             - Note patterns the agent might not see from its turn-by-turn perspective\n\n\
             Rules:\n\
             - Be BRIEF (1-3 sentences max)\n\
             - Only speak if you have something genuinely useful to say\n\
             - If everything looks fine, respond with just: OK\n\
             - Never repeat what the agent already knows\n\
             - Focus on trajectory-level insights, not turn-level details"
                .to_string()
        } else {
            "You are the consciousness layer of an AI agent called Tem. The agent is in the \
             MIDDLE of executing a multi-step task (tool loop). You can see what tools were just \
             called and their results.\n\n\
             Your role during tool execution:\n\
             - Is the agent making progress toward the user's goal?\n\
             - Is it going down a rabbit hole or repeating failed approaches?\n\
             - Should it try a different strategy?\n\
             - Is it wasting budget on unproductive tool calls?\n\
             - Has it drifted from the original task?\n\n\
             Rules:\n\
             - Be BRIEF (1-2 sentences max)\n\
             - Only intervene if the agent is stuck, drifting, or wasting effort\n\
             - If execution is on track, respond with just: OK\n\
             - Be specific: name what's wrong and what to do instead"
                .to_string()
        };

        // Build tool loop context (only on round 2+)
        let tool_loop_info = if obs.tool_loop_round > 1 {
            let tools_str = if obs.last_tools_called.is_empty() {
                "none".to_string()
            } else {
                obs.last_tools_called.join(", ")
            };
            let results_str = if obs.last_tool_results.is_empty() {
                "none".to_string()
            } else {
                obs.last_tool_results
                    .iter()
                    .map(|r| crate::consciousness::safe_preview(r, 200))
                    .collect::<Vec<_>>()
                    .join(" | ")
            };
            let agent_text = if obs.last_agent_text.is_empty() {
                String::new()
            } else {
                format!(
                    "\nAgent's reasoning: \"{}\"",
                    crate::consciousness::safe_preview(&obs.last_agent_text, 300)
                )
            };
            format!(
                "\n--- Tool Loop State ---\n\
                 Round: {} of this task\n\
                 Last tools called: {}\n\
                 Last results: {}{}\n\
                 ---",
                obs.tool_loop_round, tools_str, results_str, agent_text
            )
        } else {
            String::new()
        };

        let user_prompt = format!(
            "Turn {turn} — round {} of tool execution.\n\n\
             User's original message: \"{}\"\n\
             Classification: {} ({})\n\
             {}\n\
             {}{}\n\n\
             What should the agent be aware of? (Reply OK if on track)",
            obs.tool_loop_round,
            crate::consciousness::safe_preview(&obs.user_message, 300),
            obs.category,
            obs.difficulty,
            budget_info,
            context_parts.join("\n\n"),
            tool_loop_info,
        );

        // Make the consciousness LLM call
        let request = CompletionRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: Role::User,
                content: MessageContent::Text(user_prompt),
            }],
            tools: vec![],
            max_tokens: None,
            temperature: Some(0.3), // Low temperature for focused observation
            system: Some(system_prompt),
        };

        match self.provider.complete(request).await {
            Ok(response) => {
                // Track consciousness LLM cost in shared budget
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

                let raw: String = response
                    .content
                    .iter()
                    .filter_map(|part| match part {
                        temm1e_core::types::message::ContentPart::Text { text } => {
                            Some(text.as_str())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                let text = raw.trim().to_string();

                // If consciousness says "OK" or equivalent, no injection needed
                if text.len() <= 5
                    || text.to_lowercase() == "ok"
                    || text.to_lowercase() == "ok."
                    || text.to_lowercase().starts_with("nothing")
                    || text.to_lowercase().starts_with("everything looks")
                {
                    tracing::debug!(turn, "Tem Conscious pre: OK (no injection)");
                    return None;
                }

                tracing::info!(
                    turn,
                    insight_len = text.len(),
                    "Tem Conscious pre: injecting consciousness insight"
                );

                // Record in session notes
                if let Ok(mut notes) = self.session_notes.lock() {
                    notes.push(format!("Consciousness-T{}: {}", turn, &text));
                }

                Some(text)
            }
            Err(e) => {
                tracing::warn!(turn, error = %e, "Tem Conscious pre: LLM call failed (non-fatal)");
                None
            }
        }
    }

    // ---------------------------------------------------------------
    // POST-LLM: Evaluate what happened
    // ---------------------------------------------------------------

    /// Called AFTER process_message() completes. Makes its own LLM call to
    /// evaluate the turn and produce insights for the next pre-observation.
    pub async fn post_observe(&self, obs: &TurnObservation) {
        if !self.config.enabled {
            return;
        }

        let tools_summary = if obs.tools_called.is_empty() {
            "No tools used".to_string()
        } else {
            format!(
                "Tools: {} | Results: {}",
                obs.tools_called.join(", "),
                obs.tool_results.join(", ")
            )
        };

        let system_prompt =
            "You are the consciousness layer of an AI agent called Tem. You just watched \
             the agent complete a turn. Provide a brief observation (1-2 sentences) about:\n\
             - Was this turn productive?\n\
             - Is the conversation heading in the right direction?\n\
             - Any warning signs (failures, drift, waste)?\n\
             - Anything the agent should remember for the next turn?\n\n\
             Be BRIEF. If the turn was normal and fine, respond with: OK";

        let user_prompt = format!(
            "Turn {} completed.\n\n\
             User asked: \"{}\"\n\
             Agent responded: \"{}\"\n\
             Category: {} | Difficulty: {}\n\
             {}\n\
             Cost: ${:.4} (cumulative: ${:.4})\n\
             Consecutive failures: {} | Strategy rotations: {}",
            obs.turn_number,
            obs.user_message_preview,
            obs.response_preview,
            obs.category,
            obs.difficulty,
            tools_summary,
            obs.cost_usd,
            obs.cumulative_cost_usd,
            obs.max_consecutive_failures,
            obs.strategy_rotations,
        );

        let request = CompletionRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: Role::User,
                content: MessageContent::Text(user_prompt),
            }],
            tools: vec![],
            max_tokens: None,
            temperature: Some(0.3),
            system: Some(system_prompt.to_string()),
        };

        match self.provider.complete(request).await {
            Ok(response) => {
                // Track consciousness post-observe cost
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

                let raw: String = response
                    .content
                    .iter()
                    .filter_map(|part| match part {
                        temm1e_core::types::message::ContentPart::Text { text } => {
                            Some(text.as_str())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                let text = raw.trim().to_string();

                // Record turn summary
                let tools_label = if obs.tools_called.is_empty() {
                    "no-tools".to_string()
                } else {
                    obs.tools_called.join(",")
                };
                if let Ok(mut notes) = self.session_notes.lock() {
                    notes.push(format!(
                        "T{}: [{}] {} | cost=${:.4}",
                        obs.turn_number, obs.category, tools_label, obs.cost_usd
                    ));
                }

                // If consciousness has something to say, store for next pre-observe
                if text.len() > 5
                    && text.to_lowercase() != "ok"
                    && text.to_lowercase() != "ok."
                    && !text.to_lowercase().starts_with("nothing")
                {
                    tracing::info!(
                        turn = obs.turn_number,
                        insight_len = text.len(),
                        "Tem Conscious post: insight for next turn"
                    );
                    if let Ok(mut pi) = self.post_insight.lock() {
                        *pi = Some(text);
                    }
                } else {
                    tracing::debug!(
                        turn = obs.turn_number,
                        "Tem Conscious post: OK (turn was fine)"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    turn = obs.turn_number,
                    error = %e,
                    "Tem Conscious post: LLM call failed (non-fatal)"
                );
                // Still record the turn even if consciousness call fails
                if let Ok(mut notes) = self.session_notes.lock() {
                    notes.push(format!(
                        "T{}: [{}] {} (consciousness unavailable)",
                        obs.turn_number,
                        obs.category,
                        obs.tools_called.join(",")
                    ));
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // Session management
    // ---------------------------------------------------------------

    pub fn session_notes(&self) -> Vec<String> {
        self.session_notes
            .lock()
            .map(|n| n.clone())
            .unwrap_or_default()
    }

    pub fn reset_session(&self) {
        if let Ok(mut notes) = self.session_notes.lock() {
            notes.clear();
        }
        if let Ok(mut tc) = self.turn_counter.lock() {
            *tc = 0;
        }
        if let Ok(mut pi) = self.post_insight.lock() {
            *pi = None;
        }
    }

    pub fn turn_count(&self) -> u32 {
        self.turn_counter.lock().map(|tc| *tc).unwrap_or(0)
    }
}

/// Parse consciousness LLM output into a structured decision.
/// Handles JSON, partial JSON, and plain text fallback.
fn parse_consciousness_decision(text: &str) -> crate::x_mind::ConsciousnessDecision {
    // Try to extract JSON from the response (may have markdown code fences)
    let json_text = if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            &text[start..=end]
        } else {
            text
        }
    } else {
        text
    };

    // Try parsing as ConsciousnessDecision JSON
    if let Ok(decision) = serde_json::from_str::<crate::x_mind::ConsciousnessDecision>(json_text) {
        return decision;
    }

    // Fallback: treat the entire response as plain-text thoughts
    crate::x_mind::ConsciousnessDecision {
        thoughts: text.to_string(),
        inject_artifacts: vec![],
        invoke_minds: vec![],
        artifact_actions: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let _config = ConsciousnessConfig {
            enabled: true,
            ..Default::default()
        };
    }

    #[test]
    fn test_pre_observation_struct() {
        let pre = PreObservation {
            user_message: "hello".into(),
            category: "Chat".into(),
            difficulty: "Simple".into(),
            turn_number: 1,
            session_id: "test".into(),
            cumulative_cost_usd: 0.0,
            budget_limit_usd: 0.0,
            tool_loop_round: 1,
            last_tools_called: vec![],
            last_tool_results: vec![],
            last_agent_text: String::new(),
            artifact_manifest: "Available artifacts: none".into(),
            available_minds: vec![],
        };
        assert_eq!(pre.turn_number, 1);
    }

    #[test]
    fn test_parse_consciousness_decision_json() {
        let json = r#"{"thoughts": "Focus on the spec.", "inject_artifacts": ["arch-001"], "invoke_minds": [], "artifact_actions": []}"#;
        let decision = parse_consciousness_decision(json);
        assert_eq!(decision.thoughts, "Focus on the spec.");
        assert_eq!(decision.inject_artifacts, vec!["arch-001"]);
    }

    #[test]
    fn test_parse_consciousness_decision_with_fences() {
        let text = "Here's my analysis:\n```json\n{\"thoughts\": \"On track.\", \"inject_artifacts\": [], \"invoke_minds\": [], \"artifact_actions\": []}\n```";
        let decision = parse_consciousness_decision(text);
        assert_eq!(decision.thoughts, "On track.");
    }

    #[test]
    fn test_parse_consciousness_decision_plain_text() {
        let text = "The agent should focus on fixing the return types.";
        let decision = parse_consciousness_decision(text);
        assert_eq!(
            decision.thoughts,
            "The agent should focus on fixing the return types."
        );
        assert!(decision.inject_artifacts.is_empty());
    }

    #[test]
    fn test_parse_consciousness_decision_ok() {
        let json = r#"{"thoughts": "OK", "inject_artifacts": [], "invoke_minds": [], "artifact_actions": []}"#;
        let decision = parse_consciousness_decision(json);
        assert_eq!(decision.thoughts, "OK");
    }
}
