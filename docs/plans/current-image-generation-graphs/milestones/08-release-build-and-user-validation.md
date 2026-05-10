# Milestone 8: Release Build And User Validation

**Goal:** Build the application and provide a testable release artifact after
the graph, diagnostics, and PyTorch/diffusers slices are validated.

**Tasks:**

- [ ] Run affected Rust tests.
- [ ] Run affected frontend tests and type checks.
- [ ] Build frontend.
- [ ] Build release binary.
- [ ] Run serialization/contract tests for Rust DTOs and frontend TypeScript
  consumers touched by stale graph inspection or image-generation planning.
- [ ] Run standards spot checks: no production `unwrap()`/`expect()` in new
  paths, no new frontend polling loops, no raw stringly public planner states,
  no unbounded queues, and no large-file threshold crossed without review.
- [ ] Run lifecycle/transport spot checks for touched backend services:
  composition-root ownership, tracked tasks, cancellation, loopback-only
  binding, bounded connection/request limits, readiness timeouts, panic/error
  reporting, and graceful shutdown.
- [ ] Run boundary spot checks: saved workflow JSON, stale graph diagnostics,
  Tauri IPC payloads, Python worker envelopes, Pumas package facts, and artifact
  descriptors have validation/round-trip coverage.
- [ ] Run accessibility spot checks for IO inspector graph/details controls and
  any new settings navigation.
- [ ] Run path/resource spot checks for Pumas package roots, artifact roots,
  executable roots, dynamic-library roots, worker-visible paths, image
  dimensions, image counts, token/context limits, byte ranges, output sizes,
  and resource estimates.
- [ ] Run default, no-default-features, and all-features checks for affected
  public crates when runtime feature flags or optional dependencies changed.
- [ ] Run dependency ownership checks for any new Rust crate, Python package,
  Node package, or runtime tool dependency. Confirm the owning crate/package
  declares what it executes, transitive dependency cost is recorded when
  material, and lockfiles are updated only when intentionally changed.
- [ ] Run cross-platform spot checks for touched runtime/device code: platform
  behavior is behind platform modules or traits, inline Rust `cfg()` blocks
  remain within the standards exception, paths use platform APIs, and spaces in
  paths remain supported.
- [ ] Run device/runtime spot checks for explicit CPU and CUDA requests on the
  developer Linux/Windows target, macOS Metal/MPS when on macOS, auto
  resolution recording, llama.cpp runtime variants, and unavailable-device
  admission diagnostics.
- [ ] Inspect git status before the release build and before final reporting.
  Resolve or explicitly document any dirty source, test, config, generated,
  lockfile, build-output, sqlite WAL/SHM, or workflow fixture files.
- [ ] Launch or smoke the app enough to confirm Juggernaut graph visibility,
  Puma model resolution, stale diagnostic behavior, and image output artifact
  retention.

**Verification:**

- `cargo test` for affected crates.
- Frontend typecheck/test commands for touched UI/services.
- Release build succeeds.
- Standards spot-check notes are added to this plan's execution notes.
- Boundary, accessibility, and path/resource spot-check notes are added to this
  plan's execution notes.
- Lifecycle/transport and feature/dependency spot-check notes are added to this
  plan's execution notes when those areas are touched.
- Dependency ownership, cross-platform, and worktree hygiene spot-check notes
  are added to this plan's execution notes.
- Device/runtime variant spot-check notes are added to this plan's execution
  notes.
- Manual smoke notes are added to this plan's execution notes.

**Status:** Not started
