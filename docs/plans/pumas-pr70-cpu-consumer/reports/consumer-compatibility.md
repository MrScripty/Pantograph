# Pumas PR70 consumer receipt

Bounded local CPU edge accepted, including independent GPT-6.1 Sol High review.
This is not release approval or a replacement for PR78's baseline pin.

| Input | Exact qualified revision |
| --- | --- |
| Pantograph candidate | `039cee079d273a8d4968df56a4b35e8eb8aacbbe` |
| Reviewed Pantograph baseline | `0dc45263d014bb9b42884242547424070ad65cc1` |
| Pumas PR70 | `df53db08e2ba224edfaf713522ddd410f7d4ac44` |
| Public Chrema | `508c8b8cb724b49c655dab8bc4c8336190e516c7` |
| Candle | `88ed7911de9e88196b1f55b199145d22647f415e` |

Source branch `qualification/pumas-pr70-cpu-consumer-2026-10-14`. Compared with
the reviewed baseline, four dependency authorities move together: manifest,
Cargo.lock, quality CI checkout and demo lock. Of1,116 parsed locked packages,
only Pumas version/source changed (`0.7.0` -> `0.8.0-rc.1`); other bindings and
checksums remain intact. No consumer API source repair was necessary.

Pumas [source-foundation receipt](https://github.com/MrScripty/Pumas-Library/blob/df53db08e2ba224edfaf713522ddd410f7d4ac44/docs/plans/v080-readiness/reports/candidate.json)
SHA256 `617b7fb1ec18c04274d7ad56d346d0a8e6d62b2b3358b1b1f18a139715fdb814`
matches the actual fresh public checkout. The declared production roots have
empty diff against `ef9eead9fe6595f5b3152ef8df19c5dace30190b`.

## Actual producer/consumer observations

Linux x86_64, Rust/Cargo1.92.0 and Node24.12.0. No Python/model provisioning.
`prepare.mjs` compiled only `backend-candle,host-dependency-inventory` with
`--locked`, built/audited public Chrema, and sealed fixture inputs. The public
Pumas owner indexed the untrained BERT-8 fixture and returned execution
descriptor v1, model-ref v1, package facts v3 and a valid selected-main HF
directory target. Actual public graph validation, executable publication,
owner dependency readiness, scheduler dispatch and CPU output completed.

Chrema run `7b34a1b3-c3a6-4d8a-abd5-f8d797418c3a` completed both components;
Pantograph run `run_06240efc-a381-4967-b6fe-280c14a59375` retained output artifact
`workflow-io-3d1762151f832acb8d076e2a0a4beea3e781b25b4a5b775b28e2e9f5bd39a562`.
The eight CPU floats match the independent fixture reference within1e-4 and
the sealed retained artifact matches the scheduler response exactly. Fresh
native/CLI inspection verified original run graph/model/runtime/library usage
links, while repeated inspection preserves original bytes. Shutdown passed;
zero model acquisition bytes. This is numerical inference with an untrained
fixture, not pretrained text generation, embedding usefulness or PR75 T3.

Prepared binary SHA256:
`47ce281f575c5acfd4dd5b5830452e27c55ef962f46f1734c27ecfadf2f36ea4`.
Candidate Cargo.lock SHA256:
`a6c31b5ae96fe80db652c862c5edac07fec1268b5a9f67f0a6c118d25ba89228`.

## Evidence and tests

[Raw candidate archive](candidate-evidence-039cee07.tar.gz):154,745 bytes,
48 regular files, SHA256
`f03d51e5f22c32558dda7ae4922f4cf623e0c5d830b491893cfb3f8c3656b41a`.
Includes complete sealed bundle, isolated owner databases, artifacts, source/
package/binary receipts and actual supporting logs. Credential/client/session/
bucket tables were checked read-only and contain zero rows; populated data are
only synthetic fixture/workflow records. No binary/package redistribution.

- Exact source19 integration contracts passed, zero failures/skips.
- Embedded-runtime default-feature suite with isolated registry:598 passed,
  zero failures,2 existing ignores.
- Existing single/rich HTTP host contracts:4 passed.
- Example warning-deny Clippy, workspace rustfmt, example ESLint, whitespace/
  traceability and manifest/lock/CI plus ONNX no-build-download metadata passed.
- Independent reviewer reran all19 contracts and fresh inspection, confirmed
  source receipt/current native hashes, exact owner schemas and minimal lock
  update, and found no functional issue. Documentation handoff findings were
  corrected without changing executable source.

Preserved prequalification observations on older34dfedd consumer code are not
substituted for this reviewed039cee07 candidate. The new worktree initially
lacked commonmark; after linking the existing same-lock tooling cache, staged
traceability passed. No installation, permission escalation or hook bypass.

Prepared candidate: `/workspace/scratch/pumas-pr70-prepared-039cee07`.
Retained bundle: `/workspace/scratch/pumas-pr70-bundle-039cee07`.
After candidate worktree removal, the copied binary and sealed fixtures remain
sufficient for fresh-process execution/inspection; no compile-time checkout
model-input dependency remains. Retain this exact prepared directory to inspect
its original receipt; a rebuilt binary receives a separate identity/bundle.

## Limits and adoption

No API/build/runtime failure was observed on this selected source consumer
edge. Pumas RPC/standalone release artifacts, historical user-library migration,
other model families/backends/targets, pretrained T3 and hosted candidate CI are
not qualified by these local gates. Pumas's own source receipt explicitly leaves
release/security/native artifact gates open. Parent and Pumas owner decide any
pin adoption and compatible checkpoint; source pinning requires no release tag.
PR78 still consumes `aa1d79ae2d53f0acb088bc85223ecc2ff46af12b`.
