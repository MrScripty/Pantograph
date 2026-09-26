# Standards and library-RPC audit — revision 2, September 25, 2026

## Scope and result

Source changes remain necessary for the requested RPC and T/I/E workflow goal. The opening audit was a bounded source/document review and corrected planning handoff, not a full standards certificate, executable MCP route, independent agent review, build, binary verification or live model result at that time. The dated execution update near the end records the later MCP route, independent artifact/build qualification and live library-only observations; it does not retroactively expand the opening audit's evidence.

Revision 2 follows the user's explicit decisions: Pumas is library/resources only; all inference stays in Pantograph's existing unified backend API; use a retained artifact independent of active upstream development; apply current Coding-Standards through MCP; and adopt the requested orchestration, implementation and read-only review roles. Any contrary optional serving route in revision 1 is withdrawn, not grandfathered as compatibility behavior.

## Updated inspection

Pantograph remains `56caed029a48db66212a953a1677a94449396d52`. Pumas main is `51301bd317d7962b51d534590a685b676de835d9`; release 0.7.0 resolves to `29242fce4ec9043becaeb061587f364ecc4e7177`. Main is 37 commits ahead. The [inventory](2026-09-25-pumas-contract-inventory.md) records the release asset/digest metadata and the relevant comparison, including unchanged library-facts handler bytes and changed shared transport/contract files. It proposes 0.7.0 as the first artifact to qualify, not as an already verified runtime.

Coding-Standards advanced from `e9f84f43f318c54599a2639e2e16dbd7bb0ce4af` to `8fef41d18c524c7ac6a1a16f242cc3f549c791a2`. The compared commit changes Engine presentation/runtime contracts, tests, generated artifacts and CI; the normative Core/Router/planning/implementation/verification/architecture/contract files previously read are unchanged in that comparison. Its `tools/standards_engine/contracts/a1-interface.toml:1-4` declares interface 37. Commit guidance was additionally read at this head. None of this establishes the user's running MCP identity; execution must inspect it.

## Concrete findings and dispositions

| ID | Finding / evidence | Revision-2 disposition |
| --- | --- | --- |
| PRPC-01 | P02/P03 full package-fact dispatch still requires a Pumas owner and rejects local-client/read-only access. U03 has an HTTP full-facts handler at release and HEAD. | Migrate actual dispatch and callers to a qualified library-only RPC mapping, not merely the selector. |
| PRPC-02 | P04 runtime package-facts resolver directly holds `Arc<PumasApi>`. | Update the real resolver/composition family; prove no embedded root ownership is required in external mode. |
| PRPC-03 | P01 pins old Pumas `f87c3da8276a914a54c6f4f36d617bef9d9f424e`. | Decide if a Rust type/client dependency is necessary, pin it coherently to the qualified boundary, and update locks/consumers; no mutable local path substitution. |
| PRPC-04 | HTTP RPC and PumasLocalClient's framed local TCP IPC differ; a method in one is not proof of another. | Qualify only actually consumed library operations on the selected transport/artifact. Pumas inference is excluded, not another route to choose. |
| PRPC-05 | P03/P04 convert through JSON and remove model-ref version fields before destination decoding. This is a migration hazard, not an asserted exploit. | Explicitly validate supported source/destination versions and semantic mapping; consolidate truly shared conversion rules without erasing independent authority. |
| PGEN-01 | P05's inspected canonical single-runtime route handles text specially then enters image projection; embeddings have no branch there. | Trace single/batch/descriptors and reuse Pantograph's lower-level embedding capability through its unified API. Pumas embedding output cannot fill this gap. |
| PGEN-02 | P06 selected text is prompt-only, non-streaming and limited to 1,024-byte input/output. | Qualify an actual useful workload and coordinate changes only where its current backend/port/output contracts need them. |
| PGEN-03 / D-02 | P07/P09 retain incomplete image bytes reaching the inspector decoder. | Keep the original fix and real large-image decoding gate. Correct library RPC cannot repair this by itself. |
| PGEN-04 | Revision 1 introduced conditional Pumas-serving adaptation. | Superseded and withdrawn: not an upstream inference-compatibility task. INF-A01 now proves the strict Pantograph execution boundary. |
| PINF-01 | User-selected invariant: Pumas inference must be unreachable from Pantograph. No present bypass was executed in this review. | Inspect current configuration/adapters as well as the new client; enforce the boundary through appropriate existing configuration/contract owners and observed real execution. |
| PART-01 | Active local Pumas changes/rebuilds are an invalid dependency fixture for this goal. | Isolated pinned artifact, explicit service/state, matching source/contract inputs and deliberate upgrades only. |
| PART-02 | 0.7.0 provides headless assets, but release metadata says prerelease/non-immutable and binary bytes were not downloaded here. | Verify digest/provenance and actual library operations. Select a newer frozen artifact only on relevant evidence. |
| PENV-01 | Actual MCP, artifact compatibility, model/backend/device, library state and display remain unqualified here. | Record exact blockers and observe the smallest real boundary that can change the decision. Continue independent eligible work. |

Pumas model selection, asset access, inference execution and result visibility are separate claims. A readable package-facts response must not hide unsupported model execution, invalid asset identity or missing native inference results.

## Comparison with current standards

| Concern | Current obligation / revision |
| --- | --- |
| Requested outcome and ownership | S01/S03/S07 preserve user intent. The plan mandates Pantograph-only inference and all three modalities; no optional gateway escape remains. |
| Current standards source | S02/S03 and the user's MCP requirement require actual runtime/content qualification and a task-fact route, not a prior summary or source path treated as live authority. |
| Current planning state | S03/S11 require one active owner, phase and next slice, concise current decisions and history in the existing ledger. Terminal plans remain historical. |
| Independent upstream baseline | S03/S04 call for the smallest real observation before expanding around design-changing external assumptions. The retained binary and actual operations, not mutable local source, decide the consumed API. |
| Composed design | All eight S07 probes now describe a library-only client, existing inference API and isolated fixture. No Pumas inference adapter or new version-management framework is justified. |
| Boundary semantics | S08/S09/S10 require complete outcome/version/identity and explicit field policy before admitting remote facts. Deserialization, HTTP success and matching names do not prove semantic compatibility. |
| Dependencies and scope | S01/S05/S06 preserve proportionality: installation is authorized when justified, not a reason for speculative infrastructure or broad global upgrades. |
| Parallel work and review | S03/S06 retain disjoint ownership, settled prerequisites and actual material review. Requested models are role policy, not availability claims. Reviewers remain read-only. |
| Commits and inventory | S14 requires exact staged-scope/sensitive/generated review, affected verification and coherent conventional commits; no automatic history rewrite or hook bypass. The inventory changes with material outcomes, not every worker turn. |
| Failing baseline | S04 requires proof that checks actually reach changed behavior. Historical 20/27 failure counts are not a fresh run and equal counts are not sufficient evidence. |

## Retained work

Preserve original M0 consolidation and scoped EX-01/03/04, RT-01/02/03, D-01 and COV-01/02/03 evidence. Reverify materially affected behavior rather than rebuilding accepted foundations. R0/R1 revise the external model-library dependency; R2 completes Pantograph modalities; R3 strengthens the user workflow; R4/R5 retain full maintained-area, consumer, lifecycle and final acceptance obligations.

D-02, RT-03 non-returning-worker/complete-cleanup evidence, COV-04/05/06, current license/support/dependency questions and inherited SDC/ALB/FE/VT/DRD/IMG obligations remain with their existing owners and P10 mapping. Scope supersession closes none of those claims. The requested new agent policy replaces historical assignments, not historical measurements.

## Evidence limits

The initial connector pass supplied source/metadata and compare results. Direct network retrieval in that pass failed; no released binary or source archive had then been downloaded/build-tested. Later dated ledger/inventory entries supersede that observation for the isolated pinned current-source qualification. No target repository source, live service, model, account or database was modified. Package checks are separate from runtime/standards acceptance and are recorded in the bundle's validation record.

## Source register


Links are pinned to the inspected source revisions. File-level source inspection establishes the described code shape; only actual execution establishes runtime behavior.

| ID | Evidence | Path |
| --- | --- | --- |
| P01 | [Pinned Pumas dependency](https://github.com/MrScripty/Pantograph/blob/56caed029a48db66212a953a1677a94449396d52/Cargo.toml) | `Cargo.toml` |
| P02 | [Selector roles and library access](https://github.com/MrScripty/Pantograph/blob/56caed029a48db66212a953a1677a94449396d52/crates/workflow-nodes/src/setup.rs) | `crates/workflow-nodes/src/setup.rs` |
| P03 | [Dispatch package-facts producer](https://github.com/MrScripty/Pantograph/blob/56caed029a48db66212a953a1677a94449396d52/crates/pantograph-embedded-runtime/src/pumas_dispatch_package_facts.rs) | `crates/pantograph-embedded-runtime/src/pumas_dispatch_package_facts.rs` |
| P04 | [Execution package-facts resolver and adaptation](https://github.com/MrScripty/Pantograph/blob/56caed029a48db66212a953a1677a94449396d52/crates/pantograph-embedded-runtime/src/runtime_host_package_facts.rs) | `crates/pantograph-embedded-runtime/src/runtime_host_package_facts.rs` |
| P05 | [Canonical runtime-host task dispatch](https://github.com/MrScripty/Pantograph/blob/56caed029a48db66212a953a1677a94449396d52/crates/pantograph-embedded-runtime/src/runtime_host_execution_port.rs) | `crates/pantograph-embedded-runtime/src/runtime_host_execution_port.rs` |
| P06 | [Selected-text contract](https://github.com/MrScripty/Pantograph/blob/56caed029a48db66212a953a1677a94449396d52/crates/pantograph-embedded-runtime/src/runtime_host_text_execution.rs) | `crates/pantograph-embedded-runtime/src/runtime_host_text_execution.rs` |
| P07 | [D-02 and desktop acceptance gaps](https://github.com/MrScripty/Pantograph/blob/56caed029a48db66212a953a1677a94449396d52/docs/plans/domain-architecture-and-multimodal/reports/desktop-review.md) | `docs/plans/domain-architecture-and-multimodal/reports/desktop-review.md` |
| P08 | [Previous active plan at inspected revision](https://github.com/MrScripty/Pantograph/blob/56caed029a48db66212a953a1677a94449396d52/docs/plans/domain-architecture-and-multimodal/plan.md) | `docs/plans/domain-architecture-and-multimodal/plan.md` |
| P09 | [Inherited issue dispositions](https://github.com/MrScripty/Pantograph/blob/56caed029a48db66212a953a1677a94449396d52/docs/plans/domain-architecture-and-multimodal/issues.md) | `docs/plans/domain-architecture-and-multimodal/issues.md` |
| P10 | [Maintained-area inventory and old-claim mapping](https://github.com/MrScripty/Pantograph/blob/56caed029a48db66212a953a1677a94449396d52/docs/plans/domain-architecture-and-multimodal/reports/repository-review.md) | `docs/plans/domain-architecture-and-multimodal/reports/repository-review.md` |
| P11 | [Native and host-consumer boundary](https://github.com/MrScripty/Pantograph/blob/56caed029a48db66212a953a1677a94449396d52/docs/headless-workflow.md) | `docs/headless-workflow.md` |
| P12 | [Accepted slice history and old standards baseline](https://github.com/MrScripty/Pantograph/blob/56caed029a48db66212a953a1677a94449396d52/docs/plans/domain-architecture-and-multimodal/execution-ledger.md) | `docs/plans/domain-architecture-and-multimodal/execution-ledger.md` |
| U01 | [Real HTTP routes and feature gates](https://github.com/MrScripty/Pumas-Library/blob/51301bd317d7962b51d534590a685b676de835d9/rust/crates/pumas-rpc/src/server.rs) | `rust/crates/pumas-rpc/src/server.rs` |
| U02 | [Framed local IPC is not HTTP RPC](https://github.com/MrScripty/Pumas-Library/blob/51301bd317d7962b51d534590a685b676de835d9/rust/crates/pumas-core/src/ipc/local_client.rs) | `rust/crates/pumas-core/src/ipc/local_client.rs` |
| U03 | [Full facts and executable target HTTP handlers](https://github.com/MrScripty/Pumas-Library/blob/51301bd317d7962b51d534590a685b676de835d9/rust/crates/pumas-rpc/src/handlers/models/imports.rs) | `rust/crates/pumas-rpc/src/handlers/models/imports.rs` |
| U04 | [Envelope, outcome dispatch, health and update streams](https://github.com/MrScripty/Pumas-Library/blob/51301bd317d7962b51d534590a685b676de835d9/rust/crates/pumas-rpc/src/handlers/mod.rs) | `rust/crates/pumas-rpc/src/handlers/mod.rs` |
| U05 | [Pumas domain/transport/provider ownership](https://github.com/MrScripty/Pumas-Library/blob/51301bd317d7962b51d534590a685b676de835d9/docs/ARCHITECTURE.md) | `docs/ARCHITECTURE.md` |
| S01 | [Current universal invariants](https://github.com/MrScripty/Coding-Standards/blob/8fef41d18c524c7ac6a1a16f242cc3f549c791a2/CORE-STANDARDS.md) | `CORE-STANDARDS.md` |
| S02 | [Current route selection](https://github.com/MrScripty/Coding-Standards/blob/8fef41d18c524c7ac6a1a16f242cc3f549c791a2/STANDARDS-ROUTER.md) | `STANDARDS-ROUTER.md` |
| S03 | [Planning authority, proportionate admission and lifecycle](https://github.com/MrScripty/Coding-Standards/blob/8fef41d18c524c7ac6a1a16f242cc3f549c791a2/workflows/planning.md) | `workflows/planning.md` |
| S04 | [Complete consumer evidence and failing-baseline treatment](https://github.com/MrScripty/Coding-Standards/blob/8fef41d18c524c7ac6a1a16f242cc3f549c791a2/workflows/verification.md) | `workflows/verification.md` |
| S05 | [Bounded investigation and implementation decision](https://github.com/MrScripty/Coding-Standards/blob/8fef41d18c524c7ac6a1a16f242cc3f549c791a2/workflows/development-proportionality.md) | `workflows/development-proportionality.md` |
| S06 | [Coherent slices and boundary completion](https://github.com/MrScripty/Coding-Standards/blob/8fef41d18c524c7ac6a1a16f242cc3f549c791a2/workflows/implementation.md) | `workflows/implementation.md` |
| S07 | [All eight composed-design probes and authority scope](https://github.com/MrScripty/Coding-Standards/blob/8fef41d18c524c7ac6a1a16f242cc3f549c791a2/topics/architecture.md) | `topics/architecture.md` |
| S08 | [Runtime proof and semantic preservation](https://github.com/MrScripty/Coding-Standards/blob/8fef41d18c524c7ac6a1a16f242cc3f549c791a2/topics/contracts.md) | `topics/contracts.md` |
| S09 | [Complete message/outcome decoding](https://github.com/MrScripty/Coding-Standards/blob/8fef41d18c524c7ac6a1a16f242cc3f549c791a2/profiles/boundaries/ipc.md) | `profiles/boundaries/ipc.md` |
| S10 | [Protocol outcome preservation](https://github.com/MrScripty/Coding-Standards/blob/8fef41d18c524c7ac6a1a16f242cc3f549c791a2/topics/contracts/protocols.md) | `topics/contracts/protocols.md` |
| S11 | [Current authority and impact-selected documentation](https://github.com/MrScripty/Coding-Standards/blob/8fef41d18c524c7ac6a1a16f242cc3f549c791a2/workflows/documentation.md) | `workflows/documentation.md` |
| S12 | [Protected operations and network/input boundaries](https://github.com/MrScripty/Coding-Standards/blob/8fef41d18c524c7ac6a1a16f242cc3f549c791a2/topics/security.md) | `topics/security.md` |
| S13 | [Required plan fields and evidence structure](https://github.com/MrScripty/Coding-Standards/blob/8fef41d18c524c7ac6a1a16f242cc3f549c791a2/templates/PLAN-TEMPLATE.md) | `templates/PLAN-TEMPLATE.md` |
| S14 | [Current commit guidance](https://github.com/MrScripty/Coding-Standards/blob/8fef41d18c524c7ac6a1a16f242cc3f549c791a2/workflows/commit.md) | `workflows/commit.md` |
| S15 | [Current Engine interface identity](https://github.com/MrScripty/Coding-Standards/blob/8fef41d18c524c7ac6a1a16f242cc3f549c791a2/tools/standards_engine/contracts/a1-interface.toml#L1-L4) | Interface 37; not proof of installed MCP identity. |

## Current execution update

The live authoring instance was actually queried and routed for the goal: implementation `0.2.0`, interface 37, instance `6a638e80-40ef-49d7-82ec-a06a947c8f62`, route snapshot `snapshot:v1:7faccce8-ab54-492b-9e67-2e29e6295b78`, 47 selected standards and zero unresolved categories. Core/Router, planning, documentation, implementation, verification, build/commit, Rust/TypeScript, IPC/interop, lifecycle, contracts, architecture, diagnostics, resilience, security and GUI/oracle/platform guidance were read. The package statement that the live MCP route still needed verification is superseded by this execution evidence; the installed routing result remains a dated project-local reference rather than a universal current-HEAD claim.

The first implementation is deliberately limited to a loopback-only, closed-operation HTTP JSON-RPC transport. Its exact source write set and focused checks are recorded in the execution ledger. The route remains blocked for consumer migration until a nonempty isolated library and service/root identity are qualified.

The subsequent RPC-01 slice extends that transport through the explicit selector, dispatch, technical-fit and runtime-host capability seams. It does not alter Pantograph inference ownership or admit a Pumas root fallback. Consumer migration is therefore partially admitted for compilation and typed projection, while executable artifact identity, producer generation identity and real T/I/E acceptance remain gated.

## 2026-09-26 — current Coding-Standards runtime recheck

The root authoring route was rechecked through the installed Coding-Standards MCP after the producer-selected directory qualification. It reported purpose `authoring`, interface 37, implementation `0.2.0`, instance `6a638e80-40ef-49d7-82ec-a06a947c8f62`, implementation digest `sha256:f7bd0d1d7cb03bc4ae2eb10aba6d840cbad9ea548c2c8a0b17958314a564c4cb`, catalog digest `sha256:3b985fc1bf612c533ab735962f82a27e081deba0ab09fd05c98e5904c59a032f`, schema digest `sha256:32be44913a5e10913bed5d08ffc0dd4961afb5c4e9bd56514e5773f95a0992f5`, `installation_state=restart-required`, and action `restart-and-reconnect`. No restart/reconnect operation is exposed in the current tool surface; standards-guided commit closure remains gated. This current runtime result supersedes neither the dated 47-standard route snapshot nor the separate reviewer-session observations, which remain recorded as distinct historical evidence.

## 2026-09-26 — fresh current route and full selected read

The continuation re-ran the authoring route after the complete Tiny SD identity qualification. Runtime identity remained interface 37, implementation `0.2.0`, instance `6a638e80-40ef-49d7-82ec-a06a947c8f62`, with implementation digest `sha256:f7bd0d1d7cb03bc4ae2eb10aba6d840cbad9ea548c2c8a0b17958314a564c4cb`, catalog digest `sha256:3b985fc1bf612c533ab735962f82a27e081deba0ab09fd05c98e5904c59a032f`, schema digest `sha256:32be44913a5e10913bed5d08ffc0dd4961afb5c4e9bd56514e5773f95a0992f5`, purpose `authoring`, and `installation_state=restart-required` / `restart-and-reconnect`.

Routing facts were captured in snapshot `snapshot:v1:46d05785-182b-4696-87d6-e739de40925f`. The supplied library/Rust/TypeScript, generated-contract/IPC/interop/persistence, implementation/verification/documentation/planning/commit and cross-cutting concern facts selected 41 standards with zero unresolved categories. Core, Router and all selected targets were read through that exact snapshot, including current commit and verification guidance. This is the current route evidence; the older 47-standard snapshot and distinct reviewer observations remain historical. No restart/reconnect capability is exposed, so commit closure remains gated by the runtime installation state.

## 2026-09-26 — latest current route and commit-policy read

The authoring interface was routed again from its latest fact catalog snapshot `snapshot:v1:17775220-9a69-4667-9fc2-4106887b7183`. Explicit library/Rust/TypeScript, generated-contract/IPC/interop/persistence, implementation/verification/documentation/planning/commit, and cross-cutting concern facts selected 41 standards with zero unresolved categories. The complete 41-target reading plan was returned, and `workflow.commit` was read from the same snapshot. Its current requirements are recorded in the execution ledger: inspect status and staged scope, review generated and sensitive-file effects, run focused affected checks, update material plan/ledger state, and make one atomic conventional commit. Runtime installation remains `restart-required`; no restart/reconnect tool is exposed, so no commit was attempted through that gate.
