//! Session budget enforcement.
//!
//! Tracks per-session limits on lines changed and files touched to prevent
//! runaway modifications. Each cambium session creates a `SessionBudget`
//! with maximum limits and calls `check()` before committing changes and
//! `record()` after each successful stage.

use std::collections::HashSet;
use std::path::PathBuf;

/// Enforces per-session limits on the amount of code change.
#[derive(Debug, Clone)]
pub struct SessionBudget {
    /// Maximum total lines (added + removed) allowed in this session.
    max_lines: usize,
    /// Maximum number of distinct files that can be touched in this session.
    max_files: usize,
    /// Current running total of lines added.
    used_added: usize,
    /// Current running total of lines removed.
    used_removed: usize,
    /// Set of files touched so far in this session.
    touched_files: HashSet<PathBuf>,
}

impl SessionBudget {
    /// Create a new budget with the given limits.
    pub fn new(max_lines: usize, max_files: usize) -> Self {
        Self {
            max_lines,
            max_files,
            used_added: 0,
            used_removed: 0,
            touched_files: HashSet::new(),
        }
    }

    /// Check whether the proposed change fits within the remaining budget.
    ///
    /// The `files` slice contains the paths touched by the proposed change.
    /// Files already touched in this session don't count against the file
    /// budget again.
    ///
    /// Returns `Ok(())` if the change fits, or `Err(reason)` explaining which
    /// limit would be exceeded.
    pub fn check(
        &self,
        lines_added: usize,
        lines_removed: usize,
        files: &[PathBuf],
    ) -> Result<(), String> {
        let new_total_lines = self.used_added + self.used_removed + lines_added + lines_removed;
        if new_total_lines > self.max_lines {
            return Err(format!(
                "line budget exceeded: {} used + {} proposed > {} max",
                self.used_added + self.used_removed,
                lines_added + lines_removed,
                self.max_lines,
            ));
        }

        let new_files: usize = files
            .iter()
            .filter(|f| !self.touched_files.contains(*f))
            .count();
        let prospective_file_count = self.touched_files.len() + new_files;
        if prospective_file_count > self.max_files {
            return Err(format!(
                "file budget exceeded: {} touched + {} new > {} max",
                self.touched_files.len(),
                new_files,
                self.max_files,
            ));
        }

        Ok(())
    }

    /// Record that a change has been made. Updates running totals.
    ///
    /// Callers should invoke `check()` first to ensure the change fits.
    /// Calling `record()` past the budget is permitted but will be reflected
    /// in subsequent `check()` calls.
    pub fn record(&mut self, lines_added: usize, lines_removed: usize, files: &[PathBuf]) {
        self.used_added += lines_added;
        self.used_removed += lines_removed;
        for f in files {
            self.touched_files.insert(f.clone());
        }
    }

    /// Return the number of lines still available in this session.
    ///
    /// Saturates at zero if the budget has been exceeded.
    pub fn remaining_lines(&self) -> usize {
        let used = self.used_added + self.used_removed;
        self.max_lines.saturating_sub(used)
    }

    /// Return the number of additional files that can still be touched.
    ///
    /// Saturates at zero if the budget has been exceeded.
    pub fn remaining_files(&self) -> usize {
        self.max_files.saturating_sub(self.touched_files.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(paths: &[&str]) -> Vec<PathBuf> {
        paths.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn new_budget_has_full_remaining() {
        let budget = SessionBudget::new(500, 10);
        assert_eq!(budget.remaining_lines(), 500);
        assert_eq!(budget.remaining_files(), 10);
    }

    #[test]
    fn check_passes_when_within_budget() {
        let budget = SessionBudget::new(500, 10);
        assert!(budget.check(100, 50, &files(&["a.rs", "b.rs"])).is_ok());
    }

    #[test]
    fn check_fails_when_lines_exceeded() {
        let budget = SessionBudget::new(100, 10);
        let result = budget.check(80, 30, &files(&["a.rs"]));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("line budget exceeded"));
    }

    #[test]
    fn check_fails_when_files_exceeded() {
        let budget = SessionBudget::new(500, 2);
        let result = budget.check(10, 0, &files(&["a.rs", "b.rs", "c.rs"]));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("file budget exceeded"));
    }

    #[test]
    fn record_updates_running_totals() {
        let mut budget = SessionBudget::new(500, 10);
        budget.record(100, 50, &files(&["a.rs", "b.rs"]));
        assert_eq!(budget.remaining_lines(), 350);
        assert_eq!(budget.remaining_files(), 8);
    }

    #[test]
    fn record_same_file_twice_counts_once() {
        let mut budget = SessionBudget::new(500, 10);
        budget.record(10, 0, &files(&["a.rs"]));
        budget.record(10, 0, &files(&["a.rs"]));
        assert_eq!(budget.remaining_files(), 9);
    }

    #[test]
    fn check_ignores_already_touched_files() {
        let mut budget = SessionBudget::new(500, 2);
        budget.record(10, 0, &files(&["a.rs", "b.rs"]));
        // Both files are already in the touched set; no new files means
        // the file budget is not exceeded.
        assert!(budget.check(10, 0, &files(&["a.rs"])).is_ok());
    }

    #[test]
    fn check_counts_new_files_only_against_file_budget() {
        let mut budget = SessionBudget::new(500, 3);
        budget.record(10, 0, &files(&["a.rs", "b.rs"]));
        // One already-touched + one new = still within budget of 3.
        assert!(budget.check(5, 0, &files(&["a.rs", "c.rs"])).is_ok());
        // One already-touched + two new = would exceed budget of 3.
        assert!(budget
            .check(5, 0, &files(&["a.rs", "c.rs", "d.rs"]))
            .is_err());
    }

    #[test]
    fn remaining_lines_saturates_at_zero() {
        let mut budget = SessionBudget::new(100, 10);
        // Force record beyond budget.
        budget.record(80, 50, &files(&["a.rs"]));
        assert_eq!(budget.remaining_lines(), 0);
    }

    #[test]
    fn remaining_files_saturates_at_zero() {
        let mut budget = SessionBudget::new(500, 2);
        budget.record(10, 0, &files(&["a.rs", "b.rs", "c.rs"]));
        assert_eq!(budget.remaining_files(), 0);
    }

    #[test]
    fn lines_added_and_removed_both_count() {
        let budget = SessionBudget::new(100, 10);
        // 60 + 50 = 110 > 100
        assert!(budget.check(60, 50, &files(&["a.rs"])).is_err());
    }

    #[test]
    fn exact_budget_fit_succeeds() {
        let budget = SessionBudget::new(100, 2);
        assert!(budget.check(50, 50, &files(&["a.rs", "b.rs"])).is_ok());
    }

    #[test]
    fn one_over_budget_fails() {
        let budget = SessionBudget::new(100, 2);
        assert!(budget.check(51, 50, &files(&["a.rs", "b.rs"])).is_err());
    }

    #[test]
    fn zero_proposal_always_passes() {
        let budget = SessionBudget::new(100, 2);
        assert!(budget.check(0, 0, &[]).is_ok());
    }

    #[test]
    fn progressive_recording_accumulates() {
        let mut budget = SessionBudget::new(300, 5);
        budget.record(50, 10, &files(&["a.rs"]));
        budget.record(30, 20, &files(&["b.rs"]));
        budget.record(40, 0, &files(&["c.rs"]));
        // used = 50+10+30+20+40+0 = 150
        assert_eq!(budget.remaining_lines(), 150);
        assert_eq!(budget.remaining_files(), 2);
    }
}
