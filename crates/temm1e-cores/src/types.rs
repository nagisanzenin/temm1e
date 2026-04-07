//! TemDOS core types — result structures and invocation metadata.

use serde::{Deserialize, Serialize};

/// Result returned by a core after completing its task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreResult {
    /// The core's final answer text.
    pub output: String,
    /// Number of tool-use rounds the core executed.
    pub rounds: usize,
    /// Total input tokens consumed across all LLM calls.
    pub input_tokens: u32,
    /// Total output tokens consumed across all LLM calls.
    pub output_tokens: u32,
    /// Total cost in USD (deducted from shared budget).
    pub cost_usd: f64,
}

/// Statistics tracked per core across invocations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoreStats {
    /// Total number of times this core has been invoked.
    pub invocations: u32,
    /// Number of invocations that produced useful output.
    pub successes: u32,
    /// Number of invocations that errored.
    pub failures: u32,
    /// Average number of tool rounds per invocation.
    pub avg_rounds: f32,
    /// Total cost across all invocations.
    pub total_cost_usd: f64,
}

impl CoreStats {
    /// Record a successful invocation.
    pub fn record_success(&mut self, rounds: usize, cost: f64) {
        self.invocations += 1;
        self.successes += 1;
        self.total_cost_usd += cost;
        let total_rounds = self.avg_rounds * (self.invocations - 1) as f32 + rounds as f32;
        self.avg_rounds = total_rounds / self.invocations as f32;
    }

    /// Record a failed invocation.
    pub fn record_failure(&mut self, cost: f64) {
        self.invocations += 1;
        self.failures += 1;
        self.total_cost_usd += cost;
    }

    /// Success rate as a fraction in [0.0, 1.0].
    pub fn success_rate(&self) -> f64 {
        if self.invocations == 0 {
            return 0.0;
        }
        self.successes as f64 / self.invocations as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_stats_record_success() {
        let mut stats = CoreStats::default();
        stats.record_success(5, 0.10);
        assert_eq!(stats.invocations, 1);
        assert_eq!(stats.successes, 1);
        assert!((stats.avg_rounds - 5.0).abs() < 0.01);
        assert!((stats.total_cost_usd - 0.10).abs() < 0.001);

        stats.record_success(3, 0.05);
        assert_eq!(stats.invocations, 2);
        assert!((stats.avg_rounds - 4.0).abs() < 0.01);
        assert!((stats.total_cost_usd - 0.15).abs() < 0.001);
    }

    #[test]
    fn core_stats_record_failure() {
        let mut stats = CoreStats::default();
        stats.record_failure(0.02);
        assert_eq!(stats.invocations, 1);
        assert_eq!(stats.failures, 1);
        assert_eq!(stats.successes, 0);
    }
}
