# Production Dispatch Producer Repair

Date: 2026-10-02. Status: `Verifying`; source review and focused evidence are
complete in scope. Gate composition passes; published-head CI remains pending.
Base: `4938e405c7f656365eefdca492774ccae110c90d`. Branch:
`codex/production-dispatch-identity`. Standards:
`MrScripty/Coding-Standards@dcc56f26e884ade260770beceba2501d3746200d`.
The [active plan](../plan.md) owns sequencing and acceptance; this report records
the bounded source/evidence result, not another plan.

## Repaired Contract

The production composition now gives its existing capability source the actual
gateway. The registration owner initializes canonical backend family and stable
per-backend residency identity, preserves an embedding host's explicit identity,
and registers available cold backends without falsely reporting loaded instances.
Normal active reconciliation also uses that owner.

The capability source retains backend-declared task, model-source and available
variant facts. Package `Transformers`/`Diffusers` labels are compared with those
typed model-source declarations, not equated to backend ID `pytorch`. Graph and
candidate projections preserve that distinction. Unsupported tasks remain
unavailable even when a model-source hint matches.

CPU and MPS singleton IDs are exposed only from available variants. Indexed
CUDA/Metal IDs require a live owner observation matching backend, runtime and
instance, plus a compatible available variant. No device index is guessed from
a class. Graph availability distinguishes an installed cold runtime from
loaded residency. Candidate selection requires the explicit observed device
and exactly one matching available variant before acquiring a reservation;
that qualified variant reaches the scheduler and unchanged text host.

No Pumas dependency upgrade, path fallback, trust relaxation, model download,
new registry/protocol, scheduling optimizer, batching change, claim change,
or persisted/public contract change is included. ReadOnly/LocalClient execution
limits and image-output support remain independent prerequisites.

## Bounded Consumers And Review

Changed files are the eight existing embedded-runtime modules:
`runtime_registry.rs`, `runtime_registry_tests.rs`,
`runtime_dispatch_capability_facts.rs`, `runtime_dispatch_candidate_provider.rs`,
`runtime_dispatch_source_snapshot.rs`, `inference_interface_facts_provider.rs`,
`workflow_service_composition.rs`, and `runtime_host_text_execution.rs`.
The existing pure graph-facts projection is crate-visible for the composed
contract regression; no external API is exported.

Initial review found two additional consumer contradictions: absent graph device
facts and cold runtimes labeled NotInstalled. The repair now covers both,
plus backend task filtering. Original complete-registration/candidate fixtures
could not decide these producer-path properties and are not used as their oracle.
Independent read-only re-review found no blocking source finding and separately
reran the connected graph-to-host, indexed live-owner device and no-reservation
negative regressions successfully with locked/offline dependencies. This accepts
the bounded source repair, not the remaining global/environment claims.
Final reviewed Rust-diff SHA-256:
`5dd0629b774e68129055d6be25cf2e1f2899b9ed443442707026b24047bed16c`.

## Evidence And Reproduction

Linux x86-64, Rust/Cargo 1.92.0, rustfmt 1.8.0, Clippy 0.1.92. Tests use
`--no-default-features --features backend-llamacpp,backend-pytorch`; this excludes
Candle/CUDA build qualification. Full workspace formatting passed.

The pinned ort-sys native download endpoint refused connection. Recovery used
Microsoft's [official ONNX Runtime 1.24.2 release](https://github.com/microsoft/onnxruntime/releases/tag/v1.24.2),
asset `onnxruntime-linux-x64-1.24.2.tgz`, checked against release SHA-256
`43725474ba5663642e17684717946693850e2005efbd724ac72da278fead25e6`.
`ORT_LIB_LOCATION` names its extracted `lib` directory,
`ORT_PREFER_DYNAMIC_LINK=1`, and the loader path includes it. The existing
relocated Python 3.12 library is included through `LIBRARY_PATH` and
`LD_LIBRARY_PATH`; no system/profile change or Cargo feature/pin replacement
was made. Set `XDG_CONFIG_HOME` to a fresh writable test directory so Pumas
fixture registries cannot touch personal configuration.

| Stage | Command / deciding scope | Result |
| --- | --- | --- |
| Formatting | `cargo fmt --all -- --check` and `git diff --check` | Passed |
| Focused producer/evidence checks | `cargo test -p pantograph-embedded-runtime --locked --offline --no-default-features --features backend-llamacpp,backend-pytorch --lib runtime_dispatch` | 46 passed |
| Connected graph-to-host regression | `runtime_host_text_execution::tests::production_registration_and_candidate_reach_text_host_without_injected_selection` | Passed: empty registry, production registration/capabilities, actual graph resolver with cold CPU, candidate provider, scheduler, real host/gateway and completed exact text; unobserved device and unsupported task assertions included |
| Owner-device correlation | `runtime_dispatch_capability_facts::tests::indexed_devices_require_matching_live_owner_observation` | Passed: actual observed index retained; unobserved/other-instance/other-runtime/inactive/unavailable cases publish no indexed device |
| Broad affected crate | Same Cargo features, `--lib` without filter | Candidate 449 passed / 3 failed; exact-base 444 passed / same 3 failed. Failure identities and assertion bodies match, not just counts |
| Full warning-deny Clippy | `cargo clippy -p pantograph-embedded-runtime --no-default-features --features backend-llamacpp,backend-pytorch --lib --tests -- -D warnings` | Failed before affected crate at pre-existing `pantograph-managed-dependencies/src/redistributables/state.rs:32` needless borrow |
| Affected-package Clippy discriminator | Same command with `--no-deps --message-format=json` | Both base and candidate emit exactly 49 diagnostic observations (11 library + 38 test). Code/message/file/source-snippet comparison has no additions or removals; still a failing gate |
| Traceability / remote CI | Composed gate dependency `9567186` and exact published implementation commit | Composed staged gate passes; implementation remote CI pending, no hook bypass or CI pass claimed |
| Real models, GPU/MPS, desktop, cold retained output | DA-03/04/05/06 procedures | Not run; not established by these tests |

The three unchanged broad failures are:

- `tests::workflow_run_execution_tests::workflow_execution_session_dispatches_through_production_embedded_image_runtime_host`: RetryDeferred instead of RuntimeHostCompleted
- `workflow_service_composition::tests::resource_backed_hosted_service_refreshes_validation_from_production_facts`: Blocked instead of Executable
- `workflow_service_composition::tests::resource_backed_hosted_service_publishes_executable_snapshot_from_production_facts`: Blocked/DriftDetected instead of RequestReady

The connected regression deliberately controls Pumas package/target inputs and
final backend effects. It exercises actual producer-to-consumer code and never
injects complete runtime registration or dispatch candidates, but does not prove
Pumas filesystem/IPC qualification, a complete GUI graph submission, real model
loading or hardware performance. Existing canonical singleton-batch, retained
text, dependent text-to-image and lifecycle regressions still pass.

## Gate Composition Evidence

The proposal fast-forwarded its base to reviewed gate commit
`9567186252f7ebb60794911baeeabce0daec5140`
([draft PR #2](https://github.com/MrScripty/Pantograph/pull/2)). It initially targets
`fix/decision-traceability-ownership-2026-10-02`; after that prerequisite merges,
retarget `main` without rewriting published history. The complete implementation
patch was preserved byte-for-byte during composition; the reviewed Rust-diff
identity above is unchanged. No runtime source changed during this step.

Fresh composed-tree staged traceability, whitespace and workspace formatting
checks pass. The previously built and qualified unchanged embedded-runtime test
binary reran 46 focused dispatch tests plus the connected production text-host
and indexed-owner-device cases, all passing. This is reuse of existing binary
evidence, not a fresh Cargo compilation of the composed tree. No new heavy build
was performed while other workspace tasks needed disk capacity. Existing broad
suite and Clippy results above retain their original scope and date.

The gate's exact published head passed 24 tests on hosted Node 24.12.0/npm 11.6.2,
but its aggregate workflows still fail on baseline debt. Independent source
review is not hosted CI acceptance. This implementation remains a draft; CI and
applicable hosted review must qualify its own exact head before integration.

## Next Integration Conditions

1. Publish this focused draft against the exact gate dependency above; retain
   its evidence limits and complete required exact-head checks. The gate itself
   must satisfy CI/review before either proposal can integrate into `main`.
2. Preserve exact-head CI and applicable Greptile/CodeRabbit review as merge
   conditions under parent coordination. Existing lint, dependency-audit,
   native-provisioning and binding-generator failures remain owned blockers.
3. Qualify a supported exact Pumas revision for authenticated full package facts,
   coherent selected artifact/path/kind and configured-root attachment before
   changing Pantograph's pin or Owner/LocalClient consumer restrictions. Upstream
   moving PR heads are not accepted dependency authority.
4. Admit only the required fixture node contracts and run the real dependent
   desktop text→image workflow, lifecycle, retained-output reopen and measured
   efficiency procedures. A standard model's empty custom-binding list is valid.
