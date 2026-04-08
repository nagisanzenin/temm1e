//! # Deploy: orchestrated binary swap with rollback.
//!
//! This module performs the operational heart of Cambium Phase 5: replacing
//! the running Tem binary with a freshly-built one, with automatic rollback
//! on health-check failure.
//!
//! ## Safety guarantees
//!
//! - The deployer NEVER touches the production install path unless explicitly
//!   configured. The CLI test points it at an isolated `/tmp` location.
//! - Every swap creates a timestamped backup. The previous binary is never
//!   deleted, only moved to the rollback directory.
//! - Health check verifies the new binary actually starts AND responds to
//!   `--version` (a fast self-check) before the swap. After the swap, an
//!   optional secondary health check can verify the running process.
//! - If ANY step fails after the swap, the deployer attempts to restore from
//!   the latest backup automatically.
//! - All commands have timeouts. No step can hang indefinitely.
//!
//! ## What this module does NOT do
//!
//! - It does NOT serialize active conversation state. Tem's state lives in
//!   SQLite (memory.db, perpetuum.db), so the binary itself is mostly
//!   stateless. In-flight messages get a brief interruption.
//! - It does NOT implement true zero-downtime. There is a ~3-5 second window
//!   between stopping the old binary and starting the new one. This is
//!   acceptable for cambium use; production HA needs a load balancer.
//! - It does NOT push or pull from any remote. All operations are local.

use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use tokio::process::Command;
use tokio::time::{sleep, timeout};

use temm1e_core::types::error::Temm1eError;

/// Maximum seconds to wait for the running process to exit after SIGTERM.
const STOP_TIMEOUT_SECS: u64 = 30;
/// Maximum seconds to wait for the new process to become healthy after start.
const HEALTH_TIMEOUT_SECS: u64 = 30;
/// Polling interval while waiting for process state changes.
const POLL_INTERVAL_MS: u64 = 200;
/// Maximum seconds for any cargo build invocation.
const BUILD_TIMEOUT_SECS: u64 = 600;
/// Maximum seconds for `--version` health check.
const VERSION_TIMEOUT_SECS: u64 = 10;

/// Configuration for a deployment session.
#[derive(Debug, Clone)]
pub struct DeployConfig {
    /// Absolute path to the currently-installed binary that will be replaced.
    /// e.g. `/tmp/temm1e-deploy-test/temm1e`
    pub installed_binary: PathBuf,
    /// Absolute path to the directory where backup binaries are stored.
    /// e.g. `~/.temm1e/cambium/rollback/`
    pub rollback_dir: PathBuf,
    /// Absolute path to the staging directory where new binaries land
    /// before they are installed. e.g. `~/.temm1e/cambium/staging/`
    pub staging_dir: PathBuf,
    /// Optional path to the PID file of the running process. If set, the
    /// deployer will SIGTERM the PID and wait for exit. If unset, the
    /// deployer assumes the binary is not running and skips the stop step.
    pub pid_file: Option<PathBuf>,
    /// Optional health check command to run after starting the new binary.
    /// Receives the new binary path as the only arg. Must exit 0 for healthy.
    /// If None, the deployer only verifies the process is alive.
    pub post_start_health_check: Option<PathBuf>,
}

/// Outcome of a deployment session.
#[derive(Debug, Clone, PartialEq)]
pub enum DeployOutcome {
    /// New binary installed and verified healthy.
    Success {
        new_binary_path: PathBuf,
        backup_path: PathBuf,
        new_pid: Option<u32>,
    },
    /// A pre-swap step failed (validation, backup, build). Old binary
    /// is still running and untouched.
    AbortedBeforeSwap { stage: String, reason: String },
    /// The swap happened, but post-swap health check failed. The old
    /// binary was restored from backup.
    RolledBack {
        stage: String,
        reason: String,
        backup_restored: bool,
    },
    /// Rollback ITSELF failed. The system is in an indeterminate state
    /// and requires manual intervention.
    RollbackFailed {
        original_failure: String,
        rollback_failure: String,
    },
}

/// Per-stage timing data captured during a deployment.
#[derive(Debug, Clone, Default)]
pub struct DeployTimings {
    pub validate_ms: Option<u64>,
    pub backup_ms: Option<u64>,
    pub stop_ms: Option<u64>,
    pub install_ms: Option<u64>,
    pub start_ms: Option<u64>,
    pub health_check_ms: Option<u64>,
    pub total_ms: u64,
}

/// Result of a complete deployment.
#[derive(Debug, Clone)]
pub struct DeployReport {
    pub outcome: DeployOutcome,
    pub timings: DeployTimings,
    pub timestamp: chrono::DateTime<Utc>,
}

/// The deployer orchestrates the binary-swap protocol.
pub struct Deployer {
    pub config: DeployConfig,
}

impl Deployer {
    pub fn new(config: DeployConfig) -> Self {
        Self { config }
    }

    /// Perform the full swap protocol with the new binary.
    ///
    /// `new_binary_source` is the path to the freshly-built binary (e.g.
    /// inside the sandbox at `<sandbox>/target/release/temm1e`).
    pub async fn swap(&self, new_binary_source: &Path) -> DeployReport {
        let start = std::time::Instant::now();
        let mut timings = DeployTimings::default();

        // Stage 1: Validate the new binary exists and runs.
        let validate_start = std::time::Instant::now();
        if let Err(reason) = self.validate_new_binary(new_binary_source).await {
            timings.validate_ms = Some(validate_start.elapsed().as_millis() as u64);
            timings.total_ms = start.elapsed().as_millis() as u64;
            return DeployReport {
                outcome: DeployOutcome::AbortedBeforeSwap {
                    stage: "validate".into(),
                    reason,
                },
                timings,
                timestamp: Utc::now(),
            };
        }
        timings.validate_ms = Some(validate_start.elapsed().as_millis() as u64);

        // Stage 2: Backup the current binary if it exists.
        let backup_start = std::time::Instant::now();
        let backup_path = match self.backup_current().await {
            Ok(path) => path,
            Err(reason) => {
                timings.backup_ms = Some(backup_start.elapsed().as_millis() as u64);
                timings.total_ms = start.elapsed().as_millis() as u64;
                return DeployReport {
                    outcome: DeployOutcome::AbortedBeforeSwap {
                        stage: "backup".into(),
                        reason,
                    },
                    timings,
                    timestamp: Utc::now(),
                };
            }
        };
        timings.backup_ms = Some(backup_start.elapsed().as_millis() as u64);

        // Stage 3: Stop the running process if a PID file is configured.
        let stop_start = std::time::Instant::now();
        if let Err(reason) = self.stop_current().await {
            timings.stop_ms = Some(stop_start.elapsed().as_millis() as u64);
            timings.total_ms = start.elapsed().as_millis() as u64;
            return DeployReport {
                outcome: DeployOutcome::AbortedBeforeSwap {
                    stage: "stop".into(),
                    reason,
                },
                timings,
                timestamp: Utc::now(),
            };
        }
        timings.stop_ms = Some(stop_start.elapsed().as_millis() as u64);

        // Stage 4: Install the new binary in place.
        let install_start = std::time::Instant::now();
        if let Err(reason) = self.install_new(new_binary_source).await {
            timings.install_ms = Some(install_start.elapsed().as_millis() as u64);
            timings.total_ms = start.elapsed().as_millis() as u64;
            // Try to rollback even though we never started.
            let rollback_result = self.rollback(&backup_path).await;
            return DeployReport {
                outcome: match rollback_result {
                    Ok(()) => DeployOutcome::RolledBack {
                        stage: "install".into(),
                        reason,
                        backup_restored: true,
                    },
                    Err(rollback_err) => DeployOutcome::RollbackFailed {
                        original_failure: reason,
                        rollback_failure: rollback_err,
                    },
                },
                timings,
                timestamp: Utc::now(),
            };
        }
        timings.install_ms = Some(install_start.elapsed().as_millis() as u64);

        // Stage 5: Start the new binary (only if a PID file was configured —
        // meaning the caller wants us to manage the process).
        let start_pid_start = std::time::Instant::now();
        let new_pid = if self.config.pid_file.is_some() {
            match self.start_new().await {
                Ok(pid) => Some(pid),
                Err(reason) => {
                    timings.start_ms = Some(start_pid_start.elapsed().as_millis() as u64);
                    timings.total_ms = start.elapsed().as_millis() as u64;
                    let rollback_result = self.rollback(&backup_path).await;
                    return DeployReport {
                        outcome: match rollback_result {
                            Ok(()) => DeployOutcome::RolledBack {
                                stage: "start".into(),
                                reason,
                                backup_restored: true,
                            },
                            Err(rollback_err) => DeployOutcome::RollbackFailed {
                                original_failure: reason,
                                rollback_failure: rollback_err,
                            },
                        },
                        timings,
                        timestamp: Utc::now(),
                    };
                }
            }
        } else {
            None
        };
        timings.start_ms = Some(start_pid_start.elapsed().as_millis() as u64);

        // Stage 6: Health check.
        let health_start = std::time::Instant::now();
        if let Some(pid) = new_pid {
            if let Err(reason) = self.health_check(pid).await {
                timings.health_check_ms = Some(health_start.elapsed().as_millis() as u64);
                timings.total_ms = start.elapsed().as_millis() as u64;
                // Stop the unhealthy new process before rolling back.
                let _ = self.stop_pid(pid).await;
                let rollback_result = self.rollback(&backup_path).await;
                let restored = rollback_result.is_ok();
                if restored {
                    let _ = self.start_new().await;
                }
                return DeployReport {
                    outcome: match rollback_result {
                        Ok(()) => DeployOutcome::RolledBack {
                            stage: "health_check".into(),
                            reason,
                            backup_restored: true,
                        },
                        Err(rollback_err) => DeployOutcome::RollbackFailed {
                            original_failure: reason,
                            rollback_failure: rollback_err,
                        },
                    },
                    timings,
                    timestamp: Utc::now(),
                };
            }
        }
        timings.health_check_ms = Some(health_start.elapsed().as_millis() as u64);

        timings.total_ms = start.elapsed().as_millis() as u64;
        DeployReport {
            outcome: DeployOutcome::Success {
                new_binary_path: self.config.installed_binary.clone(),
                backup_path,
                new_pid,
            },
            timings,
            timestamp: Utc::now(),
        }
    }

    /// Verify the candidate binary exists and runs `--version` cleanly.
    pub async fn validate_new_binary(&self, source: &Path) -> Result<(), String> {
        if !source.exists() {
            return Err(format!("New binary does not exist at {}", source.display()));
        }
        let metadata = tokio::fs::metadata(source)
            .await
            .map_err(|e| format!("Failed to stat {}: {e}", source.display()))?;
        if !metadata.is_file() {
            return Err(format!("Not a file: {}", source.display()));
        }

        // Run `--version` to verify it's an actual executable that runs.
        let output = timeout(
            Duration::from_secs(VERSION_TIMEOUT_SECS),
            Command::new(source).arg("--version").output(),
        )
        .await
        .map_err(|_| format!("--version timed out after {VERSION_TIMEOUT_SECS}s"))?
        .map_err(|e| format!("Failed to spawn {}: {e}", source.display()))?;

        if !output.status.success() {
            return Err(format!(
                "--version exited with code {:?}: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(())
    }

    /// Backup the currently-installed binary to the rollback directory.
    /// Returns the absolute path of the backup file.
    pub async fn backup_current(&self) -> Result<PathBuf, String> {
        if !self.config.installed_binary.exists() {
            // Nothing to back up — first install.
            tokio::fs::create_dir_all(&self.config.rollback_dir)
                .await
                .map_err(|e| format!("Failed to create rollback dir: {e}"))?;
            return Ok(self.config.rollback_dir.join("none"));
        }

        tokio::fs::create_dir_all(&self.config.rollback_dir)
            .await
            .map_err(|e| format!("Failed to create rollback dir: {e}"))?;

        let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
        let backup_name = format!(
            "{}-{}",
            self.config
                .installed_binary
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "binary".into()),
            timestamp
        );
        let backup_path = self.config.rollback_dir.join(backup_name);

        tokio::fs::copy(&self.config.installed_binary, &backup_path)
            .await
            .map_err(|e| format!("Failed to copy backup: {e}"))?;

        Ok(backup_path)
    }

    /// Stop the running process if a PID file is configured.
    pub async fn stop_current(&self) -> Result<(), String> {
        let pid_file = match &self.config.pid_file {
            Some(p) => p,
            None => return Ok(()),
        };

        if !pid_file.exists() {
            return Ok(());
        }

        let pid_str = tokio::fs::read_to_string(pid_file)
            .await
            .map_err(|e| format!("Failed to read PID file: {e}"))?;
        let pid: u32 = pid_str
            .trim()
            .parse()
            .map_err(|e| format!("Invalid PID in file: {e}"))?;

        if !is_process_alive(pid) {
            // Stale PID file — clean up.
            let _ = tokio::fs::remove_file(pid_file).await;
            return Ok(());
        }

        self.stop_pid(pid).await?;

        // Clean up the now-stale PID file.
        let _ = tokio::fs::remove_file(pid_file).await;
        Ok(())
    }

    /// Send SIGTERM to a PID and wait for it to exit.
    async fn stop_pid(&self, pid: u32) -> Result<(), String> {
        // Send SIGTERM via `kill` (cross-platform-ish; Windows would need taskkill).
        #[cfg(unix)]
        {
            let status = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .status()
                .await
                .map_err(|e| format!("Failed to spawn kill: {e}"))?;
            if !status.success() {
                return Err(format!("kill -TERM exited with {:?}", status.code()));
            }
        }
        #[cfg(windows)]
        {
            let status = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T"])
                .status()
                .await
                .map_err(|e| format!("Failed to spawn taskkill: {e}"))?;
            if !status.success() {
                return Err(format!("taskkill exited with {:?}", status.code()));
            }
        }

        // Poll until the process exits, with timeout.
        let deadline = std::time::Instant::now() + Duration::from_secs(STOP_TIMEOUT_SECS);
        while std::time::Instant::now() < deadline {
            if !is_process_alive(pid) {
                return Ok(());
            }
            sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
        }

        // Last resort: SIGKILL.
        #[cfg(unix)]
        {
            let _ = Command::new("kill")
                .args(["-KILL", &pid.to_string()])
                .status()
                .await;
        }

        if is_process_alive(pid) {
            return Err(format!(
                "PID {pid} did not exit within {STOP_TIMEOUT_SECS}s and SIGKILL failed"
            ));
        }
        Ok(())
    }

    /// Move the new binary into the installed path. Atomic on the same
    /// filesystem; falls back to remove+copy across filesystems (the remove
    /// step is also necessary on macOS to avoid code-signing cache issues
    /// when overwriting existing Mach-O binaries).
    pub async fn install_new(&self, source: &Path) -> Result<(), String> {
        if let Some(parent) = self.config.installed_binary.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create install parent: {e}"))?;
        }
        // Try rename first (atomic on same FS, also avoids the macOS issue
        // because rename replaces the inode atomically rather than overwriting
        // the file content in place).
        if tokio::fs::rename(source, &self.config.installed_binary)
            .await
            .is_ok()
        {
            return Ok(());
        }
        // Fall back to remove + copy (cross-filesystem).
        // The remove forces a new inode, which avoids macOS code-signing
        // cache invalidation when overwriting existing Mach-O binaries.
        if self.config.installed_binary.exists() {
            let _ = tokio::fs::remove_file(&self.config.installed_binary).await;
        }
        tokio::fs::copy(source, &self.config.installed_binary)
            .await
            .map_err(|e| format!("Failed to copy new binary: {e}"))?;
        let _ = tokio::fs::remove_file(source).await;
        // Ensure executable bit is set on Unix.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&self.config.installed_binary)
                .map_err(|e| format!("Failed to stat installed binary: {e}"))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&self.config.installed_binary, perms)
                .map_err(|e| format!("Failed to chmod installed binary: {e}"))?;
        }
        Ok(())
    }

    /// Start the newly-installed binary in the background, recording its PID.
    /// Detects immediate crashes via `try_wait` after a brief settling period.
    pub async fn start_new(&self) -> Result<u32, String> {
        let pid_file = self
            .config
            .pid_file
            .as_ref()
            .ok_or_else(|| "start_new called without a pid_file".to_string())?;

        // Spawn detached: redirect stdout/stderr to /dev/null.
        let mut child = std::process::Command::new(&self.config.installed_binary)
            .arg("start")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .stdin(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn new binary: {e}"))?;

        let pid = child.id();

        // Write PID file.
        if let Some(parent) = pid_file.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        tokio::fs::write(pid_file, pid.to_string())
            .await
            .map_err(|e| format!("Failed to write PID file: {e}"))?;

        // Brief wait then check for immediate exit. If the process crashed
        // on startup, try_wait() will reap it and we can detect the failure
        // before treating the spawn as successful.
        sleep(Duration::from_millis(500)).await;
        match child.try_wait() {
            Ok(Some(status)) => {
                let _ = tokio::fs::remove_file(pid_file).await;
                return Err(format!(
                    "Process exited immediately with status {:?}",
                    status.code()
                ));
            }
            Ok(None) => {
                // Still running — good.
            }
            Err(e) => {
                return Err(format!("Failed to check process status: {e}"));
            }
        }

        // Detach: forget the Child so the destructor doesn't try to wait/kill.
        std::mem::forget(child);
        Ok(pid)
    }

    /// Verify the new process is alive after startup.
    pub async fn health_check(&self, pid: u32) -> Result<(), String> {
        let deadline = std::time::Instant::now() + Duration::from_secs(HEALTH_TIMEOUT_SECS);

        // Wait for the process to be alive.
        while std::time::Instant::now() < deadline {
            if is_process_alive(pid) {
                break;
            }
            sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
        }
        if !is_process_alive(pid) {
            return Err(format!(
                "PID {pid} did not start within {HEALTH_TIMEOUT_SECS}s"
            ));
        }

        // Optional secondary health check.
        if let Some(check_path) = &self.config.post_start_health_check {
            let output = timeout(
                Duration::from_secs(HEALTH_TIMEOUT_SECS),
                Command::new(check_path)
                    .arg(&self.config.installed_binary)
                    .output(),
            )
            .await
            .map_err(|_| format!("health check timed out after {HEALTH_TIMEOUT_SECS}s"))?
            .map_err(|e| format!("Failed to spawn health check: {e}"))?;
            if !output.status.success() {
                return Err(format!(
                    "Post-start health check exited with {:?}: {}",
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }

        // Process is up and (optionally) responding. Wait briefly to give it
        // time to crash if it's going to.
        sleep(Duration::from_secs(2)).await;
        if !is_process_alive(pid) {
            return Err(format!("PID {pid} crashed within 2s of startup"));
        }

        Ok(())
    }

    /// Restore the binary from a backup file.
    ///
    /// Removes the existing installed binary first (rather than copying over
    /// it). On macOS, copying over an existing Mach-O binary triggers
    /// code-signing cache invalidation and the next exec is killed by signal.
    /// Removing first forces a fresh inode and bypasses the issue.
    pub async fn rollback(&self, backup: &Path) -> Result<(), String> {
        if !backup.exists() || backup.file_name().map(|n| n == "none").unwrap_or(false) {
            return Err("No backup available to restore".to_string());
        }
        // Delete first to force a new inode (avoids macOS code-signing cache).
        if self.config.installed_binary.exists() {
            tokio::fs::remove_file(&self.config.installed_binary)
                .await
                .map_err(|e| format!("Failed to remove old binary before rollback: {e}"))?;
        }
        tokio::fs::copy(backup, &self.config.installed_binary)
            .await
            .map_err(|e| format!("Failed to restore backup: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&self.config.installed_binary)
                .map_err(|e| format!("Failed to stat restored binary: {e}"))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&self.config.installed_binary, perms)
                .map_err(|e| format!("Failed to chmod restored binary: {e}"))?;
        }
        Ok(())
    }

    /// Build a release binary inside the given workspace directory.
    /// This is the production path: caller passes a Sandbox directory.
    pub async fn build_release(
        &self,
        workspace: &Path,
        bin_name: &str,
    ) -> Result<PathBuf, Temm1eError> {
        let result = timeout(
            Duration::from_secs(BUILD_TIMEOUT_SECS),
            Command::new("cargo")
                .args(["build", "--release", "--bin", bin_name])
                .current_dir(workspace)
                .output(),
        )
        .await
        .map_err(|_| Temm1eError::Tool("cargo build timed out".into()))?
        .map_err(|e| Temm1eError::Tool(format!("Failed to spawn cargo: {e}")))?;
        if !result.status.success() {
            return Err(Temm1eError::Tool(format!(
                "cargo build failed: {}",
                String::from_utf8_lossy(&result.stderr)
            )));
        }
        let path = workspace.join("target").join("release").join(bin_name);
        if !path.exists() {
            return Err(Temm1eError::Tool(format!(
                "Build succeeded but binary not at {}",
                path.display()
            )));
        }
        Ok(path)
    }
}

/// Cross-platform process liveness check.
///
/// Distinguishes a running process from a zombie. On Unix, `kill -0` returns
/// success for zombies (because zombies are still PID-table entries that can
/// receive signals), so we additionally check process state via `ps -o state=`.
/// A state starting with 'Z' is a zombie and counts as dead.
fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // Linux fast-path: check /proc for the process and read its state.
        let proc_path = format!("/proc/{pid}/status");
        if let Ok(content) = std::fs::read_to_string(&proc_path) {
            // Look for "State:	Z" (zombie) — anything else is alive.
            for line in content.lines() {
                if let Some(state) = line.strip_prefix("State:") {
                    return !state.trim().starts_with('Z');
                }
            }
            return true;
        }

        // macOS / BSD path: use kill -0 + ps to detect zombies.
        let signal_check = std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !signal_check {
            return false;
        }

        // kill -0 succeeded — verify it's not a zombie via ps.
        let ps_output = std::process::Command::new("ps")
            .args(["-o", "state=", "-p", &pid.to_string()])
            .output();
        if let Ok(output) = ps_output {
            let state = String::from_utf8_lossy(&output.stdout).trim().to_string();
            // 'Z' means zombie. Empty means ps couldn't find it.
            if state.is_empty() {
                return false;
            }
            return !state.starts_with('Z');
        }

        // If ps fails entirely, trust kill -0.
        true
    }
    #[cfg(windows)]
    {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}")])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_config(installed: PathBuf, rollback: PathBuf, staging: PathBuf) -> DeployConfig {
        DeployConfig {
            installed_binary: installed,
            rollback_dir: rollback,
            staging_dir: staging,
            pid_file: None,
            post_start_health_check: None,
        }
    }

    /// Create a tiny shell-script "binary" that responds to --version.
    async fn write_fake_binary(path: &Path, exit_code: i32, version_string: &str) {
        let script = format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo '{version_string}'; exit {exit_code}; fi\nsleep 60\n"
        );
        tokio::fs::write(path, script).await.unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(path, perms).unwrap();
        }
    }

    #[tokio::test]
    async fn validate_existing_binary_succeeds() {
        let tmp = tempdir().unwrap();
        let bin = tmp.path().join("fake-bin");
        write_fake_binary(&bin, 0, "fake 1.0.0").await;

        let config = make_config(
            tmp.path().join("installed"),
            tmp.path().join("rollback"),
            tmp.path().join("staging"),
        );
        let deployer = Deployer::new(config);
        assert!(deployer.validate_new_binary(&bin).await.is_ok());
    }

    #[tokio::test]
    async fn validate_missing_binary_fails() {
        let tmp = tempdir().unwrap();
        let config = make_config(
            tmp.path().join("installed"),
            tmp.path().join("rollback"),
            tmp.path().join("staging"),
        );
        let deployer = Deployer::new(config);
        let result = deployer
            .validate_new_binary(&tmp.path().join("nonexistent"))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[tokio::test]
    async fn validate_failing_version_fails() {
        let tmp = tempdir().unwrap();
        let bin = tmp.path().join("bad-bin");
        write_fake_binary(&bin, 1, "broken").await;

        let config = make_config(
            tmp.path().join("installed"),
            tmp.path().join("rollback"),
            tmp.path().join("staging"),
        );
        let deployer = Deployer::new(config);
        let result = deployer.validate_new_binary(&bin).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exited"));
    }

    #[tokio::test]
    async fn backup_current_creates_timestamped_copy() {
        let tmp = tempdir().unwrap();
        let installed = tmp.path().join("installed/temm1e");
        tokio::fs::create_dir_all(installed.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&installed, "old binary content")
            .await
            .unwrap();

        let config = make_config(
            installed.clone(),
            tmp.path().join("rollback"),
            tmp.path().join("staging"),
        );
        let deployer = Deployer::new(config);
        let backup = deployer.backup_current().await.unwrap();
        assert!(backup.exists());
        let content = tokio::fs::read_to_string(&backup).await.unwrap();
        assert_eq!(content, "old binary content");
        // Backup name should contain "temm1e"
        let name = backup.file_name().unwrap().to_string_lossy();
        assert!(name.contains("temm1e"));
    }

    #[tokio::test]
    async fn backup_current_when_no_install_returns_none_marker() {
        let tmp = tempdir().unwrap();
        let config = make_config(
            tmp.path().join("nonexistent/temm1e"),
            tmp.path().join("rollback"),
            tmp.path().join("staging"),
        );
        let deployer = Deployer::new(config);
        let backup = deployer.backup_current().await.unwrap();
        assert_eq!(backup.file_name().unwrap(), "none");
    }

    #[tokio::test]
    async fn install_new_replaces_installed_binary() {
        let tmp = tempdir().unwrap();
        let installed = tmp.path().join("installed/temm1e");
        let new = tmp.path().join("new/temm1e");
        tokio::fs::create_dir_all(installed.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::create_dir_all(new.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&installed, "old").await.unwrap();
        tokio::fs::write(&new, "new").await.unwrap();

        let config = make_config(
            installed.clone(),
            tmp.path().join("rollback"),
            tmp.path().join("staging"),
        );
        let deployer = Deployer::new(config);
        deployer.install_new(&new).await.unwrap();

        let content = tokio::fs::read_to_string(&installed).await.unwrap();
        assert_eq!(content, "new");
        // Source should be moved (not copied), so it shouldn't exist anymore.
        assert!(!new.exists());
    }

    #[tokio::test]
    async fn rollback_restores_from_backup() {
        let tmp = tempdir().unwrap();
        let installed = tmp.path().join("installed/temm1e");
        let backup = tmp.path().join("rollback/temm1e-bak");
        tokio::fs::create_dir_all(installed.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::create_dir_all(backup.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&installed, "broken new binary")
            .await
            .unwrap();
        tokio::fs::write(&backup, "good old binary").await.unwrap();

        let config = make_config(
            installed.clone(),
            tmp.path().join("rollback"),
            tmp.path().join("staging"),
        );
        let deployer = Deployer::new(config);
        deployer.rollback(&backup).await.unwrap();

        let content = tokio::fs::read_to_string(&installed).await.unwrap();
        assert_eq!(content, "good old binary");
    }

    #[tokio::test]
    async fn rollback_with_none_marker_fails() {
        let tmp = tempdir().unwrap();
        let config = make_config(
            tmp.path().join("installed"),
            tmp.path().join("rollback"),
            tmp.path().join("staging"),
        );
        let deployer = Deployer::new(config);
        let result = deployer.rollback(&tmp.path().join("rollback/none")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn stop_current_with_no_pid_file_is_noop() {
        let tmp = tempdir().unwrap();
        let config = make_config(
            tmp.path().join("installed"),
            tmp.path().join("rollback"),
            tmp.path().join("staging"),
        );
        let deployer = Deployer::new(config);
        assert!(deployer.stop_current().await.is_ok());
    }

    #[tokio::test]
    async fn stop_current_cleans_stale_pid_file() {
        let tmp = tempdir().unwrap();
        let pid_file = tmp.path().join("temm1e.pid");
        // Use a PID that almost certainly doesn't exist (very high number).
        tokio::fs::write(&pid_file, "999999").await.unwrap();

        let mut config = make_config(
            tmp.path().join("installed"),
            tmp.path().join("rollback"),
            tmp.path().join("staging"),
        );
        config.pid_file = Some(pid_file.clone());
        let deployer = Deployer::new(config);
        assert!(deployer.stop_current().await.is_ok());
        assert!(!pid_file.exists(), "stale PID file should be cleaned up");
    }

    #[tokio::test]
    async fn deploy_outcome_variants_construct() {
        // Sanity check: all variants are usable.
        let _ = DeployOutcome::Success {
            new_binary_path: PathBuf::from("/x"),
            backup_path: PathBuf::from("/y"),
            new_pid: Some(1),
        };
        let _ = DeployOutcome::AbortedBeforeSwap {
            stage: "x".into(),
            reason: "y".into(),
        };
        let _ = DeployOutcome::RolledBack {
            stage: "x".into(),
            reason: "y".into(),
            backup_restored: true,
        };
        let _ = DeployOutcome::RollbackFailed {
            original_failure: "x".into(),
            rollback_failure: "y".into(),
        };
    }

    #[tokio::test]
    async fn full_swap_without_pid_file_succeeds() {
        // End-to-end swap of a fake binary, no process management.
        let tmp = tempdir().unwrap();
        let installed = tmp.path().join("installed/temm1e");
        let new = tmp.path().join("new/temm1e");
        tokio::fs::create_dir_all(installed.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::create_dir_all(new.parent().unwrap())
            .await
            .unwrap();

        // Create the "old" installed binary
        write_fake_binary(&installed, 0, "old 1.0.0").await;
        // Create the "new" candidate binary
        write_fake_binary(&new, 0, "new 2.0.0").await;

        let config = make_config(
            installed.clone(),
            tmp.path().join("rollback"),
            tmp.path().join("staging"),
        );
        let deployer = Deployer::new(config);
        let report = deployer.swap(&new).await;

        match report.outcome {
            DeployOutcome::Success { .. } => {}
            other => panic!("Expected Success, got {other:?}"),
        }
        // The installed binary should now be the new one (moved, not copied).
        assert!(installed.exists());
        assert!(!new.exists(), "source should be moved into installed path");
        // A backup should exist in the rollback dir.
        let backups: Vec<_> = std::fs::read_dir(tmp.path().join("rollback"))
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(backups.len(), 1);
    }

    #[tokio::test]
    async fn swap_aborts_on_invalid_new_binary() {
        let tmp = tempdir().unwrap();
        let installed = tmp.path().join("installed/temm1e");
        tokio::fs::create_dir_all(installed.parent().unwrap())
            .await
            .unwrap();
        write_fake_binary(&installed, 0, "old").await;

        let bad_new = tmp.path().join("nonexistent");

        let config = make_config(
            installed.clone(),
            tmp.path().join("rollback"),
            tmp.path().join("staging"),
        );
        let deployer = Deployer::new(config);
        let report = deployer.swap(&bad_new).await;

        match report.outcome {
            DeployOutcome::AbortedBeforeSwap { stage, .. } => {
                assert_eq!(stage, "validate");
            }
            other => panic!("Expected AbortedBeforeSwap, got {other:?}"),
        }
        // Installed binary should be untouched.
        assert!(installed.exists());
        let content = tokio::fs::read_to_string(&installed).await.unwrap();
        assert!(content.contains("old"));
    }
}
