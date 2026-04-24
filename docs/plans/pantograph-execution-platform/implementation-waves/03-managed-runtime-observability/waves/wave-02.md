# Wave 02: Runtime Observability Implementation

## Objective

Implement runtime-owned execution context and diagnostics behavior in bounded
parallel slices.

## Workers

| Worker | Primary Write Set | Report |
| ------ | ----------------- | ------ |
| runtime-context-capabilities | `crates/pantograph-embedded-runtime/src/` context and capability modules | `reports/wave-02-worker-runtime-context-capabilities.md` |
| diagnostics-event-adapter | `crates/pantograph-embedded-runtime/src/workflow_runtime.rs` and diagnostics projection modules | `reports/wave-02-worker-diagnostics-event-adapter.md` |
| cancellation-progress-guarantee | `crates/pantograph-embedded-runtime/src/` lifecycle and guarantee modules | `reports/wave-02-worker-cancellation-progress-guarantee.md` |

## Worker Boundaries

- `runtime-context-capabilities` owns context construction and managed
  capability traits.
- `diagnostics-event-adapter` owns baseline event projection from scheduler,
  runtime, and node-engine facts.
- `cancellation-progress-guarantee` owns cancellation tokens, progress handles,
  task lifecycle classification, and guarantee downgrade tests.

## Shared Files

Public facade exports, `Cargo.toml`, `Cargo.lock`, and ADR files are host-owned.

## Forbidden Files

- `crates/pantograph-diagnostics-ledger/`
- host binding generation
- GUI diagnostics views

## Verification

```bash
cargo test -p pantograph-embedded-runtime
cargo test -p node-engine
```

## Integration Order

1. Integrate `runtime-context-capabilities`.
2. Integrate `diagnostics-event-adapter`.
3. Integrate `cancellation-progress-guarantee`.
4. Host resolves shared facade and manifest changes.

## Escalation Rules

- Stop if durable ledger persistence is required.
- Stop if ordinary node code must hand-author baseline diagnostics.
