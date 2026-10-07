# Native CPU inspector snapshot repair

The isolated source branch is `fix/native-inspector-projection-load`, based on
`b547a3bbadfd1ed8f851444f6dd542c03c957931` (tree
`1cd5d3ea8bac995964f07d6a88ad1a490b065f60`, parent
`cd5f512c6febc371ba876a27f33b3914b3a26594`). No applicable AGENTS or local skill
instructions were found. Existing worktrees and their pending edits are preserved.

## Diagnosis and source change

Failed native [run 37576570678](https://github.com/MrScripty/Pantograph/actions/runs/37576570678),
job `112646694609`, executed that exact base. Its captured inspection response
contains a current I/O projection, four graph nodes, three edges and three artifact
records, but omits `node_statuses`. The Rust inspection response explicitly uses
`serde(default, skip_serializing_if = "Vec::is_empty")` for this field. TypeScript
instead declared it as a required array, and the inspector assigned it directly.
The derived graph presenter then iterates `undefined` and throws
`TypeError: statuses is not iterable`.

This failure is reproduced by replaying the actual captured response through the
graph presenter. The service now normalizes omitted `node_statuses`, `io_artifacts`
and `retention_summary` collections to empty arrays. Those three fields share the
same Rust wire omission contract and are required by inspection consumers. Graph,
artifact, projection and nonempty collection identities are preserved. Missing
node statuses remain unknown; the repair does not manufacture completion records.

There is also a separate WebDriver observation defect. The failed screenshot and
DOM contain selected run `run_e4c74e6e-8819-4309-94da-fcd907ba8cd9`, while the
whole-inspector `getText()` response excludes its visible header text. The native
test now waits for the displayed run header and checks its DOM text. Its subsequent
visible graph node, exact artifact card, Read button, displayed preview and actual
vector equality assertions remain intact. JavaScript failures and inspector DOM
state are captured before the after-test backend queries. Tauri IPC is untouched.

## Local verification and limits

- All 62 focused service, presenter, subscription and qualification tests pass.
  Added regressions cover omitted inspection collections, preservation of populated
  collections, a second run with empty collections, and diagnostics with Tauri's
  read-only invoke property.
- Typecheck, targeted ESLint, accessibility, critical anti-pattern and Git whitespace
  checks pass.
- Controlled replay of the original response now accepts the four-node, three-edge
  graph and the vector sink I/O row, with freshness `Current at seq 12`.
- A controlled replay using the actual compiled Svelte script and captured IPC
  response reproduces the baseline derived-presenter exception. With the repaired
  service it finishes loading, retains the current projection and three artifact
  records, and exposes an empty status map without an inspector error. This probe
  exercises component script and derived state, not mounted markup.

The native log did not capture the inspector request lifecycle or JavaScript
exception. The captured producer/consumer mismatch is a demonstrated source defect;
these controlled tests do not prove native rendering acceptance. No native rerun,
public write, Cargo build, model/runtime download, credential or network change was
performed. Chromium could not start with its sandbox in this executor, and no
sandbox configuration was changed. GPU and pretrained production loader qualification
are outside this repair.

The original artifact `11463153287` is preserved locally with all 25 members and the
full masked job log. Archive SHA-256:
`a6e444c4bba05150da9deb71ba9a4bcded70cb6e067a4959907406eba34942a9`.
The local evidence directory is
`/workspace/qualification-evidence/native-desktop-cpu/inspector-projection-diagnosis`.
It retains original member hashes, screenshots, runtime/output records, replay
script and check logs. Original evidence has not been replaced by repaired output.

Before an authorized native rerun, acceptance remains: select the same completed
run visibly, render its captured graph, select `vectors`, show the exact retained
artifact card, use Read, and display all eight actual finite values matching the
retained vector body and CPU oracle. The scoped completed `candle.cpu` timeline
assertion must also pass. Exported vectors alone do not satisfy display acceptance.
Independent scheduler cleanup is unchanged.
