# Combined native qualifier for inspector PR 64 and CPU capability PR 65

This qualification branch, `qual/cold-candle-inspector-local-2026-10-07`, starts
at reviewed combined production `b9089dd965844a3873b6d75b8ac2419c1a3a78fd`,
tree `988838ba5b8b527e6c1911b600c48f467a3366a5`. Its parent is the exact
three-file inspector candidate `2a748f113cbedbeffb0c218d8b2a6936628f49ba`
([PR 64](https://github.com/MrScripty/Pantograph/pull/64)). Its second production
commit carries the independently reviewed five-file CPU slice
`6338b71a55371317c5ace6adbeacdec469c96950`, tree
`f31ffcb4fce4a47166d0f8824ce3183d86fc7b24`
([PR 65](https://github.com/MrScripty/Pantograph/pull/65)). Both production slices
start at frozen main `a8483e511dcec4f36e269e6e4debf181a318222f`; their path sets
are disjoint and every combined blob matches its independent candidate.

The CPU slice enrolls the owner-advertised stopped `candle.cpu` capability in
production hosted composition and retains actual CPU device evidence in cold
inference descriptors. It creates no ready model, runtime instance, admission
budget or reservation. QA does not inject a registration or preload a backend.
Qualified source `2590a5337fe30688c8cd29dc60aa2cb518ad9422`, failed main-based
run `37586061519`, its 16 original artifact members and all prior evidence remain
preserved. That run failed before graph saving because cold Candle enrollment
was absent. It does not qualify this combination.

The QA overlay is reused in order from preserved commits
`8717dc69e294ca6dbc89046b13403eba3ad926c7` and
`4f2d7090543c5ec4619b6801cca4daa09aa2cb8d`. Relative to the resulting `4f2d7090`
QA file contents, this qualifier changes only two paths: the existing workflow's
push branch, exact combined-source guard and step label; and this report's
production-source/qualification provenance. All scripts, test helpers, fixtures,
ordinary native interaction and actual acceptance assertions remain byte-for-byte
unchanged from that preserved main-based qualifier. A fresh run must record its
own exact source, graph, execution and display evidence.

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
combined production base. Research-scheduler migration, rerank and audio remain
outside this repair. Beyond the explicit CPU capability production slice, broader runtime/backend
repairs from the older successful qualification branch are not included.

## Acceptance and interpretation

Keep the existing native gates: real Tauri IPC, cold Candle owner and typed-device
discovery, saved/reopened graph, typed ports and painted edges, public scheduler
submission, scoped completed `candle.cpu` timeline, retained vector output and
CPU oracle, exact visible run, captured graph, native pointer selection, exact
artifact card, Read and displayed vector body equality. No application state,
handlers, synthetic events or weakened assertions may substitute for those steps.

The main-based combination differs from the earlier qualified runtime tree, as
documented in the separate production reports. This run can expose a missing prerequisite or
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
