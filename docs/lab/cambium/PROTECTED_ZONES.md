# Cambium: Protected Zones

> **Status:** Authoritative reference -- enforced by verification pipeline  
> **Date:** 2026-04-08  
> **Purpose:** Defines immutable files, approval-required zones, and autonomous zones for cambium

---

## Trust Levels

| Level | Name | Policy |
|-------|------|--------|
| 0 | Immutable Kernel | NEVER modified by cambium. SHA-256 checksums verified before every growth cycle. Any mismatch aborts the pipeline. |
| 1 | Approval Required | Self-grow may propose changes but MUST NOT apply them without explicit human approval. Full pipeline verification required. |
| 2 | Autonomous Full Pipeline | Self-grow may modify autonomously. All compilation gates must pass: `cargo check`, `cargo clippy`, `cargo fmt --check`, `cargo test`. |
| 3 | Autonomous Basic | Self-grow may modify with minimal verification (syntax/lint only). |

---

## Level 0 -- Immutable Kernel

Every file listed below has a frozen SHA-256 checksum. The cambium pipeline verifies these checksums at the start of every growth cycle. If any checksum does not match, the entire cycle is aborted.

### Core Trait Definitions

| File | SHA-256 |
|------|---------|
| `crates/temm1e-core/src/traits/mod.rs` | `50195cf4b481bb4271e16c40877b9bd91d76133b09e77929e550c207cc8b5552` |
| `crates/temm1e-core/src/traits/channel.rs` | `2169ce6d4486c06307583414756bfde157f2eea2b75e00cb9582dda64ed1e2e5` |
| `crates/temm1e-core/src/traits/provider.rs` | `d2017a8d442c55e845941b89b17f1ab898f86b2a9ee5f4db39200847fdb32c22` |
| `crates/temm1e-core/src/traits/memory.rs` | `3811f91034a5d2b5ab0b008afaf7bc9cc0d5fbadcc677f69766dd9c202d3bca5` |
| `crates/temm1e-core/src/traits/tool.rs` | `5678df6bcd512d285c7e8ecc0c2bc047ededbc14c6a3b10423cf89337d2bd79c` |
| `crates/temm1e-core/src/traits/vault.rs` | `5a3843974e2a2f1e24747faeaeb1ee933d1077f55cb432a84dbe48cf2221a243` |
| `crates/temm1e-core/src/traits/filestore.rs` | `1471a9f2dbbe1296a5784865fa0f1b375a061f3863360d475dd3e72b3fe22754` |
| `crates/temm1e-core/src/traits/orchestrator.rs` | `2d691138c3baa315065393b155f49f183e22aa386938f7ec95b489aff561254a` |
| `crates/temm1e-core/src/traits/tenant.rs` | `a6aab50a65d8c02a111eb40b0981d678d37301a4792059ba42b8344d964dae7e` |
| `crates/temm1e-core/src/traits/identity.rs` | `bdcfa207886b3487691e201628b67d995c515255ae32df8b12511dd1cdaea62b` |
| `crates/temm1e-core/src/traits/observable.rs` | `f16e16294a01f6d14e61f4343e0987fe519e7d328251b5671fcba5c9a834832b` |
| `crates/temm1e-core/src/traits/tunnel.rs` | `35eca098389b2c76b78c9d361d99aa2396c305db381f3df52d8051e97d8e7fb2` |
| `crates/temm1e-core/src/traits/usage.rs` | `de5ddfb76938e769f19a9a1bd09a6dcb2e054c6765c2b511671c76e40d630197` |
| `crates/temm1e-core/src/traits/setup.rs` | `391c46f9873bcc1d164bd1540818e686e66c04de9eaabc5756ec540a14536201` |
| `crates/temm1e-core/src/traits/peripheral.rs` | `7920a7163ed76f06fdba3931e577ad038e73e287c35ce6fa3bc0479631ebe0ef` |

### Vault (Secrets Management)

| File | SHA-256 |
|------|---------|
| `crates/temm1e-vault/src/lib.rs` | `ad60f0e2c7b119ae2d1f0fc72c77f64a1cf3e212cbc3dea95b810dde962a4ef1` |
| `crates/temm1e-vault/src/local.rs` | `05c8c9a0fb004bb4f4eac54f6eb62df4fc966555cd50d1f70a33f615383f70ca` |
| `crates/temm1e-vault/src/detector.rs` | `e0b7e450219cbd1091f580785c0b1b500f4be5d9ba04e1acda8e017775989e8f` |
| `crates/temm1e-vault/src/resolver.rs` | `6589f8c355b4b523146b79a8d2f9a2475ea23c70ea44535ef8b744e0c1916e0f` |

### Workspace Root

| File | SHA-256 |
|------|---------|
| `Cargo.toml` (workspace) | `267c33d36782b192dd060bc049608285c49d726fa34aeeae3e21be14dfb7aa9a` |
| `build.rs` | `7a977171a283a182553e73b65685f67c1323442daf6928c79fc81a5e6bebf797` |

### Cambium Theory and Policy

| File | SHA-256 |
|------|---------|
| `docs/lab/cambium/THEORY.md` | Computed on finalization |
| `docs/lab/cambium/PROTECTED_ZONES.md` | Self-referential -- checksum updated on each release |

---

## Level 1 -- Approval Required

Self-grow may analyze and propose changes to these paths. Changes MUST NOT be applied without explicit human approval. Full pipeline verification is required before presenting the proposal.

| Pattern | Rationale |
|---------|-----------|
| `crates/temm1e-agent/**` | Agent runtime loop -- the brain of the system. Logic errors here affect every user interaction. |
| `crates/temm1e-gateway/**` | Network entry point and session management. Security boundary for all inbound traffic. |
| `crates/temm1e-providers/**` | API key handling and LLM request construction. Errors break all AI functionality. |
| `crates/temm1e-perpetuum/**` | Autonomous behavior control. Bugs here cause Tem to act without user intent. |
| `crates/temm1e-memory/**` | Data persistence layer. Corruption loses conversation history. |
| `src/main.rs` | Root wiring layer. Changes here affect how every crate interacts at startup. |

---

## Level 2 -- Autonomous with Full Pipeline

Self-grow may modify these paths autonomously. ALL compilation gates must pass before changes are committed:

```
cargo check --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo test --workspace
```

| Pattern | Rationale |
|---------|-----------|
| `crates/temm1e-tools/**` | Leaf crate. New tools add capability without changing contracts. |
| `crates/temm1e-skills/**` | Leaf crate. Skill registry is additive. |
| `crates/temm1e-channels/**` | Leaf crate. Each channel is isolated behind feature flags. |
| `crates/temm1e-cores/**` | Leaf crate. Cores are sandboxed sub-agents that inform but never steer. |
| `crates/temm1e-anima/**` | Leaf crate. Affects tone and style, not core behavior. |
| `crates/temm1e-observable/**` | Observation-only. Does not affect runtime behavior. |
| `crates/temm1e-hive/**` | Isolated subsystem behind feature flag. |
| `crates/temm1e-distill/**` | Closed-loop subsystem. Failures degrade to cloud provider. |

---

## Level 3 -- Autonomous Basic

Self-grow may modify these paths with minimal verification (syntax and lint checks only). No full compilation gate required.

| Pattern | Rationale |
|---------|-----------|
| `docs/**` | Documentation files. No runtime impact. |
| `tests/**` | Integration test files. Cannot break production. |
| `~/.temm1e/skills/*.md` | User-installed skill definitions. Markdown only. |
| `~/.temm1e/cores/*.md` | User-installed core definitions. Markdown only. |

---

## Checksum Verification Protocol

Before every cambium cycle:

1. Compute SHA-256 of every Level 0 file listed above.
2. Compare against the checksums in this document.
3. If ANY checksum does not match: abort the cycle, log the mismatch, alert the user.
4. If all checksums match: proceed with the growth cycle.

This verification is deterministic and LLM-agnostic. It cannot be argued with, convinced, or bypassed. The checksums are the ground truth.
