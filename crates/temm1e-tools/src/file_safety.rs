//! File operation safety guards.
//!
//! Tem is designed for full computer use on the user's behalf. The file tools
//! intentionally allow read/write across the entire filesystem the user can
//! reach. This module enforces the only two boundaries that matter:
//!
//! 1. **System integrity** — paths that would brick the OS if written to.
//!    These are catastrophic and irreversible. Even when running as root,
//!    Tem refuses to write to them via the file tool.
//!
//! 2. **Tem self-integrity** — paths that would crash the running Tem
//!    instance if overwritten. The currently-running binary and the
//!    immutable watchdog binary.
//!
//! Reads are NEVER blocked here. The OS already gates reads via Unix
//! permissions, and reading a file does not brick anything.
//!
//! Backend systems (Cambium deploy, Vigil inbox, Vault, Memory, etc.) bypass
//! this module entirely by using `tokio::fs::` directly. This module only
//! protects the LLM-controlled file tool path.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Cached canonical path of the currently-running temm1e binary.
static RUNNING_BINARY: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Cached canonical path of the temm1e-watchdog binary (sibling of main binary).
static WATCHDOG_BINARY: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Initialize the running binary and watchdog paths. Call once at startup.
/// Safe to call multiple times — only the first call has effect.
pub fn init() {
    let _ = RUNNING_BINARY.get_or_init(|| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.canonicalize().ok())
    });
    let _ = WATCHDOG_BINARY.get_or_init(|| {
        let main = RUNNING_BINARY.get().and_then(|o| o.as_ref())?;
        let parent = main.parent()?;
        #[cfg(target_os = "windows")]
        let watchdog_name = "temm1e-watchdog.exe";
        #[cfg(not(target_os = "windows"))]
        let watchdog_name = "temm1e-watchdog";
        let candidate = parent.join(watchdog_name);
        candidate.canonicalize().ok().or(Some(candidate))
    });
}

/// Check whether writing to `path` would damage the system or the Tem instance.
///
/// Returns `Some(reason)` if the write is blocked, `None` if allowed.
/// The path should already be canonicalized by the caller.
pub fn is_catastrophic_write(path: &Path) -> Option<&'static str> {
    // Tem self-protection: never overwrite the running binary or watchdog.
    if let Some(Some(running)) = RUNNING_BINARY.get() {
        if path == running {
            return Some("would corrupt the currently-running Tem binary");
        }
    }
    if let Some(Some(watchdog)) = WATCHDOG_BINARY.get() {
        if path == watchdog {
            return Some("would corrupt the immutable Tem watchdog binary");
        }
    }

    let path_str = path.to_string_lossy();

    #[cfg(target_os = "windows")]
    {
        // Windows: case-insensitive matching.
        let lower = path_str.to_lowercase();
        // SAM hive and other registry hives.
        if lower.contains("\\windows\\system32\\config\\sam")
            || lower.contains("\\windows\\system32\\config\\security")
            || lower.contains("\\windows\\system32\\config\\system")
            || lower.contains("\\windows\\system32\\config\\software")
        {
            return Some("Windows registry hive — system would not boot");
        }
        // Boot manager.
        if lower.starts_with("c:\\boot\\") || lower.starts_with("c:\\bootmgr") {
            return Some("Windows boot manager — system would not boot");
        }
        // Physical drives.
        if lower.starts_with("\\\\.\\physicaldrive") || lower.starts_with("\\\\.\\harddisk") {
            return Some("raw physical drive — would wipe the disk");
        }
        // Windows kernel and core system files.
        if lower.starts_with("c:\\windows\\system32\\ntoskrnl")
            || lower.starts_with("c:\\windows\\system32\\hal.dll")
            || lower.starts_with("c:\\windows\\system32\\winload")
        {
            return Some("Windows kernel — system would not boot");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Unix/Linux/macOS: case-sensitive matching.

        // Bootloader and kernel.
        if path_str.starts_with("/boot/")
            || path_str == "/boot"
            || path_str.starts_with("/efi/")
            || path_str == "/efi"
        {
            return Some("bootloader/kernel path — system would not boot");
        }

        // macOS boot services.
        if path_str.starts_with("/System/Library/CoreServices/boot.efi")
            || path_str.starts_with("/System/Library/Kernels/")
        {
            return Some("macOS boot path — system would not boot");
        }

        // Raw disk devices — writing wipes the disk.
        if path_str.starts_with("/dev/sd")
            || path_str.starts_with("/dev/nvme")
            || path_str.starts_with("/dev/disk")
            || path_str.starts_with("/dev/hd")
            || path_str.starts_with("/dev/rdisk")
            || path_str.starts_with("/dev/mmcblk")
        {
            return Some("raw disk device — would wipe the disk");
        }

        // Authentication databases — wrong format = sudo lockout.
        if path_str == "/etc/shadow"
            || path_str == "/etc/gshadow"
            || path_str == "/etc/sudoers"
            || path_str.starts_with("/etc/sudoers.d/")
            || path_str == "/etc/passwd"
            || path_str == "/etc/group"
        {
            return Some("authentication database — would lock out users");
        }

        // Mount config — wrong format = unbootable.
        if path_str == "/etc/fstab" || path_str == "/etc/crypttab" {
            return Some("mount config — system would not boot");
        }

        // Kernel firmware control.
        if path_str.starts_with("/sys/firmware/") || path_str.starts_with("/sys/power/") {
            return Some("kernel firmware/power control — could brick hardware");
        }

        // Kernel sysrq trigger — instant reboot/halt.
        if path_str == "/proc/sysrq-trigger" {
            return Some("kernel sysrq trigger — would reboot/halt the system");
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn allows_normal_user_paths() {
        assert!(is_catastrophic_write(&PathBuf::from("/home/user/code/main.rs")).is_none());
        assert!(is_catastrophic_write(&PathBuf::from("/tmp/scratch.txt")).is_none());
        assert!(is_catastrophic_write(&PathBuf::from("/etc/hosts")).is_none());
        assert!(is_catastrophic_write(&PathBuf::from("/etc/nginx/nginx.conf")).is_none());
        assert!(is_catastrophic_write(&PathBuf::from("/var/log/syslog")).is_none());
    }

    #[test]
    fn allows_tem_self_files() {
        // Tem manages its own dir freely
        assert!(
            is_catastrophic_write(&PathBuf::from("/home/user/.temm1e/credentials.toml")).is_none()
        );
        assert!(is_catastrophic_write(&PathBuf::from("/home/user/.temm1e/memory.db")).is_none());
        assert!(is_catastrophic_write(&PathBuf::from("/home/user/.temm1e/vault.enc")).is_none());
    }

    #[test]
    fn allows_credentials() {
        // Reading SSH keys is allowed (this fn is for writes, but the test
        // documents intent — credentials are NOT in the catastrophic list)
        assert!(is_catastrophic_write(&PathBuf::from("/home/user/.ssh/id_rsa")).is_none());
        assert!(is_catastrophic_write(&PathBuf::from("/home/user/.aws/credentials")).is_none());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn blocks_authentication_databases() {
        assert!(is_catastrophic_write(&PathBuf::from("/etc/shadow")).is_some());
        assert!(is_catastrophic_write(&PathBuf::from("/etc/sudoers")).is_some());
        assert!(is_catastrophic_write(&PathBuf::from("/etc/sudoers.d/custom")).is_some());
        assert!(is_catastrophic_write(&PathBuf::from("/etc/passwd")).is_some());
        assert!(is_catastrophic_write(&PathBuf::from("/etc/gshadow")).is_some());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn blocks_bootloader() {
        assert!(is_catastrophic_write(&PathBuf::from("/boot/vmlinuz")).is_some());
        assert!(is_catastrophic_write(&PathBuf::from("/boot/grub/grub.cfg")).is_some());
        assert!(is_catastrophic_write(&PathBuf::from("/efi/EFI/BOOT/BOOTX64.EFI")).is_some());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn blocks_disk_devices() {
        assert!(is_catastrophic_write(&PathBuf::from("/dev/sda")).is_some());
        assert!(is_catastrophic_write(&PathBuf::from("/dev/sda1")).is_some());
        assert!(is_catastrophic_write(&PathBuf::from("/dev/nvme0n1")).is_some());
        assert!(is_catastrophic_write(&PathBuf::from("/dev/disk0")).is_some());
        assert!(is_catastrophic_write(&PathBuf::from("/dev/disk0s1")).is_some());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn blocks_mount_config() {
        assert!(is_catastrophic_write(&PathBuf::from("/etc/fstab")).is_some());
        assert!(is_catastrophic_write(&PathBuf::from("/etc/crypttab")).is_some());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn blocks_kernel_control() {
        assert!(is_catastrophic_write(&PathBuf::from("/sys/firmware/efi")).is_some());
        assert!(is_catastrophic_write(&PathBuf::from("/sys/power/state")).is_some());
        assert!(is_catastrophic_write(&PathBuf::from("/proc/sysrq-trigger")).is_some());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn blocks_windows_system_paths() {
        assert!(
            is_catastrophic_write(&PathBuf::from("C:\\Windows\\System32\\config\\SAM")).is_some()
        );
        assert!(
            is_catastrophic_write(&PathBuf::from("c:\\windows\\system32\\config\\sam")).is_some()
        );
        assert!(is_catastrophic_write(&PathBuf::from("C:\\Boot\\BCD")).is_some());
        assert!(is_catastrophic_write(&PathBuf::from("\\\\.\\PhysicalDrive0")).is_some());
    }
}
