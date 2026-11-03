# CPU composition evidence

Implemented and exercised source:
`6f16d9f6dad7bf763088528addf185a2e6a9cced`, based on main
`2253cbe6fe8f1969b347c61fccba31859b13a4ba`. The subsequent evidence commit
changes only this plan/report/archive; executable source and dependency inputs
remain identical. Linux x86_64, Rust/Cargo 1.92.0, Node 24.12.0. Local execution;
hosted CI is a separate pending check at publication.

## Observed end-to-end result

- Public Chrema `508c8b8cb724b49c655dab8bc4c8336190e516c7` was built and
  audited, then consumed through `agent-platform/journal`.
- Pumas `aa1d79ae2d53f0acb088bc85223ecc2ff46af12b` indexed the seeded
  untrained BERT-8 fixture and produced execution descriptor v1, package facts
  v3, model reference v1 and a valid selected-`main` HF directory load target.
- Candle `88ed7911de9e88196b1f55b199145d22647f415e` executed the actual CPU
  graph after owner validation/publication and dependency readiness. Output
  matched the independent fixture reference within `1e-4`. Weight identity:
  6,464 bytes; BLAKE3
  `48f4dd9c7cfc9c2f7a2c38db3e4397068bfc2a5e6ebd9b16431162d86c732b09`.
- Chrema run `ec714553-73a8-4454-a71b-b27ff8181669` completed both actual
  component assignments. It links Pantograph run
  `run_4f12e882-afb7-4c0c-9ed7-6711d9994d78`, model
  `embedding/qualification/synthetic-bert-8` and retained output
  `workflow-io-77ea02de11098e5c0d62c6e07f0c67ab49e619232005405952f9f5028f339a95`.
- After the producer exited, separate native and JavaScript processes read
  retained attribution/diagnostics, original run graph, library-usage provenance
  and exact output body. Repeated fresh CLI inspection preserved every original
  evidence file hash. The inspector creates no inference runtime or replacement
  run; projection rebuilds occur in its temporary verified copy.
- Native shutdown passed. Zero model acquisition bytes. The Chrema journal is
  an exported memory inspection record, not PostgreSQL persistence or a
  resumable workflow checkpoint. No trained-model usefulness is claimed.

Prepared binary SHA256:
`609d244e186e4c17b085407239265c09b75fee67060da44349ed0a289ae4efd1`.
Root Cargo.lock SHA256:
`42c43401def2b14f41eead4f533f3b48f463a80602271d87602cec705d0ac002`.
The full source/package/binary receipt is inside the bundle.

Archive disclosure check: opened the extracted databases read-only and counted
their actual rows. Attribution `client_credentials`, `clients`, `client_sessions`,
`buckets`, `default_bucket_assignments` and `session_lifecycle_records` all
contain **zero rows**. Credential column names are empty standard schema, not
credential data. Populated records are the isolated synthetic model, one graph
version/validation/run snapshot, 13 diagnostic events and their projections.
The registry has one synthetic library and zero live instances/config rows.
The GitHub connector verified destination `MrScripty/Pantograph` (public,
repository ID 1133193034) and authenticated push/admin permission.

## Evidence and reproduction

[Raw archive](cpu-evidence-6f16d9f6.tar.gz): 177,493 bytes, 53 regular files,
SHA256 `7d2c4d70acc478651d4d9a19d75ea2d198095b39cb98060e8d000fa12b3db654`.
It contains the complete sealed bundle, isolated owner databases, retained bodies,
fixture files, public journal export, preparation receipt, actual run and fresh
inspection results, successful checks and both failed test-setup attempts.
It excludes the 182 MiB native executable and generated Chrema package copies;
their hashes are retained. Archive inspection needs no execution. To execute its
inspector, retain the exact prepared binary/package directory from that run;
recompiling a different binary does not satisfy its original identity receipt.

Local retained prepared candidate:
`/workspace/scratch/demo-prepared-6f16d9f6`.
Local complete bundle: `/workspace/scratch/demo-bundle-6f16d9f6`.
Fresh inspector command:

```sh
node examples/pumas-chrema-cpu-demo/inspect.mjs \
  --prepared /workspace/scratch/demo-prepared-6f16d9f6 \
  --out /workspace/scratch/demo-bundle-6f16d9f6
```

[Fresh-checkout procedure](../../../../examples/pumas-chrema-cpu-demo/README.md)
builds a new identified candidate and produces a new bundle. The archive is
corruption evidence relative to its local hashes, not a signed attestation.

## Verification results

| Command/procedure | Actual result |
| --- | --- |
| `prepare.mjs` at implementation commit, exact public Chrema checkout | package build/audit, locked Candle-only native build and source identity passed |
| `DEMO_PREPARED=... CHREMA_CHECKOUT=... node --test examples/pumas-chrema-cpu-demo/composition.test.mjs` | 13 passed, zero failures/skips; actual CPU run, two fresh CLI reads, pin/dirty-source rejection, timeout/kill/reap, nonzero exit, checksum corruption, crossed model/run provenance, missing stores and reused roots |
| `run.mjs`, then a new `inspect.mjs` process | completed, then verified; original stores unchanged |
| `python3 -m unittest discover -s examples/cpu-embedding-demo -p 'test*host.py'` | existing single/rich native HTTP host contracts: 4 passed |
| `PUMAS_REGISTRY_DB_PATH=/workspace/scratch/demo-embedded-test-registry.db cargo test --locked -p pantograph-embedded-runtime --features host-dependency-inventory --lib` | 598 passed, zero failures, 2 existing ignores; default backend features, isolated registry |
| `npm test` | node-engine 261 passed / 1 existing ignore; workflow-nodes 168 passed |
| affected example `cargo clippy --locked -p pantograph-embedded-runtime --no-default-features --features backend-candle,host-dependency-inventory --example cpu_embedding_workflow_probe --no-deps -- -D warnings` | passed; transitive inference dead-code warnings remain outside this owner lint scope |
| `cargo fmt --all -- --check`, example ESLint, whitespace and staged traceability | passed; traceability had zero mapped authority impacts and does not replace semantic review |
| `python3 scripts/check-onnx-no-build-download.py` | manifest/lock/CI pin and no-build-download metadata checks passed; no ONNX execution claim |

The supporting Rust/HTTP/pre-push checks ran before the implementation commit
with identical affected executable source. Exact-commit prepare, 13 integration
contracts and retained-bundle run/inspection ran afterward. No previous saved
binary, historical log, GPU observation or remote-worker result is substituted.

## Failed attempts and remaining holds

The reduced Candle-only example compiles and runs. The existing embedded-runtime
library test module does not compile under that feature selection: 56 errors
including default-only `InferenceGateway::new` calls. This is recorded as a
failed attempted test matrix, not waived or labeled passing. The default-feature
suite initially ran with the platform Pumas registry path and produced 556
passes, 42 failures and 2 ignores; all failures hit read-only
`/home/agent/.config/pumas`. Isolating its registry inside writable scratch
resolved the setup and produced the successful 598/2 result without a source
change, permission escalation or user-state mutation.

PR75 remains draft at `2f1e38fd846ae089b47c5a2ba4dccf1fcf892cc1`; required
trained-model public text generation T3 is still held by its owner. No qualified
trained weights were available in inspected local model locations. This demo
does not close that gate. PR77 at `20ad4ffa29085a66eecd4506a6e90ecd9b028f85`
was mergeable and a local merge-tree check against the admitted base succeeded;
none of its files or source was reused. PR74/76 are already in the base.

No upstream Chrema change is required. Pumas candidate main
`ef9eead9fe6595f5b3152ef8df19c5dace30190b` was observed separately and has not
been adopted or qualified by this slice. Parent and the Pumas candidate owner
must coordinate any successor pin and compatible release checkpoint. No release
tag is created, and this draft does not authorize merging or releasing.
