# TEMM1E Release Protocol

**MANDATORY checklist before pushing any release to `main`.** Claude MUST execute every step and verify results before committing.

## Pre-Release Verification

### 1. Compilation Gates (ALL must pass)

```bash
cargo check --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo test --workspace
```

Record the test count from the output. Every `test result: ok` line's passed count must be summed.

### 2. Collect Metrics

Run these and record the values:

```bash
# Test count
cargo test --workspace 2>&1 | grep 'test result' | awk '{sum += $4} END {print sum}'

# Source files and lines
find . -name '*.rs' -not -path './target/*' | wc -l
find . -name '*.rs' -not -path './target/*' | xargs wc -l | tail -1

# Crate count
ls crates/ | wc -l
```

### 3. Version Bump

Update version in `Cargo.toml` (workspace.package.version). This propagates to all crates.

**File:** `Cargo.toml` line ~22
```toml
[workspace.package]
version = "X.Y.Z"
```

### 4. README.md — Update ALL of These

| Location | What | How to get value |
|----------|------|------------------|
| Line ~13 | Version badge | Match Cargo.toml version |
| Line ~14 | Test count badge | From step 2 |
| Line ~15 | Provider count badge | Count providers in Supported Providers table |
| Line ~23 | Version tagline (`**vX.Y: ...`) | New feature headline |
| Line ~25 | Hero line (`XXK lines \| N tests`) | From step 2 |
| Line ~94 | Lines of Rust metric | From step 2 (exact count + file count) |
| Line ~95 | Tests metric | From step 2 |
| Line ~97 | Workspace crates metric | From step 2 (`crates/ count + 1 binary`) |
| Line ~101 | AI providers metric | Count all providers including variants |
| Line ~103 | Agent tools metric | Count tools in Tools table |
| Line ~354 | Architecture crate count text | Match workspace crates metric |
| Line ~356-372 | Architecture tree | Must list all crates in `crates/` |
| Line ~425 | `temm1e update` example version | Match Cargo.toml version |
| Line ~443 | Dev section test count | From step 2 |
| Release Timeline | New entry at TOP | Date, version, features, test count |
| **Tem's Lab section** | Add subsection for new cognitive systems | If the release adds a new cognitive system (crate in temm1e-*), add a Tem's Lab subsection with: what it does, how it works, key metrics/benchmarks, A/B test results if applicable, and links to research papers/design docs. Follow the existing subsection format (see Lambda Memory, Conscious, Perpetuum as examples). |

### 5. CLAUDE.md — Update Stale References

| Location | What |
|----------|------|
| Line ~7 | Crate count ("X crates plus a root binary") |
| Workspace structure | Must list all crates in `crates/` |

### 6. src/main.rs — Check Version-Sensitive Code

| What | How to verify |
|------|---------------|
| `default_model()` | All providers have entries, new providers added |
| System prompt provider list | All providers listed with models |
| `auth status` output | Recommended model is correct |

### 7. Interactive Interface Parity Gate — MANDATORY

**Every interactive interface must be fully wired before release.** TEMM1E has
independent initialization paths for:

| Interface | Code path | Notes |
|---|---|---|
| **TUI** (`temm1e tui`) | `crates/temm1e-tui/src/agent_bridge.rs :: spawn_agent()` | Primary install method per README — new users hit this first |
| **CLI chat** (`temm1e chat`) | `src/main.rs :: Commands::Chat` | Primary self-test vehicle |
| **Server/messengers** (`temm1e start`) | `src/main.rs :: Commands::Start` | Routes Telegram/Discord/WhatsApp/Slack through the shared agent init |

Each path maintains its own tool-list assembly, Hive init (or lack of),
agent construction, and background-service wiring. **Wiring a feature into
one path does NOT wire it into the others.** This has silently drifted
multiple times — v5.4.0 shipped with JIT `spawn_swarm` registered only in
server; TUI has been missing a dozen subsystems for multiple releases.

#### Parity matrix (update every release)

Before pushing a release, confirm every shipped feature is wired in every
interactive interface. Current snapshot (update at each release):

| Feature | Server | CLI chat | TUI |
|---|:---:|:---:|:---:|
| Hive + JIT `spawn_swarm` | ✓ | ✓ (v5.4.0) | **must wire** |
| Consciousness observer | ✓ | ✓ | **must wire** |
| Social intelligence / user profile | ✓ | ✓ | **must wire** |
| Personality config (`.with_personality`) | ✓ | ✓ | **must wire** |
| Perpetuum (`.with_perpetuum_temporal`) | ✓ | ✓ | **must wire** |
| MCP servers | ✓ | ✓ | **must wire** |
| Custom tools + `SelfCreateTool` | ✓ | ✓ | **must wire** |
| TemDOS cores + `invoke_core` | ✓ | ✓ | **must wire** |
| Eigen-Tune engine | ✓ | ✓ | **must wire** |
| Witness / Cambium trust / auto-oath | ✓ | — | — (opt-in, OK to defer) |
| Shared memory strategy (`/memory lambda`) | ✓ | ✓ | **must wire** |
| Vault + skill_registry wiring | ✓ | ✓ | **must wire** |

#### Per-interface verification steps

For **every** interface above, run a smoke test and confirm the feature's
startup log appears. Example registration-log anchors:

```bash
# JIT swarm wired?
./target/release/temm1e chat < /dev/null 2>&1 | grep "JIT spawn_swarm tool registered"
./target/release/temm1e tui < /dev/null 2>&1 | grep "JIT spawn_swarm tool registered"

# Consciousness wired?
./target/release/temm1e chat < /dev/null 2>&1 | grep "Tem Conscious: LLM-powered consciousness initialized"

# Hive instance created (needed for swarm)?
./target/release/temm1e chat < /dev/null 2>&1 | grep "Many Tems initialized"
```

For every feature listed in the release: include a greppable registration
log message, run the smoke test against EACH interface, and paste the
greps into the release report. A missing log = a missing wiring = blocker
for release unless the release notes EXPLICITLY declare non-parity for
that interface.

#### Rules

1. **Never declare a feature "shipped" based on one interface's logs.**
   CLI chat passing ≠ TUI passing ≠ server passing. Each must be checked.
2. **"Feature wasn't triggered" must be distinguished from "feature wasn't
   registered."** Grep for the registration log first; then grep for the
   execution log. Skipping step one confuses a wiring bug with a
   behavioural outcome.
3. **When adding a feature, add its registration log alongside the code.**
   Future wiring checks depend on this anchor.
4. **Opt-in features** (Witness, Cambium, auto_planner_oath) may legitimately
   be absent from interactive interfaces, but the release notes must call
   that out.
5. **Non-interactive paths** (MCP client only, tool servers, background
   cron) are out of scope for the parity gate but still need their own
   smoke tests.

### 8. Final Verification

After all edits, re-run:

```bash
cargo check --workspace
cargo test --workspace 2>&1 | grep 'test result' | awk '{sum += $4} END {print sum}'
```

Confirm test count still matches what you wrote in README.

### 9. Commit and Push

```bash
git add -A
git commit -m "vX.Y.Z: <one-line summary>"
git push origin main
```

### 10. Tag and Release

**CRITICAL — this triggers the GitHub release pipeline.**
Without the tag, no binaries are built and no GitHub release is created.

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

After pushing the tag:
1. GitHub Actions `release.yml` triggers automatically
2. CI runs checks (cargo check, test, clippy, fmt)
3. Builds 4 binaries (linux-musl, linux-desktop, macos-x86, macos-arm)
4. Creates GitHub Release with binaries + checksums + auto release notes
5. **Verify the release**: `gh run list --limit 1` and check the Actions tab

Do NOT declare the release done until the workflow completes successfully
and the GitHub Release page shows all 4 binaries.

## Files That Do NOT Need Updating

- **`docs/benchmarks/BENCHMARK_REPORT.md`** — Version in title reflects when benchmark was taken. Only update if benchmarks are re-run.
- **`crates/temm1e-skills/src/lib.rs`** — Test fixtures use hardcoded version strings. These are test data, not release metadata.
- **`Cargo.lock`** — Auto-generated from Cargo.toml changes.
- **Release Timeline old entries** — Historical entries are frozen. Never modify past versions.

## Common Mistakes

| Mistake | Consequence |
|---------|-------------|
| Bump README but not Cargo.toml | `temm1e -V` shows old version |
| Bump Cargo.toml but not README badges | GitHub page shows old version |
| Forget `temm1e update` example version | Users see wrong version in help output |
| Forget test count in dev section | `cargo test` comment says wrong number |
| Forget architecture tree | New crate invisible in docs |
| Forget CLAUDE.md crate count | Claude starts sessions with wrong context |
| Forget `default_model()` for new provider | Omitting model in config crashes with wrong default |
| Push without running tests | Broken code on main |
| Push without tagging | **No GitHub release created, no binaries built, users stuck on old version** |
| Tag before pushing commit | Tag points to wrong commit |
