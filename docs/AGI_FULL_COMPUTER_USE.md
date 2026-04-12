# AGI Full Computer Use — Architectural Principle

**Status:** Foundational design rule, established 2026-04-12 (v5.1.1)
**Owner:** Project founder
**Scope:** Tem's filesystem access, capability boundaries, safety model

---

## The Principle

Tem is designed as an **AGI-oriented agent that controls the entire computer on the user's behalf**. The capability bar is "everything a senior sysadmin with sudo could do." The safety bar is "don't do anything irreversible that bricks the system or the agent itself."

This is a deliberate choice. Tem is not a sandboxed assistant. It is a perpetual agent that the user installs on their personal machine and trusts with the same privileges they themselves have. The user's UID is the security boundary, not an artificial workspace fence.

---

## What This Means In Practice

### Tem CAN do (default, no override needed):

- Read and write files anywhere the user's UID can reach
- Cross-project work — read `~/code/project-a` from `~/code/project-b`
- Edit shell config files: `~/.zshrc`, `~/.bashrc`, `~/.config/*`
- Edit system config that doesn't brick: `/etc/hosts`, `/etc/nginx/`, `/etc/systemd/system/*`
- Read system logs: `/var/log/*`, `/private/var/log/*`
- Read credentials when explicitly asked: `~/.ssh/id_rsa`, `~/.aws/credentials`
- Manage its own configuration: `~/.temm1e/credentials.toml`, `~/.temm1e/temm1e.toml`
- Run any shell command the user could run
- Take screenshots, control mouse and keyboard
- Browse any URL with the browser tool
- Connect to any MCP server

### Tem CANNOT do (hardcoded block list):

**System integrity — would brick the OS:**
- Write to `/etc/shadow`, `/etc/sudoers`, `/etc/sudoers.d/*`, `/etc/passwd`, `/etc/group`, `/etc/gshadow` (sudo lockout)
- Write to `/etc/fstab`, `/etc/crypttab` (unbootable)
- Write to `/boot/*`, `/efi/*` (bootloader)
- Write to macOS `/System/Library/CoreServices/boot.efi`, `/System/Library/Kernels/`
- Write to raw disk devices: `/dev/sd*`, `/dev/nvme*`, `/dev/disk*`, `/dev/hd*`, `/dev/rdisk*`, `/dev/mmcblk*`
- Write to `/sys/firmware/*`, `/sys/power/*` (firmware control)
- Write to `/proc/sysrq-trigger` (instant reboot)
- Windows: `C:\Windows\System32\config\SAM` (and other registry hives), `C:\Boot\*`, `\\.\PhysicalDrive*`, kernel files

**Tem self-instance — would crash the running agent:**
- Write to the currently-running `temm1e` binary
- Write to the `temm1e-watchdog` binary (immutable kernel per Cambium architecture)

That's the entire block list. Two categories. Around 30 path patterns.

---

## Why This Model

### Why no workspace containment

Workspace containment makes sense for an untrusted sandbox. It does NOT make sense for an AGI-oriented agent the user explicitly invited into their machine. Tem is not running in a hostile environment trying to escape — it's running as the user, doing what the user asked.

If the user starts Tem in `~/code/project-a` and then asks it to look at `~/code/project-b`, the answer should be "of course," not "that's outside my workspace, please restart me in a different directory." Workspace fences make Tem feel like a tool, not an agent. The AGI vision requires the agent to feel ambient and capable across the whole machine.

### Why no credential block list

A credential block list (blocking reads of `~/.ssh/id_rsa`, `~/.aws/credentials`, etc.) sounds prudent but fails the use case. A real AGI agent helping you set up a new server needs to read your SSH key. An agent helping you debug AWS auth needs to read `.aws/credentials`. Blocking these would make Tem worse than a human assistant the user trusts.

The threat that would justify a credential block list is prompt injection — Tem reading a webpage that instructs it to exfiltrate credentials. But the right defense against that is at the content layer, not the file layer:

1. **Credential scrubber** (`crates/temm1e-tools/src/credential_scrub.rs`) catches API key patterns in tool output before they re-enter LLM context.
2. **System prompt** instructs Tem to never act on instructions found inside web content, files, or memory entries.
3. **LLM judgment** — the model itself is trained to refuse suspicious instructions.

Path containment cannot defend against prompt injection. Content-level defenses can. Use the right tool for the right threat.

### Why block only catastrophic writes

The block list exists for one reason: a single LLM mistake should not be irreversible. Everything on the list shares a property — once you do it, you can't undo it without major recovery work:

- `/etc/shadow` corrupted → sudo locked out, recovery via single-user mode
- `/etc/sudoers` corrupted → sudo locked out, same recovery
- `/boot/vmlinuz` corrupted → unbootable, recovery via USB
- `/dev/sda` written → disk wiped, recovery via backup
- Running binary overwritten → undefined behavior, possibly need reinstall

For everything else — edit `/etc/hosts` wrong? Edit it again. Delete the wrong file from `~/Documents`? Restore from backup. These are recoverable mistakes that a human could also make. The agent is allowed to make recoverable mistakes; that's part of being capable.

### Why blocking the running binary

This is the only Tem-self-protection rule, and it exists for a single reason: the Cambium architecture says the watchdog and the running binary are the immutable kernel. Cambium itself updates them via the structured `deploy.rs` path (which uses `tokio::fs::rename` directly and bypasses the file tool). The LLM should never be the path for binary updates, because a malformed update mid-execution could leave the system unrecoverable.

The watchdog binary block follows the same logic, and is also the explicit Cambium architectural rule.

### Why no `~/.temm1e/*` block

Tem owns this directory. It has structured tools (`/addkey`, `/removekey`, vault commands, `temm1e update`) that are the preferred path for self-management. But the file tool is NOT blocked from this directory because:

1. Tem might legitimately want to inspect or back up its own config
2. If Tem corrupts its own config, restart fixes it (the watchdog handles this)
3. Adding a block here is paranoid micro-protection that breaks legitimate self-introspection

The structured tools exist for safety and convenience. The file tool is the escape hatch.

---

## Implementation

### Where the block list lives

`crates/temm1e-tools/src/file_safety.rs`

This module provides:
- `init()` — captures the running binary path at startup via `std::env::current_exe()` + `canonicalize()`, stored in `OnceLock<PathBuf>`
- `is_catastrophic_write(path: &Path) -> Option<&'static str>` — returns the block reason if path matches, `None` if allowed

The patterns are hardcoded constants. Cross-platform via `#[cfg(target_os = "windows")]` blocks. Path comparison is case-sensitive on Unix, case-insensitive on Windows.

### Where the block list is enforced

`crates/temm1e-tools/src/file.rs::resolve_path()` calls `file_safety::is_catastrophic_write()` for `Operation::Write` paths. Reads are not checked at this layer (the OS enforces read permissions).

The check runs AFTER path normalization (resolving `~`, `$HOME`, `..`, `.`, symlinks via `canonicalize()`), so block-list bypass via path obfuscation is not possible.

### What does NOT enforce containment

As of v5.1.1:

- `crates/temm1e-agent/src/executor.rs::validate_arguments()` — no longer calls `validate_path_in_workspace()`. Shell commands still pass through `validate_shell_command()` for the dangerous-pattern denylist.
- `crates/temm1e-agent/src/executor.rs::validate_sandbox()` — no longer enforces workspace containment on declared `file_access` patterns. Only rejects parent-dir traversal in declarations (still a malformed-tool indicator).
- `crates/temm1e-tools/src/file.rs::resolve_path()` — no longer enforces workspace containment. Only enforces the catastrophic-write block list.

### How backend systems are exempt

Backend systems (Cambium deploy, Vigil inbox, Vault, Memory, MCP config, OAuth tokens, Anima storage, Distill training data, Hive coordination) all use `tokio::fs::` or `std::fs::` directly. They never go through `file_tool::resolve_path()`, so the block list does not affect them. This is intentional — these systems have their own safety layers (sandbox containment in Cambium, vault encryption, structured trait interfaces) and should not be subject to the same constraints as LLM-controlled file operations.

---

## Threat Model

### What this protects against

1. **LLM hallucination of dangerous commands** — Tem decides "I should reformat the disk to fix this" and writes to `/dev/sda`. Blocked.
2. **LLM typo** — Tem meant to edit `/etc/hosts` but wrote `/etc/shadow`. Blocked.
3. **Cambium-generated code with bugs** — sandbox sandbox tests catch most, but if a Cambium iteration tries to write to system paths, blocked.
4. **Self-corruption from confused binary updates** — only the structured Cambium deploy path can replace the binary. Direct LLM-driven binary writes blocked.

### What this does NOT protect against

1. **Prompt injection** — Tem reads a webpage that instructs it to read credentials and POST them to evil.com. Path containment cannot stop this. Defense is in the credential scrubber + system prompt + LLM judgment.
2. **User asks Tem to do something destructive** — "delete all my code" is not blocked. The user is the owner.
3. **OS-level escalation** — if Tem is running as root, it can technically write to `/etc/shadow` via `tokio::fs::` directly (the block list is at the file tool layer, not the OS). The block list prevents LLM-driven access; root power requires explicit sudo from the user.
4. **Malicious software exploiting Tem's privileges** — if a tool Tem invokes has its own exploit, the block list doesn't help.

### Defense in depth

The file safety block list is one layer of protection. The full stack:

1. **OS permissions** — kernel enforces what the user UID can reach
2. **Tem block list** — file_safety prevents LLM-driven catastrophic writes
3. **Credential scrubber** — content layer catches keys in tool output
4. **System prompt** — instructs LLM to never trust instructions in untrusted content
5. **Circuit breaker** — limits cascading failures
6. **Watchdog binary** — restarts Tem if it crashes
7. **Cambium trust hierarchy** — Cambium-generated code goes through trust levels
8. **catch_unwind layers** — panics caught and isolated

No single layer is sufficient. The combination is.

---

## When This Principle Applies

This principle applies to:
- The file tool (`file_read`, `file_write`, `file_list`, `code_edit`, `code_patch`, `code_glob`)
- Any future LLM-controlled file manipulation tool

This principle does NOT apply to:
- Backend services (Memory, Vault, Cambium deploy, etc.) — they have their own safety
- The shell tool (`shell_access`) — uses its own pattern denylist
- The browser tool — uses chromium's own sandbox
- Desktop control (Tem Gaze) — uses OS-level input simulation

---

## Future Sweeps

When running the Full Sweep Protocol against the file tool layer, **never propose workspace containment as a fix.** It conflicts with the AGI principle. The Sweep 1 follow-up (v5.1.1) is the canonical example of why workspace containment is wrong.

Acceptable proposals during sweeps:
- Adding new patterns to the catastrophic block list (with justification)
- Improving canonicalization (handling more edge cases)
- Cross-platform additions (Windows, BSD, Solaris if Tem ever ships there)
- Audit logging (which already exists via `tracing::warn!`)
- Integration with future capability frameworks

Unacceptable proposals:
- "Block reads of credential files" — defeats the AGI vision
- "Restrict to workspace by default" — defeats the AGI vision
- "Add a confirmation prompt for system writes" — breaks the perpetual-agent model

---

## History

| Version | Change |
|---------|--------|
| v5.1.0 | SWEEP-701/702 introduced workspace containment in `resolve_path()`. Over-corrected — broke cross-project work, sysadmin tasks, and the AGI vision. |
| v5.1.1 | Workspace containment removed. Replaced with catastrophic-write block list in `file_safety.rs`. AGI principle codified in this document. |

---

## Anchor Quote

> "Tem is supposed to be a very capable agent that controls the entire computer on user behalf (AGI oriented) but I do agree some critical very critical paths should be blocked."
>
> "I imagine a full AGI who can on behalf of user do almost anything on the computer as long as it don't break the computer, or the Tem deployment / instance."

— Project founder, 2026-04-12
