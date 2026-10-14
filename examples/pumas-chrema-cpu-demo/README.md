# Pumas -> Pantograph -> public Chrema, on CPU

This example composes two host components with Chrema's public journal API:
native embedding execution and retained-state inspection. Pumas indexes the
committed untrained BERT-8 model, supplies its descriptor/package facts/load
target, and Pantograph validates, publishes and executes the saved graph through
its scheduler. The actual vector must match the fixture's independent numerical
reference. A second native process reopens SQLite attribution/diagnostics and
artifact bodies before Chrema can mark the composition complete.

**Qualification:** actual numerical CPU inference with an untrained fixture.
There is no trained-model usefulness claim. PR75's trained-text T3 gate remains
separate. This host supports Linux CPU, Rust 1.92.0 and the repository's Node
24.12.0. It needs no Python environment, model download, API key or database
server. Build prerequisites and Cargo dependencies must already be available.

## Run from a fresh checkout

Use a clean public Chrema checkout at the exact revision in
[compatibility-lock.json](compatibility-lock.json). Do not reuse private material
or substitute the current main branch. Prepare explicitly builds and audits the
public package and builds Pantograph with `--locked` and only
`backend-candle,host-dependency-inventory`. It copies the built executable and
records source, package, lockfile, fixture and binary hashes. It copies the full
CPU fixture input set and passes that sealed directory to the native producer;
execution does not read model inputs from the original compile-time checkout.
It never installs tools or weights. Cargo may acquire its pinned dependencies
using the configured cache.

From the Pantograph repository, with the pinned toolchains on PATH:

```sh
git clone https://github.com/MrScripty/Chrema.git /absolute/path/chrema-public
git -C /absolute/path/chrema-public checkout --detach 508c8b8cb724b49c655dab8bc4c8336190e516c7

node examples/pumas-chrema-cpu-demo/prepare.mjs \
  --chrema /absolute/path/chrema-public --out /absolute/path/new-prepared
node examples/pumas-chrema-cpu-demo/run.mjs \
  --prepared /absolute/path/new-prepared --out /absolute/path/new-evidence
node examples/pumas-chrema-cpu-demo/inspect.mjs \
  --prepared /absolute/path/new-prepared --out /absolute/path/new-evidence
```

Output directories must be new and absolute. Wrong revisions, dirty Chrema
source, changed Pumas/Candle resolution, corrupt prepared files or reused roots
refuse execution. Updates require an explicit lock update and rerunning the
integration contracts; mutable upstream heads cannot silently change this demo.
The source revision plus source hashes identify a prepared build, including any
local source edits. Run uses that immutable prepared candidate and verifies its
files; it does not rebuild from the current checkout or accept another binary.

## What remains inspectable

`bundle.json` links the Chrema run, Pantograph run, model and retained vector
artifact. Its checksums bind the exported inspection record to:

- `chrema-journal.json`: actual public Chrema version, assignments, events,
  terminal outcome and summary. This is an exported memory-journal inspection
  record, not a live PostgreSQL journal or continuation checkpoint.
- `native/pumas-model.json`: owner-produced model index record, execution
  descriptor v1, package facts v3, model reference v1 and resolved HF load target.
- `native/result.json`: actual native scheduler response, fixture weight hash,
  numerical-reference check, zero acquisition bytes and successful shutdown.
- `native/attribution.db`, `native/diagnostics.db`, saved graph/validation files
  and `native/artifacts/`: original owner state, including retained input and
  output bodies. `retained-inspection.json` contains real reopened projections.
- `dependency-receipt.json` and component stdout/stderr: consumed commits,
  source/lock/package/binary identity and process outcomes.

The inspector verifies the sealed bundle, then reopens a temporary verified
copy of the owner stores with the same prepared binary. Projection rebuilding
mutates its copy, while repeated inspection preserves original evidence bytes.
It creates no inference runtime and never resubmits the workflow. Checksums
detect corruption relative to the local receipt; they are not a signature or
attestation against someone rewriting the entire receipt and bundle together.

The default native deadline is 60 seconds. `run.mjs --timeout-ms <1..60000>`
can shorten it. A failed or timed-out component yields a failed Chrema assignment
and run, retains diagnostic files, and exits nonzero. A deadline terminates the
owned process group, escalates to SIGKILL after one second and awaits exit.
An abruptly killed native process cannot certify native shutdown; its partial
owner stores remain evidence, and no inspection/completion claim is made.
Leader exit does not cancel the grace/escalation obligation. The helper awaits
the direct child and observes that its Linux process group has no executing
members; orphan zombies are terminal execution state, not active workers.
Evidence and prepared trees reject symlinks in every ancestor and descendant.
The private inspection copy is dereferenced and checked against the original
file hashes before owner APIs open it.
Earlier exact prepared receipts remain valid for retained-state inspection,
which reads no model fixture. New execution requires sealed fixture inputs;
older preparations must be rebuilt before producing another workflow.

## Integration contracts

```sh
DEMO_PREPARED=/absolute/path/new-prepared \
CHREMA_CHECKOUT=/absolute/path/chrema-public \
node --test examples/pumas-chrema-cpu-demo/composition.test.mjs
```

The suite requires the actual selected public package and native binary; missing
dependencies fail setup. It covers CPU output, fresh-process reopening, exact
model/run/artifact links, immutable repeated reads, dependency drift/dirty source,
failure logs, termination escalation, corruption, crossed provenance, missing
stores and reused roots. A dedicated PR CPU workflow builds these dependencies,
runs the suite and uploads a complete demonstration bundle.

[Scoped plan](../../docs/plans/pumas-chrema-cpu-composition/plan.md)
