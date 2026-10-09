# Pantograph Research Sources

These books and papers inform development; they do not admit implementation
work or change Pantograph's supported capabilities. Start with the
[active domain-architecture plan](../plans/domain-architecture-and-multimodal/plan.md)
for scope and acceptance, the [ADR index](../adr/README.md) for accepted
decisions, and [Architecture](../../ARCHITECTURE.md) for current ownership.

## Editions And Evidence

All five manuscripts are the 2 October 2026 research editions. Their Pantograph
source inspection is pinned to
[`4938e405c7f656365eefdca492774ccae110c90d`](https://github.com/MrScripty/Pantograph/commit/4938e405c7f656365eefdca492774ccae110c90d).
Statements such as "current" or "already implements" describe that snapshot,
not later revisions. Source inspection is not build, deployment, crash-test,
security-containment, or hardware-performance evidence.

| Reading | Status and use |
| --- | --- |
| [Designing Compatible Inference Backends](inference-backends/manuscript/designing-compatible-inference-backends.md) | Book: source-grounded comparison and proposed compatibility architecture. Chapter 9 audits Pantograph; chapters 10–16 propose contracts, qualification and a native roadmap. |
| [Pantograph Node Execution and Dynamic Extensions](node-execution-and-dynamic-extensions.md) | Paper: section 5 audits the node boundary; sections 6–14 propose managed invocation, lifecycle, extension admission and qualification. |
| [Pantograph Execution Ledger and Provenance](execution-ledger-and-provenance.md) | Paper: section 3 audits existing evidence/artifact paths; later sections propose identity, commitment, retention and evaluation contracts. |
| [A Resource Safe Learning Scheduler for Pantograph](scheduler/resource-safe-learning-scheduler.md) | Revised paper: inspected production constraints, audited v1 simulation findings, and a proposed v2 research contract. It predates the v2 evaluation below. |
| [Pantograph scheduler v2 evaluation](scheduler/v2-evaluation.md) | Later C3-corrected standalone simulation report, with historical C1/O1 results and C3 regression evidence kept separate. No Pantograph implementation or GPU benchmark. |

The [source manifest](source-manifest.json) records exact SHA-256 identities,
edition information and the separate C3 simulator fingerprint. Manuscripts,
their linked figures and the inference book's
[primary-source index](inference-backends/references/primary-source-index.json)
are byte-for-byte copies of the supplied research editions. Bibliographies,
pinned source links, original diagram sources and limits remain with each work.
No third-party papers, upstream source snapshots or PDF editions are bundled.

## Read Against The Active Acceptance Criteria

The following links are design inputs to the plan's existing DA criteria,
not new acceptance criteria or claims that an item is satisfied.

| Active obligation | Relevant research |
| --- | --- |
| DA-01 review and DA-02 domain ownership | Inference [chapter 9 source audit](inference-backends/manuscript/designing-compatible-inference-backends.md#9-what-pantograph-has-today) and [chapter 10 contract owners](inference-backends/manuscript/designing-compatible-inference-backends.md#10-separating-identity-capability-and-execution); node [section 5 integration gaps](node-execution-and-dynamic-extensions.md#5-current-implementation-and-integration-gaps). |
| DA-03 real dependent text-to-image workflow | Inference [chapter 12 data contracts](inference-backends/manuscript/designing-compatible-inference-backends.md#12-data-contracts-across-modalities) and [chapter 13 qualification](inference-backends/manuscript/designing-compatible-inference-backends.md#13-qualification-before-promises). A simulation or independent text/image runs cannot satisfy the real mixed-workflow criterion. |
| DA-04 failure, cancellation and resource ownership | Node [section 7 lifecycle](node-execution-and-dynamic-extensions.md#7-lifecycle-cancellation-retry-cache-and-state) and [section 13 falsifiers](node-execution-and-dynamic-extensions.md#13-qualification-claims-and-falsifiers); scheduler [sections 4–6](scheduler/resource-safe-learning-scheduler.md#4-state-and-authority-at-node-and-stage-boundaries). |
| DA-05 retained-output lifetime and cold reopen | Ledger [section 7 commitment](execution-ledger-and-provenance.md#7-crash-consistency-and-result-commitment), [section 8 retention](execution-ledger-and-provenance.md#8-retention-deletion-and-privacy) and [section 14 validation](execution-ledger-and-provenance.md#14-validation-criteria-before-stronger-guarantees). |
| DA-06 measured orchestration and resource budgets | Scheduler [section 9 measurement](scheduler/resource-safe-learning-scheduler.md#9-learn-host-and-phase-behavior-without-future-leakage), [section 10 planning cost](scheduler/resource-safe-learning-scheduler.md#10-completion-oriented-multi-event-planning) and v2 [real planning overhead](scheduler/v2-evaluation.md#planning-time-and-unfinished-experiments). |
| DA-07 final integrated evidence | Inference [chapter 13](inference-backends/manuscript/designing-compatible-inference-backends.md#13-qualification-before-promises), node [section 13](node-execution-and-dynamic-extensions.md#13-qualification-claims-and-falsifiers), and ledger [section 14](execution-ledger-and-provenance.md#14-validation-criteria-before-stronger-guarantees) supply possible falsifiers. Each required production claim still needs its own environment-qualified evidence. |

For the selected Pumas owner/client integration, preserve the
[plan's producer/consumer contract](../plans/domain-architecture-and-multimodal/plan.md#pumas-ownerclient-integration--selected-design).
The inference book distinguishes Pantograph's consumed Pumas revision
`f87c3da8276a914a54c6f4f36d617bef9d9f424e` from its separately inspected newer
upstream snapshot. Neither a newer producer nor a research proposal silently
replaces the application's dependency pin, artifact identity, or load-target
authority.

## Scheduler Results Need Their Denominators

The [C3 fresh holdout](scheduler/v2-evaluation.md#final-c3-visibility-correction-and-fresh-holdout)
has 56 runs: 55 terminal and one budget-exceeded run. Search versus
earliest-finish has nine wins and one loss across ten complete pairs; the
iterative search run did not complete. This is one fresh seed, not broad
generalization evidence. The earlier C1/O1 complete-pair comparison has 27
wins, one tie and two losses, with three of 33 search runs budget-exceeded.

C3 search averaged **116.72 seconds of planner CPU** and **116.79 seconds of
whole-run CPU**; earliest-finish averaged **0.19 seconds of whole-run CPU**.
These are real CPU costs on the evaluation host. Modeled execution seconds
are synthetic and are not hardware-calibrated. Planner columns omit
interrupted calls and therefore understate planning cost for interrupted runs.
The report preserves losing batching/split cases and the tiny oracle's missed
batch-of-three optimum. Favorable simulated schedules do not establish useful
production throughput.

C2/C3 corrected hidden-demand and forecast-metadata leakage, then cleanup-event
ordering that could reveal latent state. The interrupted seed-9001 population,
fresh seed-9029 population, and historical regression are separately disclosed.
Do not use pre-correction results as evidence for the corrected information
boundary or describe historical reruns as unseen holdouts.

## Reproduction And Maintenance Boundary

This is a reading collection, not a self-contained experiment package. The
scheduler manuscripts refer to their original standalone companions: simulator
source, frozen contracts, raw runs, traces, tests and independent review files.
Those companions are not bundled here, and no public download location is
established by this import. Their reproduction commands and relative evidence
filenames refer to those packages, not this repository checkout. This import
checks manuscript/asset fidelity and links; it does not rerun their experiments
or independently certify all reported results.

The inference book retains its 15 linked PNG diagrams and editable SVG
originals; the v2 report retains both linked figures and their SVG originals.
PDFs, covers, rendering tools and unused companion figures are omitted.
Research recommendations remain proposals until adopted through the owning
plan or ADR. When development invalidates a dated observation, update current
guides and acceptance evidence rather than silently rewriting the research
edition. Git history preserves replaced editions without a parallel archive.
