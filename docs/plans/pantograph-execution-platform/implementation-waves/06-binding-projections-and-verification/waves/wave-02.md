# Wave 02: Binding Projection Implementation

## Objective

Implement thin projection DTOs and wrappers over the frozen native Rust base
API.

## Workers

| Worker | Primary Write Set | Report |
| ------ | ----------------- | ------ |
| uniffi-projections | `crates/pantograph-uniffi/` | `reports/wave-02-worker-uniffi-projections.md` |
| rustler-projections | `crates/pantograph-rustler/` | `reports/wave-02-worker-rustler-projections.md` |

## Worker Boundaries

- `uniffi-projections` owns non-BEAM FFI DTOs, error envelope conversion, and
  generated-artifact configuration needed by C# and Python lanes.
- `rustler-projections` owns BEAM/Rustler DTO projection, NIF error mapping,
  and BEAM resource lifecycle wrappers.

## Shared Files

Native Rust base API crates, generated artifacts, workspace manifests,
lockfiles, and package metadata are host-owned unless wave `01` assigns one
explicit owner.

## Forbidden Files

- Canonical node, runtime, attribution, or ledger semantics.
- Hand-edited generated binding artifacts.
- GUI implementation.

## Verification

```bash
cargo test -p pantograph-uniffi
cargo test -p pantograph-rustler
```

## Integration Order

1. Integrate `uniffi-projections`.
2. Integrate `rustler-projections`.
3. Host resolves shared generated-artifact and manifest updates.

## Escalation Rules

- Stop if a binding lane requires semantics absent from the Rust base API.
- Stop if generated artifacts must be hand-edited.
