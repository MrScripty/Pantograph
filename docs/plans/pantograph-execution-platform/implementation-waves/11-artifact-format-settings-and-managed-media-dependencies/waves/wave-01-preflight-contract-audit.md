# Wave 01: Preflight Contract Audit

## Objective

Complete the Stage `11` start gate, audit current inline media paths, and freeze
backend-owned contracts before parallel source implementation starts.

## Dependencies

None beyond Stage `06` repair completion and the Stage `11` plan.

## Workers

Single host-owned wave. Explorer agents may inspect code, but shared contract
files are host-owned until frozen.

## Write Set

- `docs/plans/pantograph-execution-platform/11-artifact-format-settings-and-managed-media-dependencies.md`
- `docs/plans/pantograph-execution-platform/implementation-waves/11-artifact-format-settings-and-managed-media-dependencies/**`
- backend contract files selected by the host after audit, initially
  `crates/pantograph-workflow-service/src/workflow/artifact_contracts.rs`
  and related public re-export/test files

## Forbidden Files

- `.pantograph/**`
- `assets/**`
- generated output under `target/**`, `dist/**`, or `src/generated/**`
- frontend implementation files unless this wave is explicitly expanded

## Standards

- `CODING-STANDARDS.md`
- `ARCHITECTURE-PATTERNS.md`
- `SECURITY-STANDARDS.md`
- `DEPENDENCY-STANDARDS.md`
- `LANGUAGE-BINDINGS-STANDARDS.md`
- `languages/rust/RUST-STANDARDS.md`
- `languages/rust/RUST-UNSAFE-STANDARDS.md`

## Verification

- `cargo test -p pantograph-workflow-service contract`
- `cargo fmt --all -- --check`
- Source-audit report review

## Report Path

`reports/wave-01-host-preflight-contract-audit.md`

## Escalation Rules

Escalate if contracts require a new crate, unsafe code, dependency manifest
changes, unmanaged host PATH discovery, or changing unrelated dirty files.

## Integration Order

Host-owned documentation and contract freeze commit lands before later waves.

