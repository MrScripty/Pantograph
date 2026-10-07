# Current-main connector, cold CPU and inspector native qualifier

This is a separate QA-only branch, `qual/connector-cpu-inspector-current-main-2026-10-07`.
It validates three independent production slices on freshly fetched main
`038dacaaa98ebd007c32e13d4608726ca5ccf63a`. No merge to main is performed.

## Production source and normal ancestry

Combined production HEAD is `85536d3982fc1fe3f6b1c1a90da30793b7537b04`, tree
`5dd020e1becc791d99b8bcae2ae9c3c83e9e193a`. Normal merges retain these sources:

- Connector `0dac67e43537180f5a76312c31b70fd0d8a2d6a1` was merged with main
  `038dacaa` in `cc78968b8579fe421a78ed8e1bde05577a20f176`, tree
  `c58989c5b1e4674d33a7f5421302c250b263e861`. Its main-relative delta is the
  same three accepted connector source/test/report blobs.
- Original inspector PR64 source `2a748f113cbedbeffb0c218d8b2a6936628f49ba`
  was merged in `b45d99cada7200d68f7cb8171f029f8eefc469a5`.
- Original cold CPU PR65 source `6338b71a55371317c5ace6adbeacdec469c96950`
  was merged in combined `85536d39`.

All four source refs are ancestors of the combined production head. The resulting
main-relative delta has eleven paths: three connector, three inspector and five
CPU paths. The current-main audio/rerank additions are inherited through main;
this qualification does not add or resume those feature projects. Auto-merged
runtime facts/composition files retain current main plus the bounded cold owner
changes. Both existing PR heads remain separate and unchanged.

## Preserved QA recipe

The previous qualifier `1f679b5a2888a07188378fde788f6c2f35af2847` is retained as
an ordinary merge parent in QA import `4eaf6eed9d467bae4801f600a23a8048943f5dad`.
The import adds exactly fourteen QA paths and no application or manifest delta
relative to combined production. Only the existing workflow branch/source guard,
step label and this report are adapted. The twelve other imported files are
byte-identical to previous QA, including every native assertion, interaction,
synthetic weight/metadata fixture and scoped output/load/hit test.

The workflow keeps its driver, toolchain/native dependency installation, CPU and
offline model environment, complete all-target normal/build/dev Cargo feature
audits, mandatory `ORT_SKIP_DOWNLOAD=1`, source recording, thirty-minute bound,
full log and artifact retention. It requires exact combined `85536d39` ancestry
before building. ORT must remain dynamic with disable-linking, no
 download-binaries/fetch-models/copy-dylibs/tls-native features and pinned Pumas
`26a84e323cae566a46a8f76bef48fa1010aed48b`.

## Prior failures and unchanged acceptance

Preserve failed main-only run
[37586061519](https://github.com/MrScripty/Pantograph/actions/runs/37586061519),
which stopped before graph save because the cold Candle registration was absent.
Preserve failed combined run
[37593069408](https://github.com/MrScripty/Pantograph/actions/runs/37593069408),
which passed cold stopped CPU registration and native four-node/three-edge
save/reopen, then stopped at rendered edge IDs because `deps-to-infer` lacked
its inference target handle. Its artifact11470176309 has eighteen intact members,
ZIP SHA256 `466bddd115583cae0f6fe40e5fdc6409a471e402cc433a84d7ef009217dea885`.
Earlier successful and failed inspector archives remain unchanged.

The accepted connector restores the registered sidecar handle in controlled
frontend tests. The native run must still prove the actual desktop result.
Retain all gates: actual Tauri IPC/cold owner, graph save/reopen, typed ports and
rendered edges, geometry/glow, ordinary validation update, submission/admission,
dependency resolution, same-run completed Candle CPU attempt, retained scoped
vector provenance and Read, exact selected run, captured inspector graph,
ordinary pointer selection and all eight displayed values matching the actual
artifact and committed CPU oracle.

The production edge component still uses URL glow filters, while the unchanged
native loop after edge IDs requires drop-shadow. This remains a known separate
gate; it is not repaired or weakened in QA. Record the first actual failure and
actual admission/result status, distinguishing unvisited gates from failed ones.
An empty JavaScript failure list is inconclusive if failure precedes listener
installation. Inspect complete saved HTML and screenshots, not truncated log
snippets. Preserve every original member and the full masked job log.

## Qualification boundary

The connector current-main frontend tests, browser rendering and source review
qualify that local slice. Combined runtime tests and the new native outcome must
be recorded against their exact heads; prior tests do not substitute for them.
No application patch, injected registration, bypassed admission or substitute
execution/output may be hidden in QA. No model/runtime binary download, GPU
operation, authentication change, main merge or manual review request occurs.
This controlled synthetic CPU test does not establish real-user discovery,
pretrained quality or full production loader/GPU acceptance.
