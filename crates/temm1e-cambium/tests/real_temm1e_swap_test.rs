//! # Real CLI test: swap an actual temm1e binary in an isolated test dir.
//!
//! This is the Phase 5d "real binary swap" test. It uses the actual built
//! temm1e release binary, NOT a shell-script fake. The test runs ONLY when
//! both `target/release/temm1e` exists AND the env var
//! `TEMM1E_CAMBIUM_REAL_TEST=1` is set, to avoid running by default.
//!
//! ## Production safety
//!
//! - The test directory is `/tmp/temm1e-deploy-test/`. The user's actual
//!   temm1e installation is NEVER touched.
//! - The test does NOT start a real temm1e server (no API keys, no channels,
//!   no LLM calls). It only swaps the binary file and verifies version.
//! - All test artifacts are cleaned up at the end (or left for inspection
//!   if `TEMM1E_KEEP_TEST_ARTIFACTS=1` is set).
//!
//! ## What this test proves
//!
//! 1. The deploy module can swap a real Rust binary, not just shell scripts.
//! 2. `--version` health check works against actual temm1e.
//! 3. Backup and rollback work with multi-megabyte binaries.
//! 4. Timing data is captured for real-world deployment.
//!
//! ## Run with
//!
//! ```sh
//! TEMM1E_CAMBIUM_REAL_TEST=1 \
//!   cargo test -p temm1e-cambium --test real_temm1e_swap_test \
//!   -- --nocapture
//! ```

use std::path::PathBuf;

use temm1e_cambium::deploy::{DeployConfig, DeployOutcome, Deployer};
use tokio::process::Command;

fn workspace_root() -> PathBuf {
    // tests/ is at crates/temm1e-cambium/tests/, so go up 3 levels.
    std::env::current_dir()
        .unwrap()
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn real_temm1e_path() -> Option<PathBuf> {
    let candidate = workspace_root().join("target/release/temm1e");
    if candidate.exists() {
        Some(candidate)
    } else {
        None
    }
}

fn should_run_real_test() -> bool {
    std::env::var("TEMM1E_CAMBIUM_REAL_TEST").unwrap_or_default() == "1"
}

#[tokio::test]
async fn real_temm1e_binary_swap_end_to_end() {
    if !should_run_real_test() {
        println!("SKIPPED: set TEMM1E_CAMBIUM_REAL_TEST=1 to enable");
        return;
    }
    let real_bin = match real_temm1e_path() {
        Some(p) => p,
        None => {
            println!("SKIPPED: target/release/temm1e not built");
            return;
        }
    };

    println!("=== REAL TEMM1E SWAP TEST ===");
    println!("Source binary:   {}", real_bin.display());
    println!(
        "Source size:     {} bytes",
        std::fs::metadata(&real_bin).unwrap().len()
    );

    let test_dir = PathBuf::from("/tmp/temm1e-deploy-test");
    let installed = test_dir.join("install/temm1e");
    let staging = test_dir.join("staging");
    let rollback = test_dir.join("rollback");

    // Setup: create directories and copy the real binary as the "old" install.
    let _ = std::fs::remove_dir_all(&test_dir);
    std::fs::create_dir_all(installed.parent().unwrap()).unwrap();
    std::fs::create_dir_all(&staging).unwrap();
    std::fs::create_dir_all(&rollback).unwrap();
    std::fs::copy(&real_bin, &installed).unwrap();
    println!("Test dir:        {}", test_dir.display());
    println!("Installed at:    {}", installed.display());

    // Capture the "before" version.
    let before_output = Command::new(&installed)
        .arg("--version")
        .output()
        .await
        .unwrap();
    let before_version = String::from_utf8_lossy(&before_output.stdout)
        .trim()
        .to_string();
    println!("Before version:  {before_version}");
    assert!(before_version.contains("temm1e"));

    // Stage a "new" binary by copying the same real binary again. In a real
    // deployment this would be the freshly-built sandbox output.
    let new_binary_source = staging.join("temm1e-next");
    std::fs::copy(&real_bin, &new_binary_source).unwrap();
    println!("New binary at:   {}", new_binary_source.display());
    println!(
        "New binary size: {} bytes",
        std::fs::metadata(&new_binary_source).unwrap().len()
    );

    // Configure the deployer (NO pid_file — pure file-level swap).
    let config = DeployConfig {
        installed_binary: installed.clone(),
        rollback_dir: rollback.clone(),
        staging_dir: staging.clone(),
        pid_file: None,
        post_start_health_check: None,
    };
    let deployer = Deployer::new(config);

    // Run the swap.
    println!("\n--- Running swap ---");
    let report = deployer.swap(&new_binary_source).await;
    println!("Outcome:         {:?}", report.outcome);
    println!(
        "Validate:        {:?} ms",
        report.timings.validate_ms.unwrap_or(0)
    );
    println!(
        "Backup:          {:?} ms",
        report.timings.backup_ms.unwrap_or(0)
    );
    println!(
        "Stop:            {:?} ms",
        report.timings.stop_ms.unwrap_or(0)
    );
    println!(
        "Install:         {:?} ms",
        report.timings.install_ms.unwrap_or(0)
    );
    println!(
        "Start:           {:?} ms",
        report.timings.start_ms.unwrap_or(0)
    );
    println!(
        "Health check:    {:?} ms",
        report.timings.health_check_ms.unwrap_or(0)
    );
    println!("Total:           {} ms", report.timings.total_ms);

    // Verify success
    let (backup_path, _new_pid) = match report.outcome {
        DeployOutcome::Success {
            backup_path,
            new_pid,
            ..
        } => (backup_path, new_pid),
        other => panic!("expected Success, got {other:?}"),
    };

    // Verify the installed binary still responds to --version
    let after_output = Command::new(&installed)
        .arg("--version")
        .output()
        .await
        .unwrap();
    let after_version = String::from_utf8_lossy(&after_output.stdout)
        .trim()
        .to_string();
    println!("\nAfter version:   {after_version}");
    assert!(after_version.contains("temm1e"));
    assert_eq!(before_version, after_version, "version mismatch after swap");

    // Verify backup exists and is readable
    println!("Backup at:       {}", backup_path.display());
    assert!(backup_path.exists());
    let backup_size = std::fs::metadata(&backup_path).unwrap().len();
    println!("Backup size:     {backup_size} bytes");
    assert_eq!(
        backup_size,
        std::fs::metadata(&real_bin).unwrap().len(),
        "backup size mismatch"
    );

    // Run rollback (sanity check)
    println!("\n--- Testing rollback path ---");
    let rollback_result = deployer.rollback(&backup_path).await;
    println!("Rollback result: {rollback_result:?}");
    assert!(rollback_result.is_ok());

    // Diagnostic: verify restored file exists and has correct size
    let restored_meta = std::fs::metadata(&installed).unwrap();
    println!("Restored size:   {} bytes", restored_meta.len());
    println!("Restored perms:  {:?}", restored_meta.permissions());

    let restored_output = Command::new(&installed)
        .arg("--version")
        .output()
        .await
        .unwrap();
    let restored_stdout = String::from_utf8_lossy(&restored_output.stdout)
        .trim()
        .to_string();
    let restored_stderr = String::from_utf8_lossy(&restored_output.stderr)
        .trim()
        .to_string();
    println!("Restored stdout: {restored_stdout}");
    println!("Restored stderr: {restored_stderr}");
    println!("Restored exit:   {:?}", restored_output.status.code());

    if restored_stdout != before_version {
        // If the binary won't run after rollback, that's a real issue worth
        // documenting. Print everything we can about the file state.
        println!("\nDIAGNOSTIC:");
        println!("  Backup metadata:");
        let bm = std::fs::metadata(&backup_path).unwrap();
        println!("    size:  {}", bm.len());
        println!("    perms: {:?}", bm.permissions());
        println!("  Installed metadata:");
        let im = std::fs::metadata(&installed).unwrap();
        println!("    size:  {}", im.len());
        println!("    perms: {:?}", im.permissions());

        // Compare bytes
        let backup_bytes = std::fs::read(&backup_path).unwrap();
        let installed_bytes = std::fs::read(&installed).unwrap();
        println!(
            "  Backup vs installed bytes equal: {}",
            backup_bytes == installed_bytes
        );
    }
    assert_eq!(restored_stdout, before_version);

    // Cleanup unless asked to keep
    let keep = std::env::var("TEMM1E_KEEP_TEST_ARTIFACTS").unwrap_or_default() == "1";
    if !keep {
        let _ = std::fs::remove_dir_all(&test_dir);
        println!("\nCleaned up test dir.");
    } else {
        println!("\nTest artifacts kept at {}", test_dir.display());
    }

    println!("\n=== REAL SWAP TEST PASSED ===");
}
