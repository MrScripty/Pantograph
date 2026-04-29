# Wave 07 Host Workbench DTO Parity

## Scope

Strengthened current Stage `12` DTO parity for workbench Network and Settings
surfaces with a shared Rust/TypeScript fixture.

## Changes

- Added `workbench_settings_network_contract.json` as a shared fixture for
  local Network status, ArtifactStore policy, artifact format settings, and
  artifact format capabilities.
- Added Rust contract-test deserialization coverage for the shared fixture.
- Added TypeScript workflow service tests that consume the same fixture through
  projection and command service methods.

## Verification

```bash
cargo test -p pantograph-workflow-service --test contract workbench_settings_network_cross_layer_fixture_deserializes
node --experimental-strip-types --test src/services/workflow/WorkflowService.projections.test.ts src/services/workflow/WorkflowService.commands.test.ts
npm run typecheck -- --pretty false
cargo test -p pantograph-workflow-service --test contract
npm run lint:full
cargo fmt --all -- --check
npm run traceability
```

## Residual Work

Remaining Stage `11` media surfaces still need parity strengthening once
producer-specific preview streams, active converter/library version capture,
and OCIO ABI validation settle.
