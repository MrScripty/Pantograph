# Pass 04: Tooling, Tests, Release, and Dependency Findings

Audit date: 2026-04-21

## Scope
This pass reviewed local quality gates, CI coverage, test strategy, dependency
ownership, launcher behavior, release readiness, and version/toolchain pinning.

## Standards Applied
- `TOOLING-STANDARDS.md`: blocking quality gates, lint ratchet, formatting, hooks, decision traceability.
- `TESTING-STANDARDS.md`: test placement, cross-layer acceptance, binding verification, replay/recovery/idempotency checks.
- `DEPENDENCY-STANDARDS.md`: lockfiles, centralized shared versions, dependency ownership.
- `LAUNCHER-STANDARDS.md`: long-form CLI, idempotent install, workflow extensions, managed state, release smoke.
- `RELEASE-STANDARDS.md`: changelog, artifacts, SBOM, toolchain pinning.
- `ACCESSIBILITY-STANDARDS.md`: semantic interactions and a11y lint enforcement.

## Local Verification Results
- `npm run lint:critical` failed on `ImageOutputNode.svelte` DOM mutation.
- `npm run typecheck` passed.
- `npm run lint:full` failed on
  `src/components/runtime-manager/ManagedRuntimeSummaryGrid.svelte` with
  `svelte/no-useless-mustaches`.
- `cargo check` passed but emitted many unused/dead-code warnings across
  `node-engine`, `workflow-nodes`, `pantograph-workflow-service`,
  `pantograph-uniffi`, and `pantograph` Tauri modules.

## Findings

### P04-F01: CI Does Not Run the Main JS/Rust Quality Gates
Severity: High

Evidence:
- `.github/workflows/` contains `runtime-separation-check.yml` and
  `headless-embedding-contract.yml`.
- There is no visible general CI workflow that runs `npm ci`, `npm run
  lint:critical`, `npm run typecheck`, `npm run test:frontend`, root `cargo
  check`, or workspace test slices on every PR.

Standards conflict:
- Tooling standards require blocking lint, typecheck, tests, and failure
  aggregation. Full lint can be temporarily non-blocking only with a ratchet.

Required direction:
- Add a general CI workflow with separate jobs for critical lint, typecheck,
  frontend tests, Rust check/tests, launcher smoke, and dependency audit.
- Keep binding-specific CI as a separate workflow or as additional jobs.

### P04-F02: Full Lint and Critical Lint Are Already Red
Severity: High

Evidence:
- Critical lint failure: `ImageOutputNode.svelte` appendChild.
- Full lint failure: `ManagedRuntimeSummaryGrid.svelte` useless mustache.

Required direction:
- Fix these before introducing a no-new-lint ratchet.
- Once zero or baselined, add `lint:no-new` and make it blocking in CI.

### P04-F03: Accessibility Enforcement Is Incomplete for Svelte
Severity: Medium

Evidence:
- `eslint.config.mjs` uses `eslint-plugin-svelte`, but not `eslint-plugin-jsx-a11y`
  because this is not JSX.
- The repo relies on Svelte rules and custom critical anti-patterns, but there
  is no explicit accessibility audit gate equivalent to the standards'
  recommended semantic interaction rules.
- Pattern scans found many `role="button"` and `onclick` sites that need
  semantic review, though some are valid Svelte 5 button usage.

Required direction:
- Add Svelte-specific a11y rule coverage and component smoke checks for keyboard
  behavior in canvas/draggable contexts.
- Audit generic interactive elements and icon-only buttons for accessible names.

### P04-F04: Test Strategy Is Split but Not Documented as a Repo Policy
Severity: Medium

Evidence:
- TypeScript tests are colocated and run through `npm run test:frontend`.
- Rust uses crate-local unit tests and some integration tests.
- Binding checks exist in scripts and GitHub Actions.
- The root docs do not clearly declare the hybrid placement strategy and which
  commands are canonical per layer.

Required direction:
- Add a testing guide or root README section documenting colocated frontend
  tests, crate-local Rust tests, binding smoke checks, and cross-layer acceptance paths.

### P04-F05: Dependency Ownership Is Partially Centralized but Inconsistent
Severity: Medium

Evidence:
- Root `Cargo.toml` centralizes several shared workspace dependencies.
- Some member crates still use direct versions for dependencies also declared
  at the workspace level, for example `serde_json = "1"`, `tokio = { version =
  "1", ... }`, `async-trait = "0.1"`, `thiserror = "1"`, and `log = "0.4"`.
- `packages/svelte-graph/package.json` has peer dependencies but no package-local
  scripts/dev dependencies for its tests, which currently run from the root.

Required direction:
- Move repeated Rust dependency versions to workspace inheritance where shared.
- Document root ownership of package test tooling or add package-local scripts
  that call root-owned tooling intentionally.

### P04-F06: Toolchain Pinning Is Missing
Severity: Medium

Evidence:
- The audit found `.editorconfig`, but no root `rust-toolchain.toml`,
  `.node-version`/`.nvmrc`, `.python-version`, or release changelog automation config.

Standards conflict:
- Release standards require toolchain pinning for reproducible builds.

Required direction:
- Add a toolchain pinning policy and files after confirming intended versions.

### P04-F07: Launcher Is Mostly Aligned but Missing `--test`
Severity: Medium

Evidence:
- `launcher.sh` uses Bash strict mode, long-form action parsing, idempotent
  dependency checks, `--run`, `--run-release`, `--build`, `--build-release`,
  `--install`, `--help`, and `--release-smoke`.
- The project has canonical test commands, but launcher lacks `--test`.
- The help does not document managed state isolation for dev/test paths.

Required direction:
- Add `--test` as the canonical local test entrypoint.
- Document any launcher-managed state mode or explicitly note when host state is used.

### P04-F08: Release Workflow Is Incomplete
Severity: Medium

Evidence:
- `CHANGELOG.md` has an `[Unreleased]` section and useful categories.
- Binding packaging scripts produce C#/native artifacts and checksums.
- There is no visible SBOM generation, release workflow, artifact version naming
  policy, or toolchain pinning.

Required direction:
- Add release artifact naming/version policy, SBOM generation, and release CI
  once compliance refactors stabilize public contracts.

## Additional Issues Outside Pure Standards Compliance
- The many Rust warnings are currently non-blocking. Some represent true stale
  code, not only style.
- `package.json` has no `format:check` or `lint:no-new`, so the tooling standard
  cannot be fully enforced without adding new scripts.

## Pass 04 Remediation Themes
1. Fix red local gates first.
2. Add general CI with failure aggregation.
3. Document the hybrid test strategy and binding acceptance paths.
4. Normalize dependency ownership.
5. Add toolchain pinning and release hardening after public contracts settle.
