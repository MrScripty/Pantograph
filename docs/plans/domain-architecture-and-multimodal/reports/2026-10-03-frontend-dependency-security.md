# Frontend dependency security repair (2026-10-03)

Status: implemented and source-reviewed; exact-head hosted and GUI qualification remain pending. Parent: PR #20, `8a08b6a010ee62fbd3b29e193c65922c0769b928`. Standards reviewed at Coding-Standards `dcc56f26e884ade260770beceba2501d3746200d`; this is newer than the active plan's planning baseline, without changing plan or release authority.

## Resolution and rationale

- Upgrade the four direct Tiptap requirements from `^3.15.3` to `^3.30.5`; all 26 locked Tiptap packages resolve to `3.30.5`, satisfying upstream's exact cohort peers.
- Upgrade Svelte `5.55.4` to `5.55.7` (requirement `^5.55.7`), and resolve its transitive devalue from `5.7.1` to `5.9.3`. No direct devalue dependency or permanent override is added.
- [Upstream pm 3.30.5](https://github.com/ueberdosis/tiptap/blob/v3.30.5/packages/pm/package.json) removes prosemirror-markdown. The regenerated graph prunes prosemirror-markdown, markdown-it, linkify-it and their unneeded dependencies; remove the obsolete markdown-it 14.1.1 override. The separate linkifyjs dependency remains and resolves to 4.3.3 through Tiptap.
- Preserve Vite 6.4.1 and Svelte Vite plugin 5.1.1. Other version changes are confined to Tiptap's ProseMirror requirements. No new package entries appear. Resolver-only peer-flag normalization affects unchanged packages; argparse, entities and escape-string-regexp become dev-only after the production chain is pruned.
- Root manifest/root lock remain the only npm resolution authority. Registry metadata selected exact candidates; temporary generation constraints prevented caret ranges selecting later releases, then were removed and the lock regenerated. Recovery is restoration of the prior manifest/lock pair, not a second active resolution.

[Official Svelte release](https://github.com/sveltejs/svelte/releases/tag/svelte@5.55.7) and [devalue release](https://github.com/sveltejs/devalue/releases/tag/v5.9.3) describe the security corrections. No source-search result is treated as proof of non-exposure. npm audit and resolver evidence prove their stated graph/advisory properties, not complete application security or release readiness.

## Advisory disposition

A fresh production audit of the parent lock reported five affected package nodes (three high, two moderate). The following advisory IDs are captured from that audit; remediation is the selected upgrade, or graph removal for markdown-it/linkify-it:

- @tiptap/core: [GHSA-cp6q-959q-f8rh](https://github.com/advisories/GHSA-cp6q-959q-f8rh), [GHSA-j95f-988m-3j2f](https://github.com/advisories/GHSA-j95f-988m-3j2f).
- devalue: [GHSA-77vg-94rm-hx3p](https://github.com/advisories/GHSA-77vg-94rm-hx3p), [GHSA-9rgm-9g3h-6x36](https://github.com/advisories/GHSA-9rgm-9g3h-6x36), [GHSA-j22f-vq7h-c4qm](https://github.com/advisories/GHSA-j22f-vq7h-c4qm), [GHSA-hx4r-w6wj-j8fg](https://github.com/advisories/GHSA-hx4r-w6wj-j8fg), [GHSA-mcm9-63f2-9j32](https://github.com/advisories/GHSA-mcm9-63f2-9j32), [GHSA-wf3x-273g-mvxv](https://github.com/advisories/GHSA-wf3x-273g-mvxv), [GHSA-4q55-j62x-fr9h](https://github.com/advisories/GHSA-4q55-j62x-fr9h).
- linkify-it: [GHSA-22p9-wv53-3rq4](https://github.com/advisories/GHSA-22p9-wv53-3rq4), [GHSA-v245-v573-v5vm](https://github.com/advisories/GHSA-v245-v573-v5vm).
- markdown-it: [GHSA-6v5v-wf23-fmfq](https://github.com/advisories/GHSA-6v5v-wf23-fmfq), [GHSA-253c-mchw-3w2r](https://github.com/advisories/GHSA-253c-mchw-3w2r).
- svelte: [GHSA-f3cj-j4f6-wq85](https://github.com/advisories/GHSA-f3cj-j4f6-wq85), [GHSA-rcqx-6q8c-2c42](https://github.com/advisories/GHSA-rcqx-6q8c-2c42), [GHSA-9rmh-mm8f-r9h6](https://github.com/advisories/GHSA-9rmh-mm8f-r9h6), [GHSA-pr6f-5x2q-rwfp](https://github.com/advisories/GHSA-pr6f-5x2q-rwfp).

## Verification and limits

Local generation and clean install used Node 24.19.0/npm 11.9.0 because the pinned Node 24.12.0/npm 11.6.2 were unavailable. No global toolchain was changed. A clean hosted install and applicable checks using both declared versions are required before qualification; the local results do not waive this requirement.

Passed against the reviewed manifest/lock: `npm ci --include=optional --no-fund --no-audit`, `npm ls --all`, `npm audit --omit=dev --audit-level=high` (zero findings), `npm run typecheck`, `npm run test:frontend` (540/540), `npm run build`, `npm run lint`, `npm run lint:full`, staged decision traceability, and whitespace validation. Existing resolver and audit checks provide the dependency regression evidence; no duplicate version-assertion harness is added.

`npm run lint:no-new` stops at the existing IoInspectorPage.svelte:520 DOM-mutation finding. Separately, `npm run lint:a11y` passes its eight parser tests but reports existing reviewed-ignore annotations at IoInspectorPage.svelte:1034 and RunGraphSnapshot.svelte:156. These unchanged UI issues belong to the parallel UI repair.

`npm run test:workflow-editor-image-gui` exits 2 at preflight because PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID is unavailable. Real Pumas model/artifact IDs, saved workflow, Python runtime, desktop drivers/display and built host remain prerequisites; no GUI success is claimed. The integration owner must obtain that qualified evidence. Rust/desktop compilation and release publication are outside this dependency slice.

The source manifest/lock pair was independently reviewed before publication at staged tree `b75bac90dbbda40c3006e3228f0ce5eab8ed8f56`. Subsequent documentation adds this report without changing those files. Keep the task worktree protected while hosted qualification or integration is pending; the integration owner controls final merge and terminal disposition.

## Hosted qualification follow-up

[Quality Gates run 37115665903](https://github.com/MrScripty/Pantograph/actions/runs/37115665903) confirms Node 24.12.0/npm 11.6.2, clean npm ci, zero production-audit findings, 540/540 frontend tests, typecheck and full lint. Its synthetic merge `e66f54bc7b3f2963956918a6a781fc3a2ecf47ca` has the same tree `1c51bc4dcce332846e550ecf6374864521a11099` as published source commit `0b5c0df663d76dc97f2ba3836d33ea2404ca79df`. The unchanged critical-lint failure remains visible.

That workflow lacked installed-tree and frontend-build checks. Add `npm ls --all` and `npm run build` to the existing dependency-audit job after clean installation and the production audit, preserving every existing command and aggregate gate. These checks prove resolver consistency and frontend asset compilation on the declared toolchain; they do not establish GUI or packaged-release acceptance. Exact-head hosted results for this workflow change remain pending.

The hosted development-inclusive install audit reports 41 affected package nodes (1 low, 5 moderate, 35 high). Dependency maintainers must triage that separate follow-up before any broader security claim; this slice does not upgrade unrelated development tools or claim zero vulnerabilities across the complete graph. Headless/Runtime Separation results remain under observation, and the real-model GUI prerequisites above remain unavailable.
