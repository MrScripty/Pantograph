# PR44 native/workflow qualification

Status on 2026-10-04: aggregate inference repair ready for independent review;
native/workflow qualification remains blocked. This report records local
execution, not a new hosted Actions run or model acceptance.

## Exact source and repair

- Requested PR44 base: `ca9edde6850cd58ade0b7e534bb4f58e704c2c64`.
- Repair: `572ccbc45c0030a7eef9938702c4c3b2f9b97d27`.
- Repair tree: `265d802354146b20bd1bf3bd6d4d3e350f3589c3`.
- Branch: `qualification/pr44-native-workflow`.
- Repair parent is exactly the requested base; full stacked ancestry is retained.
- Pumas dependency remains pinned to `f87c3da8276a914a54c6f4f36d617bef9d9f424e`.

The repair changes five files with 61 insertions. A shared test-only Tokio mutex
protects 52 Python fixture lifetimes across text, image and worker-contract
suites. Synchronous tests acquire it before the GIL; two asynchronous lifecycle
tests retain it across worker execution. The GIL can be released during imports
and worker execution and does not isolate process-global `sys.modules` or
patched worker functions. Runtime behavior and existing assertions are unchanged.
Quality Gates adds the full PyTorch-enabled inference library run while retaining
all focused checks, audit and traceability gates.

The requested base failed seven of 682 tests in the ordinary parallel aggregate;
all 682 passed serially. Failures included replaced/missing module symbols,
altered envelope validation and worker-state interference. This demonstrates a
failure present at the requested head, not its historical introducing commit.

## Executed checks

| Check | Result |
| --- | --- |
| Baseline parallel inference aggregate | 675 passed, 7 failed |
| Baseline diagnostic serial inference aggregate | 682 passed |
| Repaired default parallel inference aggregate | 682 passed |
| Rebuilt inference binary, three separate 16-thread runs | 682 passed in each run |
| Inference library and tests, Clippy with warnings denied | Passed |
| Fourteen domain packages, unit/integration/doc targets | 566 passed, zero ignored |
| Node-engine and workflow-nodes, including doctests | 426 passed; one unit test and 14 doctests ignored |
| Frontend discovery, including command compatibility | 547 passed |
| All tooling test files | 45 passed |
| Full frontend lint, TypeScript, Rust formatting and diff whitespace | Passed |
| Production dependency audit | Zero vulnerabilities |
| Installed dependency tree | Passed; optional native addons absent |
| Critical lint, accessibility and explicit base-range traceability | Passed |
| Staged repair critical lint/accessibility/traceability | Passed, five paths and zero mapped impacts |
| Canonical image-workflow and scheduler-only guard scripts | Passed |

The frontend invocation included command-related arguments but ran the complete
discovered suite; it was not a filtered run. Aggregate domain packages were
`pantograph-scheduler`, `pantograph-runtime-host-contracts`,
`pantograph-diagnostics-ledger`, `pantograph-media-conversion`,
`pantograph-dependency-environment-service`, `pantograph-dependency-planning`,
`pantograph-managed-dependencies`, `pantograph-runtime-registry`,
`pantograph-runtime-attribution`, `pantograph-runtime-identity`,
`pantograph-node-contracts`, `pantograph-inference-interface-contracts`,
`pantograph-path-security` and `pantograph-timing-contracts`.

The source qualification commands include:

```bash
cargo test --locked -p inference --features backend-pytorch --lib
cargo clippy --locked -p inference --features backend-pytorch --lib --tests -- -D warnings
cargo test --locked -p node-engine -p workflow-nodes
node --test scripts/*.test.mjs
npm run lint:full
npm run typecheck
npm audit --omit=dev --audit-level=high
npm ls --all
cargo fmt --all -- --check
TRACEABILITY_STAGED_ONLY=1 npm run lint:no-new
```

The earlier complete-range gate explicitly compared main
`4938e405c7f656365eefdca492774ccae110c90d` with the requested PR44 base.

## Environment and native blockers

The prepared environment is Debian GNU/Linux 13.6, with pinned Node 24.12.0,
npm 11.6.2, Rust 1.92.0 and Python 3.12.3. Builds used the existing two-job CPU
configuration and a 600-second timeout. In this environment, activate
`/workspace/pantograph-tools/activate.sh` before running checks. PyO3 test
execution also required the process-local `LD_LIBRARY_PATH` to include the
existing `/workspace/pantograph-tools/python/cpython-3.12.3-linux-x86_64-gnu/lib`.
The first inference executable exited 127 without that loader path; its build
had succeeded. No Python installation or system configuration was changed.

`cargo test --locked -p pantograph-workflow-service` stopped in `ort-sys` while
fetching ONNX Runtime 1.24.2. No workflow-service tests executed.
After the parent reported the approved network allowance saved and republished,
exactly one follow-up HEAD request at 2026-10-04T02:53:35Z used a 15-second
connect timeout, 45-second total timeout, zero retries and no redirect following:

```text
https://cdn.pyke.io/0/pyke:ort-rs/ms@1.24.2/x86_64-unknown-linux-gnu.tar.lzma2
curl exit: 56
CONNECT proxy status: 403 Forbidden
Origin response: none (http_code 000)
```

The existing task remained denied. No further request or native build followed.
A fresh environment with the updated policy is required before retrying.

`scripts/check-tauri-command-state-tests.sh` stopped building `glib-sys` because
`glib-2.0 >= 2.70` pkg-config metadata was unavailable. Zero of its five IPC tests
executed. Read-only package and pkg-config probes confirmed these missing native
prerequisites from the repository's shared Ubuntu installer:

```text
libayatana-appindicator3-dev
libglib2.0-dev
libgtk-3-dev
libjavascriptcoregtk-4.1-dev
librsvg2-dev
libsoup-3.0-dev
libwebkit2gtk-4.1-dev
libxdo-dev
protobuf-compiler
```

`build-essential`, compilers, make and `libssl-dev` are installed. The functional
`pkg-config` command is supplied by installed `pkgconf`/`pkgconf-bin`; the absent
`pkg-config` package name is not a missing tool. `protoc`, `tauri-driver`,
`WebKitWebDriver`, `Xvfb`, `dotnet`, `elixir` and `erl` were not found.
Neither DISPLAY nor WAYLAND_DISPLAY was set. No system packages were installed;
the repository installer targets Ubuntu, while this host is Debian.

## Acceptance limits and handoff

No actual desktop IPC round trip, GUI, model loading or model inference was
executed. Existing Tauri fixtures use MockRuntime dispatch; Python contract and
lifecycle fixtures use stubs, including tests whose names mention production or
real loaders. Passing these tests does not prove inference acceptance.
Model-dependent smokes still require canonical Pumas model/artifact IDs, a saved
image workflow and native/runtime prerequisites. No model downloads were made.
C#/BEAM/package acceptance, the full workspace feature matrix and native linkage
were not locally executed. Previously green CI was reported by the parent,
not independently re-queried here.

Local detailed logs and the full text handoff remain in
`/workspace/pantograph-cache/qualification-*`. In particular, the before/after
logs are `qualification-inference-retry.log` and
`qualification-inference-fixed.log`; the blocker evidence is
`qualification-workflow-service.log`, `qualification-tauri-ipc.log`,
`qualification-onnx-policy-check.log` and its `.headers` companion;
`qualification-native-prerequisites.txt` records the read-only inventory.
These local files are not assumed to transfer to a fresh environment; this
tracked report preserves their results and exact source identities.

The original verified `pr44-native-qualification.bundle` remains in the local
cache and contains repair 572ccbc on this branch, requiring the exact PR44 base.
This report is a documentation-only follow-up to that unchanged repair. The
parent owns independent review and coordinated integration. No PR44/47 metadata
or review requests are part of this publication. No credentials, permissions,
network/security, speed or paid settings were changed.
