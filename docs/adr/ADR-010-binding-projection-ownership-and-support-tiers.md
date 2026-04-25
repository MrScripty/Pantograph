# ADR-010: Binding Projection Ownership And Support Tiers

## Status

Accepted.

## Context

Pantograph needs host-language access to the execution-platform authoring and
runtime surface without creating separate host-owned node catalogs,
compatibility rules, diagnostics semantics, or artifact lifecycle policy.

Earlier ADRs establish the backend ownership of workflow service boundaries,
canonical node contracts, composed contracts, runtime attribution, runtime
observability, and durable model/license diagnostics. Stage 06 projects those
contracts into native host lanes.

Host lanes have different maturity. C# has generated UniFFI artifacts,
dotnet-backed smoke tests, and packaged quickstart verification. Python has no
host package or generated artifact yet. BEAM has a Rustler wrapper and smoke
fixture source, but this host cannot execute the BEAM smoke because `mix` is not
installed.

## Decision

Native Rust remains the canonical application API for execution-platform
contracts.

`pantograph-uniffi` owns the non-BEAM FFI projection through the
`pantograph_headless` product-native library. It may expose host-safe JSON
methods, generated binding metadata, lifecycle methods, and error envelopes,
but it must project backend-owned contracts from `pantograph-workflow-service`,
`pantograph-node-contracts`, `pantograph-embedded-runtime`, diagnostics ledger
owners, and node-engine registries.

`pantograph-rustler` owns the Elixir/BEAM projection. Rustler NIFs may project
backend-owned workflow, registry, and diagnostics facts, but Rustler must not
become a separate product API owner for node definitions or graph semantics.

Generated host-language files are release artifacts. They are regenerated from
the same native library metadata and are not hand-maintained source.

Support tiers are evidence-based:

- Native Rust is supported for implemented execution-platform surfaces.
- C# over `pantograph_headless` is supported only for surfaces covered by real
  generated/native artifact smoke and packaged quickstart verification.
- Python host bindings are unsupported until a real host package, generated
  artifact, and import/load smoke command exist.
- Elixir/BEAM remains experimental until the BEAM smoke command can run against
  the real Rustler artifact in the supported host toolchain.

Host bindings must not advertise a supported lane or supported surface without a
language-native command that loads the real generated/native artifact.

## Consequences

- Binding crates stay thin and do not duplicate backend domain semantics.
- Host palettes, inspectors, and insert flows consume backend-discovered node
  definitions and queryable ports instead of wrapper-local metadata.
- C# can be advanced surface by surface as generated-artifact smokes expand.
- Python work must start by adding an actual generated/native artifact path
  rather than a placeholder wrapper.
- BEAM documentation must remain experimental where verification depends on a
  toolchain unavailable on the current host.
- Release and CI work must keep generated bindings, native libraries, package
  manifests, and quickstart examples version-matched.

## Related ADRs

- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-006-canonical-node-contract-ownership.md`
- `docs/adr/ADR-008-durable-model-license-diagnostics-ledger.md`
- `docs/adr/ADR-009-composed-node-contracts-and-migration.md`
