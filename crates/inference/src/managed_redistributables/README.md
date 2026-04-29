# managed_redistributables

## Purpose

This directory owns Pantograph-managed media redistributables that are not
runtime sidecars. It covers tool binaries such as `ffmpeg`, `ocioconvert`, and
`oiiotool`, plus native library/artifact dependencies such as OpenColorIO.

## Contents

| File | Description |
| ---- | ----------- |
| `catalog.rs` | Static catalog entries, platform metadata, support checks, and expected-file validation. |
| `contracts.rs` | Public DTOs for managed media dependencies, status projection, state, and leases. |
| `operations.rs` | Status, install-from-staging, select/default/activate, lease, and remove operations. |
| `paths.rs` | App-owned managed-dependency paths, platform keys, expected file paths, and timestamp helpers. |
| `state.rs` | Schema-versioned durable JSON state load/save and state-entry helpers. |

## Invariants

- Readiness is based only on app-owned managed dependency files, never host
  `PATH` or system library probing.
- Tool binaries and native library artifacts use managed redistributable terms,
  not managed runtime sidecar names.
- Network download and checksum verification are intentionally outside this
  module until source artifacts are pinned.
