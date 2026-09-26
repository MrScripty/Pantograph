# Plan: Isolated Pumas library RPC and Pantograph-owned inference workflows

**Plan status:** `Active` — continuation; revision 2 changes planning authority, not implemented source.

**Current phase:** R2 — qualify Pantograph-owned text/image/embedding gateway routes and dependency/retention behavior; a producer-selected text target now executes through the real embedded runtime-host port while image-target identity remains gated.

**Next slice:** `RPC-01` — qualify the remaining producer-selected image target and prove the RPC-backed runtime-host path for the remaining modalities. Tiny Aya now supplies a producer-issued selected-artifact identity and ready no-custom Transformers directory target, and a Pantograph-owned text run returned generated text; Tiny SD still has no producer-issued ID, while Z-Image retains missing text-encoder shard diagnostics and an unadmitted `ZImagePipeline`.

**Acceptance status:** `blocked` — required live artifact/model/desktop evidence is not established here.

**Decision owner:** Puma / MrScripty. **Integration owner:** the Codex primary session, requested GPT-6 Luna Max.

**Execution ledger:** [execution-ledger.md](execution-ledger.md). **Issues:** [issues.md](issues.md).

**Reports:** [audit](reports/2026-09-25-standards-and-rpc-audit.md), [library contract and baseline inventory](reports/2026-09-25-pumas-contract-inventory.md), [acceptance procedures](reports/2026-09-25-end-to-end-acceptance.md).

## Objective

A user can connect Pantograph to an independently retained Pumas Library RPC artifact, discover and select appropriate library models/assets, author/save/run workflows, and retrieve complete, correctly attributed text, images and embeddings. Every inference request executes through **Pantograph's existing unified inference-backend API** and its backend implementations. The desktop must demonstrate all three task families, actual generated text feeding image and embedding tasks, and complete retained outputs after a policy-qualified cold reopen.

Pumas supplies model-library facts, resources, requirements, saved model settings and explicitly authorized acquisition/retention. Pantograph owns workflow semantics, scheduling, resource admission, backend selection, inference runtime/model loading and unloading, execution, cancellation, artifacts and presentation. Pumas is never an inference provider for Pantograph.

Finish the inherited required standards/remediation objective too. Report product functionality and whole-repository compliance separately until both have matching evidence. This planning package certifies neither.

## Authority, history and inspected identities

Keep `docs/plans/domain-architecture-and-multimodal/plan.md` as the sole active sequencing authority. Use operation `continue` after checking actual local lifecycle and reconciling this revision. The older remediation and image plans remain Superseded; accepted documentation consolidation and scoped EX/RT/tooling repairs remain historical evidence. Keep original issue IDs, the [prior-claim mapping](reports/repository-review.md), reports and execution ledger. Do not rewrite terminal historical plans to satisfy newer templates.

Inspected Pantograph: `56caed029a48db66212a953a1677a94449396d52`. Pumas release 0.7.0: `29242fce4ec9043becaeb061587f364ecc4e7177`; upstream main: `51301bd317d7962b51d534590a685b676de835d9`. Current inspected Coding-Standards: `8fef41d18c524c7ac6a1a16f242cc3f549c791a2`, Engine interface 37. These are evidence references, not an exact-HEAD requirement for every Pantograph edit.

Revision 2 supersedes the previous package's permission to delegate inference to Pumas and its use of the active local Pumas checkout/service as a target. It also replaces historical model-routing assignments with the role policy below. Accepted code evidence is retained only within its original scope; affected behavior needs current regression evidence.

## Reconciliation of work after the inspected baseline

The checkout contains committed work after the package's inspected Pantograph revision. Preserve it as history and evidence, but requalify it against revision 2 before treating it as active authority:

- Commit `4938e405` selected a shared Pumas owner/client integration and recorded owner-dependent consumer design. Its source and review evidence remain in the existing ledger and commit history; revision 2 withdraws any active-Pumas or Pumas-inference permission and requires the independent RPC-00 baseline first.
- The accepted D-02 complete-image Inspector repair and browser evidence remain valid within their recorded scope. They do not establish real-model, Pumas, embedding, cold-reopen or Pantograph-only inference acceptance; R3 must recheck the affected boundary where necessary.
- The newer issues and ledger entries are preserved below. They are not overwritten by this update, and their unresolved blockers are carried into RPC-00/R1 rather than silently closed.


## Current Coding-Standards MCP requirement

Use the actual Coding-Standards MCP for planning, implementation, verification and commits. Check its installation/runtime interface, accepted guidance identity and purpose-qualified availability; retrieve Core, route real task facts, and follow the required closure. An online MCP or source checkout is not itself evidence that the selected guidance is current. Repair setup/drift through the supported mechanism; do not manufacture authority through authoring access or use unqualified prose as a substitute.

Record one relevant standards baseline and route per admitted slice. Reconcile material changes at integration boundaries. Missing applicable guidance blocks its dependent write/commit decision while bounded read-only discovery and independently qualified work continue. These planning files summarize project decisions, not a replacement standards corpus. The live authoring MCP was executed for RPC-00/RPC-01 transport admission; final standards/commit closure remains pending.

### Current route refresh — 2026-09-26

The current authoring runtime was rechecked before continuing: interface 37, implementation `0.2.0`, instance `6a638e80-40ef-49d7-82ec-a06a947c8f62`, implementation digest `sha256:f7bd0d1d7cb03bc4ae2eb10aba6d840cbad9ea548c2c8a0b17958314a564c4cb`, catalog digest `sha256:3b985fc1bf612c533ab735962f82a27e081deba0ab09fd05c98e5904c59a032f`, schema digest `sha256:32be44913a5e10913bed5d08ffc0dd4961afb5c4e9bd56514e5773f95a0992f5`, purpose `authoring`, and `installation_state=restart-required` with action `restart-and-reconnect`.

The fresh route snapshot is `snapshot:v1:46d05785-182b-4696-87d6-e739de40925f`. It selected 41 standards with zero unresolved fact categories for the current library/Rust/TypeScript/generated-contract/IPC/interop/persistence implementation, verification, documentation, planning and commit work. Core, Router and every selected target were read through the same snapshot, including the current commit guidance. This route is the current standards evidence for the continuation; the earlier 47-standard route remains historical. Because the runtime still requires restart/reconnect and no such operation is available in the current tool surface, standards-dependent commit closure remains gated.

## Scope and constraints

In scope: external library RPC, consumed contract/dependency migration, isolated reference artifact, library discovery and freshness, model details/requirements/settings, asset/load-target resolution, Pantograph backend/preflight/scheduler paths, all three inference families, complete artifacts, GUI/headless consumers, lifecycle and inherited required standards obligations.

Out of scope: Pumas inference or serving control, Pumas private provider protocols, Whip-Docs integration, a code parser, another inference framework, a second scheduler/catalog, remote/LAN exposure, arbitrary model-family support, automatic model-code trust, upstream source changes and external publication.

### Mandatory execution boundary

Use only library/resource operations on the Pumas client. Do not call Pumas `/v1` routes (including served-model discovery), inference RPC, provider-private endpoints, profile/serving start or Pumas-owned inference through any proxy. This prohibition includes optional paths, compatibility fallbacks and demonstrations. Existing non-Pumas backends may keep their own compatible-provider protocols under Pantograph's support contract; do not globally ban an endpoint path used by unrelated legitimate providers.

All text/image/embedding work uses the existing unified Pantograph inference API. Preserve one canonical runtime route and its typed unsupported outcomes. Pumas metadata does not prove that a Pantograph adapter supports a model, grant custom-code permission or authorize a runtime launch. Asset resolution is not inference. Keep model/library/artifact identity distinct from Pantograph runtime/backend and task identity.

HTTP JSON-RPC `/rpc` and core framed IPC are different contracts. Select the exact supported library operation/transport before implementing it. RPC loss does not authorize claiming the library root, directly opening its database, guessing storage paths, substituting another instance/model, or invoking inference through Pumas.

### Immutable integration fixture, not the active development checkout

Before baseline selection, inspect current upstream HEAD and the relevant release-to-head delta. The initial candidate is the host-appropriate 0.7.0 headless artifact; its exact metadata and provisional rationale are in the inventory. GitHub labels it prerelease and not immutable: retain verified bytes by digest and resolve its source commit rather than trusting a moving tag/name.

Use a newer artifact only for a demonstrated required library contract, correctness, security or compatibility reason. If no suitable remote artifact exists, build an exact upstream commit in an independent checkout and separate build target with controlled dependencies. Retain its binary, hash, source, feature/toolchain and lock/build provenance. Do not use a worktree linked to the active local Pumas repository, its target directory, mutable path override or development service. No unrecorded switch to new upstream bytes mid-slice or acceptance run.

Use an explicit independent endpoint, process owner, library root/database, configuration, registry/cache and fixture lifecycle. Preserve shared/user data and production services. Provision models through supported library operations and qualified asset policies; do not attach an older candidate to a newer live writable model database. A missing required operation changes the recorded candidate decision or creates an upstream blocker; it never authorizes an inference fallback.

### Dependency and execution authority

Necessary dependencies may be installed and repository code run under this goal. Added dependencies require a concrete consumer need, owned source/version/lock/provenance, alternatives and trust/license consideration, lifecycle and relevant tests. Prefer existing mechanisms and isolated environments. This permission does not grant arbitrary global upgrades, new paid services/credentials, large downloads or destructive/shared-system changes without their actual authority.

Retain ownership of async work, inference workers and results through the applicable terminal observation. Elapsed time or silence alone is not failure. Keep connection setup, library reads, Pantograph task duration, cancellation request and confirmed stop distinct. Do not copy Pumas serving lifetime or input limits into Pantograph's backend contracts or replay uncertain inference automatically.

## Binding decisions

| Decision | Owner | Selection and supersession |
| --- | --- | --- |
| Inference | Pantograph unified inference API and backend owners | All T/I/E execution remains in Pantograph; any earlier conditional Pumas-inference path is withdrawn. |
| Pumas responsibility | Pumas library service; Pantograph library client | Library/resources only; validate supported RPC facts and resolve assets without duplicating library authority. |
| Baseline | Integration owner with Astra planning advice | Compare upstream before choosing; initially evaluate pinned 0.7.0 headless bytes; select newer only for a documented relevant need. |
| Isolation | Fixture/dependency owner | Independent retained artifact, service and writable state; active Pumas development cannot become a build/runtime dependency. |
| Standards | Current Coding-Standards MCP | Actual routed and available guidance governs all planning, implementation and commits. |
| Coordination | Luna Max integration role | Use the role policy below and maximize independent concurrency; shared authorities have one writer. |
| Completeness | Product and verification owners | All three real desktop modalities, dependency flow and retained outputs plus inherited required claims; no selector-only or provider-only substitute. |

## Objective acceptance

The new integrated candidate has no satisfied product/workflow acceptance row yet. RPC-A00 has a qualified artifact/isolated-baseline result; the report defines the remaining procedures and records evidence when obtained.

| ID | Observable criterion | Kind | Environment | Mode | Status |
| --- | --- | --- | --- | --- | --- |
| RPC-A00 | Chosen artifact/contract/build is pinned and reproduced independently of the active Pumas development checkout, service and mutable state. | release-artifact | representative | automated | qualified for retained 0.7.0 bytes and isolated root |
| RPC-A01 | A separately running qualified Pumas artifact supplies the actual Pantograph library facts/assets path; Pantograph never owns its root/database. | system | required-real | automated | partial: producer-supplied Tiny Aya selected identity and `ready/ready` directory target now execute through Pantograph's selected-text path; Z-Image also supplies a target but is incompatible, while complete Tiny SD remains identity-gated |
| RPC-A02 | Every consumed library request/result/event preserves producer semantics, outcome, correlation, identity, version and extra-field policy. | contract | representative | automated | partial: positive file target, synthetic and producer-derived directory targets, authenticated IPC selector/load-target/facts, Tiny Aya selected identity and Pantograph target projection, and negative outcomes observed; HTTP selector snapshot remains `-32601/not_found`, while Tiny SD has no producer identity and Z-Image retains missing-shard diagnostics |
| RPC-A03 | Library outage, replacement, cursor gaps and stale selection retain truthful refresh/failure behavior without cross-instance state or replay of uncertain mutations. | integration | representative | automated | partial: same-root producer restart requalified the target, and a stopped producer yielded a typed client transport error without stale replay; cursor gaps, cross-instance replacement and stale-selection recovery remain open |
| INF-A01 | All actual T/I/E executions traverse Pantograph's unified inference API; observed Pumas traffic contains only admitted library/resource operations and zero inference/serving calls. | system | required-real | automated | partial: producer-qualified Tiny Aya text executed through Pantograph after library-only RPC reads; real producer-selected image/embedding execution and complete three-family observation remain open |
| DA-01 | Inherited maintained areas have current applicable-standards review and evidenced dispositions; required violations remain open until resolved. | focused | not-applicable | either | pending |
| DA-02 | Integration has one owner per concern; current composition/change/deletion evidence and real consumer/migration dispositions agree. | integration | representative | either | pending |
| DA-03 | Real desktop produces complete text, decodable images and meaningful embeddings; generated text actually feeds both downstream task families with correct identity. | user-workflow | required-real | automated | blocked |
| DA-04 | Missing facts, invalid data, worker failure, cancellation, disconnect and shutdown preserve accurate terminal/unknown results, resource ownership and unrelated work. | system | representative | automated | pending |
| DA-05 | Saved graphs and complete retained outputs survive the chosen cold-reopen/version/retention procedure; affected IPC/native/host consumers agree. | contract | representative | automated | partial: controlled fresh-process retained-output reopen; desktop/saved-graph scope open |
| DA-06 | Qualified selector/dispatch/reuse measurements distinguish inference cost from orchestration and satisfy recorded workload-specific targets. | system | required-real | automated | blocked |
| DA-07 | Required static/test/security/accessibility/dependency/target/artifact obligations and independent reviews pass; actual MCP-guided process/commit evidence is retained. | system | required-real | either | pending |

DA-03 procedures T/I/E/C are all mandatory. INF-A01 needs both construction/source-route review and observed actual execution; a grep result or an idle Pumas fixture is not sufficient. DA-07 aggregates references and retains the separate original evidence kinds; it cannot downgrade a required artifact/platform claim. Explicit maintainer scope decisions must remain honest, not reclassify missing required evidence as a pass.

## Simplicity and ownership review

**Composed-design review applicability:** `applicable`. These eight answers select the direction; RPC-00 completes the actual consumed-wire and fixture facts before implementation.

1. **Independent concerns:** Pumas owns model-library information/assets/acquisition; Pantograph owns inference backends and workflow execution; the integration client owns library transport projection; the fixture owner owns the frozen dependency's build and process. None owns the others' policy.
2. **State, identity, value, time, policy and mechanism:** separate library/model/artifact revision, wire version, artifact digest, service/root identity, observation cursor, Pantograph runtime/backend/task/attempt, retained result and standards revision. A new Pumas build prompts explicit client qualification, not a new inference engine or rewritten historical result.
3. **Caller and composition knowledge:** the composition root supplies an explicit library endpoint/client lifetime and existing inference runtime. Callers receive validated library facts or Pantograph execution outcomes. They do not know Pumas's registry paths, internal providers, framing, storage layout or build directory.
4. **Representative changes:** library-wire changes affect the client/mapping/tests; inference changes affect Pantograph backends/contracts; model metadata changes trigger existing freshness handling; artifact candidate upgrades affect dependency qualification; presentation changes stay out of execution ownership.
5. **Stable interfaces and hidden knowledge:** stable model references, library result contracts and the existing unified inference API remain distinct. Hide transport and local fixture mechanics. Any retained Rust dependency is commit-pinned and does not grant root ownership or require mutable source adjacency.
6. **Independent evolution, failure and verification:** Pumas development can proceed or disappear while the retained fixture works. Library outage remains an explicit input/readiness outcome; inference availability is separately established through Pantograph. Qualify artifact, adapter, backend and desktop boundaries separately and then together.
7. **Deletion result:** removing the library client would spread transport/version/outcome knowledge into callers; removing a Pumas inference adapter removes forbidden complexity entirely, so none is introduced. Prefer a small ordinary inventory/build record over a new version registry, audit framework or second model catalog.
8. **Necessary and cumulative complexity:** independent deployment, model trust, async lifecycle and retained multimodal values are inherent. Existing owners contain them. A retained binary plus isolated state stabilizes the external boundary without coupling the active upstream checkout or expanding inference authority.

## Milestones and admission

R0 has the exact documentation write set, and the reviewed RPC-01 transport-only slice has its exact source/dependency write set recorded below. Later source sets are selected from real callers when dependencies settle, within this authorized goal; normal refinement does not require approval per file. Independent R2/R3/R4 work with settled contracts may be admitted alongside R1, so the sequence is a dependency order, not a universal serialization barrier.

| Milestone | Goal / preserved contract | Allowed writes now | Gate | State |
| --- | --- | --- | --- | --- |
| R0 / RPC-00 | Route current standards, compare upstream/release, qualify a frozen library artifact, inventory actual operations/callers and native backend tuples, admit the next production slice. | Exact documentation set below; read-only source; controlled isolated dependency provisioning/build/probes are authorized outside application source. | Current MCP route; candidate rationale/digests; actual library observations or precise blockers; bounded consumer dispositions and coherent next source/test write set. | Complete for baseline; external identity/nonempty-library gates remain open |
| R1 / RPC-01 | Library-only RPC from explicit configuration through selector, dispatch/preflight and executable asset resolution; preserve current unified inference API. | Reviewed transport plus the explicit `PumasSelectorAccess::Rpc` projection/configuration and the enumerated dispatch/runtime-host/technical-fit consumer seams; no inference implementation or owner-root fallback. | RPC-A00/01/02 and a useful canonical Pantograph inference path; no forbidden Pumas calls. | In progress: RPC capability seam admitted; artifact identity and real execution remain gated |
| R2 / MOD-01 | Complete Pantograph-owned text/image/embedding routes, input/output contracts and dependent execution. | Admitted embedding runtime-host projection, gateway handoff, typed vector contracts and focused mapping tests; remaining source writes are selected from settled callers. | Real backend T/I/E and C, INF-A01 and complete artifacts. | In progress: embedding source seam admitted; real execution, fan-out and retained-output evidence remain gated |
| R3 / UI-01 | Complete user selection/author/save/run/inspect, D-02 repair, embedding presentation and cold reopen. | None until exact relevant files are selected. | Real desktop DA-03/05 and affected lifecycle/accessibility checks. | Planned |
| R4 / QUAL-01 | Close inherited current-standards/lifecycle/consumer findings and measured efficiency while preserving the usable path. | None until each coherent finding family is admitted. | Required DA-01/02/04/05/06/07 dispositions and evidence. | Planned |
| R5 / ACCEPT-01 | Independent final material review, integrated acceptance, current inventory and coherent commits. | Current plan/evidence and exact affected guides; source repairs return to their owning slice. | All required rows satisfied, retained artifact identities match, reviews complete, history/resources appropriately accounted for. | Planned |

### MOD-01 source slice admitted alongside RPC-01

The next independent source slice now exists for Pantograph-owned embeddings: runtime-host projection, gateway-owned llama.cpp execution, bounded typed vectors, scheduler result/input mapping, workflow JSON projection, and generated-text-to-embedding input coverage. It deliberately preserves the current R1 phase and RPC-01 next slice because executable artifact identity and real execution remain gated. The batch route is semantically complete per member but sequential pending native multi-input qualification; this is implementation progress, not product acceptance.

The source slice now also has a controlled scheduler fan-out regression: one generated text result is consumed by both image and embedding downstream members through the existing runtime-host batch handoff, and text/image/embedding terminal values are retained. The fixture uses a controlled runtime-host port and therefore proves scheduler dependency, correlation, projection and retention behavior only; it is not real backend, desktop or DA-03 evidence. Its parent test now launches a separate test process to reopen the persisted run and all three output bodies, advancing cold-reopen evidence without closing the saved-graph/desktop claim.

That same fixture now drops and recreates file-backed diagnostics and artifact-store services, then queries the persisted completed run and reads the retained text, image-reference and embedding bodies by their projected artifact identities. This is controlled same-process service/store reopen evidence for serialization and projection continuity; it is not a fresh-process saved-graph/desktop cold-reopen result and does not close DA-05.

The RPC baseline was also requalified against a disposable Pumas source clone pinned at `96f859443460ad8e4799d563528aaba113ccff21`, built with `--no-default-features` so Pumas inference plugins were absent. That candidate imported a real Granite embedding GGUF and returned complete library facts, but its HTTP surface still supplied no selected-artifact identity and rejected selector snapshots. The Pantograph RPC seam therefore retains its explicit typed `needs_detail`/fail-closed behavior; this evidence advances the external qualification boundary but does not permit inference, filename guessing or product acceptance.

Direct Pantograph gateway qualification now covers generated text, embeddings and a real local Tiny SD Turbo image through the existing unified API. The text run returned `"Amber Bridge"`, embedding returned two 384-dimensional nonzero vectors, and image returned one 256×256 PNG. These are backend/gateway proofs using explicitly copied local assets, not the missing RPC-selected image target, desktop workflow, or full DA-03 acceptance. The image worker's first-use CUDA metrics failure was repaired with device-context ownership; the focused worker suite is 8/8. Final standards state, independent reviews and coherent commits remain open.

### Producer-qualified Tiny Aya text execution — 2026-09-26

The pinned current-source no-inference Pumas binary (`96f859443460ad8e4799d563528aaba113ccff21`, `--no-default-features`, SHA-256 `3163f4d89d535870cb4ba2549adcbf47af435685576379a862e7ba422badf27c`) indexed a path-normalized Tiny Aya fixture in `/tmp/pantograph-rpc-tinyaya-20260926i`. Library-only `get_models`, package-facts and owner-fresh load-target calls returned model `llm/cohere2/coherelabs--tiny-aya-water__full_repo`, selected artifact `coherelabs--tiny-aya-water__files_b673ab802c36`, contract-3 `hf_compatible_directory` facts, `Cohere2ForCausalLM`, `requires_custom_code=false`, `library_owned`, `valid`, and a `ready/ready` absolute directory target.

The ignored repository acceptance `runtime_host_execution_port::tests::real_rpc_selected_tiny_aya_text_executes_through_runtime_host_port` consumed that exact target through `EmbeddedRuntimeHostExecutionPort`. Its typed Pumas resolvers normalized the `pumas://models/` URI for the library operation, reconciled the producer's omitted revision, and stripped only the owner-local absolute identity path while preserving the selected artifact ID and executable `local_load_path`. Pantograph's PyTorch/Transformers worker loaded both shards with `trust_remote_code=false` and `local_files_only=true`, then returned generated text `"Amber Bridge "` under the runtime-host's bounded eight-token default. No Pumas inference, serving, `/v1`, provider or profile operation was called. This closes producer-qualified Pantograph text execution only; image/embedding producer targets, real three-family fan-out, desktop acceptance and cold saved-graph evidence remain open.

### Producer-selected Qwen embedding target remains facts-gated — 2026-09-26

A fresh isolated current-source producer root `/tmp/pantograph-rpc-qwen-embed-20260926a` indexed the retained producer-selected Qwen3 Q4_K_M GGUF. Library-only `get_models`, package facts and owner-fresh load-target calls returned model `embedding/qwen3/qwen--qwen3-embedding-8b-gguf__q4_k_m`, selected artifact `qwen--qwen3-embedding-8b-gguf__q4_k_m`, valid `gguf`/`library_owned` facts, `requires_custom_code=false`, `artifact_state=ready`, `entry_path_state=ready`, and a file target. The exact GGUF SHA-256 is `3fcd3febec8b3fd64435204db75bf0dd73b91e8d0661e0331acfe7e7c3120b85`.

The producer facts declare `task_type_primary=unknown`, so Pantograph's real runtime-host embedding acceptance fails closed with an explicit unsupported package-task diagnostic before any llama.cpp load or inference. It does not infer embedding from the model path or task intent. This is stronger producer-selected embedding boundary evidence, but not successful embedding execution; the producer must supply canonical embedding task facts before RPC-backed embedding can be admitted. No Pumas inference, serving, `/v1`, provider or profile operation was called.

### Runtime-host identity and cancellation hardening — 2026-09-26

The image handoff and planner now require one nonblank producer-selected artifact ID and exact agreement across scheduler selection, package facts and the Pumas load target. The runtime-host Pumas target and package-facts resolvers also reject ready/facts responses whose producer model or conflicting nonempty revision differs from the scheduler selection before any Pantograph identity normalization. Focused planner, handoff, resolver-model, resolver-revision and producer-identity tests cover these cases; this keeps the complete Tiny SD candidate gated when its producer supplies no selected artifact ID.

The text runtime-host path now polls cancellation while both owner-fresh load-target and package-facts RPC futures are in flight, drops a pending future before returning the typed cancellation response, and gives cancellation precedence when a delayed resolver fails. `text_port_cancels_while_waiting_for_load_target_resolution`, `text_port_cancellation_wins_over_in_flight_load_target_failure` and `text_port_cancellation_wins_over_in_flight_package_facts_failure` cover those boundaries. These are lifecycle hardening results, not closure of producer-selected image/embedding execution or desktop/saved-graph cold reopen.

### RPC-00 exact documentation write set

- `docs/plans/README.md`
- `docs/plans/domain-architecture-and-multimodal/plan.md`
- `docs/plans/domain-architecture-and-multimodal/issues.md`
- `docs/plans/domain-architecture-and-multimodal/execution-ledger.md`
- `docs/plans/domain-architecture-and-multimodal/reports/2026-09-25-standards-and-rpc-audit.md`
- `docs/plans/domain-architecture-and-multimodal/reports/2026-09-25-pumas-contract-inventory.md`
- `docs/plans/domain-architecture-and-multimodal/reports/2026-09-25-end-to-end-acceptance.md`

Artifact download/build and disposable fixture state have their own recorded external paths and cleanup/retention owner. They are not writes to the active Pumas development repository. Production client/backend changes begin only after their exact admitted source/test set is recorded.

### RPC-01 transport-seam disposition

The first implementation slice is intentionally limited to a closed HTTP JSON-RPC transport in `workflow-nodes/src/pumas_rpc.rs`. It accepts only literal loopback `/rpc` URLs, disables redirects and ambient proxy routing, rejects credentials and non-loopback hosts, bounds response bodies, applies a request timeout, validates JSON-RPC version/correlation and result-versus-error field presence, and preserves typed remote error codes/classes. Its operation enum contains only library/resource methods: model listing/search, execution descriptor, package facts/summary, summary snapshot, update feed and artifact load-target resolution. It exposes no inference, serving, profile, provider-private or arbitrary-method route.

Focused evidence is `cargo test -p workflow-nodes --features model-library pumas_rpc --locked` with nine passing tests under loopback-capable escalation. The transport source/dependency write set is `crates/workflow-nodes/src/pumas_rpc.rs`, `crates/workflow-nodes/src/lib.rs`, `crates/workflow-nodes/Cargo.toml` and `Cargo.lock`. Astra High and Sol Extra High independently reviewed the seam; the first review found and the implementation repaired credential redaction, literal-loopback enforcement, JSON `null`/absent-field handling and request bounds. The final narrow review required an explicit forward-compatible envelope-field policy, which is now documented and covered by a regression. The current final reviews are complete: the security/lifecycle review found no remaining concrete source defect, while the standards review left only the cross-session Coding-Standards runtime-state discrepancy. Commit remains gated on reconciling that authority. This slice does not migrate `PumasApi` owner callers, establish producer/root identity, or claim successful facts/asset mapping from the empty fixture. Those remain the next RPC-01 gate together with configuration and all affected consumer dispositions.

### Current-source producer-selected directory qualification — 2026-09-26

The pinned current-source no-inference producer was run against a fresh isolated root containing a path-normalized copy of the retained Z-Image-Turbo metadata and read-only symlinks to its actual library bytes. The real Pantograph RPC client decoded `get_models`, package facts and `resolve_model_artifact_load_target`; the facts supplied selected artifact `tongyi-mai--z-image-turbo__bundle_e07edb6c5bfd`, `diffusers_bundle`, `requires_custom_code=false`, `validation_state=valid`, `storage_kind=library_owned`, and the exact directory target. The producer returned `artifact_state=ready` and `entry_path_state=ready`. The same HTTP surface returned `-32601/not_found` for `model_library_selector_snapshot`.

This advances RPC-A01/A02 beyond the synthetic fixture, but it does not admit execution: package facts report missing root-level text-encoder shard paths for Z-Image, and its pipeline is `ZImagePipeline`, outside Pantograph's currently qualified closed Stable Diffusion loader. The retained Tiny SD directory independently has all twelve expected files, `requires_custom_code=false`, valid library-owned facts and `StableDiffusionPipeline`, but its producer metadata has no selected-artifact identity; owner-fresh resolution therefore returns `ambiguous/missing_selected_artifact`. Neither candidate may be promoted by guessing an ID, rewriting metadata or treating a path as selection. The next source/evidence slice is a producer-issued identity for the complete Tiny SD target or another complete producer target matching an admitted Pantograph backend, followed by replacement and outage/reopen checks.

### Complete Tiny SD facts with missing producer identity — 2026-09-26

Normal current-source Pumas indexing was run in isolated root `/tmp/pantograph-rpc-tinysd-20260926e` against a path-normalized copy of the retained Tiny SD metadata and read-only symlinks to its bytes. The producer returned model `diffusion/cc-nms/tiny-sd-turbo` and package-facts contract 3 with `diffusers_bundle`, `library_owned`, `validation_state=valid`, `requires_custom_code=false`, `StableDiffusionPipeline`, Stable Diffusion family evidence, and all twelve expected component/config/weight/tokenizer files present. The facts retain the existing invalid `feature_extractor` diagnostic, which Pantograph's closed loader already treats as an absent optional image-processor component.

The producer's `get_models` record and package facts did not carry a selected-artifact ID. An owner-fresh load-target request containing only the model identity returned typed `artifact_state=ambiguous`, `entry_path_state=ambiguous`, and diagnostic `missing_selected_artifact`; supplying the observed absolute path without an identity returned `artifact_missing` rather than authorizing path promotion. The HTTP selector snapshot remained `-32601/not_found`. This is the strongest current complete-admitted candidate, but it is still a fail-closed identity blocker; no inference was run and the isolated owner was stopped with an empty registry.

The available library-only acquisition routes were also exercised in fresh disposable roots: `import_external_diffusers_directory` returned success but left the indexed model without a selected-artifact ID, `import_model_in_place` generated idempotent metadata/facts with the same missing identity, and the lower-level `import_model` route successfully indexed a retained Granite GGUF but likewise exposed no selected artifact while reporting `task_type_primary=unknown`. The analogous direct directory import failed during producer reconciliation at a safetensors weight path before producing a selectable package. All operations were stopped before any inference route; neither authorizes Pantograph to derive or promote an ID locally or infer the embedding task. The producer identity/task gate therefore remains genuine rather than an omitted import step.

A read-only scan of the current shared-library diffusion metadata found no alternate complete admitted candidate: Tiny SD is the only record declaring `pipeline_class=StableDiffusionPipeline`, and it is the same record without `selected_artifact_id`. Other diffusion records with producer-selected identities declare different pipelines or do not have executable package facts. This closes the candidate-search branch without changing the fail-closed identity rule.

### Authenticated framed-IPC qualification — 2026-09-26

The pinned current-source no-inference producer was also exercised through its separate authenticated framed local-IPC contract, using a fresh registry file under `/tmp/pantograph-rpc-zimage-20260926d` and the same isolated Z-Image root. The framed request used the producer's documented 4-byte big-endian length prefix and JSON-RPC operation `model_library_selector_snapshot`; the registry supplied the ephemeral connection token, which is intentionally not retained in project evidence. The producer returned one selector row with the producer-selected identity `tongyi-mai--z-image-turbo__bundle_e07edb6c5bfd`, `artifact_state=ready`, `entry_path_state=ready`, `storage_kind=library_owned`, and a complete cached summary. Authenticated framed `resolve_model_artifact_load_target` returned the same ready directory target, and the package-facts batch route regenerated the same contract-3 facts.

This explains the transport split rather than closing it: the same producer's HTTP `/rpc` route returns `-32601/not_found` for selector snapshots, while its owner-local authenticated IPC route supports selector snapshots and the typed load-target/facts operations. Pantograph's production seam remains the explicitly configured closed HTTP JSON-RPC client; the IPC evidence is retained as a separate qualified Pumas contract and is not silently treated as an HTTP fallback or a reason to discover/claim a Pumas root. The producer process was stopped cleanly and its isolated registry had no remaining instance row. The Z-Image target still reports missing text-encoder shard paths and an unadmitted `ZImagePipeline`, so the next gate remains a complete producer-provided directory target matching an admitted Pantograph backend.

### Linked generated-text fan-out through Pantograph — 2026-09-26

A disposable single-process qualification harness used the existing Pantograph `InferenceGateway` for all three executions. It first called `execute_typed` through the copied llama.cpp `10883` runtime and LFM2.5 GGUF, receiving generated text `"Amber Bridge"`. That exact returned string was then passed unchanged as both the image plan's `prompt` and the single embedding input; the image used the previously serialized synthetic ready Tiny SD directory target and Pantograph's PyTorch backend, while embedding used Pantograph's llama.cpp embedding mode. The run returned one nonzero 384-dimensional embedding and one nonempty 256x256 `image/png` payload (`bytes_base64=145876`), with each runtime stopped cleanly.

This is the required generated-text dependency at the Pantograph gateway boundary: downstream inputs came from the text result rather than a second literal fixture prompt. The text/embedding assets were the independently copied local GGUFs (SHA-256 `67b42337951a2b3140c0c2eb49b5bbbdd14653114337ac15397f68dc664648d4` and `d4b41f5d7db712806722103a1c6aba2f0fe99f77740501d6f313c8240641f145`), and the image run used the existing `.venv` Torch/Diffusers environment (`torch 2.10.0+cu128`, `diffusers 0.37.0`). The image target remains synthetic contract evidence rather than a producer-selected target, so this advances Pantograph fan-out and backend evidence without closing DA-03, INF-A01, or the real RPC-selected execution gate.

## Agent roles and coordination

| Role | Model/effort | Contract |
| --- | --- | --- |
| Primary orchestration and integration | GPT-6 Luna Max | Verify state, maintain constraints/inventory, assign disjoint sets, routine coordination, concise handoff inspection, final acceptance and coherent commits. |
| Planning and final review | GPT-6 Astra High | Design/admission recommendations for integration; final material review is read-only. |
| Implementation | GPT-6 Sol Medium | Admitted code and tests, especially complex/sensitive work. |
| Document discovery | GPT-6 Luna High | Read-only, bounded: latest inventory first, then relevant architecture/plan and `context.md` passages; include `architecture-plan` if present. Return constraints, concrete gaps and exact file/line evidence. |
| Independent architecture/security/lifecycle review | GPT-6 Sol Extra High | Read-only; reuse for narrow repair verification. |
| Preferred suitable implementation | Passeur MCP / Muse Spark 1.3 Contributor | Settled, bounded implementation/fixtures to reduce cost; never planning or review. |

These are requested assignments, not claims that a model or service is available. Verify exact capabilities/effort and primary-model state; report mismatches. If the Muse lane is unavailable, use Sol Medium for appropriate implementation and record it. Missing required review is an unsatisfied gate, not permission for self-review. Reviewers never edit or commit; implementation owners repair and reviewers inspect.

Verify Passeur's canonical Pantograph root/worktree binding and actual registered Muse capability before submission. Give each assignment goal, relevant facts/standards, material/base identity, exact primary and adjacent write sets, shared/forbidden files, acceptance, output and escalation. Shared contracts, generated files, lockfiles, common fixtures, plan and index are single-owner writes. Worker commits require explicit branch/commit ownership under the same standards; the root owns final integration.

Maximize independent parallelism among discovery, bounded planning, stable-slice review and admitted implementation. Do not implement an unsettled dependency or treat review of changing bytes as final. Apply concurrent-plan safeguards when outstanding proposals can become stale. Coordinate builds/GPU and fixture state to preserve valid evidence. Keep worker lifecycle control evidence-based rather than timer-based. Reuse concise findings and reviewers, not duplicated broad scans.

## Evidence, inventory and commits

Use [the acceptance procedures](reports/2026-09-25-end-to-end-acceptance.md). Extend existing tests before adding new evidence machinery. Tests must reach and discriminate the affected behavior under existing failures; unchanged failure counts alone are insufficient. A fake model, Pumas provider output, selector list or complete metadata envelope cannot establish a real Pantograph workflow.

Update the existing inventory/plan/issues/ledger at material change and acceptance boundaries, including actual artifact and standards identities, consumer mappings, roles, review dispositions, checks and exactly one next slice. Keep dated detail in history; no parallel inventory or per-agent diary. Obtain Sol Extra High review for important architecture/security/lifecycle changes and Astra High final material review; Luna Max runs integrated final acceptance on reviewed bytes. Separate API-equivalent cost estimates from real bills and use only observed data if costs are recorded.

Through current MCP-provided Commit guidance, inspect status, stage only the coherent admitted set, inspect the staged diff and sensitive/generated effects, run required checks/hooks, then commit the implementation and materially changed inventory/evidence. No arbitrary commit count, forced commit topology, implicit hook bypass, history rewrite or unrequested push. Branch/worktree isolation follows actual concurrency/risk and terminal resource ownership.

## Blockers and re-plan triggers

Unverified: complete consumed-operation parity; selector discovery; executable directory-target mapping; qualified image/embedding model/runtime/device and display; full scheduler/desktop lifecycle, cold-reopen and target evidence; final standards/commit closure. Tiny Aya now provides one real runtime-host text-generation acceptance through the unified Pantograph path. Qwen embedding remains correctly blocked by producer task evidence `unknown`, while the complete producer-selected image target and desktop/cold-reopen evidence remain open. Each item blocks only the decision or claim that depends on it.

Replan for material contract/ownership/trust/compatibility/lifecycle/output/acceptance changes, a required library operation missing from the selected artifact, unsafe deployment sharing, or an actual new affected semantic owner. Refine write sets without restarting planning when the original decisions remain sound. A new upstream commit is a review input, not an automatic replan or artifact update. Preserve the immutable selected candidate during a run.

## Completion

Continue through the complete goal, not merely RPC discovery or plan creation. Final acceptance requires all mandatory claims, independent review and truthful inventory/commit/resource state. Preserve required-real blockers and report separate product/compliance status; none of text, image, embeddings or Pantograph-only inference may be dropped to close the plan.
