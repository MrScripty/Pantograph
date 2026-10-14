# M2 acceptance and main integration

M2 is accepted for the bounded opt-in retained-CPU service advisory in the
[canonical plan](../plan.md). The fixed actual CPU cohort and its immutable
raw/source/fixture/binary evidence remain in the
[CPU report](2026-10-13-m2-cpu-results.md). There is no new CPU cohort or broader
speedup, native desktop, GPU, default activation or remote qualification.

## Required PR verification

Reviewed head: `8f9afd72a9e4b83b43e049722161eeaac5e933d3`.
All required CI completed successfully on 2026-10-14:

- [Quality Gates](https://github.com/MrScripty/Pantograph/actions/runs/38484792616): all13 jobs passed, including the warning audits and native text controls.
- [Headless Workflow Contract](https://github.com/MrScripty/Pantograph/actions/runs/38484792619): passed.
- [Runtime Separation Check](https://github.com/MrScripty/Pantograph/actions/runs/38484792765): passed.

Independent GPT-6.1 Sol / High source, raw-only arithmetic/output/lifecycle,
final report and fresh integration reviews passed. No blocking review thread or
requested change existed. The CodeRabbit success status was not used as
independent approval. The required local gates are preserved in the CPU report.

## Authorized merge and main identity

The user specifically approved merging ready PR76 into main. The normal GitHub
merge action pinned the reviewed head and succeeded; prior automatic approval
rejections were not bypassed. [PR76](https://github.com/MrScripty/Pantograph/pull/76)
is closed and merged at 2026-10-14 00:49:03 UTC.

Merge: `0f6972af3f70da75479f176d12c3bc57ea0d318e`.
Parents: `07b23d47b2cdfae5669bbda78515c9c728081031` and
`8f9afd72a9e4b83b43e049722161eeaac5e933d3`.
Tree: `129c5471ced07a4ce1f2adb637d64ca7860a75c2`.

`git fetch origin main`, exact remote ref, commit parent/tree inspection,
ancestor check and whole-tree diff verify main contains the reviewed feature
with an identical source/evidence tree. All12 delivered-file SHA256SUMS entries
pass after the merge. No repeated tests or model execution are needed to prove
identity; the post-merge push workflows provide separate integration checks.
PR75 remains draft and its required real-model T3 hold is unchanged.

## Post-merge push CI

All three normal push workflows completed successfully for exact merge
`0f6972af3f70da75479f176d12c3bc57ea0d318e`:

- [Runtime Separation](https://github.com/MrScripty/Pantograph/actions/runs/38492288359): passed.
- [Headless](https://github.com/MrScripty/Pantograph/actions/runs/38492288374): passed.
- [Quality](https://github.com/MrScripty/Pantograph/actions/runs/38492288367): all13 jobs passed.

The first observation was still running; the final observation on 2026-10-14
confirmed all workflows and all13 Quality jobs succeeded. Remote main remained
`0f6972af`, with the verified identical source/evidence tree. These are normal
integration checks, not broader GPU or native desktop qualification.

The public GitHub GET
`actions/runs?head_sha=0f6972af3f70da75479f176d12c3bc57ea0d318e&event=push`
returns these runs. The specialized commit-workflow helper filters to PR runs
and returns no main push runs; an empty result is not absent CI.

This documentation follow-up is a separate publication proposal. It changes no
implementation or raw CPU evidence and supplies no additional main-merge
authority. The broad plan remains Active; the next step is independently
reviewing a bounded M3 contract, before any M3 implementation.
