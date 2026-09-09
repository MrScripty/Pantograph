# Plan: Dependency, Release, And Documentation Remediation

> Superseded on 2026-09-08 by the [domain architecture and multimodal plan](../../domain-architecture-and-multimodal/plan.md).
> The remaining body is historical scope/evidence, not implementation authority.
> Outstanding claims and findings transfer to the successor; none are accepted by supersession.

**Plan status:** `Superseded`

**Current phase:** Superseded by the domain architecture and multimodal plan.

**Next slice:** `none`
cohorts, supported-target decision, and satisfaction contracts in the canonical
policy and artifact-plan files without mutating dependency resolution.

**Acceptance status:** `pending`

**Execution ledger:** [execution-ledger.md](execution-ledger.md)

**Issues:** [issues.md](issues.md)

**Reports:** `none`

**Related ADRs:** [ADR index](../../../adr/README.md)

**Source audit:**
[05-dependencies-release-and-documentation.md](../../../audits/2026-09-03-current-standards/05-dependencies-release-and-documentation.md)

## Objective

Give Pantograph one auditable dependency and release identity from declared
requirements through final candidate bytes, with truthful licensing,
cross-platform, launcher, and documentation contracts and no competing active
plan or lockfile authority.

## Objective Acceptance

| ID | Observable criterion | Kind | Environment | Mode | Schedule | Owner | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| DRD-A01 | Every npm, Cargo, Python, Git, Action, system, generator, and release-tool requirement has an owner, source/identity constraint, target/profile, satisfaction proof, update/removal policy, and typed unavailable/unsupported/invalid outcome. | `contract` | `not-applicable` | `automated` | pull request | Dependency policy owner | `pending` | Dependency-contract validator and manifest/lock review |
| DRD-A02 | Root lockfiles and declared source identities are the only active resolution authorities; Pumas resolves to one revision, stale Tauri lock state is absent, and clean-runner commands consume locked inputs. | `contract` | `representative` | `automated` | pull request | Cargo/npm/Python resolution owners | `pending` | Clean locked resolution plus identity-negative fixtures |
| DRD-A03 | Project and package publication status, authoritative license terms, provenance, obligations, and required notices agree in source metadata and each shipped artifact. | `release-artifact` | `representative` | `manual` | pull request and release candidate | Licensing owner | `pending` | License decision record, obligation inventory, and recorded artifact inspection |
| DRD-A04 | One concise current image-generation `plan.md` owns current state and next slice; history/issues are separate, ADR identities/indexes are unique/current, and entry-point/setup/compliance docs are truthful. | `contract` | `not-applicable` | `manual` | migration slice | Documentation/plan owners | `pending` | Plan-structure, link, index, and current-authority review |
| DRD-A05 | The release contract identifies each desktop, native, binding, metadata, and notice artifact by version/revision, target triple, architecture, ABI/cohort, format, consumer, and relationship, with no OS-only or caller-guessed target. | `contract` | `not-applicable` | `automated` | release candidate | Release/artifact-plan owner | `pending` | Artifact-plan validation and target-negative fixtures |
| DRD-A06 | SBOM, checksums, notices, and provenance are generated from and verified against the final collected artifact set using pinned tools. | `release-artifact` | `required-real` | `automated` | release candidate per required target | Release metadata owner | `pending` | Final-set manifest, hashes, SBOM/notices/provenance validation |
| DRD-A07 | An explicitly authorized immutable candidate source builds the complete required target matrix; exact packages load/start through accepted claim evidence, and publication cannot run while any required claim or destination authority is unresolved. | `release-artifact` | `required-real` | `automated` | release candidate | Release integration owner | `pending` | Candidate workflow and verification-portfolio results |
| DRD-A08 | Launcher dependency checks prove declared versions/identities, mutate only under `--install`, recheck after mutation, and preserve per-requirement and delegated failures. | `system` | `representative` | `automated` | pull request | Launcher/dependency owner | `pending` | Launcher contract fixtures and clean-environment run |

## Scope

### In Scope

- Dependency requirements, source identity, locks, satisfaction, provisioning,
  audit coverage, and tool bootstrap for npm, Cargo, Python, Pumas, Candle,
  GitHub Actions, native/system packages, generators, and release tools.
- Release units, version relationships, supported targets, binding cohorts,
  artifact collection, checksums, SBOM, provenance, notices, candidate pipeline,
  and recovery/publication contracts.
- Project/package license and publication decisions.
- Current-plan migration, ADR identity/index repair, and truthful repository,
  setup, compliance, dependency, release, and binding documentation.

### Out Of Scope

- Product behavior fixes owned by security, architecture, frontend, or
  verification/tooling plans.
- Redesigning decision-traceability automation or exact-artifact smoke; this
  plan supplies their mapping/artifact contracts and consumes their evidence.
- Publishing a release, selecting credentials, or changing an external channel
  without separate explicit authority.
- Rewriting historical audits/plans merely to match current standards.
- Claiming byte-for-byte reproducibility unless a separately accepted claim
  defines controlled inputs and comparison evidence.

## Constraints And Assumptions

- Coding-Standards revision `82a0ddf315a08364357f6564018e37bdbeb72a1a`
  is the routed authority; normative changes require re-routing.
- Existing toolchain pins and root npm/Cargo locks are preserved unless their
  owning contract and migration evidence select a replacement.
- `packages/svelte-graph` remains internal/private unless an independently
  published consumer and release contract are identified.
- The repository owner's Apache-only versus MIT-or-Apache intent is unresolved;
  release acceptance cannot infer it from current contradictory files.
- User changes in the Pumas proposals or other shared surfaces must be
  integrated, not overwritten.

## Binding Decisions

| Decision | Owner | Evidence | Supersedes |
| --- | --- | --- | --- |
| Ecosystem manifests plus their selected root lock/locked-input files own consumed dependency identity; `docs/dependency-policy.md` owns requirement, satisfaction, audit, and lifecycle policy without copying resolved graphs. | Dependency owners | DEP-01/02 | Ambient imports/directories, CI clones, and duplicate lock/prose identities |
| `release/artifact-plan.json` owns release-unit roles, target matrix, artifact/cohort relationships, and required metadata; `docs/release.md` owns channel, procedure, recovery, and consumer guidance. | Release owner | REL-01/02 | OS-only labels, script defaults, and policy-outline artifact identity |
| Candidate generation and publication are separate transitions. This plan builds/verifies candidates; publication remains unavailable until destination, channel, authority, credentials, recovery, and every required claim are accepted. | Release integration owner | Current Release workflow | Tag-pattern or successful-build implied publication |
| Root `Cargo.lock` is the Rust workspace resolution authority. Delete `src-tauri/Cargo.lock` only after locked workspace evidence proves no independent consumer remains. | Cargo resolution owner | DEP-02 | Stale second Cargo resolution snapshot |
| Cargo's Pumas declaration/lock identity is canonical; remove the unrelated CI checkout and sibling symlink unless a real non-Cargo consumer is identified. | Pumas dependency owner | DEP-02 | `PUMAS_LIBRARY_REF` and repeated sibling bootstrap |
| Keep the Rust workspace policy at unique identity `ADR-017-rust-workspace-policy.md`; do not restore a redirect artifact at its former duplicate ID. | ADR owner | DOC-03 and current Documentation workflow | Duplicate accepted ADR-003 identity |
| Migrate the active image plan in place: concise `plan.md`, separate ledger/issues, explicitly historical reports, and no second next-step authority. | Image-plan owner | DOC-02 and Planning workflow | Append-only plan plus authoritative recovery overlay |

## Evidence And Oracle Plan

| Claim | Deciding oracle | Independent authority | Intended negative evidence | Unsupported/unavailable boundary |
| --- | --- | --- | --- | --- |
| DRD-A01/A02/A08 | Native resolver/package metadata and isolated launcher fixtures compare declared requirement with consumed identity before/after authorized mutation. | Registry/index metadata, lock hashes, artifact metadata, and declared consumer—not import/directory presence | Wrong Pumas revision, stale/missing lock, version-mismatched Python package, empty `node_modules`, unauthorized mutation | Missing resolver/source evidence is `unavailable`; unsupported target/profile is `unsupported`. |
| DRD-A03 | Reviewed authoritative terms and per-artifact obligation inventory are compared with source/package metadata and collected bytes. | Upstream license/notice sources plus repository-owner project-license decision | Conflicting project license, unknown provenance, missing/stale notice, wrong artifact attachment | Missing legal authority remains `unavailable`, not guessed compatibility. |
| DRD-A04 | Structure/link/index checks plus owner review establish one current plan/ADR/document authority. | Current plan state and accepted ADR/document owners | Two next slices, duplicate ADR ID, obsolete setup command, current docs pointing to legacy standards | Historical prose is not current authority and need not be rewritten. |
| DRD-A05/A06 | Schema validation and target runners compare the final collected set and generated metadata to the accepted artifact plan. | Immutable source revision, native target/toolchain output, and final artifact bytes | Caller-spoofed target, wrong architecture/ABI, missing/extra/stale artifact, repo-directory SBOM | A required target/tool is `unavailable`; it cannot fall back to host inference. |
| DRD-A07 | Candidate workflow consumes accepted claim IDs from the verification portfolio and verifies exact same-run artifacts before publication handoff. | Verification plan's claim registry and candidate artifact manifest | Ambiguous dispatch, unsatisfied claim, wrong source/version, premature publication | Missing channel/authority/credential keeps publication unavailable without blocking local candidate construction. |

## Systemic Finding Audit

- **Invariant/owners:** a requirement or artifact has one canonical identity;
  Dependencies owns requirements/resolution, Licensing owns terms/obligations,
  Release owns shipped identity, and Documentation projects accepted facts.
- **Bounded authorities/consumers:** ecosystem manifests and locks,
  requirements files, launcher, CI Actions, packaging/SBOM scripts, license and
  notice files, release/binding docs, ADR index, compliance entry points, and
  the active image plan.
- **Expansion facts:** a new dependency unit, release consumer/channel, target,
  artifact role, licensed material, publication promise, or current authority
  expands the population.
- **Dispositions:** every selected consumer uses the canonical identity, is
  explicitly internal/supporting, migrates, or is removed; unresolved release
  or legal facts block the affected claim.
- **Alternatives:** delete duplicate CI/lock/doc state, deepen native resolver
  checks, separate candidate/publication, minimize release units, and generate
  metadata from the final set before adding more tooling.
- **Stopping condition:** every bounded consumer has an evidence-backed
  disposition, no competing active identity remains, and DRD-A01–DRD-A08 pass.
- **Composition comparison:** canonical manifests/locks plus one artifact plan
  replace ambient checks and identities repeated across scripts, CI, and prose.

## Simplicity And Ownership Review

**Applicability:** `applicable`

- Independent concepts and dimensions: requirements/resolution, provisioning, licensing,
  release identity, target support, publication, and documentation history stay
  separate.
- State, identity, value, time, policy, and mechanism: manifests/locks own consumed
  dependency state; the artifact plan owns intended release composition; each
  candidate manifest owns resolved version/revision/target/ABI/hash. Project,
  dependency, generator, ABI, package, and source versions are distinct.
  Supported-target and binding-consumer matrices define real overlaps. Any
  source, lock, target, ABI, license, or artifact-set change invalidates its
  dependent evidence.
- Caller and composition-root knowledge: launcher and CI receive requirement/artifact identities
  from their owners; callers do not invent import names, target labels, paths,
  versions, or alternate sources.
- Representative change paths and forced owners: dependency updates change manifest/lock and owned
  evidence; target additions change artifact plan/pipeline/docs; license changes
  change affected metadata/notices; plan changes update only current authority.
- Stable Interfaces versus hidden knowledge: dependency satisfaction and artifact-
  plan schemas are stable interfaces; resolver commands, CI provider syntax,
  packaging paths, and archive mechanics remain adapters.
- Independent evolution, testing, failure, and replacement: dependency, license, planning migration, packaging,
  and publication contracts have separate gates and can fail/roll back without
  creating alternate authority.
- Necessary complexity and containment: target-specific Python locks and artifact cohorts
  exist only where actual consumers/targets require them; no universal matrix
  or release tool is introduced.
- Deletion and cumulative machinery result: remove stale Cargo lock, Pumas CI clone/symlink, ambient
  satisfaction checks, unversioned/guessed artifact identity, repo-scan SBOM,
  duplicate plan authority, and stale discovery prose.

## Cross-Plan Dependencies And Order

1. Verification/tooling Milestone 1 replaces retired traceability before this
   plan changes source or migrates documentation; this plan only updates mapped
   artifact rows after that contract exists.
2. Milestones 0–1 establish dependency/release authority and consumed identity.
3. Milestone 2 resolves licensing/publication identity before an artifact plan
   can be accepted.
4. Milestone 3 migrates plan/ADR/discovery documentation after dependency and
   license facts are stable, preserving history without competing authority.
5. Milestone 4 builds complete candidate artifacts and metadata.
6. Verification/tooling exact-artifact claims consume those artifacts; Milestone
   5 then records actual release proof. Security, architecture, and frontend
   blocking claims must also pass before publication handoff.

Shared `package*.json`, workflows, launcher, README/policy files, claim maps,
and active plans have one serial integration owner.

## Milestones

### Milestone 0: Establish Dependency And Release Authority

**Goal:** Define dependency/release units, consumers, targets, identities,
satisfaction contracts, artifact relationships, channels, and recovery before
resolver or packaging mutation.

**Allowed write set:**

- `docs/dependency-policy.md`
- `docs/release.md`
- `release/README.md`
- `release/artifact-plan.json`
- `docs/headless-workflow.md`
- `docs/plans/current-standards-remediation/dependencies-release-and-documentation/plan.md`
- `docs/plans/current-standards-remediation/dependencies-release-and-documentation/execution-ledger.md`
- `docs/plans/current-standards-remediation/dependencies-release-and-documentation/issues.md`

**Tasks:**

- [ ] Classify each dependency and audit-tool unit by owner, lifecycle, target,
  source identity, satisfaction, mutation authority, and typed failure.
- [ ] Select release units, version relationships, binding cohorts, actual
  supported targets, candidate channel, artifact set, publication/recovery
  authority, and required claim IDs; unresolved facts remain explicit blockers.

**Acceptance gate:** DRD-A01 contract review and DRD-A05 artifact-plan schema
fixtures pass without relying on current launcher/packaging defaults.

**Status:** `Planned`

### Milestone 1: Reconcile Resolution And Provisioning Identity

**Goal:** Make local, launcher, and clean-runner dependency satisfaction consume
one identity per requirement.

**Allowed write set:**

- `Cargo.toml`
- `Cargo.lock`
- `src-tauri/Cargo.lock`
- `package.json`
- `package-lock.json`
- `requirements.txt`
- `requirements-diffusion.txt`
- `requirements-dllm.txt`
- `requirements/locks/`
- `launcher.sh`
- `scripts/check-dependency-contracts.mjs`
- `.github/workflows/quality-gates.yml`
- `docs/dependency-policy.md`
- `docs/development.md`

**Tasks:**

- [ ] Resolve Pumas/Candle update policy, remove duplicate Pumas checkout state,
  prove the root Cargo lock, and remove the stale Tauri lock.
- [ ] Select target/profile-specific Python locked inputs and integrity data,
  including pip/bootstrap and direct wheels; validate npm from its lock rather
  than directory presence.
- [ ] Make `--install` the only mutation path and re-check each requirement
  after authorized installation; add Rust/Python/Actions/tool provenance audit
  claims selected in Milestone 0.

**Acceptance gate:** DRD-A01, DRD-A02, and DRD-A08 pass in a clean representative
environment; wrong-version/source and no-mutation fixtures fail correctly.

**Status:** `Planned`

### Milestone 2: Resolve Licensing And Publication Identity

**Goal:** Align project/package metadata and final-artifact obligations with
authoritative terms and actual publication status.

**Allowed write set:**

- `Cargo.toml`
- `packages/svelte-graph/package.json`
- `packages/svelte-graph/README.md`
- `LICENSE`
- `LICENSE-MIT`
- `NOTICE`
- `THIRD_PARTY_NOTICES.md`
- `docs/licensing-policy.md`
- `docs/dependency-policy.md`
- `docs/release.md`
- `release/artifact-plan.json`
- `scripts/check-release-licenses.mjs`

**Tasks:**

- [ ] Obtain the repository owner's project-license decision; align all
  manifests/texts without guessing, and mark Svelte Graph private unless a real
  independent publication contract is found.
- [ ] Record exact third-party provenance/terms and per-artifact obligations;
  define notice generation/inspection from the final set.

**Acceptance gate:** DRD-A03 contract evidence passes; unknown or conflicting
terms and missing notices block distribution.

**Status:** `Planned`

### Milestone 3: Migrate Current Documentation Authority

**Goal:** Restore one current plan/ADR/document authority after traceability is
standards-compliant and dependency/license facts are settled.

**Allowed write set:**

- `docs/plans/current-image-generation-graphs/plan.md`
- `docs/plans/current-image-generation-graphs/execution-ledger.md`
- `docs/plans/current-image-generation-graphs/issues.md`
- `docs/adr/ADR-017-rust-workspace-policy.md`
- `docs/adr/README.md`
- `README.md`
- `docs/README.md`
- `docs/audits/2026-09-03-current-standards/README.md`
- `docs/plans/documentation-consolidation/plan.md`
- `docs/development.md`
- `docs/headless-workflow.md`
- `docs/runtime-operations.md`
- `docs/dependency-policy.md`
- `docs/release.md`
- `scripts/decision-traceability-map.tsv`

**Tasks:**

- [x] Preserve the consolidated image plan, separate ledger/issues, and single
  next-step authority established by the documentation consolidation plan.
- [x] Preserve the unique ADR-017 Rust-workspace identity and current ADR index.
- [ ] Correct setup, dependency, release, compliance, and discovery prose and
  update exact traceability mappings for changed durable artifacts.

**Acceptance gate:** DRD-A04 structure/link/index/current-authority review and
the new traceability contract pass.

**Status:** `Planned`

### Milestone 4: Build The Final Candidate Artifact Set

**Goal:** Produce versioned target-correct artifacts and metadata from one
immutable candidate source.

**Allowed write set:**

- `release/README.md`
- `release/artifact-plan.json`
- `scripts/package-uniffi-csharp-artifacts.sh`
- `scripts/generate-release-sbom.sh`
- `scripts/check-release-artifact-set.mjs`
- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `Cargo.toml`
- `.github/workflows/release-candidate.yml`
- `docs/release.md`
- `docs/headless-workflow.md`
- `bindings/csharp/PACKAGE-README.md`
- `scripts/README.md`

**Tasks:**

- [ ] Derive artifact names and manifests from accepted version/source/target/
  architecture/ABI/cohort facts; reject caller labels that contradict bytes.
- [ ] Pin generator, archiver, SBOM, and metadata tools; collect the exact set,
  then generate/verify checksums, notices, provenance, and SBOM from final bytes.
- [ ] Add explicit candidate dispatch and target matrix with least privilege and
  no publication credentials.

**Acceptance gate:** DRD-A05 and DRD-A06 pass on every required target; wrong,
missing, duplicate, unexpected, and stale artifact fixtures fail.

**Status:** `Planned`

### Milestone 5: Prove Release Readiness And Close

**Goal:** Run accepted behavior and exact-artifact claims, then permit only the
explicitly authorized publication handoff.

**Allowed write set:**

- `release/artifact-plan.json`
- `.github/workflows/release-candidate.yml`
- `docs/release.md`
- `docs/dependency-policy.md`
- `docs/licensing-policy.md`
- `README.md`
- `docs/README.md`
- `docs/plans/current-standards-remediation/dependencies-release-and-documentation/plan.md`
- `docs/plans/current-standards-remediation/dependencies-release-and-documentation/execution-ledger.md`
- `docs/plans/current-standards-remediation/dependencies-release-and-documentation/issues.md`

**Tasks:**

- [ ] Consume verification/tooling exact package-load/desktop-start claim
  results and every other required behavior claim for the same candidate.
- [ ] Verify publication destination/channel/authority/recovery or leave
  publication explicitly unavailable; record final evidence and dispositions.

**Acceptance gate:** DRD-A01–DRD-A08 pass. Publication, if separately
authorized, accepts only the verified immutable candidate and exact artifact
set; otherwise candidate readiness is recorded without publication.

**Status:** `Planned`

## Migration And Recovery

- Change one resolver unit at a time; manifest/lock/consumer evidence is one
  atomic migration. Restore the prior accepted pair if compatibility fails;
  never retain two active identities as fallback.
- Delete the Tauri lock or Pumas CI clone only after consumer inventory and
  locked clean-runner proof. Git retains recovery history, not runtime fallback.
- Migrate the active plan in place and mark moved history non-authoritative;
  never leave two current phases or next slices.
- Candidate artifacts are immutable and versioned. A failed claim blocks
  promotion; never overwrite bytes under an existing identity.
- Before publication, record destination-specific stop-promotion, withdrawal/
  deprecation, notification, correction, and residual-exposure procedures.

## Blockers

- `none` for the next slice.
- Milestone 1 source changes wait for the verification/tooling traceability
  replacement.
- Milestone 2 cannot complete until the repository owner resolves Apache-only
  versus MIT-or-Apache licensing.
- Milestone 5 waits for required product and exact-artifact claims from the
  security, architecture, frontend, and verification/tooling plans.

## Risks

- Platform/Python profile expansion may multiply locks and artifacts; only
  accepted consumers/targets may add variants.
- License metadata can appear consistent while final packages omit obligations;
  acceptance inspects collected artifacts, not source fields alone.
- Plan compaction can discard current decisions; migrate authority/history in
  one reviewed slice with link and one-next-slice checks.
- Shared workflows, manifests, locks, and docs can conflict with adjacent plans;
  integrate serially and rerun affected contract/artifact evidence.

## Re-Plan Triggers

- A dependency, target, release unit, channel, artifact role, licensed material,
  or independent consumer appears outside the bounded inventory.
- Pumas has a required non-Cargo consumer, or `src-tauri` has a real independent
  resolution/release contract.
- The license owner selects terms incompatible with a required dependency or
  distribution, or legal interpretation remains unavailable.
- A supported target cannot produce/load the accepted artifact cohort, a final-
  set tool cannot describe the bytes, or required publication recovery is
  unsupported.
- An upstream plan changes required claim semantics or cumulative release/
  dependency machinery exceeds the admitted composition.

## Final Acceptance

- Acceptance status: `pending`
- Deferred follow-ups: `none`
- Final status: `Superseded`
