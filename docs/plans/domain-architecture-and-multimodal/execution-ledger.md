# Execution Ledger

## 2026-09-08 — Plan prepared

- User priorities: coherent maintainable domain architecture, complete current-standards review/remediation, and functional text/image model workflows; execution by GPT-6 Astra medium.
- Pantograph baseline: `2ba2efb1c`; standards baseline: `366c1d90a24bbfb50973f62b155a5f3396c0f107`.
- Created the plan, issues and planning-baseline report. No product code changed and no model execution, full audit, test-baseline run or standards-compliance acceptance performed.
- Existing remediation and image plan authority remains unchanged until the explicit M0 handoff.
- Initial mixed fixture assumes generated text is consumed by image inference; user clarification is tracked separately.
- Protected pre-existing changes: modified `PROPOSAL-pumas-artifact-load-target-resolution.md` and untracked `PROPOSAL-pumas-library-fast-model-snapshot.md`.
- Plan status Planned; acceptance pending. No implementation agent was launched or model configuration changed.
- Added documentation proportionality at the user's request: one plan, event-driven updates, concise evidence links, no routine diaries or duplicated design records. This uses existing standards allowances; no standards exception or external standards edit was made.
- Replaced the initial all-medium execution policy at the user's request: Astra medium owns analysis/design/review; Astra low owns implementation and routine checks. Use bounded handoffs and decision-specific escalation with unchanged acceptance gates. The restarted session supports ten subagents, used only when independent work justifies them. No agents launched by this planning update.
- Added a bounded Luna max versus Astra low implementation pilot at the user's request: four comparable useful tasks after source-backed admission, common Astra medium review/acceptance, and total effort/cost including repair and rescue. Model selection remains provisional and task-specific; no pilot tasks or cost measurements have run yet.

## 2026-09-08 — M0 accepted; bounded review and pilot start

- User requested starting with subagents and cost-effectiveness testing. Four explicitly configured Astra medium reviewers produced execution, runtime, desktop and repository reports. They performed bounded source review, not full DA-01 acceptance.
- M0 handoff: old portfolio, its five children and image plan are Superseded. All CSA, CSR, SDC, ALB, FE, VT, DRD and IMG claims map to current owners/milestones in `reports/repository-review.md`; pending claims remain pending.
- Standards baseline unchanged; untracked standards prototype is excluded as non-policy. Executable Engine route selected Core/Router, Implementation, Verification, Contracts, Proportionality, Code Design, Documentation, Architecture and Planning with no unresolved M0 routing facts (snapshot `snapshot:v1:799ae75c-4c68-4b73-b5e0-e08a1d75e009`). Per-slice production routes remain required.
- Reviewed population: 1,482 tracked files, all workspace/maintained frontend, worker, binding, tool/CI, fixture, asset and documentation roots; explicit generated/external exclusions and unknown consumer/target obligations in coverage report.
- Product fixture: actual `text` output feeds next inference `prompt`; two retained outputs in one run, policy-qualified cold reopen in the same isolated root without destructive consume acknowledgement. Desktop report owns exact fixture procedure. No configured real model pair/workflow exists in the inspected saved-workflow set.
- Initial performance targets for later DA-06 measurement: warm local model-selector rows p95 <=250 ms and already-ready task dispatch overhead p95 <=500 ms, excluding declared batching/admission wait and model computation; qualify cold/warm workload and machine before acceptance. These are engineering targets, not measured results. Missing selected models/runtime/device prevent baseline qualification now.
- Local tools: Cargo 1.92.0, Node 24.12.0, Python 3.12.3; project Python package metadata and GUI tool presence recorded by reviewers. `nvidia-smi` cannot communicate with the NVIDIA driver; this does not rule out CPU/other devices. Required model IDs, environment mappings, saved mixed fixture and qualified display/runtime are missing. DA-03 is blocked; independent implementation is available.
- Confirmed EX-01 solo batch starvation, RT-01 incomplete Diffusers code authorization, RT-02 image-only canonical host, D-01 smoke cleanup leak, D-02 partial image decoding, COV-01 omitted tests and COV-02 retired package-doc source. See reports for exact evidence and limits.
- Baseline `npm run test:frontend` exits 0 with 71 file-level passes (~516 ms), but omits 19 tracked files. Direct presenter run executes 25 assertions; an intentional failure probe confirms runner failure propagation. No full repository green claim.
- P01/P02 admitted with disjoint write sets and unchanged acceptance standards. Supporting M4 work begins before runtime M2 while consequential design remains with medium analysis. P03/P04 deferred until justified, comparable slices exist.
- Per-agent tokens/billing unavailable from exposed tools. Compare visible review/repair/elapsed evidence; no measured cost winner yet.

| Task | Model/effort | First-pass result | Review/repair | Elapsed / usage | Acceptance |
| --- | --- | --- | --- | --- | --- |
| P01 cleanup | Luna max | Normal/signal tests passed; startup and test-timeout handling required review corrections | Medium review found missing early-signal replay, default INT launch semantics and exact-child timeout cleanup; Luna repaired, no implementation rescue | Total wall time not instrumented; independent five-test suite 154 ms / billing unavailable | Accepted: 5/5, bash syntax, focused ESLint, independent medium review |
| P02 discovery | Astra low | Runner/full suite passed after one fixture-environment correction; initial exclusions needed review repair | One substantive medium finding: generic build/target exclusions hid valid source tests; repaired and independently reviewed | Total wall time not instrumented; independent full suite 693 ms / billing unavailable | Accepted: 90/90 files, 539/539 assertions; runner 3/3, targeted ESLint |

- P02 verification: root independently ran `npm run test:frontend` with required child-process access, 539 passed/0 failed; log `/tmp/pantograph-p02-integrated-20260908.log` is transient supporting output. Medium reviewer independently checked 90/90 discovery and the three runner fixtures. New nested tests, failure propagation, empty discovery and preserved legitimate source-directory names are covered.
- P01 admission refined before implementation: a medium subprocess probe showed that plain removal of exec would regress INT/TERM delivery and delete state before the child exits. Required direct-child signal/reaping preservation was added; this is parent design refinement, not a Luna implementation defect. Broader GUI/driver process-tree behavior remains unproven.
- P01 final verification: root independently ran `node --test scripts/check-workflow-editor-image-generation-gui-smoke.test.mjs` with child-process access: five passed, none failed, ~154 ms; reviewer independently passed five cases (~163 ms). Scope covers success, child failure, INT/TERM, default INT launch disposition and exact temporary-root cleanup. `bash -n` and focused ESLint pass. Transient root evidence: `/tmp/pantograph-p01-integrated-20260908.log`.
- Root paused Luna's long turn to request a bounded status; returned status confirmed completed repairs/tests and no active child session. No other model completed its implementation. Root added only an explanatory comment for scoped monitor mode after review; semantics unchanged.
- Initial comparison: both candidates met their scoped quality gates with medium review; P02 needed one substantive exclusion repair and a test-fixture environment correction; P01 needed lifecycle corrections after parent scope refinement. Different task difficulty, interrupted status collection, missing total latency and unavailable billing prevent a cost/model ranking. P03/P04 remain unassigned; no extra benchmark work admitted merely to fill the sample.
- Next slice: medium admission of RT-01 closed-only loader invariants, followed by Astra low implementation and medium review outside the cheap-model pilot. RT-02 text host and EX-01 solo dispatch remain downstream execution work. Full audit, real workflow and release/target acceptance are still incomplete.

## 2026-09-08 — RT-01 and feature-portability repair admitted

- Medium design admitted the five-file closed-only Stable Diffusion adapter contract in `reports/runtime-review.md`. Explicit installed components replace generic dynamic pipeline/component selection; scheduler configuration is preserved, local safetensors-only loading is required, and validated provenance precedes cache reuse. This deliberately reduces the direct CLI's formerly generic pipeline surface. Custom-code authorization remains a later coherent feature, not a fabricated permission.
- Root reproduced the existing defect with `python3 /tmp/pantograph_rt01_loader_probe.py`: prohibited custom pipeline reached the model-loading effect with `trust_remote_code=true` (expected red assertion). Implementation also reproduced it through the actual imported worker module, with only dependency effects stubbed. No real models or custom Python code ran.
- Production standards route: snapshot `snapshot:v1:356e0989-668e-4a06-bfac-dc7c640f0b4b`, 19 selected standards, no unresolved routing facts. Rust foreign-memory requirements apply only if that mechanism changes; adding owned embedded module source does not introduce raw-memory work. Root remains the serial plan/integration owner.
- Existing default-feature PyTorch image-worker baseline: 7 passed. PyTorch-only test compilation instead exposed a separate feature-gated constructor defect in the gateway alias test. Medium admitted P03's single-file Mock constructor repair, retaining real registry switching and both PyTorch assertions. The failing baseline is recorded separately from RT-01.
- Astra low owns RT-01; Luna max owns P03. P03 is low-complexity fixture portability, useful supplemental pilot evidence but not comparable to the consequential loader repair. P04 remains unassigned; no artificial benchmark task is created. Implementation start observation: approximately 03:08 UTC September 9, inclusive elapsed evidence only; billing remains unavailable.

## 2026-09-08 — RT-01 and P03 accepted within their bounded claims

- RT-01 now has one private built-in bundle adapter, with a small admission/construction interface. The resident worker owns reuse/device/offload lifecycle; transport wrappers preserve typed denial. Embedded Rust and filesystem consumers load the same helper. Exact production/test scope expanded to six files after the isolated text fixture exposed its new helper dependency.
- Corrected the initial no-pickle claim before acceptance: safetensors indices can name other shard formats. Enforce qualified Diffusers 0.37.0 behavior, the existing Transformers Torch >=2.6 safety gate, and explicit Transformers `weights_only=True`; no model-supplied safe globals or unrestricted retry. Root's actual installed Diffusers/Transformers deserializers both rejected a harmless unsupported-global payload. This CPU-only serialization probe ran no models or generation (`/tmp/pantograph-rt01-deserializer-probe.log`).
- First implementation's eight focused tests passed. Independent medium review found two P2 admission gaps (tokenizer fallback metadata; symlink-loop typed failure/cache revocation). Astra low fixed both, added warm-resident regressions, and medium rereview accepted. Root integration additionally found the introduced text-fixture helper-registration omission. Its repair exposed a pre-existing missing batch-projector stub; both fixture corrections passed medium review. No higher-effort model implemented the repairs.
- Root final image-worker verification: 8/8 with defaults plus PyTorch, and 8/8 with PyTorch only, including the real loader/adapter. Logs `/tmp/pantograph-rt01-integrated-default.log` and `-pytorch-only.log`. All four existing text/lifecycle fixture consumers passed in separate Cargo processes; combined processes cannot substitute for this import-isolation proof. Affected Python syntax, Rust formatting and diff whitespace pass.
- Broader root PyTorch subsystem: 247 passed, 1 failed (`/tmp/pantograph-rt01-integrated-backend.log`). Medium confirmed COV-03's unchanged fixture/validator mismatch exists in HEAD. Warning-deny Clippy with `--no-deps` reaches 54 library / 55 test diagnostics in untouched files (`/tmp/pantograph-rt01-clippy.log`); no changed-line diagnostics. These are recorded baseline obligations, not a green full-suite/static-compliance claim. No warning suppression or unrelated repair was used.
- P03 accepted: independent medium source review had no findings, both feature variants passed initially and in root's final isolated checks, and PyTorch-only worker compilation now succeeds. Single constructor substitution preserved real registry transition and both assertions. Gateway file-wide rustfmt has pre-existing differences; the changed line itself conforms.
- EX-01 follow-up found EX-03: the 30-second collection window can consume the entire event lease and no renewal owner exists for longer execution. An expiry-only singleton patch is rejected. Integrator selects immediate opportunistic grouping of 1–8 compatible ready tasks; medium must settle running claim ownership/abandoned recovery before implementation. Details and deterministic test seams are in the execution report.
- RT-01's bounded trust repair is accepted; M2 remains Active. Full audit, static baseline remediation, canonical text-host route, singleton/long-running ownership and real desktop mixed-workflow acceptance remain incomplete. No real models, downloads, external Pumas mutation or commits occurred. Protected proposal hashes are unchanged.

| Task | Model/effort | First-pass result | Review/repair | Elapsed / usage | Acceptance |
| --- | --- | --- | --- | --- | --- |
| P03 feature fixture (low complexity) | Luna max | Both feature variants and original seven image tests passed | No review findings or repair; one constructor substitution | Approx. 4 minutes implementation/check/report interval; billing unavailable | Accepted; root independently passed both variants |
| RT-01 trust adapter (consequential, outside matched pilot) | Astra low | Eight focused tests passed; parent then corrected serialization design | Two P2 admission repairs plus introduced fixture-registration integration repair; pre-existing batch fixture corrected separately; medium rereview accepted | Approx. 12 minutes through implementation, reviews, integration and repair; billing unavailable | Bounded trust contract accepted; broader baseline failures recorded |

P03 adds useful evidence that Luna can complete a tightly specified fixture repair, but its difficulty is not comparable to RT-01. Missing billed/token usage and unmatched scope still prevent a cost winner. Keep Astra medium design/review, Astra low consequential implementation, and Luna max bounded implementation trials; P04 remains unassigned until a useful comparable task exists.

## 2026-09-08 — EX-01/EX-03 scoped ownership admitted

- User continued. Astra medium traced the only two private in-memory claim repositories; there is no separately deployed or durable lease owner. Both event (30 seconds) and batch (1 second) expiry can undermine a live execution. Adopt scoped ownership with fenced identities under the existing repository locks and worker JoinSet, retaining finite expiry for unowned work. The report defines exact transfer and abnormal-exit rules; no heartbeat service or timer is introduced.
- Immediate opportunistic 1–8 grouping is the selected policy. Remove the collection window and its worker timer/retired outcome; maintain compatibility, ordering, per-member results and empty/stale-claim rejection. Owned post-dispatch abandonment is failed/fenced, not permission to replay or claim that external resources stopped.
- Exact five-file implementation admitted in the plan. Astra low started at approximately 03:28 UTC September 9; medium will independently review. Root baselines pass: event 48, assignment 29, worker 24, batch 14. These old tests do not prove the new lifetime/singleton invariants.
- Standards route uses snapshot `snapshot:v1:356e0989-668e-4a06-bfac-dc7c640f0b4b`: 15 applicable standards, no unresolved facts; adds Concurrency and Rust Async to the architecture/contracts/library implementation and verification route. No new wire/persistence schema is admitted.
- Parallel RT-02 analysis established prompt→text projection but found active-model use, missing executable-target binding, premature-EOF success and unobserved blocking-worker completion. Inference owns those selected-residency/worker-lifecycle prerequisites; workflow EX-03 only owns its event/batch claims. The existing host string limit remains 1024 bytes until separately changed; no truncation or invented success is allowed.

## 2026-09-08 — Fourth pilot and observed token evidence

- Medium bounded COV-03 to one positive fixture among 19 worker JSON fixtures. Astra low changed only nested source version 2→3; independent medium review found no defects. Existing load-envelope cases (4), missing-fields case (1) and invalid-source integration case (1) pass. The implementation caught that the last case requires `--test model_contracts`, avoiding a zero-match library invocation. No validator or negative fixture was weakened. Root independently reran the broader PyTorch subsystem: 248/248 pass (`/tmp/pantograph-p04-integrated-backend.log`), closing COV-03.
- Correction to earlier availability statements: collaboration tools do not expose usage, but this session's local rollout logs do contain per-agent `token_usage_record` data. Root read only metadata/usage for this thread's four pilot children. Summed unique response usage agrees with each final thread total. These are implementation-agent totals including its repairs/status turns; they exclude separate medium design/review and root integration. No billed currency is exposed.
- Evidence source: this thread's pilot rollout files under `/home/jeremy/.codex/sessions/2026/09/08/`, temporary extracted summary `/tmp/pantograph-pilot-token-evidence.json`. Cached input is included in total input, so the table separates it from uncached input. Reasoning is a subset of output and is not counted twice. Elapsed time spans session start to last recorded model response, including tools and coordination waits; it is not model-only latency.

| Pilot | Model | Uncached input | Cached input | Output | Observed elapsed |
| --- | --- | ---: | ---: | ---: | ---: |
| P01 cleanup | Luna max | 154,509 | 4,643,072 | 31,623 | 14m 18s |
| P02 discovery | Astra low | 52,120 | 1,100,416 | 5,996 | 4m 43s |
| P03 feature fixture | Luna max | 85,339 | 953,600 | 6,600 | 2m 48s |
| P04 source fixture | Astra low | 30,370 | 363,392 | 1,642 | 1m 37s |

The four-task pilot is now complete. Both models produced accepted bounded work. Astra low used fewer implementation tokens in the two small fixture tasks; differing context, commands and coordination mean this is not a causal benchmark. Total accepted-change cost and a monetary winner remain unknown. Keep Astra low as implementation default, especially for consequential domain/lifecycle changes; retain Luna as a qualified option for tightly specified small repairs, with no demonstrated cost advantage warranting expansion. Stop automatic pilot expansion and continue product work under medium design/review.

## 2026-09-08 — EX-01/EX-03 accepted; backend lifecycle next

- Astra low implemented immediate compatible 1–8 dispatch and scoped event/batch ownership. Strong proofs stay with active execution; weak repository references prevent time-based reclamation while owned, without putting ownership in copied snapshots. Unowned expiry remains. Existing supervision observes branch failure and drains cancellation/shutdown through host settlement; indeterminate work remains fenced without invented resource-release evidence.
- Medium review drove exact-proof authorization, atomic current-membership evaluation/claim/transfer, and terminal settlement of observed failed/deferred attempts independently of scheduler retry. Accepted/indeterminate host responses reject before any member publication. Low implemented all repairs; independent medium final review accepted with no remaining source findings. No higher-effort implementation rescue.
- Full integration exposed six existing session tests whose mandatory single-call grouping contradicted the new immediate policy. Medium admitted one additional test file. Low migrated assertions to bounded groups, exactly-once run/member/attempt identities, and independent submitted-session correlation for error-only APIs, preserving existing behavioral assertions. Root caught and low repaired a helper-reference compilation error. Medium accepted the test migration.
- Root final `cargo test -p pantograph-workflow-service --lib --offline`: **848 passed, 20 failed**, 868 total (`/tmp/pantograph-ex03-final.log`). All five implementation module populations pass: event 52, assignment 22, worker 28, batch 18, session API 8; all six migrated session tests pass. Controlled cases cover long-lived exclusion, wrong/copied proofs, duplicate publication, singleton retained text input, cancellation/drain, panic, retry/defer and indeterminate responses.
- Baseline attribution: root backed up the five current implementation files, temporarily tested their HEAD versions, then restored the exact bytes in `finally`. Baseline: 843 passed, 20 failed, 863 total (`/tmp/pantograph-ex03-head-baseline.log`). Final failure identities exactly equal those 20 baseline failures; COV-05 owns graph validation, technical-fit, classification and session-capacity follow-up. This is scoped acceptance, not a green full-crate claim.
- Root six-file rustfmt and diff checks pass. Final warning-deny Clippy remains 134 library / 149 test diagnostics (`/tmp/pantograph-ex03-final-clippy.log`); zero changed-primary-line hits, and implementation inspection identified no introduced consequence anchored elsewhere. No suppression or unrelated lint repair. Full standards acceptance remains incomplete.
- Next prerequisite is RT-03: backend-owned PyTorch text jobs and awaited fallible stop/replacement, with exact core and current consumer paths in the runtime report. Route/admit that slice before implementation; RT-02 then binds selected Pumas target/model/device and projects text through the canonical host. Non-returning Python/Torch calls must remain pending rather than falsely acknowledge shutdown. Real mixed workflow/model qualification and DA-04 remain unaccepted.
- Protected proposal hashes unchanged. No real models, downloads, external Pumas mutation or commits. The plan remains Active; Astra medium design/review and Astra low implementation remain the default.

## 2026-09-08 — API-dollar evaluation correction

User clarified that API cost matters more than time. The earlier inference from fewer Astra tokens to no demonstrated Luna monetary advantage was inadequate and is superseded here. Verified current [API pricing](https://developers.openai.com/api/docs/pricing) and exact [Astra](https://developers.openai.com/api/docs/models/gpt-6-astra) / [Luna](https://developers.openai.com/api/docs/models/gpt-5.6-luna) pages: Standard USD per million uncached input / cached input / output is Astra $10 / $1 / $50 and Luna $0.20 / $0.02 / $1.20. Fast/priority rates are twice those amounts. These are API-equivalent estimates, not an invoice; rollout usage lacks delivered service-tier/billed-charge fields. Local session metadata reports Pro usage. Show both rates rather than inventing a historical tier.

Rechecked unique per-response records for all four implementation threads. Maximum request input: P01 135,841; P02 60,031; P03 77,167; P04 28,482. No request crosses the 272K long-context threshold; cache-write tokens are zero. Formula: `(uncached_input × input_rate + cached_input × cache_rate + output × output_rate) / 1,000,000`. Output already contains reasoning; do not add reasoning again. All implementation repair/status turns are included.

| Pilot | Implementation model | Implementation Standard USD | Implementation Fast USD | Attributable medium design/review Standard USD | Implementation + attributable design/review Standard USD |
| --- | --- | ---: | ---: | ---: | ---: |
| P01 cleanup | Luna max | 0.161711 | 0.323422 | 1.525448 | 1.687159 |
| P02 discovery | Astra low | 1.921416 | 3.842832 | 1.755962 | 3.677378 |
| P03 feature fixture | Luna max | 0.044060 | 0.088120 | 1.320318 | 1.364378 |
| P04 source fixture | Astra low | 0.749192 | 1.498384 | 3.293032 | 4.042224 |

Review attribution uses task-specific reviewer turns, including full context tokens charged on those requests. P01: desktop reviewer turns starting 02:45:12, 02:50:36 and 02:57:00 UTC September 9. P02: coverage reviewer 02:45:45. P03: coverage reviewer 03:10:23. P04: coverage diagnosis 03:19:16, admission 03:31:45 and desktop final review 03:33:15. Their maximum request inputs are below 272K and cache writes are zero. Initial broad reviews and shared root planning/integration/coordination are not arbitrarily allocated; these are attributable subtotals, not total accepted-change costs. Temporary arithmetic: `/tmp/pantograph-pilot-api-costs.json`; source rollout directory and implementation counts are recorded above.

Implementation totals: Luna $0.205771 versus Astra $2.670608 at Standard rates (about 13× difference for these two different tasks per model); Fast equivalents $0.411541 versus $5.341216. Including attributable design/review gives Luna $3.051537 versus Astra $7.719602 Standard, or twice those subtotals if all calls use Fast. This is useful observed cost evidence, not a same-task causal benchmark. Medium review is a substantial expense and remains part of the quality gate; consolidate coherent reviews and avoid unnecessary repeated context.

Routing correction: prefer Luna max for the qualified bounded repair classes with unchanged medium review and acceptance gates. Keep Astra low provisional for consequential trust/lifecycle/domain changes beyond Luna's tested population; that is a quality-scope decision, not a claim of lower API cost. Future useful comparisons prioritize API dollars including attributable repair/review, with elapsed time secondary. No new benchmark or product implementation is launched by this cost correction.

## 2026-09-08 — RT-03 implementation and continued cost evaluation admitted

- User authorized continuing product work while evaluating delegation by API dollars, explicitly including quality checks and required reviews. RT-03 owns the current evaluation unit; implementation lanes are different task classes, not a controlled head-to-head benchmark.
- One bounded Astra-medium handoff confirmed the existing design and found omitted stop consumers: embedded lifecycle controller traits/reclaim helpers and Tauri recovery/window shutdown. The exact expanded population is admitted in the plan. No failure-swallowing compatibility shim is accepted. Astra low implements core producer/gateway ownership; Luna max implements the specified coordinated consumer/error migration. One integrated medium review will judge the composition.
- Standards Engine route at snapshot `snapshot:v1:356e0989-668e-4a06-bfac-dc7c640f0b4b`: 21 selected standards, no unresolved categories; Rust API/evolution/diagnostics join existing architecture/concurrency/verification authority. Rust interop foreign-memory clauses remain conditional: no raw borrowed-memory mechanism is introduced. Internal coordinated Rust signatures change; public wire formats remain unchanged.
- Before RT-03 edits, root full inference defaults+PyTorch serialized library baseline: 663/663 pass (`/tmp/pantograph-rt03-baseline-inference.log`). Embedded library baseline: 407 pass / 27 fail (`/tmp/pantograph-rt03-baseline-embedded.log`); retain failure identities for introduced-regression attribution. These runs executed no real models.
- Cost population: dedicated `rt03_design`, `rt03_core`, `rt03_consumers`, subsequent integrated reviewer and all follow-up repair turns, plus root turn `01a08481-1c39-7d82-920b-bc6a259055cd` from 04:50:49 UTC September 9. Root integration/verification/orchestration is charged to RT-03 shared overhead rather than omitted or arbitrarily assigned to a model. Count each recorded response once at verified rate assumptions; disclose publication cutoff and unavailable billing/tool charges. No separate quality-check work is treated as free. Temporary checkpoint: `/tmp/pantograph-rt03-cost-checkpoint.json`.

## 2026-09-08 — RT-03 accepted; complete recorded cost population

- Owned PyTorch text jobs now retain producer completion for streaming and nonstreaming generation. Receiver loss requests cooperative cancellation; bounded-channel sends respond to cancellation; successful done follows observed producer completion. Ordinary loads/unloads drain accepted work; explicit stop/switch cancels and awaits it. The existing backend guard owns lifecycle state publication. Removed the unused free Transformers unload bypass after medium confirmed no consumers. Non-returning Python/Torch steps remain pending, not falsely cancelled or reusable.
- Astra low core implementation passed its initial focused checks but required stronger production Python iterator evidence at root integration. That fixture now exercises successful load/unload/stop ordering through actual Rust adapters and controlled Python effects, plus pre-load producer failure and effectful replacement failure. Medium review found premature loss of retained metadata; the first repair was too broad because Python can unload before replacement fails. Final repair preserves pre-effect state and clears readiness/model metadata before effectful load. Medium rereview accepted. Root static checks also required changed-line formatting and one private stream type alias. No higher-effort model implemented the repairs.
- Luna max migrated the enumerated callers and added embedded/Tauri registry and recovery failure tests. Medium found stop_llm returned before reconciliation; Luna fixed it. Root caught one invalid async closure during migration, one unused test mock, an incorrect new Ready-versus-Failed expectation (canonical observation prioritizes retained active residency), and changed-line formatting; Luna repaired these without weakening typed failure/identity/error checks. Repeated unanswered checkpoints led to one interruption/resumption and root taking integration build ownership. This coordination cost is included, not hidden as free effort.
- Final core semantic checks: root inference defaults+PyTorch library **671/671** (`/tmp/pantograph-rt03-final-inference.log`). PyTorch-only library **650/650** passed before the reviewed metadata repair; the repair's production fixture and final default+PyTorch suite passed afterward. Root embedded **408 passed / 27 failed**, with failure identities exactly equal to its pre-change **407/27** baseline (`/tmp/pantograph-rt03-embedded-candidate.log`). Node-engine affected core executor **55/55**. Desktop test compilation with `--no-default-features --features backend-llamacpp,backend-pytorch` passes; recovery gate **1/1**, final runtime-registry tests **12/12** (`/tmp/pantograph-rt03-recovery-test.log`, `/tmp/pantograph-rt03-final-registry-tests.log`). Candle trait source migrated; CUDA/default-Candle runtime validation is not claimed.
- Final static evidence: private helper rustfmt passes; comparing rustfmt edits with added-line scopes across affected files leaves **zero introduced formatting findings**, preserving unrelated baseline formatting. Diff whitespace passes. Inference warning-deny Clippy is back to **54 library / 55 test** baseline diagnostics with zero changed-line hits (`/tmp/pantograph-rt03-final-clippy-inference.log`). Embedded Clippy reports **11 library / 38 test** diagnostics, zero changed-line hits (`/tmp/pantograph-rt03-clippy-embedded.log`). Full repository standards/static acceptance remains pending under COV-04/05/06; no suppressions or unrelated cleanup.

API-equivalent estimates below include every recorded response in the dedicated implementation, design and review threads, all repairs/checks in those threads, carried RT-03 design, and this root integration/verification/orchestration turn, through **2026-09-09T05:23:19.943Z**. Standard/Fast USD rates and formula are established in the preceding pricing entry; cached and reasoning tokens are not double-counted. These are not invoiced charges. Historical mixed RT-02/RT-03 discovery outside the identified design turn remains unallocated program overhead; no false all-history total is claimed. Current reporting calls after this cutoff will be included at the next checkpoint. Local shell/Cargo runtime has no separately observed API tool charge; API calls that request and interpret those checks are included.

| Cost owner | Uncached input | Cached input | Output incl. reasoning | Standard USD | Fast USD |
| --- | ---: | ---: | ---: | ---: | ---: |
| Astra medium integrated review + rereviews | 48,142 | 800,512 | 2,677 | 1.415782 | 2.831564 |
| Astra medium bounded design | 25,934 | 370,432 | 2,611 | 0.760322 | 1.520644 |
| Luna max consumers + repairs/checks | 370,955 | 16,704,000 | 54,732 | 0.473949 | 0.947899 |
| Astra low core + repairs/checks | 79,533 | 4,329,728 | 20,246 | 6.137358 | 12.274716 |
| root integration/verification/orchestration | 206,719 | 18,376,576 | 27,818 | 21.834666 | 43.669332 |
| carried RT03 design | 11,227 | 1,352,960 | 6,282 | 1.779330 | 3.558660 |
| **Recorded total** | | | | **32.401407** | **64.802815** |

Evidence: source rollout metadata/usage under the same root session and dedicated RT-03 agent paths, exact turn IDs/cutoffs in `/tmp/pantograph-rt03-costs.json`; temporary aggregation `/tmp/pantograph-rt03-costs.py`. Each response ID is counted once. All counted requests are below the 272K long-context threshold and recorded cache writes are zero. Neither an implementation-only subtotal nor an arbitrary split of shared root/reviewer calls is used as total accepted-change cost.

Delegation finding: Luna completed the larger coordinated consumer migration under explicit design and unchanged review gates at low direct API cost; it required the listed review/test/static corrections and coordination recovery. Astra low completed the more consequential producer/lifecycle core with the listed acceptance and repair work. The task classes differ, so this is not a causal head-to-head ranking. Prefer Luna for enumerated consumer migrations whose outcome/error contracts are settled; retain Astra low provisionally for consequential new ownership mechanisms. Root overhead dominates this slice: consolidate check/result packets and bounded context, avoid schema/report dumps and unnecessary status loops, while preserving needed independent review. Track regressions against these task classes in future product work.

RT-03 is accepted only for its owned/cooperative termination and observed failure contract. RT-02 selected-target/model/device execution and canonical text host projection are next; real mixed desktop runs, non-cooperative bounded termination, full audit and full static compliance remain pending. Protected Pumas proposal hashes are unchanged. No real models, downloads, external Pumas edits or commits occurred.

## 2026-09-08 — RT-02 selected text execution admitted

- Medium confirmed the existing design against accepted RT-03 source. Exact disjoint inference/host write sets and shared gateway API are admitted in M2; Astra low owns inference, Luna max owns the settled host projection. Existing Pumas-target conversion may be exposed for reuse without changing image semantics. Cargo ownership is serialized; independent medium integrated review remains required.
- Standards HEAD remains `366c1d90a24bbfb50973f62b155a5f3396c0f107`; the unrelated untracked proportionality prototype is excluded. Engine snapshot `snapshot:v1:fa0d36ff-66ff-4516-8b52-0649fbd96b75` resolves 25 standards with zero unresolved facts, covering affected Rust API/async/security/interop, selection/path/trust contracts, architecture, diagnostics, implementation and verification. No wire/persistence or external Pumas change is admitted.
- Cost population is this root turn and dedicated `rt02_*` threads, including cost extraction, all checks, review and repairs. Shared root/design/review costs remain separate from implementation lanes; prior RT-03 reporting tail is carried separately when observable. Estimates retain the ledger's Standard/Fast pricing assumptions, response-ID deduplication and explicit cutoff limitations.

- Integrated medium review found that EX-01/03 routes singleton groups through the batch host. RT-02 therefore includes sequential text member execution there; a further review repair preserves producer/cleanup failures instead of overwriting them when cancellation arrives. Luna's host handoff lacked canonical retained-result evidence; Astra low owns the remaining host integration, tests and repairs. One additional admitted Candle-only test gate enables the requested no-Candle matrix.
- The dependent runtime fixture exposed EX-04: session_scheduler_runner's all-Ready check, event selection independent of scheduler readiness, and runtime_branch_batch_execution's premature whole-run finalization form one coherent continuation defect. Medium identified the four existing workflow owner files and a scheduler-ready/proof-gated continuation contract retaining the run responder through task-result storage and downstream advancement. This is a separate lifecycle slice, not a host-only repair; RT-02 acceptance cannot establish dependent text→image execution. No expected-bug assertion substitutes for that acceptance.

## 2026-09-08 — RT-02 scoped acceptance; EX-04 next

- Independent medium review accepts selected-text execution with scheduler-selected model/runtime/device and a separately validated local Pumas directory target. Logical package facts remain unchanged; the actual Transformers envelope uses the executable target while preserving model/artifact identity. Known custom code, mismatched identities/placement and file targets reject before effects. Existing backend ownership spans replacement, selected loading, terminal stream observation and cooperative cleanup. Errors/premature EOF/cancellation produce no completed text. A production Rust→controlled Python fixture exposed and repaired PyTorch's string-only message decoding so typed text-part arrays preserve the actual prompt.
- Canonical singleton service→scheduler batch→embedded host→gateway execution now proves exact prompt, completed run status, retained text artifact and retrieved artifact bytes. Sequential text batches preserve per-member identity/outcomes, exact 1024-byte Unicode output, visible oversize rejection, producer errors and cleanup failures concurrent with cancellation. Text requires no image sink. Existing image single/batch/sink regressions pass; the owner-Pumas image fixture still fails on its existing read-only database condition before host execution. Real models and real Pumas text-target qualification are not proven.
- Final `cargo test -p inference --features backend-pytorch --lib -- --test-threads=1`: **675/675** (`/tmp/pantograph-rt02-inference-final.log`). Final `cargo test -p pantograph-embedded-runtime --no-default-features --features backend-llamacpp,backend-pytorch --lib -- --test-threads=1`: **415 passed / 27 failed** (`/tmp/pantograph-rt02-embedded-final.log`). Root independently compared failure identities with RT-03: exactly the same 27, no additions/removals. Eight new host tests pass, including the retained singleton proof. The one Candle-specific test remains enabled only with its feature.
- Warning-deny Clippy with `--lib --tests --no-deps --message-format=json -- -D warnings` remains inference **54 lib / 55 test**, embedded **11 lib / 38 test**, with no introduced primary-span findings against the pre-slice snapshots. Evidence: `/tmp/pantograph-rt02-clippy-deny.{log,jsonl}` and `/tmp/pantograph-rt02-embedded-clippy.{log,jsonl}`. Introduced-code formatting and root scoped whitespace checks pass. Full static/repository compliance remains open. Protected proposal hashes match the user-provided hashes. No commits, real-model runs, downloads or external Pumas modifications occurred.
- EX-04 remains the next separate lifecycle slice: the dependent fixture revealed connected readiness, event-selection and premature run-finalization defects. The existing plan now records its coherent source set and continuation contract. A temporary unaccepted dependent fixture patch is retained at `/tmp/pantograph-ex04-dependent-fixture.patch`; it is not a passing acceptance test.

API-equivalent cost through **2026-09-09T06:14:18.172Z**, using the preceding ledger pricing assumptions (USD, not invoices):

| Cost owner | Uncached input | Cached input | Output incl. reasoning | Standard USD | Fast USD |
| --- | ---: | ---: | ---: | ---: | ---: |
| Root integration/verification/orchestration (Astra low) | 145,695 | 6,175,232 | 13,867 | 8.325532 | 16.651064 |
| Astra medium admission | 63,732 | 596,352 | 3,340 | 1.400672 | 2.801344 |
| Luna max cost extraction (shared) | 70,472 | 865,024 | 21,806 | 0.057562 | 0.115124 |
| Astra low inference + host rescue/checks/repairs | 338,568 | 13,287,168 | 39,606 | 18.653148 | 37.306296 |
| Luna max host implementation/checks/handoff | 467,666 | 15,320,320 | 60,380 | 0.472396 | 0.944791 |
| Astra medium review/repairs review + EX-04 design (shared) | 229,089 | 5,157,760 | 9,856 | 7.941450 | 15.882900 |
| **RT-02 session total** | | | | **36.850760** | **73.701519** |

The prior RT-03 reporting tail is separately carried at **$0.830548 Standard / $1.661096 Fast**, not charged to either RT-02 implementation lane. All dedicated RT-02 thread responses and the current root turn are included, deduplicated by response ID; cached input is part of input and reasoning is part of output. No counted request exceeds 272K input and recorded cache writes are zero. Delivered service tier and invoice/tool charges are unavailable; Standard/Fast remain estimates. Shared review includes EX-04 discovery/design and is not arbitrarily split into implementation costs. The cost helper itself is counted. Source metadata/turn IDs and cutoff are in `/tmp/pantograph-rt02-costs.json`, reusable aggregation `/tmp/pantograph-rt02-costs.py`, checkpoint `/tmp/pantograph-rt02-cost-checkpoint.json`; subsequent final reporting calls are outside this cutoff.

Delegation finding: Luna's direct host cost remained small, but the lane did not deliver canonical workflow evidence before handoff; Astra low completed the consequential batch/retention integration and repairs. Initial admission also missed the canonical singleton-through-batch path, and downstream EX-04 defects enlarged shared analysis. These task classes and the rescue are not a controlled model comparison; an implementation-only Luna subtotal is not an accepted-change cost. Root used 98 recorded responses and 6,175,232 cached input tokens through this cutoff. Root dollar overhead is lower than RT-03's recorded subtotal, but frequent waiting/reporting still created avoidable coordination overhead. Continue medium review and bounded context; settle canonical entrypoints before delegating consumer projection, and transfer consequential unresolved integration promptly rather than repeating checkpoints.

## 2026-09-08 — EX-04 continuation and Spark COV-02 evaluation admitted

- User routing now assigns GPT-5.3 Codex Spark to real small, well-defined changes/repairs, Luna max to larger enumerated settled changes, Astra low to complex implementation/integration/rescue, and Astra medium to consequential design and substantive independent review. Spark delegation is available and has completed the four-literal COV-02 packaging repair candidate; actual model usage and any unavailable pricing are recorded at closure. Root and shared design/review remain part of cost accounting.
- Medium confirmed EX-04's five-file write set and canonical path. Astra low owns four workflow-service lifecycle files; Luna max owns the existing embedded workflow fixture file. Scheduler progress selects a Ready task/proof before exact run/task event claim; task completion becomes owned continuation until all tasks complete. The same responder and existing EX-03 proofs must survive cross-run batches and shutdown. No detached task, new wire schema or premature whole-run finalizer is admitted. Standards HEAD remains `366c1d90a24bbfb50973f62b155a5f3396c0f107`; EX-04 Engine snapshot `snapshot:v1:d239bd87-b960-4a38-8cc1-4d3e7e9e27d6` resolves 13 standards with zero unresolved categories. Exact scopes/contracts live in M2.
- COV-02 has an independent single-file write set and Engine route `snapshot:v1:1356b67e-26ae-4c5b-9e65-6e3e8f9ef58e`. Root verified that the candidate changes exactly four literals, its isolated smoke uses the actual script, and both archived manifests resolve to the exact live guide bytes. Stub compilation/generation does not establish native binding or release acceptance. Medium final review remains required.
- Cost population is this root continuation turn and dedicated `ex04_*` threads, including costs, checks, repairs and independent review. COV-02 is distinct from EX-04 implementation; shared coordination/design/review is not arbitrarily allocated between them. Carry the previous RT-02 reporting tail separately where observable, without counting the prior RT-03 tail again. No artificial benchmark work or per-agent documentation is introduced.

- COV-02 accepted by independent medium review: Spark changed exactly four guide-path literals; isolated actual-script packaging smoke, both archived manifest guide-byte checks and checksums pass (`/tmp/cov02-evidence.log`). User subsequently authorized standards-aligned milestone commits; route additionally includes `workflow.commit`, staged-scope review and conventional atomic messages. Protected proposals and unrelated changes remain excluded.
- A second proposed Spark test migration was invalid: core/root misidentified a diagnostic-detail failure as an event-state assertion change. Spark changed scheduler state instead, root caught it, and Spark restored the exact baseline bytes. No test migration remains; core preserves the original pending diagnostic. All attempted/revert/coordination costs stay counted, not presented as an accepted Spark repair.

- User explicitly authorized history rewriting to add missing explanatory commit bodies. Reworded only the three new local commits, preserving every tree/author/order, generating valid signatures, and retaining `refs/heads/archive/ex04-before-commit-bodies-20260909` at the old tip. Mapping: packaging `fc230c6f`→`b0bf3d1d`; tooling `700d3091`→`a30ad209`; worker fixture `16cd532a`→`773478f0`. Index bytes were verified unchanged; no worktree files were replaced. No remote/shared refs contained these local commits. New milestone commits require explanatory bodies.
- Accepted runtime prerequisites RT-01/EX-03/RT-03/RT-02 committed together as `be56e50b`, after independent staged-scope review. All 49 staged paths matched the admitted accepted population: 44 unchanged current prerequisite files, five pre-EX04 snapshot versions. No EX-04 changes leaked into this prerequisite milestone; current worktree versions remained intact. Protected proposals and unrelated media-sink attribute formatting were excluded. Tooling's eight focused regressions were rerun successfully with subprocess execution enabled after sandbox EPERM; the worker fixture retains its prior accepted contract/full-inference evidence.

## 2026-09-09 — EX-04 accepted; milestone commits and complete-cost checkpoint

- Independent Astra medium review accepts EX-04 under its controlled integration contract. Scheduler progress selects a Ready task with dependency proof before claiming the exact run/task event. Completed task/assignment/event settlement precedes an owned continuation retaining the same supervised run responder. Only all-complete graphs finalize. Completed upstream tasks are validated and skipped during composed readiness recovery; inconsistent event/task state still rejects.
- Review exposed a cross-run loss: an unrelated batch member's disconnected completion receiver could discard successful sibling continuations. Astra low separated delivery loss from structural registry errors and added a real continuation regression. Pure batch finalization no longer retains unnecessary async/host parameters. Missing pending-task diagnostic detail was repaired rather than weakening the established consumer test.
- Five added embedded cases pass through the public canonical path: dependent text→text retention; exact generated text consumed by image execution with retained reference→binary linkage; original caller remaining pending behind a deterministic downstream gate; explicit controlled producer retryable failure with no downstream execution; and composed Stale→Fresh readiness recovery in the same run without upstream replay or duplicate recovery execution. These use controlled backends/resolvers and valid fixed PNG bytes, not real model/Pumas/device/desktop qualification.
- Luna provided the initial larger fixture population but needed batch-capability/output-target and stronger evidence repairs. A repair message sent to an already completed agent did not resume it; explicit followup corrected that coordination error. The later bounded repair lane still lacked a complete handoff, so Astra low took sole fixture ownership, completed gate/error/retention/recovery repairs and ran final checks. Node-status/media projection assumptions were replaced with actual scheduler-timeline identities and retained artifact linkage. All attempts, waiting/coordination and rescue are counted.
- Final workflow command `cargo test -p pantograph-workflow-service --lib --offline`: **852 passed / 20 failed** (`/tmp/pantograph-ex04-workflow-final.log`), exactly the prior 20 failure identities. Final embedded command `cargo test -p pantograph-embedded-runtime --no-default-features --features backend-llamacpp,backend-pytorch --lib -- --test-threads=1`: **420 passed / 27 failed** (`/tmp/pantograph-ex04-embedded-final.log`), exactly the prior 27. Root independently compared both failure sets. Five dependent cases pass in `/tmp/pantograph-ex04-embedded-dependent-repaired.log`.
- Affected warning-deny Clippy with `--lib --tests --no-deps -- -D warnings` stays workflow **134 lib / 149 test**, embedded **11 lib / 38 test**; diagnostic path/message populations and counts are unchanged, with no suppressions. An initial dependency-inclusive command stopped at the existing managed-dependencies borrow lint; the package-scoped command supplies the affected evidence. Introduced formatting and whitespace checks pass. Exact commands/comparisons are in `/tmp/pantograph-ex04-core-verification.json`; formatting comparison `/tmp/pantograph-ex04-format-check.json`. Full clean-suite/static acceptance remains open.
- EX-04 committed as `7941e428` after source, verification and staged-scope review of exactly eight frozen files. Accepted prerequisite commits are `b0bf3d1d` (COV-02), `a30ad209` (tooling), `773478f0` (worker fixture), and `be56e50b` (owned selected runtime execution). All have explanatory bodies and valid signatures. The authorized message-only rewrite mapping and retained archive ref are recorded above. Current plan/issue/evidence publication is a single documentation milestone, not reconstructed historical plan versions. Protected proposals and unrelated media-sink formatting remain outside commits. No real-model runs, downloads or external Pumas edits occurred.

API-equivalent **known-rate subtotal** through **2026-09-09T07:10:30.184Z** uses the ledger’s recorded Standard/Fast USD assumptions. It is not an invoice or a complete dollar total: Spark and the opaque approval-review model have no verified rates.

| Cost owner | Uncached input | Cached input | Output incl. reasoning | Standard USD | Fast USD |
| --- | ---: | ---: | ---: | ---: | ---: |
| Root coordination/verification/commits (Astra low) | 101,711 | 15,852,672 | 34,146 | 18.577082 | 37.154164 |
| Astra medium design/routing/recovery decisions (shared) | 98,811 | 2,415,360 | 5,796 | 3.693270 | 7.386540 |
| Spark low packaging + misrouted repair/revert | 140,317 | 1,302,528 | 15,129 | unknown | unknown |
| Luna max cost bookkeeping (shared) | 274,346 | 8,989,184 | 50,593 | 0.295364 | 0.590729 |
| Astra low implementation, fixture rescue and checks | 225,730 | 20,885,376 | 49,985 | 25.641926 | 51.283852 |
| Luna max initial fixtures and repair attempts | 649,676 | 18,437,376 | 73,214 | 0.586540 | 1.173079 |
| Astra medium independent review/rereviews | 131,007 | 6,446,848 | 8,271 | 8.170468 | 16.340936 |
| Astra medium commit preparation (shared) | 38,336 | 472,320 | 3,327 | 1.022030 | 2.044060 |
| Root approval review (unknown model/rate) | 27,448 | 47,360 | 635 | unknown | unknown |
| **Known-rate subtotal** | | | | **57.986680** | **115.973360** |

Prior RT-02 reporting tail **$0.386194 Standard / $0.772388 Fast** and interstitial shared cost Q&A **$0.110256 / $0.220512** are separately carried; the previously carried RT-03 tail is not counted again. Deduplicated response IDs cover all dedicated children, all root continuation steering/commit work since the pinned EX-04 start, and observed root approval-support calls. Cached input is included in input, reasoning in output; no counted request exceeds 272K and recorded cache writes are zero. Delivered service tier/invoice charges are unavailable. Temporary source selection/usage evidence: `/tmp/pantograph-ex04-costs.json`, checkpoint `/tmp/pantograph-ex04-cost-checkpoint.json`, reusable `/tmp/pantograph-ex04-costs.py`. Final publication calls after this cutoff remain a reporting tail.

Spark evaluation: actual rollout model is `gpt-5.3-codex-spark` with low effort. Its four-literal packaging fix was accepted after an actual-script archive smoke and independent review; the initial proposed direct-copy smoke was strengthened before implementation. Its second attempted repair was based on a mistaken handoff and changed the wrong state; the entire test edit was reverted, so it is not another accepted change. That thread’s combined attempted/accepted work remains one explicit cost population. Spark pricing is unverified; base GPT-5.3 Codex pricing is not substituted, and unknown USD is not zero.

Delegation finding: this session supports one small accepted Spark repair, not a savings claim or broad qualification. Luna delivered initial enumerated fixtures but Astra integration/rescue was necessary for the deciding evidence. Shared design, substantive review, coordination failures, historical milestone commit preparation and user-authorized message correction are explicit shared costs, not arbitrarily assigned to a model’s implementation subtotal. Different work classes and the unpriced models prevent a controlled dollar comparison. Continue the user’s four-model policy; send explicit followup tasks to resume completed agents and transfer unresolved consequential integration promptly. Do not infer a cheaper accepted result from the small direct Luna subtotal or an unknown Spark rate.

## 2026-09-09 — Local branch cleanup

- Removed fully merged `fix-59d9eb7-clean` (`4cb8a763dceb44b396f0ffbc8be122bed930728e`). Retired the temporary commit-message backup branch after preserving its exact tip `16cd532a46087ee3643deb234a85786367683f68` as `refs/tags/archive/ex04-before-commit-bodies-20260909`. The three original trees match their recorded accepted replacements on main; no commits were discarded. Only `main` remains as a local branch. Main's tip and the existing index/worktree state were verified unchanged by cleanup; remote refs and the detached worktree were untouched.

## 2026-09-09 — D-02 complete-image projection admitted

- Standards HEAD remains `366c1d90a24bbfb50973f62b155a5f3396c0f107`. Medium source review confirms the desktop report's repair contract: image reads request the complete existing body, text previews retain their 64 KiB bound, and incomplete image responses from Read or Read Stream reject before display. Inspector owns decode errors, object URL replacement/disposal and stale read completion; Download retains its existing behavior.
- Exact production write set: `src/components/workbench/ioInspectorPresenters.ts`, `src/components/workbench/ioInspectorPresenters.test.ts`, and `src/components/workbench/IoInspectorPage.svelte`. Existing `tests/e2e/workflow-editor-image-generation/workflow-editor-image-generation.e2e.mjs` is admitted only for actual decode/dimension assertions. Independently admit `tests/e2e/io-inspector-image-preview/run.mjs` and `fixture.ts` for a controlled real-WebKit component regression with a valid image larger than 64 KiB and failure/lifecycle cases. No artifact storage, backend, dependency or external Pumas changes are admitted. Root owns the existing plan, issues and ledger.
- Luna max implements the settled production contract; Sol medium owns the disjoint browser fixture/checks; Astra medium owns admission and substantive independent review. Costs include each implementation/check/repair lane plus shared design, review, root integration, bookkeeping and approval support. Dedicated `d02_*` rollout responses and this root implementation turn form the population; prior reporting/cleanup tails remain separate. Synthetic browser evidence cannot establish real-model mixed-workflow acceptance.

- Canonical manual route: implementation, verification, development proportionality and commit; frontend, TypeScript/async; contracts, concurrency, diagnostics and accessibility; GUI/oracle/platform verification details, with their Requires. Root control-file edits also select planning/documentation. The executable Router reports unavailable runtime dependencies in this environment; no installation or policy change was made. Source review preserves consumed IPC/persistence semantics without a schema migration.


## 2026-09-09 — D-02 accepted: complete Inspector images

- Luna max implemented full image requests using the card's MIME/payload-kind/format classification while preserving 64 KiB text reads. Inspector rejects incomplete image Read and Read Stream responses, reports decode failures, preserves replacement images against stale events and invalidates pending reads across refresh, selection and unmount. Download's existing body/anchor/timer path is unchanged. No storage, IPC, runtime, dependency or external Pumas changes occurred.
- Independent Astra medium review found and verified repairs for response MIME overriding the image-card completeness gate, decode errors suppressed during a pending refresh, failed inspection refresh leaking removed preview URLs, and reactive URL capture in stale image events. Sol medium added deciding browser regressions. The initial harness printed PASS but did not exit because signal termination leaves Node's exitCode null; root identified the erroneous second exit wait. Sol repaired owned process-group/session cleanup and removed positively identified earlier harness processes. Functional PASS alone was not accepted as terminal verification.
- Final independent `node tests/e2e/io-inspector-image-preview/run.mjs`: **exit 0**, all assertions pass, cleanup complete. Linux WebKitGTK MiniBrowser and WebKitWebDriver run on DISPLAY `:0.0`, with an owned loopback Vite server and controlled service responses/subscription replacement. The actual Inspector reads the exact payload ID without ranges and decodes a browser-generated **360,210-byte PNG at 320×320**. Tests cover malformed image, partial image Read/Stream including octet-stream response MIME, read failure, bounded text, current/detached image errors, pending refresh rejection, failed-inspection cleanup, stale read after removal and completion after unmount. Browser-owned URLs and test processes are cleaned up. This is controlled component/browser evidence, not retained-store, Tauri IPC, real model or DA-03 acceptance.
- Presenter tests and final `npm run typecheck` pass. Luna's frontend suite reports **540 passed**; targeted ESLint, e2e syntax and whitespace checks pass. Sol's `npm run build -- --mode test` passes. Full lint retains pre-existing PumaLib writable-derived, Download DOM-mutation and accessibility-review diagnostics; independent review confirmed Download unchanged. No Cargo checks were needed. The existing real-model smoke now requires decode and positive dimensions but was not run. Protected proposal hashes and unrelated embedded media-sink formatting remain untouched.
- The next slice qualifies the actual Pumas executable targets, models/runtime/device and desktop mixed-workflow fixture. Full audit/static compliance and required-real acceptance remain open.

API-equivalent known-rate checkpoint through **2026-09-09T18:06:37.401Z**, USD (not invoices):

| Owner | Uncached input | Cached input | Output incl. reasoning | Standard USD | Fast USD |
| --- | ---: | ---: | ---: | ---: | ---: |
| Luna max implementation/checks/repairs | 497,513 | 10,078,464 | 47,447 | 0.358008 | 0.716017 |
| Sol medium browser implementation/checks/repairs | 134,051 | 10,926,976 | 31,017 | 5.527334 | 11.054669 |
| Astra medium admission (shared) | 69,755 | 480,896 | 3,262 | 1.341546 | 2.683092 |
| Astra medium independent review | 84,829 | 5,567,744 | 6,866 | 6.759334 | 13.518668 |
| Approval review (unknown rate) | 123,775 | 1,056,000 | 2,559 | unknown | unknown |
| Astra low root integration/coordination (shared) | 124,579 | 8,678,400 | 12,441 | 10.546240 | 21.092480 |
| Luna max bookkeeping (shared) | 348,909 | 5,883,136 | 37,357 | 0.232273 | 0.464546 |
| **Known-rate subtotal** | | | | **24.764736** | **49.529471** |

Current official [API pricing](https://developers.openai.com/api/docs/pricing) and exact Astra/Luna/Sol model pages confirm Standard input/cached/output rates per million of Astra $10/$1/$50, Luna $0.20/$0.02/$1.20 and Sol $4/$0.40/$20; Fast is 2×. Above 272K input the entire request uses 2× input/cache and 1.5× output rates; cache writes are 1.25× ordinary input. No counted request crosses that threshold; recorded cache writes are zero. Cached tokens are a subset of input and reasoning a subset of output, neither added twice. Actual delivered service tier, invoice charges and approval-review rates remain unavailable; unknown charges are not free.

Selection: root thread `01a084a6-36af-78c2-a6c8-6c0b4c2f9b77`, current root turn `01a0873a-4600-7da1-882d-7bf4cd71731d` (user marker `continue with implementaiton`), dedicated `d02_*` descendants and observed approval support sharing that root turn. Deduplicate `token_usage_record` by response_id. The checkpoint includes 473 responses (31 unpriced approval requests). Prior EX-04 post-cutoff root reporting/cleanup is carried separately at **$4.130504 Standard / $8.261008 Fast**, with its approval requests unpriced; no earlier carried tail is charged again. Historical non-root tails outside this selection are not claimed complete. Local rollout paths, response IDs, model/effort/tier, counts, maximum request sizes and exact selection are in `/tmp/pantograph-d02-costs.json`; rerun `python3 /tmp/pantograph-d02-costs.py --output /tmp/pantograph-d02-costs.json`. Final review/publication/commit calls after this explicit checkpoint remain a reporting tail for the next accounting update.

Delegation result: Luna delivered this settled production repair without Astra implementation rescue, but substantive medium review found material lifecycle defects. Sol delivered a useful independent browser fixture and repaired its own lifecycle failures; this is a different task class, not a controlled Luna/Sol comparison. Keep medium review and charge every repair/check to its lane. The largest known component remains shared root coordination ($10.546240 at 93 responses); waiting and repeated status communication still create substantial overhead. Small direct Luna dollars do not establish the complete accepted-change cost. This slice provides one accepted integrated outcome at the combined subtotal, not seven separately accepted model changes. Retain the reusable cost extractor to avoid repeating bookkeeping construction; no new repository tracking framework was added.

## 2026-09-09 — Real-pair qualification blocked by owner facts

- Astra medium admitted an evidence-first qualification slice with no production write set. Canonical manual routing selected Core/Router; Planning, Implementation, Verification, Development Proportionality and Documentation; and GUI/platform verification. The standards checkout remains exactly `366c1d90a24bbfb50973f62b155a5f3396c0f107` except its previously excluded untracked proportionality prototype. The executable Router was not run and no executable-routing success is claimed.
- Read-only inspection of `/media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pumas-Library/shared-resources/models/models.db` against pinned Pumas `f87c3da8276a914a54c6f4f36d617bef9d9f424e` found no presently qualified text/image pair. Tiny SD is the only identified built-in pipeline candidate in the inspected library, but its contract-v3 package facts contain `diffusers_component_reference_invalid` for `feature_extractor`. Cached text candidates use retired package-facts contract version 1; their filesystem paths and old `valid` labels are not current executable-target authority. Pinned Pumas already owns `OwnerFresh` resolution; Pantograph must not guess paths or treat cached rows as readiness. No external Pumas source or state was changed.
- The selected `.venv` imports Torch 2.10.0+cu128, Transformers 4.52.4, Diffusers 0.37.0, Accelerate 1.12.0, Safetensors 0.7.0 and Pillow 12.1.1. Torch reports no CUDA/MPS device; the host had 59,592,716,288 available bytes during the probe. These are environment facts, not model compatibility or CPU-budget acceptance. Desktop tools and DISPLAY are present, but no mixed saved fixture exists and the image-only Tiny SD graph retains stale ports.
- Next evidence is freshly hydrated package facts plus owner-fresh typed load-target outcomes for one compatible text model and Tiny SD, followed by actual selected-runtime/device execution. OwnerFresh refreshes external-asset state before indexed resolution; it does not itself regenerate retired package-fact caches. Only after those prerequisites may the existing GUI wrapper, WDIO timeout/config and scenario be admitted. DA-03/DA-05, real retained outputs, desktop IPC, cleanup under real inference and full static/audit acceptance remain blocked. Temporary `/tmp` probe construction attempted to reuse the pinned Rust API but its dependency build required an unavailable ORT download; it provides no acceptance evidence.
- API-equivalent known-rate checkpoint through `2026-09-09T18:30:00.314Z`: Astra medium admission **$1.323140 Standard / $2.646280 Fast** (12 responses); Astra medium independent review in progress **$0.398986 / $0.797972** (6); root Sol low qualification/integration **$1.762256 / $3.524512** (34). Combined **$3.484382 / $6.968764**. This is not an invoice; final review/publication/commit calls remain a tail and approval-review/tool charges remain unknown, not zero. The selection is new root thread `01a08768-669b-7e03-ab8b-8d95d5583893`, root turn `01a08768-7d08-7853-8176-1b2978318d0a`, and `/root/qualification_*` descendants, deduplicated by `response_id`; `/tmp/pantograph-qualification-costs.json` retains the exact population. The prior D-02 checkpoint and separately carried EX-04 tail are not counted again.

## 2026-09-09 — Pinned Pumas owner qualification produced no executable pair

- A bounded probe compiled against pinned Pumas `f87c3da8276a914a54c6f4f36d617bef9d9f424e` and opened an isolated `/tmp` owner library containing full copies of Tiny Aya and Tiny SD. This avoided the live library, global registry, orphan adoption, watcher and download-recovery effects of stock `pumas-rpc`. The only network action downloaded Cargo's mandatory pinned ORT build dependency after approval. No live Pumas model/index/source state or Pantograph production source changed.
- `resolve_model_package_facts` hydrated Tiny Aya to contract v3 with `Cohere2ForCausalLM`, `CohereTokenizerFast`, no required custom code, text→text task facts and a valid 6,719,899,482-byte component layout. Its typed OwnerFresh response reports Ready/Ready and `load_path_kind: directory`, but `local_load_path` names `model-00001-of-00002.safetensors`; that is not an executable Transformers directory target.
- Tiny SD hydrated to contract v3 as a 2,419,218,923-byte Stable Diffusion bundle without required custom code, while retaining `diffusers_component_reference_invalid` for `feature_extractor`. Its typed OwnerFresh response is Missing/Missing with `artifact_missing`: the selected artifact path in the returned model reference is not present in indexed package facts. No path fallback or local correction is admissible in Pantograph.
- These isolated outcomes qualify the copied bytes and pinned owner behavior only; they do not establish live-library readiness, model inference, CPU performance, retained outputs, desktop IPC or DA-03/DA-05. Runtime/device and GUI-harness work remain blocked until the Pumas-owned target defects are repaired and the canonical owner qualification passes. The protected proposals already describe this external dependency area and remain unchanged.
- Updated cumulative API-equivalent checkpoint for this root turn through `2026-09-09T18:41:35.810Z`: Astra medium design/admission **$1.323140 Standard / $2.646280 Fast** (12 responses); Astra medium independent reviews **$0.967394 / $1.934788** (12); root Sol low qualification/integration **$2.384574 / $4.769149** (43). Combined **$4.675108 / $9.350217**. This supersedes, rather than adds to, the earlier same-root checkpoint. Final publication/commit remains a tail; approval-review, tool and invoice charges remain unknown. Exact deduplicated evidence is `/tmp/pantograph-pumas-owner-costs.json`.

## 2026-09-09 — Current Pumas consumer comparison supersedes external-fix prerequisite

- User clarified that Pantograph's `f87c3da` pin predates substantial Pumas changes. Read-only comparison with local committed Pumas `b7eba4ce01a45cf540659fff7d04fc913d6a3d0e` (97 newer commits) replaces the assumption that external fixes must precede all Pantograph work. Existing Pumas working changes remain excluded and untouched. Prior Tiny Aya/Tiny SD results are evidence for the old pinned revision and isolated copied state, not current upstream acceptance or failure.
- Astra medium found the load-target DTO/resolver, package-facts context and selector snapshot implementation unchanged across those revisions. Download/recovery/storage changes can alter their indexed inputs. Current source still permits the directory-kind/primary-file mismatch mechanism; current identity/cache inputs must be qualified before assigning Tiny SD's old `artifact_missing` outcome to current Pumas. Neither an upgrade-only fix nor a current external defect has been demonstrated.
- Pantograph's selector supports Owner/LocalClient/ReadOnly, but `runtime_host_load_target.rs` and `workflow_service_composition.rs` require an owning `PumasApi`; `pumas_dispatch_package_facts.rs` cannot hydrate full facts through LocalClient/ReadOnly. Current Pumas LocalClient forwards targets and fact summaries, not full package facts; ReadOnly rejects OwnerFresh. A resolver-only client substitution would leave execution incomplete. Next admission must choose a supported ownership/facts contract, pin the reviewed revision, enumerate actual affected consumers and qualify snapshot→selected reference→full facts→target using that lifecycle. No path guessing, fabricated artifact identity or summary-for-full-facts substitution is admitted.
- This correction changes only the current plan, issues and this ledger. No dependency bump, Cargo/model execution, download, live library mutation or external source edit occurred. Protected proposals and unrelated media-sink formatting remain preserved. Astra owns consequential migration decisions; implementation remains unadmitted until the supported contract is selected.
- API-equivalent checkpoint through `2026-09-09T18:59:41.581Z`: Astra medium comparison **$0.871818 Standard / $1.743636 Fast** (12 responses); Astra low shared root integration/coordination **$1.973344 / $3.946688** (12). Combined **$2.845162 / $5.690324**, using the recorded rates, not invoices. Current root turn `01a08787-af40-7a71-b483-fea2a560d7b1` in root thread `01a084a6-36af-78c2-a6c8-6c0b4c2f9b77`, plus dedicated `pumas_current_contract` descendant, deduplicated by response_id. `/tmp/pantograph-current-pumas-costs.json` retains tokens/model/effort/tier and exact population. Only its current_turn population is charged here; its inherited historical-tail aggregate is excluded to avoid double-counting prior checkpoints. Final publication/commit and preceding explanatory Q&A are outside this checkpoint; unknown billed tier/approval/tool charges remain unpriced.

## 2026-09-09 — Shared Pumas/Pantograph owner-client design selected

- Compared committed Pumas `3b0d5ee4eda4d68ae33a158162883208e4608edb` with Pantograph's current consumers; separate uncommitted Pumas builder/process-lifecycle work was inspected only where relevant and is not treated as shipped. Standards remain `366c1d90a24bbfb50973f62b155a5f3396c0f107`; selected Architecture/Code Design/Contracts and owner/client lifecycle concerns supplement the existing planning/verification route. The codebase-design skill supported the interface/locality assessment. No external edits, Cargo, model execution or downloads occurred.
- Astra medium selected a narrow coordinated change in both repositories. Pumas already has the domain full-facts resolver (`api/models.rs`, `model_library/library.rs`) and owner dispatch branch (`api/state.rs`); the closed typed local protocol and LocalClient omit it. Complete that existing operation with bounded typed requests, required connection-token checks and public response/error handling. Reuse Pumas's cache/inspection and existing transport. No new aggregate execution operation, HTTP endpoint, general connection module or Pantograph cache is justified.
- Pantograph already has Owner/LocalClient/ReadOnly adapters in `workflow-nodes/src/setup.rs`. Route full-facts/target consumers and hosted composition through them, removing owner-only execution restrictions once the client contract is available. Acquisition must attach to the configured canonical launcher root before read-only fallback, never another connectable library; attached clients do not shut down an external owner. ReadOnly remains valid for indexed browsing, not OwnerFresh execution.
- Keep list snapshots cheap and selected full-facts hydration targeted. Full-facts cache reuse still builds inspection context and checks metadata fingerprints; it is not a weight copy. OwnerFresh load-target lookup refreshes external validation, not full-facts hydration. Existing identity/path/kind/schema checks are useful but no current content fingerprint, immutable file lease or atomic facts/target guarantee exists. Preserve selection mismatch and worker-open failures instead of claiming a combined call solves consistency. Current model identity/path-shape qualification remains distinct from completing transport.
- The active domain-architecture plan now owns the composed design, candidate source scope, deletion/locality reasoning and acceptance gate. Implementation must freeze current revisions and enumerate exact affected tests and independent dependency pins before writes; unrelated Pumas work and protected proposals remain excluded. Rejected alternatives: Pantograph-only owner mode retains the lifecycle coupling; using HTTP RPC adds another client despite the existing same-device transport; summary substitution drops execution evidence; a new aggregate API/schema or cache adds machinery without a demonstrated guarantee or measured need.

API-equivalent checkpoint through **2026-09-09T23:43:08.278Z** (USD, recorded pricing assumptions, not invoices):

| Owner | Responses | Uncached input | Cached input | Output incl. reasoning | Standard | Fast |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Luna max consumer inventory (unfinished handoff) | 51 | 275,188 | 6,820,096 | 11,582 | 0.205338 | 0.410676 |
| Astra medium shared design | 21 | 65,045 | 941,440 | 4,121 | 1.797940 | 3.595880 |
| Root integration/coordination (shared) | 27 | 27,160 | 4,690,176 | 6,767 | 5.300126 | 10.600252 |

Known-rate subtotal **$7.303404 Standard / $14.606808 Fast**. Current root turn `01a08885-4a8e-7902-a3db-f0adc510c020` and dedicated `pumas_shared_design`/`pumas_consumer_fields` descendants are selected by root_turn_id and deduplicated response_id. Source evidence `/tmp/pantograph-pumas-design-costs.json`; reusable adapted helper `/tmp/pantograph-pumas-design-costs.py`. Historical-tail output is excluded to avoid recounting other sessions. Cached input and reasoning are not added twice; actual billed tier/approval/tool charges remain unknown, and final publication/commit is a tail.

Delegation lesson: Astra delivered the consequential source-backed comparison. The Luna inventory lane did not supply a consolidated handoff before the bounded design stop and was interrupted; its attempts and root waiting remain charged, not an accepted independent change. The existing source trace and Astra comparison establish the selected contract; do not invent a completed field inventory. Use a shorter, explicitly bounded consumer enumeration or Sol medium for a future such task when coordination cost outweighs the direct model saving. This is one design outcome, not a controlled model benchmark.

## 2026-10-05 — Workflow top-k candidate and current M3 sequencing

- Feature milestone `9d6646a47c0dda8d391266970e160f0fb53aacb3`, tree
  `6b4e09ae6b7ad80cf010babb1096072ce6a1edad`, is published on
  `feat/workflow-text-top-k`, directly based on PR54
  `d6e9fcd15b135bedf36437ab2eceba229a0c9e2c`. Optional U64 `top_k`
  projects into existing typed sampling options, accepts zero, and leaves
  omitted controls at backend defaults. The [feature report](reports/2026-10-05-workflow-text-top-k.md)
  records boundaries, test coverage and final hosted/native commands.
- Contract tests: 20 passed; PyTorch-enabled inference tests: 686 passed.
  Normal-default embedded library/test compiler-Clippy check passed with ONNX
  downloading disabled. Native host tests are compiled but unexecuted locally;
  cloud ONNX/native execution remains deferred. Formatting, critical,
  accessibility and staged/range traceability gates accompany publication.
- Source reconciliation confirms current Pumas pin `2243a2b6` already exposes
  authenticated full facts, and Pantograph's existing facade and dispatch/host
  composition already support Owner/LocalClient. Plan header and DA-I03/04 now
  distinguish implemented source from pending qualification; the old
  producer/consumer implementation step is retired from current sequencing.
- Parent owns composition with accepted repair series `c83d5179`, `029ac704`,
  `cd54e12`, independent review and merge. That series is not included here;
  graph/reservation/observation files are untouched. Final hosted/native tests
  and current owner-produced model/device qualification precede required-real
  dependent text-to-image acceptance. No DA-03/07 closure, broader runtime
  capability, model inference or speculative scheduler work is claimed.

The parent subsequently published those repairs as `f56e2b5a` → `f9fb470c` →
`78bc71931772d891a6b5555076a072a63fe969a7`, exact tree
`141a0eae52ec7aa587536da297dd14a652836431`, and authorized composition.
PR54's remote head/ancestry match that source. The feature branch merges it
without conflicts, preserving the separate feature and plan milestones; all
five repair paths match the published bytes. The composed normal-default
embedded compiler-Clippy check, formatting and critical gate pass. Native
execution remains deferred and the parent's repair-head CI does not substitute
for qualification of the final feature composition. The plan now sequences
parent review and final hosted/native acceptance of that composition.

Independent top-k review then identified real sampling failure for k above
vocabulary; the earlier recording/envelope evidence and no-blocker assessment
were provisional. Separate sampler milestone `3bf3eb45` caps positive k at
logits width using Transformers semantics and preserves explicit non-streaming
zero. Four real CPU tensor/streaming/kwargs tests pass and detect both defects
in the frozen original; [sampling evidence](reports/2026-10-05-top-k-vocabulary-sampling.md)
distinguishes this from model inference.

Separate PR54 fixture successor `174c1950`, based directly on `78bc7193`,
persists canonical built-in definitions and asserts public I/O discovery while
retaining exact final text and zero runtime loads. The [fixture handoff](reports/2026-10-05-pr54-session-output-discovery.md)
records the full finalization trace and exact native test names. Eleven
supported node-engine tests pass; native public-session execution remains
deferred. Their conflict-free composition preserves the frozen top-k branch
and both independent commits. The composed 686-test inference suite and normal
embedded library/test compiler-Clippy check pass; final hosted/native scenario
execution and parent review remain outstanding.


## 2026-10-05 — Finite temperature and real CPU hosted qualification

Peer source review accepts frozen `174c1950` and `3bf3eb45`; composition
`60197971` remains unchanged pending exact-head hosted/native execution.
The existing review logs and exact source/hash inventory are saved in Library
`libfile_cdf0c1d747c0819183dfb308ea0875d0`, without expensive reruns. Small
CI successor `a6d1fd15` adds real offline CPU sampler tests to the existing
focused-test job and retains their log; hosted provisioning is not yet claimed.

The separate [temperature milestone](reports/2026-10-05-workflow-text-temperature.md)
adds an optional zero-inclusive descriptor, a finite JSON-number host variant,
and checked f32 conversion. Existing integer ports and omitted defaults are
preserved. Shared contracts pass 73 tests, inference passes 688, and five real
CPU tests include actual Transformers generation and streaming sampling on
fixed logits. Native descriptor/host/workflow tests are compiler checked only.
Parent owns publication and review; required-real acceptance remains open.


## 2026-10-05 — Nucleus sampling and CI failure propagation

Independent review found the frozen CPU CI step could mask unittest failures
through tee. Separate corrective commit `fddae90d` explicitly enables Bash
pipefail and corrects the earlier report; exact-script red/green shell checks
return failing/passing status correctly while retaining logs. A green run from
uncorrected `a6d1fd15` alone cannot qualify sampling. Frozen histories and
Library packets remain unchanged.

The separate [top_p milestone](reports/2026-10-05-workflow-text-top-p.md) starts
from temperature `e0293ebf` and integrates that CI correction. It reuses the
finite-number host contract, exposes an optional [0,1] input without a default,
and repairs streaming ties/cutoffs to match actual Transformers nucleus
sampling. Six real CPU tests pass, including 375 interaction cases; the new
matrix detects 112 failures against the frozen sampler. Shared tests pass 74,
inference passes 690, and embedded library/test compilation passes. Native
host/source execution and hosted provisioning remain pending; parent owns
independent review/publication, including any forthcoming finite-contract
corrections. No model weights, ONNX retry or complete-workflow acceptance.


## 2026-10-05 — Exact-head native text-control route and dependency blocker

Continued from frozen `a8c6970f` in the newly ready environment. Embedded host
and workflow binary attempts failed at missing ONNX symbols; one standard
locked build returned HTTP 403 for the pinned ONNX Runtime 1.24.2 artifact.
Zero native host/workflow tests executed. No further dependency download,
substitution, pin change or model-weight download occurred.

The separate [qualification route](reports/2026-10-05-text-control-native-qualification-route.md)
adds exact-source/cleanliness and nonempty-suite guards, real CPU tests,
pipefail/log capture and an independent required job in existing Quality Gates.
A controlled Rust-command fixture with actual CPU tests proves 112 frozen-source
sampling failures return exit 1 and six corrected tests return exit 0. Those
controlled statuses do not count as native execution. Shared serialization
passes 74 tests. Parent retains hosted/native qualification and publication;
next planned feature is DA-03's desktop-authored dependent real text-to-image
workflow after owner identity/target/device and desktop prerequisites.

## 2026-10-05 — Shared backing resource admission candidate

Continued independent scheduler delivery from frozen qualification `6aa6b717`
on `feat/scheduler-shared-resource-admission`. Research and current registry
inspection identify per-runtime capacity as the bounded prerequisite: two runtime
claims can spend one backing pool independently. The
[candidate](reports/2026-10-05-shared-resource-admission.md) adds explicit shared
RAM/VRAM domains to existing evaluation, authoritative admission and provisional
custody. Eleven public tests exercise actual contention, unified-memory sums,
replacement rollback/transfer, margins and configuration. Full portable suites
pass 106 registry plus 133 scheduler tests. Production physical bindings and
desktop activation remain open; native sampling qualification stays blocked.
Parent retains review/PR/hosted execution; old milestones remain frozen.

## 2026-10-05 — Explicit shared-resource startup composition candidate

The parent froze `27af8aa3` and authorized connecting its shared admission API
through the application. The [successor](reports/2026-10-05-shared-resource-composition.md)
adds optional domains to existing AppConfig/config.json and uses the registry's
portable factory in actual desktop setup before gateway/workflow startup. No
backing topology is inferred from device selection. Empty/absent configuration
retains the previous empty registry; malformed declarations fail startup, and
live domain edits require restart. Eight actual portable composition tests cover
persisted declarations, shared contention, unified memory and rollback/transfer.
Native AppConfig tests are authored but unexecuted because GTK/WebKit prerequisites
are absent. Parent retains desktop/native qualification and review; frozen
sampling source and PR54 diagnostics remain unchanged.

## 2026-10-05 — AppConfig filesystem repair and production composition qualification

Continued separately from frozen `73211ddc` on
`qualification/shared-resource-app-config`. Peer review found `Path::exists`
discarded permission/metadata failures and broken symlinks as apparent absence.
The [repair and qualification](reports/2026-10-05-app-config-startup-qualification.md)
uses a fallible read and confirms genuine absence, preserving other filesystem,
JSON and domain errors. The actual AppConfig implementation now lives in a
production crate consumed by Tauri setup, so full settings, persistence and
startup registry composition execute without unrelated GUI/inference dependencies.
Ten AppConfig tests and the existing 114 registry/133 scheduler tests pass;
the exact frozen loader fails three new filesystem regressions. No native
runtime is simulated. Official APT update was attempted with owner authorization
but returned OS permission denial; desktop compilation fails at missing
`glib-2.0.pc`. Tauri setup/IPC and desktop-process qualification stay blocked.
Parent retains review/publication; old source milestones and PR54 remain frozen.

## 2026-10-05 — Explicit retained model resource accounting

Parent froze `eabbcc83` and authorized the next independent scheduler capability.
Source inspection confirms shared domains sum task leases only; model residency
metadata and candidate loaded-memory estimates do not keep an idle producer's
allocations charged. The [successor](reports/2026-10-05-resident-resource-accounting.md)
adds explicit model/instance resident declarations at the registry owner, counts
them in local/shared admission, and preserves envelopes after task cleanup.
Unknown loaded shared-pool members fail admission with typed unavailable
diagnostics instead of exposing free capacity. Fifteen new portable tests cover
lifecycle, identity freshness, real contention, unified kinds, overflow and
provisional replacement. Current host producers still need a bridge publishing
per-kind estimates; peak task envelopes remain conservatively charged in full.
No hardware measurements, native execution or pin changes are claimed. Parent
retains review/publication; frozen qualifications and PR54 remain unchanged.


## 2026-10-05 — Ordered PyTorch producer resident estimates

Continued from frozen `254aef1` on `feat/runtime-producer-resident-estimates`.
The [producer bridge](reports/2026-10-05-runtime-producer-resident-estimates.md)
uses explicit startup estimates for the exact observed model target, publishing
coherent source/sequence/model/instance facts from the existing gateway owner.
Old stops and loads cannot replace newer allocation facts; explicit zero and
missing per-kind estimates remain distinct. Effectful load failure retains the
previous envelope and blocks shared admission until owner evidence resolves it.
PyTorch shutdown must acknowledge cleanup even after failed load erased metadata.
Existing peak task claims/custody remain fully charged. Portable owner/config
fixtures qualify the logical accounting; native GUI/GPU execution and other
producers remain unqualified. No previous branch rewrite or PR/review creation.

## 2026-10-06 — Isolated PR56 correctness review checkpoint

Parent paused unrelated image scheduler work for five PR56 review findings.
The [saved repair checkpoint](reports/2026-10-06-pr56-correctness-review-checkpoint.md)
records four reproducible correctness defects and actual child-process evidence
contradicting the reaped-child summary. Source `5da09171` is based on the published
import-only integration `b3c58756`; feature histories remain separate. Full comment
body reads were Forbidden, with no credential or access-route changes. Exact
comment disposal and integration publication remain blocked on those bodies.
Test successor `a92f67ff` passes all affected suites: 805 inference, 503 embedded,
141 registry and 14 config, plus all 661 frontend assertions. Strict all-target
Clippy and no-download/gate checks pass. The first combined run exposed six
zero-probe fixture assumptions, corrected without weakening uncertainty checks.
No native desktop/GPU/model execution claim is added.

The parent subsequently supplied all five original full review bodies. Exact
validation found two missing cases in the summary-based recovery checkpoint:
Candle supervised publication still reused its counter value, and an explicit
VRAM-only task still failed on unknown RAM residency. Source `4f877149` closes both
with deterministic regressions against actual owner publication and all four
registry admission paths. The pinned Tokio InvalidInput allegation is incorrect;
Unix native child tests and the exact dependency implementation support rejecting
it without changing production shutdown. No review CLI or denied fetch is used.
Final source qualification passes 806 inference, 503 embedded and 142 registry
checks with strict all-target mixed-backend Clippy. Config/frontend source remains
unchanged from its passing qualification. The independent repair branch is
published normally; the parent alone updates PR56 and replies/resolves reviews.

## 2026-10-06 — PR56 config fixture contract correction

The parent's re-review record 4191994790 identifies malformed startup-domain keys
and incomplete AppConfig fields in the controlled handler regression. The
isolated successor of `13d24e8` uses the actual registry startup JSON and a strict
complete-payload save expectation. A new rejection regression fails against the
old mock and passes after repair; all three focused handler tests, TypeScript,
formatting, critical/accessibility gates, lint and traceability pass. Production
code is unchanged. Parent retains PR56 advancement; the combined image-controls
candidate is preserved independently. See the existing
[checkpoint report](reports/2026-10-06-pr56-correctness-review-checkpoint.md).
