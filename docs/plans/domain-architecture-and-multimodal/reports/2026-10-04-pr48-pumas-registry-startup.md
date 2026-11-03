# PR48 Pumas registry startup diagnosis and test repair

Separate branch: `repair/pr48-pumas-registry-test-startup`.
Source repair: `78d35f9ec3c4d24d092ff92eb7c26a503774b3d6`.
Source tree: `dfe5deaa446530866a0ea2075442ff6f1e1545f1`.
Its direct parent is accepted PR48 head
`ea0378defadebb8665bc964c352166f0fbb15bb7`, tree
`f03ad681e6952266f48a1907e07cb39b05e00328`. All stacked ancestry is retained.
Independent composition review accepted that parent; this repair still needs
parent review and full embedded-runtime qualification.

## Hosted evidence and source identity

[Quality run 37181984567, job 111376229170](https://github.com/MrScripty/Pantograph/actions/runs/37181984567/job/111376229170)
ran the PR merge snapshot `2e9829e8944c9750357c0443a7800aa6e1443a40` of
ea0378de into main `b84d7ec49773ab306432b393b735e4aae2e027b1`. The run API
identifies ea0378de as its PR head and attempt 1. Embedded-runtime finished with
454 passing tests and one failure. At package-facts source line 340, the test
`owner_api_projects_diffusers_package_facts_without_paths` panicked on
`PumasApi::build()` returning `DatabaseBusy`, extended SQLite code 5,
"database is locked". This occurs before explicit index rebuilding and before
Pantograph package-facts projection. The hosted log has no native backtrace or
SQL-statement context.

Cargo.toml, Cargo.lock, the Quality workflow, and package-facts source have no
diff between ea0378de and b84d7ec. The parent reports all four postmerge main
workflows passed. The sibling checkout uses workflow ref 66c0c11, but the Rust
dependency is separately locked to Pumas-Library
`f87c3da8276a914a54c6f4f36d617bef9d9f424e`; the cached checkout HEAD was verified
at that exact commit. Its registry source blob is
`97ff4eb5c50881e4c07f81006df387f44314006a`.

## Reproduced contention

Each owner fixture has an independent temporary model directory. Nevertheless,
the pinned builder unconditionally calls `LibraryRegistry::open()` at the shared
platform database before initializing the model library. Registry startup opens
a new connection, configures WAL/timeout/synchronous/temp-store pragmas, then
ensures the schema. The process-local connection mutex does not coordinate
different builder connections to that shared database.

An offline Rust harness compiled the exact, unchanged pinned registry source
with bundled rusqlite 0.32.1. Threads synchronized at a barrier, each opening
the same fresh registry and registering/claiming/cleaning up a different library
path. All SQLite operations used the real pinned code and SQLite engine.

| Diagnostic run | Concurrent cold opens | Initialized-database control | Serial cold control |
| --- | --- | --- | --- |
| 100 batches, 128 threads | 2 failures / 12,800 operations | 0 / 12,800 | 0 / 100 |
| 200 batches, 128 threads, captured backtraces | 7 failures / 25,600 operations | 0 / 25,600 | 0 / 200 |

Every reproduced failure returned the same DatabaseBusy/code-5 error while
opening the registry. Captured backtraces locate it in
`LibraryRegistry::configure_connection`, before schema completion. This
demonstrates a real cold registry configuration race exposed by parallel owner
fixtures. It is the best-supported explanation for the hosted failure; the
hosted log alone cannot establish that its failing SQL statement is identical.
No Pantograph production lifecycle regression was demonstrated.

## Repair and exact source write set

One cfg(test) helper prepares the platform registry through OnceLock, retains
that connection for the test process, then returns the unchanged Pumas builder.
All 27 existing owner-fixture builder sites use it. Only initial shared registry
preparation is coordinated; API builds and test bodies still run concurrently.
Errors still fail tests. No retry, error suppression, timeout change, dependency
change, environment mutation, or assertion weakening was introduced.

The source commit changes exactly these ten paths under
`crates/pantograph-embedded-runtime/src/`:

- `lib.rs`: cfg(test) module declaration.
- `pumas_test_support.rs`: new shared preparation and real-registry regression.
- `lib_tests.rs`: one builder reference.
- `lib_tests/edit_session_execution_tests.rs`: four builder references.
- `lib_tests/workflow_run_execution_tests.rs`: one builder reference.
- `pumas_dispatch_package_facts.rs`: three builder references.
- `runtime_host_execution_port.rs`: one builder reference.
- `task_executor_tests/puma_lib.rs`: five builder references.
- `technical_fit.rs`: two builder references.
- `workflow_service_composition.rs`: ten builder references.

After reversing only those references and the module declaration, all nine
existing files are byte-identical to ea0378de. Every edited call is in existing
test-only code. Production behavior and all existing projection, ownership,
read-only-access, lifecycle, and cleanup assertions are preserved. This report
is the sole additional path in the documentation successor.

## Executed checks and limits

Tools: Node 24.12.0, npm 11.6.2 and Rust 1.92.0 via the existing activation file.
No speed/credit, credential, permission or network setting was changed.

- The checked-in regression passed in the isolated Rust harness: one test,
  eight fresh databases and 32 concurrent owners per database (256 operations).
  It executes the checked-in preparation helper and the real pinned registry,
  verifies successful instance claims/cleanup and all registered owner rows.
- `node --test scripts/*.test.mjs`: 63 passed, zero failures/skips/cancellations.
- `cargo fmt --all -- --check` and whitespace checks passed.
- Explicit ea0378de-to-source `npm run lint:no-new` passed: critical checks,
  all 27 scanner cases, accessibility scan, and traceability (10 paths,
  zero mapped impacts). Audit/traceability gates and workflow files are intact.

The isolated harness supplies error/platform support and compile-only API
builder signatures; no API builder behavior is executed there. It verifies
real registry/SQLite behavior, not the full Pumas API or Pantograph test binary.
Full `cargo test -p pantograph-embedded-runtime --lib` was not executed locally.
The existing ORT/CDN restriction remains; no download retry or bypass was made.
Native ORT qualification remains parent-owned. Genuine GUI/WebKit, native IPC,
native package loading, C#/BEAM hosts, model loading and inference were not run.
Earlier frontend qualification remains prior evidence, not a new run here.

Local supporting evidence is in `/workspace/pantograph-cache/`:
`pr48-registry-stress.log`, `pr48-registry-backtrace.log`,
`pr48-registry-repair-regression.log`, `pr48-registry-tooling.log`,
`pr48-registry-fmt.log`, `pr48-registry-range-gates.log`, and
`pr48-hosted-failure-excerpt.log`. The harness and its locked dependencies are
in `pr48-registry-repro/`; `diagnostic-main.rs` preserves the failing harness.
PR48 metadata, its branch, bot requests and merging remain untouched.
