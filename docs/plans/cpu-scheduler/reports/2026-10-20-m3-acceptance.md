# M3 published-head acceptance review

Status: **Accepted within the reviewed CPU service-opportunity scope**. C1–C5
are satisfied at the exact published head. The actual final Quality summary
succeeded, independently verified at02:23 UTC. The admitted M4 local production
dependency is satisfied. No merge or further publication is authorized.

## Exact source and CI provenance

PR81 published head `b63aa3ebe5fab1531aef952a35ce74932bf76b30`, tree
`defca3d306f4824726fa6094b2bba04a3128e570`, remains unchanged and clean in
`/workspace/pantograph-m3`. Normal checkout jobs tested PR merge commit
`a9292d48a49ac05ddec711397159077b3e68f9f0`; fetched merge tree is exactly
`defca3d306f4824726fa6094b2bba04a3128e570`. Thus those jobs tested the published
source tree, not a different integration tree. CPU Composition and Native Text
Controls explicitly checked out the published head itself.

The exact-head API query returns all four applicable workflows. All16 jobs succeeded:13 Quality jobs including the actual final summary,
Headless Workflow Contract, Runtime Separation and CPU Composition. Their full logs and API checkpoint are retained in
[ci/logs.tar.gz](m3-service-v1/ci/logs.tar.gz), with
[member hashes](m3-service-v1/ci/log-sha256.json). The15 substantive logs contain published-head attribution; all16 logs have
zero GitHub error markers. The summary log records actual required gate results. Independent source review checked
actual checkout, relevant command/test results and audit wrappers; format and
warning-deny Clippy themselves succeeded, not merely their surrounding jobs.

| Workflow | Retained run | Observed conclusion |
| --- | --- | --- |
| [Headless Workflow Contract](https://github.com/MrScripty/Pantograph/actions/runs/39341838018) |39341838018 |success |
| [Runtime Separation](https://github.com/MrScripty/Pantograph/actions/runs/39341838040) |39341838040 |success |
| [CPU Composition](https://github.com/MrScripty/Pantograph/actions/runs/39341838152) |39341838152 |success |
| [Quality Gates](https://github.com/MrScripty/Pantograph/actions/runs/39341838081) |39341838081 |all13 jobs success, including final summary |

Final checked2026-10-20 at02:23 UTC: summary job `118394536482` completed
successfully at02:00:50 UTC; its actual log reports all10 required gates and
format/Clippy audits successful. The earlier01:21 queued checkpoint is retained
as historical evidence alongside `final-status.json` in the archive. The unchanged
`review-notes.json` is the historical conditional review, whose then-current
`accepted: false`/queued disposition predates this final acceptance. These
normal workflows do not add a GPU, UI, native-batching, semantic-model-quality or
performance claim to M3's narrow CPU service-opportunity qualification.

## Retained objective review

Independent GPT-6.1 Sol High source/design reviewers approved production source,
fixed native producer, corrected JSON-only oracle and final report/evidence.
Workflow1008 tests pass locally and in Headless CI. Existing scoped native
lifecycle3 tests, scheduler205 tests, corrected checker4 tests and declared
pre-push261+168 tests pass with disclosed existing ignores. The [M3 report](2026-10-19-m3-service-opportunities.md)
and its exact-source manifests/archives own the detailed objective evidence.

All original41 raw members and evidence hashes remain intact. The sole fixed
native cohort completed ten outputs and matching cleanup receipts, with protected
background grants at positions3 and7. Its original process exit1 and failed local-
task-name checker remain preserved. Independently reviewed saved-data reanalysis
passed without native retry; timing-ring cancellation exclusion remains explicitly
source-assisted aggregate evidence without independent run/attempt duration
attribution. Existing CPU-14 stopping metadata and CPU-17 optional-consumer warning
limits remain disclosed. No producer success or performance result is invented.

## Gate disposition

Independent final source/evidence review accepts M3-C1–C5 within the disclosed
scope. The M3 branch/head/tree remain frozen. Parent authorized entry into the
already admitted M4 production slice **locally in persistent storage** after this
actual successful summary; no new model cohort ran for this reconciliation.

Publication remains blocked: automatic approval review rejected uploading the
12-file local checkpoint07138639 payload to MrScripty/Pantograph via Git Data as
nonpublic source/test egress without exact-payload approval. The approval question
is unanswered. Acceptance and green CI do not authorize retry, another route or
subsequent payload publication. Parent owns integration and branch cleanup.
