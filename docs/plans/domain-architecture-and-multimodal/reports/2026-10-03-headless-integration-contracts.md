# Headless Integration Contract Fixtures

## Source-grounded scope

PR #8 head `333e53db2b60c85c0190fd6ba34b75d3d5ecce46` passed all 878
workflow-service library tests in hosted job 111081103534, then failed three of
29 integration contracts. Runtime Separation passed; aggregate Quality Gates
remains red. This repair changes test fixtures only and is stacked on PR #8.

The shared run projection fixture omitted `memory_failure_kind`,
`observed_peak_ram_bytes` and `observed_peak_vram_bytes`, which the canonical
ledger run-list/detail records serialize as null when unavailable. Add those
fields to both rows, preserving the exact Rust round-trip assertions and the
TypeScript projection consumer's shared fixture.

The execution fixture supplied canned vector outputs through the obsolete host
`run_workflow` hook, but current session execution reads `workflow_graph` and
runs scheduler tasks. Its missing graph failed before its output assertions.
Use a dedicated text-input to text-output graph host, with explicit non-runtime
capabilities and I/O, for the two execution contracts. Assert that graph and I/O
callbacks occurred and that the scheduler returns the supplied text. Remove the
unused canned run hook. Preserve vector response serialization and deserialization
as a separate exact DTO round-trip test; no vector execution support is claimed.

The invalid-output fixture must also supply the text input: target discovery is
validated after all tasks complete, so missing inputs would instead leave a task
AwaitingInputs. Assert the exact non-discoverable output message and original
InvalidRequest variant. With no ledger, terminal finalization preserves that
error without a diagnostics wrapper. The previous missing-graph admission error
had a wrapper because admission records canonical errors separately. Retain this
actual admission/error-link contract in a dedicated missing-graph regression,
including real nonblank run attribution and the unavailable-ledger explanation.
Production scheduling, diagnostics and host APIs are unchanged.

## Verification and qualification

- Focused TypeScript projection tests: 12 passed, zero failed.
- Rustfmt and whitespace checks passed. No local Cargo build was attempted:
  generated intermediates were reclaimed for the approved Android emulator and
  current disk capacity does not support a fresh Rust build.
- Fresh hosted Rust integration qualification is required after publication.
  Preserved old executables are not evidence for this changed source.
- Independent integrator source review accepted staged tree
  `2c4b27a6fcccc182d90e597df4f275984cbe4fa3` after reading all three changed
  files and the relevant host, validation and diagnostics sources. This is source
  review acceptance, not Rust execution evidence; this report update follows it.
- Existing frontend lint, scheduler Clippy,
  dependency audit and native Rustler failures remain separate. C# full binding
  and native packaging qualification previously remained blocked downstream of
  the failing integration contracts. No checks or assertions were waived.

## Hosted compilation correction

Initial exact-head hosted run 37098781826 failed compilation with E0046 before
executing contracts: `WorkflowHost::run_workflow` remains a required compatibility
trait method. Both fixture hosts now implement it as an explicit panic guard,
so accidental execution through that hook fails instead of returning canned
outputs. This preserves the real scheduler path and all assertions. Rustfmt and
staged whitespace pass; fresh hosted compilation and execution remain required.
Independent integrator review accepted the exact two-hook correction at staged
`08b64cd03e0ce645b3aae6e0a5363ff3674397b3`; it satisfies the required trait
signature while retaining fail-on-legacy-bypass behavior. This acceptance is
source-only and does not replace hosted Rust execution.
