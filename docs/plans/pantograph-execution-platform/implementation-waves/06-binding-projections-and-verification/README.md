# 06 Binding Projections And Verification Waves

## Purpose

Define concurrent waves for Stage `06`, native Rust base API projection,
UniFFI/Rustler binding DTOs, and language-native host verification.

## Stage Objective

Project backend-owned execution-platform contracts into C#, Python, and
Elixir/BEAM after the native Rust base API is resolved, with host-language
tests strong enough to make future binding additions repeatable.

## Waves

| Wave | Purpose |
| ---- | ------- |
| `waves/wave-01.md` | Host-owned Rust API/support-tier freeze and smoke command selection. |
| `waves/wave-02.md` | Parallel UniFFI and Rustler projection work. |
| `waves/wave-03.md` | Parallel language-native host smoke/acceptance tests. |
| `waves/wave-04.md` | Host-owned artifact/version integration, docs, ADR, and gate. |

## Global Host-Owned Files

- workspace manifests and lockfiles
- generated host binding artifacts
- package metadata shared by multiple lanes
- public native Rust facade files
- ADR and release note files

## Stage Verification

```bash
cargo test -p pantograph-uniffi
cargo test -p pantograph-rustler
cargo check --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

Host-language verification commands must be recorded in wave `01` before a lane
is marked supported.

## Re-Plan Triggers

- Native Rust base API is not stable enough to bind.
- C# or Python cannot provide language-native tests that load the real native
  artifact.
- A host lane needs semantics not present in the backend-owned Rust API.
