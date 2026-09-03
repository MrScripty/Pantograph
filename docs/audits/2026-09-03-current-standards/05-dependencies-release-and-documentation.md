# Focused Audit: Dependencies, Release, And Documentation

Implementation plan: [Dependencies, release, and documentation remediation](../../plans/current-standards-remediation/dependencies-release-and-documentation/plan.md)

## Scope

This audit covers dependency identity and satisfaction, lockfiles,
supply-chain and license evidence, launcher provisioning, packaging, release
units and targets, SBOMs, repository setup, ADR discovery, plans, and decision
traceability.

Applicable current standards are Dependencies, Licensing, Cross-Platform,
Security, Build, Release, Launcher, Documentation, Planning, Tooling, and
Verification.

## Assessment

Pantograph has pinned language toolchains, root Rust/npm lockfiles, checksums,
an SBOM script, and detailed release intent. The executable mechanisms do not
yet prove the dependency or release identities claimed by the documentation.
Documentation machinery is also rooted in an older standards model.

## Findings

### DEP-01 — High: dependency satisfaction does not prove requirements

launcher.sh treats any node_modules directory as npm satisfaction. For Python,
it checks only whether top-level imports succeed, not declared versions,
source, wheel identity, or compatible transitive state. It also upgrades pip
to an unconstrained latest version.

Python requirements are mostly unpinned. requirements-dllm.txt downloads a
direct binary wheel without an integrity hash. The root Cargo manifest follows
Candle's main branch, relying on lockfile state rather than expressing an
update policy.

CI audits production npm packages only. No current contract covers Rust,
Python, GitHub Action, tool, unused-dependency, provenance, or third-party
license review.

### DEP-02 — High: Pumas and lockfile authorities have drifted

Cargo consumes Pumas revision f87c3da, while quality CI separately checks out
66c0c11 and repeatedly creates a sibling symlink. The test strategy still calls
Pumas a sibling path dependency even though Cargo uses a Git revision.

src-tauri/Cargo.lock is a tracked stale second lockfile from before the Tauri
crate joined the root workspace. The root lockfile is the active Cargo
resolution authority.

These duplicate identities make clean-runner and local dependency claims hard
to interpret.

### REL-01 — High: package identity does not match release policy

package-uniffi-csharp-artifacts.sh labels every non-Darwin/non-Windows host
linux-x64 without checking CPU architecture, accepts caller-provided labels
without validating the built library, and emits unversioned archives.

Generated package manifests omit product version, source revision, target
triple, ABI identity, and the relationship between native and generated
binding units. Documentation describes Linux, macOS, and Windows loading, but
CI builds/packages only Linux x64 and no canonical support matrix qualifies
those claims.

### REL-02 — High: release and SBOM evidence is not final-artifact evidence

There is no release workflow. The former release-policy document is an outline rather than
an implemented procedure.

The release smoke does not launch the release binary. The SBOM script relies on
an ambient unpinned syft and scans the repository directory rather than the
collected artifact bytes. Checksums alone do not close version, target,
provenance, binding-cohort, or content obligations.

### DOC-01 — High: decision-traceability enforces retired rules

[check-decision-traceability.sh](../../../scripts/check-decision-traceability.sh)
defaults to broad source roots, requires a README in every changed directory,
requires eleven fixed headings, accepts any changed ADR for every directory,
infers branch ranges, and silently exits success when it cannot resolve input.

The current Documentation workflow explicitly rejects each of those defaults.
Automation must map exact trigger paths or contract identities to the durable
knowledge they can change and to its canonical owner. Range mode must receive
explicit base/head revisions and evaluate prior and current mapping state.

### DOC-02 — High: active planning authority is split and append-only

The current image-generation plan is roughly 9,800 lines and mixes current
decisions with dated execution narration. A separate recovery document also
claims current next-step authority, while there is no execution ledger or
issues file.

Current Planning standards require one concise current plan, separate history
and issues, stable acceptance claims, one current phase, and one next slice.
Historical content should remain available without competing for authority.

### DOC-03 — Medium: ADR and compliance discovery is stale

- Two accepted ADR files use ADR-003.
- The ADR index omits the Rust workspace and verification-baseline ADRs.
- The previous compliance audit cites compatibility indexes that are now
  non-normative.
- A now-removed compliance tracker marks an old checklist complete even though current
  gates and standards differ.
- Root setup instructions name older Linux Tauri packages than CI installs.

These are discovery and truthfulness issues, not reasons to rewrite every
historical document.

### LIC-01 — Medium: license and publication identity is unresolved

Cargo declares MIT OR Apache-2.0, while the checked-in license/notice material
describes Apache-2.0 and no MIT license text is present. There is no release
composition check for third-party notices or license obligations.

packages/svelte-graph is described as reusable but does not declare whether it
is private/internal or publishable. That decision affects metadata, consumer
compatibility, and release evidence.

## Preserved Strengths

- Rust, Node, npm, and Python toolchains are explicitly pinned.
- Workspace crates are marked non-publishable.
- Root Cargo and npm lockfiles exist.
- Launcher parsing uses strict mode, structured arguments, and explicit exit
  codes.
- Packaging emits checksums, and an SBOM mechanism exists.
- Historical audits, plans, and ADRs preserve valuable design context.

## Follow-Up Audit Boundaries

1. Define the dependency units and satisfaction proof for npm, Cargo, Python,
   Pumas, Actions, and external build tools.
2. Resolve the active lockfile, Pumas revision, and Python artifact authorities.
3. Define release units, channels, versions, target triples, support matrix,
   binding cohorts, and withdrawal/recovery behavior.
4. Generate provenance, checksums, notices, and SBOM evidence from the final
   collected artifacts.
5. Replace traceability with exact impact mappings.
6. Migrate active plans to plan/ledger/issues separation and repair ADR
   identity/indexing without rewriting historical decisions.
7. Resolve the repository and package license/publication promises.
