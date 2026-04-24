# Wave 02: Attribution Implementation

## Objective

Implement durable attribution in non-overlapping worker slices after the
contract freeze.

## Workers

| Worker | Primary Write Set | Report |
| ------ | ----------------- | ------ |
| attribution-domain-storage | `crates/pantograph-runtime-attribution/` | `reports/wave-02-worker-attribution-domain-storage.md` |
| workflow-service-cutover | `crates/pantograph-workflow-service/` | `reports/wave-02-worker-workflow-service-cutover.md` |

## Worker: attribution-domain-storage

Owns validated IDs, records, lifecycle state machine, command errors,
repository traits, SQLite schema/migrations, digest-only credential storage,
and attribution crate README.

Allowed adjacent write set:

- none without host approval.

Forbidden:

- `crates/pantograph-workflow-service/`
- binding crates
- GUI files
- workspace manifests unless host assigns dependency ownership.

Verification:

```bash
cargo test -p pantograph-runtime-attribution
```

## Worker: workflow-service-cutover

Owns workflow-service use cases that consume durable attribution, create
workflow-run records before execution, and remove or internalize affected
workflow-session public surfaces.

Allowed adjacent write set:

- `crates/pantograph-embedded-runtime/` only for compile fixes caused by renamed
  workflow-service APIs.

Forbidden:

- `crates/pantograph-runtime-attribution/` except read-only context.
- binding crates unless the host creates a separate wave.
- GUI files.

Verification:

```bash
cargo test -p pantograph-workflow-service
```

## Shared Files

`Cargo.toml`, `Cargo.lock`, public facade exports, and ADRs are host-owned.

## Integration Order

1. Integrate `attribution-domain-storage`.
2. Run `cargo test -p pantograph-runtime-attribution`.
3. Integrate `workflow-service-cutover`.
4. Run targeted workflow-service tests.
5. Host resolves shared manifest or facade changes in a separate integration
   commit if needed.

## Escalation Rules

- Stop if a worker needs another worker's primary write set.
- Stop if preserving a backward-compatible workflow-session wrapper seems
  necessary; the stage plan requires clean replacement or removal.
