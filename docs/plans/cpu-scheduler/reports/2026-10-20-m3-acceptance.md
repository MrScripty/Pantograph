# M3 published-head acceptance review

Status: **Pending final Quality summary**. C1–C4 evidence is satisfied; M3-C5 and
the M4 production dependency remain pending. A successful substantive matrix is
not substituted for a queued final required check. No merge is authorized.

## Exact source and CI provenance

PR81 published head `b63aa3ebe5fab1531aef952a35ce74932bf76b30`, tree
`defca3d306f4824726fa6094b2bba04a3128e570`, remains unchanged and clean in
`/workspace/pantograph-m3`. Normal checkout jobs tested PR merge commit
`a9292d48a49ac05ddec711397159077b3e68f9f0`; fetched merge tree is exactly
`defca3d306f4824726fa6094b2bba04a3128e570`. Thus those jobs tested the published
source tree, not a different integration tree. CPU Composition and Native Text
Controls explicitly checked out the published head itself.

The exact-head API query returns all four applicable workflows. All15 substantive
jobs succeeded:12 Quality jobs, Headless Workflow Contract, Runtime Separation
and CPU Composition. Their full logs and API checkpoint are retained in
[ci/logs.tar.gz](m3-service-v1/ci/logs.tar.gz), with
[member hashes](m3-service-v1/ci/log-sha256.json). All15 logs contain published-head
attribution and zero GitHub error markers. Independent source review checked
actual checkout, relevant command/test results and audit wrappers; format and
warning-deny Clippy themselves succeeded, not merely their surrounding jobs.

| Workflow | Retained run | Observed conclusion |
| --- | --- | --- |
| [Headless Workflow Contract](https://github.com/MrScripty/Pantograph/actions/runs/39341838018) |39341838018 |success |
| [Runtime Separation](https://github.com/MrScripty/Pantograph/actions/runs/39341838040) |39341838040 |success |
| [CPU Composition](https://github.com/MrScripty/Pantograph/actions/runs/39341838152) |39341838152 |success |
| [Quality Gates](https://github.com/MrScripty/Pantograph/actions/runs/39341838081) |39341838081 |all12 substantive jobs success; final summary queued |

Last checked2026-10-20 at01:21 UTC: final summary job `118394536482` is
queued with null conclusion in the retained
checkpoint. Acceptance waits for its actual successful completion/log. These
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

Independent final review found no remaining source or evidence blocker. M3-C5 is
still pending only the observed final Quality summary result. M4's admitted
contract and controlled registry fixtures may progress independently, but no M4
production edit or native qualification is admitted by a queued status. Once the
actual summary succeeds, retain its log/final metadata, record exact-head M3
acceptance on the dependent branch, and enter the already admitted M4 production
slice. Preserve the M3 branch/head; parent owns integration and branch cleanup.
