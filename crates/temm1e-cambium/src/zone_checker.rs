//! Protected zone enforcement.
//!
//! The zone checker classifies file paths into trust levels and validates
//! that immutable (Level 0) files have not been tampered with by comparing
//! SHA-256 checksums against a known manifest.

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use temm1e_core::types::cambium::TrustLevel;

/// A manifest of protected zones and immutable file checksums.
///
/// Zone patterns are matched in order using `starts_with` on the path string
/// representation. The first matching pattern wins. Files that match no
/// pattern default to `TrustLevel::ApprovalRequired` (safe default).
pub struct ZoneManifest {
    /// SHA-256 checksums for Level 0 (immutable) files.
    pub immutable_checksums: HashMap<PathBuf, String>,
    /// Ordered list of (pattern, trust_level) pairs.
    /// Patterns are matched via `starts_with` on the path string.
    pub zone_patterns: Vec<(String, TrustLevel)>,
}

impl ZoneManifest {
    /// Create from explicit data (for testing and from parsed PROTECTED_ZONES.md).
    pub fn new(
        immutable_checksums: HashMap<PathBuf, String>,
        zone_patterns: Vec<(String, TrustLevel)>,
    ) -> Self {
        Self {
            immutable_checksums,
            zone_patterns,
        }
    }

    /// Compute SHA-256 of a file.
    pub async fn compute_checksum(path: &Path) -> Result<String, std::io::Error> {
        let bytes = tokio::fs::read(path).await?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let digest = hasher.finalize();
        Ok(format!("{:x}", digest))
    }

    /// Validate that no Level 0 files have been modified.
    ///
    /// Returns `Ok(())` if all immutable files match their expected checksums,
    /// or `Err(violations)` with a list of human-readable violation messages.
    pub async fn validate_immutable_kernel(&self, workspace: &Path) -> Result<(), Vec<String>> {
        let mut violations = Vec::new();

        for (relative_path, expected_hash) in &self.immutable_checksums {
            let full_path = workspace.join(relative_path);

            match Self::compute_checksum(&full_path).await {
                Ok(actual_hash) => {
                    if actual_hash != *expected_hash {
                        violations.push(format!(
                            "immutable file modified: {} (expected {}, got {})",
                            relative_path.display(),
                            &expected_hash[..12],
                            &actual_hash[..12],
                        ));
                    }
                }
                Err(e) => {
                    violations.push(format!(
                        "immutable file unreadable: {} ({})",
                        relative_path.display(),
                        e,
                    ));
                }
            }
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    /// Get trust level for a file path by matching patterns.
    ///
    /// Patterns are checked in order -- first match wins.
    /// Unknown files default to `ApprovalRequired` (safe default).
    pub fn trust_level_for(&self, path: &Path) -> TrustLevel {
        let path_str = path.to_string_lossy();

        for (pattern, level) in &self.zone_patterns {
            if path_str.starts_with(pattern.as_str()) {
                return *level;
            }
        }

        // Safe default: require approval for unknown files.
        TrustLevel::ApprovalRequired
    }

    /// Classify a set of changed files. Returns the HIGHEST trust level needed.
    ///
    /// Because `TrustLevel` is ordered with `Immutable` (0) at the bottom and
    /// `AutonomousBasic` (3) at the top, the "highest trust needed" means the
    /// *lowest* numeric level (most restrictive). We return the minimum
    /// `TrustLevel` across all changed files.
    ///
    /// If the set is empty, returns `AutonomousBasic` (least restrictive).
    pub fn classify_changes(&self, changed_files: &[PathBuf]) -> TrustLevel {
        if changed_files.is_empty() {
            return TrustLevel::AutonomousBasic;
        }

        changed_files
            .iter()
            .map(|f| self.trust_level_for(f))
            .min()
            .unwrap_or(TrustLevel::ApprovalRequired)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn sample_manifest() -> ZoneManifest {
        let mut checksums = HashMap::new();
        checksums.insert(
            PathBuf::from("Cargo.toml"),
            "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        );

        let patterns = vec![
            ("crates/temm1e-vault/".to_string(), TrustLevel::Immutable),
            (
                "crates/temm1e-core/".to_string(),
                TrustLevel::ApprovalRequired,
            ),
            (
                "crates/temm1e-tools/".to_string(),
                TrustLevel::AutonomousFull,
            ),
            ("skills/".to_string(), TrustLevel::AutonomousBasic),
        ];

        ZoneManifest::new(checksums, patterns)
    }

    #[test]
    fn trust_level_vault_is_immutable() {
        let manifest = sample_manifest();
        let level = manifest.trust_level_for(Path::new("crates/temm1e-vault/src/lib.rs"));
        assert_eq!(level, TrustLevel::Immutable);
    }

    #[test]
    fn trust_level_core_requires_approval() {
        let manifest = sample_manifest();
        let level = manifest.trust_level_for(Path::new("crates/temm1e-core/src/types/error.rs"));
        assert_eq!(level, TrustLevel::ApprovalRequired);
    }

    #[test]
    fn trust_level_tools_autonomous_full() {
        let manifest = sample_manifest();
        let level = manifest.trust_level_for(Path::new("crates/temm1e-tools/src/browser.rs"));
        assert_eq!(level, TrustLevel::AutonomousFull);
    }

    #[test]
    fn trust_level_skills_autonomous_basic() {
        let manifest = sample_manifest();
        let level = manifest.trust_level_for(Path::new("skills/weather.md"));
        assert_eq!(level, TrustLevel::AutonomousBasic);
    }

    #[test]
    fn trust_level_unknown_defaults_to_approval_required() {
        let manifest = sample_manifest();
        let level = manifest.trust_level_for(Path::new("some/random/file.txt"));
        assert_eq!(level, TrustLevel::ApprovalRequired);
    }

    #[test]
    fn first_pattern_match_wins() {
        let patterns = vec![
            ("crates/temm1e-vault/".to_string(), TrustLevel::Immutable),
            ("crates/".to_string(), TrustLevel::AutonomousBasic),
        ];
        let manifest = ZoneManifest::new(HashMap::new(), patterns);

        // Vault path should match the first (more specific) pattern.
        let level = manifest.trust_level_for(Path::new("crates/temm1e-vault/src/lib.rs"));
        assert_eq!(level, TrustLevel::Immutable);

        // Other crates path should match the second (broader) pattern.
        let level = manifest.trust_level_for(Path::new("crates/temm1e-agent/src/lib.rs"));
        assert_eq!(level, TrustLevel::AutonomousBasic);
    }

    #[test]
    fn classify_changes_empty_returns_autonomous_basic() {
        let manifest = sample_manifest();
        let level = manifest.classify_changes(&[]);
        assert_eq!(level, TrustLevel::AutonomousBasic);
    }

    #[test]
    fn classify_changes_returns_most_restrictive() {
        let manifest = sample_manifest();
        let changed = vec![
            PathBuf::from("skills/new_skill.md"),
            PathBuf::from("crates/temm1e-core/src/types/config.rs"),
        ];
        let level = manifest.classify_changes(&changed);
        // ApprovalRequired (1) is more restrictive than AutonomousBasic (3).
        assert_eq!(level, TrustLevel::ApprovalRequired);
    }

    #[test]
    fn classify_changes_immutable_blocks_all() {
        let manifest = sample_manifest();
        let changed = vec![
            PathBuf::from("skills/new_skill.md"),
            PathBuf::from("crates/temm1e-vault/src/lib.rs"),
        ];
        let level = manifest.classify_changes(&changed);
        assert_eq!(level, TrustLevel::Immutable);
    }

    #[test]
    fn classify_single_file() {
        let manifest = sample_manifest();
        let changed = vec![PathBuf::from("crates/temm1e-tools/src/browser.rs")];
        let level = manifest.classify_changes(&changed);
        assert_eq!(level, TrustLevel::AutonomousFull);
    }

    #[tokio::test]
    async fn compute_checksum_of_known_content() {
        let dir = tempdir().expect("failed to create tempdir");
        let file_path = dir.path().join("test.txt");
        tokio::fs::write(&file_path, b"hello world")
            .await
            .expect("failed to write");

        let checksum = ZoneManifest::compute_checksum(&file_path)
            .await
            .expect("failed to compute checksum");

        // SHA-256 of "hello world" is well-known.
        assert_eq!(
            checksum,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[tokio::test]
    async fn compute_checksum_nonexistent_file_errors() {
        let result = ZoneManifest::compute_checksum(Path::new("/nonexistent/file.txt")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn validate_immutable_kernel_passes_when_matching() {
        let dir = tempdir().expect("failed to create tempdir");
        let file_path = dir.path().join("kernel.rs");
        tokio::fs::write(&file_path, b"fn main() {}")
            .await
            .expect("failed to write");

        let checksum = ZoneManifest::compute_checksum(&file_path)
            .await
            .expect("failed to compute checksum");

        let mut checksums = HashMap::new();
        checksums.insert(PathBuf::from("kernel.rs"), checksum);

        let manifest = ZoneManifest::new(checksums, vec![]);
        let result = manifest.validate_immutable_kernel(dir.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn validate_immutable_kernel_fails_when_modified() {
        let dir = tempdir().expect("failed to create tempdir");
        let file_path = dir.path().join("kernel.rs");
        tokio::fs::write(&file_path, b"fn main() {}")
            .await
            .expect("failed to write");

        let mut checksums = HashMap::new();
        checksums.insert(
            PathBuf::from("kernel.rs"),
            "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        );

        let manifest = ZoneManifest::new(checksums, vec![]);
        let result = manifest.validate_immutable_kernel(dir.path()).await;
        assert!(result.is_err());

        let violations = result.unwrap_err();
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("immutable file modified"));
    }

    #[tokio::test]
    async fn validate_immutable_kernel_fails_when_missing() {
        let dir = tempdir().expect("failed to create tempdir");

        let mut checksums = HashMap::new();
        checksums.insert(
            PathBuf::from("missing.rs"),
            "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        );

        let manifest = ZoneManifest::new(checksums, vec![]);
        let result = manifest.validate_immutable_kernel(dir.path()).await;
        assert!(result.is_err());

        let violations = result.unwrap_err();
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("immutable file unreadable"));
    }

    #[tokio::test]
    async fn validate_immutable_kernel_empty_manifest_passes() {
        let dir = tempdir().expect("failed to create tempdir");
        let manifest = ZoneManifest::new(HashMap::new(), vec![]);
        let result = manifest.validate_immutable_kernel(dir.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn validate_multiple_files_reports_all_violations() {
        let dir = tempdir().expect("failed to create tempdir");
        let file_a = dir.path().join("a.rs");
        let file_b = dir.path().join("b.rs");
        tokio::fs::write(&file_a, b"aaa").await.expect("write a");
        tokio::fs::write(&file_b, b"bbb").await.expect("write b");

        let bad_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let mut checksums = HashMap::new();
        checksums.insert(PathBuf::from("a.rs"), bad_hash.to_string());
        checksums.insert(PathBuf::from("b.rs"), bad_hash.to_string());

        let manifest = ZoneManifest::new(checksums, vec![]);
        let result = manifest.validate_immutable_kernel(dir.path()).await;
        assert!(result.is_err());

        let violations = result.unwrap_err();
        assert_eq!(violations.len(), 2);
    }
}
