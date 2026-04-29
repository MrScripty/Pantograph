# Wave 03 Worker Lease Attribution

## Scope

Worker-owned hardening for managed media dependency lease plans in
`crates/inference`, without adding a dependency on the new
`pantograph-media-conversion` crate.

## Files Changed

- `crates/inference/src/managed_media_dependencies.rs`
- `crates/inference/src/lib.rs`
- `crates/inference/tests/managed_media_dependencies.rs`
- `docs/plans/pantograph-execution-platform/implementation-waves/13-managed-media-conversion-leases/reports/wave-03-worker-lease-attribution.md`

## Implementation

- Added a host-neutral lease holder convention:
  `workflow_run:{workflow_run_id}/node:{node_id}/port:{port_id}/conversion:{conversion_id}`.
- Added helper validation/formatting functions for the holder convention. Each
  component must be non-empty, 128 characters or fewer, and limited to ASCII
  letters, digits, `:`, `.`, `_`, or `-`.
- Enforced holder validation before acquiring any managed redistributable
  leases, so malformed attribution fails before partial acquisition.
- Included the validated holder on each media dependency lease token. A
  multi-dependency plan now exposes dependency id, version, lease id, holder,
  install root, and expected files through the lease entries.
- Re-exported the holder convention helpers through the `inference` crate
  facade to match the existing managed media dependency API pattern.
- Kept the worker boundary independent from `pantograph-media-conversion`;
  mapping from this string convention to typed conversion ids remains host-owned.

## Tests

- Added direct convention coverage for accepted and rejected holders.
- Strengthened color-managed image acquisition coverage to assert all stable
  attribution fields for `oiiotool`, `ocioconvert`, and `OpenColorIO` leases.
- Strengthened failure rollback coverage to assert partially acquired leases are
  released when later color-managed dependency acquisition fails.
- Strengthened release coverage to assert all acquired dependency leases are
  removed from persisted managed redistributable state after plan release.

## Verification

- Passed: `cargo test -p inference --test managed_media_dependencies`
- Passed: `cargo fmt --all -- --check`

## Deferred Work

- Host/conversion integration should map typed `workflow_run_id`, `node_id`,
  `port_id`, and `conversion_id` values into the holder convention before
  calling the worker lease planner.
- Host/conversion integration should map each lease entry back into typed
  per-conversion dependency attribution records, preserving dependency id,
  version, lease id, install root, expected files, and holder.
- Any future typed holder contract should live at the host/media-conversion
  boundary and be passed into this worker as a validated string to avoid a crate
  dependency from `inference` to `pantograph-media-conversion`.
