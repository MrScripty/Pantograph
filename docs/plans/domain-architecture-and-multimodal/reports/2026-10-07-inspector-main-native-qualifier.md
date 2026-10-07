# Separate native qualifier for inspector PR 64

This qualification branch, `qual/inspector-snapshot-main-2026-10-07`, starts at
production candidate `2a748f113cbedbeffb0c218d8b2a6936628f49ba`, tree
`b51ec2fc3b5983c6f5e8f40bb75984dcd997bb4d`, parent main
`a8483e511dcec4f36e269e6e4debf181a318222f`. The production draft is
[PR 64](https://github.com/MrScripty/Pantograph/pull/64). Qualified source
`2590a5337fe30688c8cd29dc60aa2cb518ad9422` and its evidence remain preserved.

## Complete QA-only delta

The qualifier adds the existing 12 QA paths from the successful qualified source:

- `.github/workflows/native-desktop-cpu-qualification.yml`;
- `scripts/check-native-desktop-cpu-prerequisites.py`,
  `scripts/prepare-native-desktop-cpu-fixture.py`,
  `scripts/run-native-desktop-cpu-qualification.sh`;
- eight files under `tests/e2e/native-desktop-cpu/`: the spec, WebdriverIO config,
  output-contract helper/tests, load-diagnostic helper/tests and hit-diagnostic
  helper/tests.

Five imported QA files are adapted. The existing workflow keeps its name,
job, supported driver, dependency/toolchain installation, complete feature audits,
offline model environment, ORT download guard, source/evidence recording and
artifact retention. Its push filter targets this separate branch and its source
ancestor check requires the reviewed production candidate instead of the older
chat qualification ancestor. The inspector spec finds the existing visible
run-header div adjacent to its heading, scoped inside `io-inspector-page`, and
still checks the exact selected run through displayed DOM text. This avoids
adding the old QA-only header test marker to production markup.
The read-only load diagnostics use that same existing header locator, with the
diagnostic regression fixture adjusted accordingly. The helper's original marker
would omit the header field on main; independent review identified this mismatch
before publication. Failure observation, pending-state distinctions and IPC
immutability assertions are unchanged.

The output-contract regression previously read an older observation from the
qualification branch's documentation evidence, which is absent on main. Its
initial local test fails with ENOENT; that failure log remains preserved. The
test now reads one additional QA fixture containing the exact run record and
artifact-query excerpt from successful native run `37582221446`, with original
source, archive and member hashes. All existing output/negative-scope/runtime
assertions remain unchanged. This captured fixture is regression input and does
not establish native acceptance for the new main-based tree.

The fixture and this report make fourteen paths. There are no changes under `src/`, `packages/`,
`src-tauri/` or `crates/`, and no manifest or lockfile changes relative to the
production candidate. Research-scheduler migration, rerank and audio remain
outside this repair. Broader runtime/backend repairs from the older successful
qualification branch are not included.

## Acceptance and interpretation

Keep the existing native gates: real Tauri IPC, cold Candle owner and typed-device
discovery, saved/reopened graph, typed ports and painted edges, public scheduler
submission, scoped completed `candle.cpu` timeline, retained vector output and
CPU oracle, exact visible run, captured graph, native pointer selection, exact
artifact card, Read and displayed vector body equality. No application state,
handlers, synthetic events or weakened assertions may substitute for those steps.

Main differs from the earlier qualified runtime tree, as documented in the
production integration report. This run can expose a missing prerequisite or
application behavior independently of the snapshot normalization. Record the
first actual failure and preserve all available source, logs, DOM, screenshot,
graph and artifact evidence. A failed prerequisite or exported vector alone
does not establish inspector display acceptance. Any needed production repair
must be separately scoped and reviewed rather than hidden in QA changes.

No model/runtime binary download is authorized. The complete effective Cargo
graph must retain `load-dynamic`/`disable-linking`, reject download features and
retain the current Pumas pin; `ORT_SKIP_DOWNLOAD=1` remains mandatory. Ordinary
official toolchain/native dependencies use the existing workflow. This controlled
synthetic CPU run does not qualify real-user discovery, GPU, pretrained models
or the full production loader. No merge or manual CodeRabbit request is made.
