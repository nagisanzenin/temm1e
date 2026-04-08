//! # temm1e-watchdog
//!
//! A minimal supervisor for the temm1e binary. The watchdog:
//!
//! 1. Reads a binary path and an optional PID file from CLI args.
//! 2. Polls the supervised process every `interval` seconds.
//! 3. If the process is dead, restarts it from the configured binary path.
//! 4. Stops after `max_restarts` restart attempts within `restart_window_secs`
//!    to prevent infinite restart loops.
//! 5. Exits cleanly on SIGINT/SIGTERM.
//!
//! ## Why a separate binary
//!
//! This is the **immutable kernel** part of Cambium. The watchdog never
//! self-modifies. Even if the temm1e binary is replaced by a buggy version,
//! the watchdog continues running with its original code and can restore
//! service.
//!
//! ## Why so simple
//!
//! No AI, no network, no fancy features. The smaller the surface, the
//! smaller the bug surface. The whole binary is < 200 lines.
//!
//! ## Usage
//!
//! ```text
//! temm1e-watchdog \
//!   --binary /path/to/temm1e \
//!   --pid-file /path/to/temm1e.pid \
//!   --interval 5 \
//!   --max-restarts 5 \
//!   --restart-window-secs 60 \
//!   --start-args "start"
//! ```

use clap::Parser;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(name = "temm1e-watchdog")]
#[command(about = "Minimal supervisor for the temm1e binary", long_about = None)]
#[command(version)]
struct Args {
    /// Absolute path to the temm1e binary to supervise.
    #[arg(long)]
    binary: PathBuf,

    /// Absolute path to the PID file. Watchdog reads this to know which
    /// process to monitor; if the file does not exist, watchdog starts
    /// the binary itself.
    #[arg(long)]
    pid_file: PathBuf,

    /// Polling interval in seconds.
    #[arg(long, default_value = "10")]
    interval: u64,

    /// Maximum number of restart attempts within `restart_window_secs`
    /// before the watchdog gives up and exits.
    #[arg(long, default_value = "5")]
    max_restarts: u32,

    /// Window in seconds within which restart attempts are counted.
    /// Restarts older than this are forgotten.
    #[arg(long, default_value = "300")]
    restart_window_secs: u64,

    /// Subcommand args to pass to the binary on start (e.g., "start").
    /// Defaults to "start".
    #[arg(long, default_value = "start")]
    start_args: String,

    /// Run one cycle and exit. Used for tests.
    #[arg(long, hide = true)]
    once: bool,
}

fn main() -> std::process::ExitCode {
    let args = Args::parse();

    eprintln!("temm1e-watchdog: starting");
    eprintln!("  binary:              {}", args.binary.display());
    eprintln!("  pid_file:            {}", args.pid_file.display());
    eprintln!("  interval:            {}s", args.interval);
    eprintln!("  max_restarts:        {}", args.max_restarts);
    eprintln!("  restart_window_secs: {}s", args.restart_window_secs);

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_for_handler = stop_flag.clone();
    if let Err(e) = ctrlc_compatible(move || {
        eprintln!("temm1e-watchdog: signal received, exiting");
        stop_flag_for_handler.store(true, Ordering::SeqCst);
    }) {
        eprintln!("temm1e-watchdog: warning: failed to install signal handler: {e}");
    }

    let mut restart_history: Vec<Instant> = Vec::new();

    loop {
        if stop_flag.load(Ordering::SeqCst) {
            eprintln!("temm1e-watchdog: exiting cleanly");
            return std::process::ExitCode::SUCCESS;
        }

        let alive = check_process_alive(&args.pid_file);
        if !alive {
            eprintln!("temm1e-watchdog: process not alive, considering restart");

            // Prune restart history outside the window.
            let cutoff = Instant::now() - Duration::from_secs(args.restart_window_secs);
            restart_history.retain(|&t| t > cutoff);

            if restart_history.len() as u32 >= args.max_restarts {
                eprintln!(
                    "temm1e-watchdog: too many restarts ({} in {}s), giving up",
                    restart_history.len(),
                    args.restart_window_secs
                );
                return std::process::ExitCode::from(2);
            }

            match restart_binary(&args.binary, &args.start_args, &args.pid_file) {
                Ok(pid) => {
                    eprintln!("temm1e-watchdog: restarted as PID {pid}");
                    restart_history.push(Instant::now());
                }
                Err(e) => {
                    eprintln!("temm1e-watchdog: restart failed: {e}");
                    // Count failed restarts toward the limit too.
                    restart_history.push(Instant::now());
                }
            }
        }

        if args.once {
            return std::process::ExitCode::SUCCESS;
        }

        thread::sleep(Duration::from_secs(args.interval));
    }
}

/// Check if the process described by the PID file is alive.
/// Returns false if the file is missing, the PID is unparseable,
/// or the process is dead.
fn check_process_alive(pid_file: &PathBuf) -> bool {
    let pid_str = match std::fs::read_to_string(pid_file) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let pid: u32 = match pid_str.trim().parse() {
        Ok(p) => p,
        Err(_) => return false,
    };
    is_process_alive(pid)
}

fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        std::path::Path::new(&format!("/proc/{pid}")).exists()
            || Command::new("kill")
                .args(["-0", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}")])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }
}

/// Spawn the binary detached and write its PID to the PID file.
fn restart_binary(binary: &PathBuf, start_args: &str, pid_file: &PathBuf) -> Result<u32, String> {
    if !binary.exists() {
        return Err(format!("binary does not exist: {}", binary.display()));
    }
    let args: Vec<&str> = start_args.split_whitespace().collect();
    let child = Command::new(binary)
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn failed: {e}"))?;
    let pid = child.id();
    if let Some(parent) = pid_file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(pid_file, pid.to_string())
        .map_err(|e| format!("failed to write PID file: {e}"))?;
    // Detach: don't wait, but also don't drop the Child without forgetting it.
    std::mem::forget(child);
    Ok(pid)
}

/// Install a signal handler that calls `f` on SIGINT/SIGTERM.
/// We use a manual approach since we don't want to add a new dep for `ctrlc`.
fn ctrlc_compatible<F>(f: F) -> Result<(), String>
where
    F: Fn() + Send + 'static,
{
    #[cfg(unix)]
    {
        use std::os::raw::c_int;
        type SigHandler = extern "C" fn(c_int);
        extern "C" {
            fn signal(signum: c_int, handler: SigHandler) -> SigHandler;
        }
        // Use a static slot for the callback. This is a single-watchdog
        // process, so a single-slot global is fine.
        static mut CALLBACK: Option<Box<dyn Fn() + Send>> = None;
        // SAFETY: only set once at startup, before any signal can fire.
        unsafe {
            CALLBACK = Some(Box::new(f));
        }
        extern "C" fn handler(_sig: c_int) {
            // SAFETY: signal handlers run in a restricted context, but
            // calling the stored closure is safe because we only set it once.
            unsafe {
                #[allow(static_mut_refs)]
                if let Some(cb) = &CALLBACK {
                    cb();
                }
            }
        }
        const SIGINT: c_int = 2;
        const SIGTERM: c_int = 15;
        unsafe {
            signal(SIGINT, handler);
            signal(SIGTERM, handler);
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = f;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn args_parse_with_defaults() {
        let args = Args::try_parse_from([
            "temm1e-watchdog",
            "--binary",
            "/usr/local/bin/temm1e",
            "--pid-file",
            "/var/run/temm1e.pid",
        ])
        .unwrap();
        assert_eq!(args.interval, 10);
        assert_eq!(args.max_restarts, 5);
        assert_eq!(args.restart_window_secs, 300);
        assert_eq!(args.start_args, "start");
        assert!(!args.once);
    }

    #[test]
    fn args_parse_with_overrides() {
        let args = Args::try_parse_from([
            "temm1e-watchdog",
            "--binary",
            "/x",
            "--pid-file",
            "/y",
            "--interval",
            "30",
            "--max-restarts",
            "3",
            "--restart-window-secs",
            "60",
            "--start-args",
            "start --daemon",
            "--once",
        ])
        .unwrap();
        assert_eq!(args.interval, 30);
        assert_eq!(args.max_restarts, 3);
        assert_eq!(args.restart_window_secs, 60);
        assert_eq!(args.start_args, "start --daemon");
        assert!(args.once);
    }

    #[test]
    fn check_process_alive_returns_false_for_missing_pid_file() {
        let tmp = tempdir().unwrap();
        let pid_file = tmp.path().join("nonexistent.pid");
        assert!(!check_process_alive(&pid_file));
    }

    #[test]
    fn check_process_alive_returns_false_for_invalid_pid() {
        let tmp = tempdir().unwrap();
        let pid_file = tmp.path().join("temm1e.pid");
        let mut f = std::fs::File::create(&pid_file).unwrap();
        writeln!(f, "not a pid").unwrap();
        assert!(!check_process_alive(&pid_file));
    }

    #[test]
    fn check_process_alive_returns_false_for_dead_pid() {
        let tmp = tempdir().unwrap();
        let pid_file = tmp.path().join("temm1e.pid");
        // PID 999999 almost certainly doesn't exist.
        std::fs::write(&pid_file, "999999").unwrap();
        assert!(!check_process_alive(&pid_file));
    }

    #[test]
    fn check_process_alive_returns_true_for_self() {
        let tmp = tempdir().unwrap();
        let pid_file = tmp.path().join("temm1e.pid");
        let my_pid = std::process::id();
        std::fs::write(&pid_file, my_pid.to_string()).unwrap();
        assert!(check_process_alive(&pid_file));
    }

    #[test]
    fn restart_binary_fails_for_missing_binary() {
        let tmp = tempdir().unwrap();
        let bin = tmp.path().join("nonexistent");
        let pid_file = tmp.path().join("temm1e.pid");
        let result = restart_binary(&bin, "start", &pid_file);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn restart_binary_writes_pid_file_for_real_binary() {
        let tmp = tempdir().unwrap();
        // Use /bin/sleep as a real binary that won't exit immediately.
        let sleep_bin = std::path::PathBuf::from("/bin/sleep");
        if !sleep_bin.exists() {
            eprintln!("/bin/sleep not available, skipping test");
            return;
        }
        let pid_file = tmp.path().join("test.pid");
        let result = restart_binary(&sleep_bin, "60", &pid_file);
        assert!(result.is_ok());
        let pid = result.unwrap();
        assert!(pid_file.exists());
        let written = std::fs::read_to_string(&pid_file).unwrap();
        assert_eq!(written.trim(), pid.to_string());
        // Cleanup: kill the sleep process.
        let _ = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status();
    }
}
