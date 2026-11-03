# Exact-head native text-control qualification

## Source and actual execution

Requested source: `a8c6970f2e8c892abc0ce5dfec7ec8051aaff243`, tree
`27703dd3994b402d3ddc70bfa94572d0b6931f25`. Existing top_k, temperature,
top_p and CI corrective milestones remain unchanged. This successor adds a
reproducible qualification route; it does not repair or change runtime behavior.
No product integration failure could be evaluated because the native binaries
could not link. Parent retains publication/review coordination.

The newly selected execution environment became ready on 2026-10-05. At the
owner's access checkpoint, `pwd` and UTC date succeeded with exit 0 at
16:39:59 UTC. Work remained available and was preserved.

| Executed attempt/check | Result |
| --- | --- |
| Embedded runtime text-host tests, normal defaults plus backend-pytorch, ORT_SKIP_DOWNLOAD=1 | Exit 101: missing ONNX link configuration and OrtGetApiBase; zero tests executed |
| One standard locked embedded build in the newly ready environment, with ORT_SKIP_DOWNLOAD unset | Exit 101: pinned ONNX Runtime 1.24.2 download returned HTTP 403; zero tests executed |
| Workflow input-mapping native test binary with ORT_SKIP_DOWNLOAD=1 | Exit 101: the same ONNX linker failure; zero tests executed |
| Shared serialization/contracts | 74 passed: 22 interface, 45 runtime-host unit and 7 accessor tests |
| Qualification script wrong source guard | Exit 2 before any suite executes |
| Full qualification-script gate fixture | Wrong head 2; empty discovery 1; failed Rust command 7; real CPU regression 1; corrected real CPU suite 0 |

The standard dependency request was:

```text
https://cdn.pyke.io/0/pyke:ort-rs/ms@1.24.2/x86_64-unknown-linux-gnu.tar.lzma2
ort-sys 2.0.0-rc.12: http status 403
```

No repeated download, substitute runtime, dependency pin change, system package
installation or model-weight download followed. Local native host/workflow
qualification remains blocked. ORT_SKIP_DOWNLOAD does not produce executable
native proof when the linker is missing these symbols.

## Reproducible route and failure propagation

Run from a clean checkout using a CPU Python environment:

```sh
PANTOGRAPH_QUALIFICATION_PYTHON=/absolute/path/to/cpu-venv/bin/python \
  bash scripts/qualify-workflow-text-controls.sh FULL_COMMIT_SHA /tmp/text-control-evidence
```

The script requires the exact full HEAD SHA, unchanged tracked source and no
untracked files under crates/scripts/.github. It executes the full native text
host module, descriptor module, workflow host-input mapping, authored source
materialization and public-session module. Discovery and result checks prevent
empty or ignored-only success. Shared wire serialization tests and all six real
CPU sampler tests follow. Source identity, tool versions, dependency skip mode,
test logs and successful-run checksums are recorded; source is checked again
after execution. Every test pipeline explicitly uses Bash pipefail.

The existing Quality Gates workflow gains an independent focused job, avoiding
dependence on unrelated broad test failures. It checks out the exact PR head SHA
or push SHA, installs the existing Ubuntu prerequisites and Rust 1.92/Python
3.12, provisions CPU Torch and the established Transformers constraint, and
executes this script with locked Cargo dependencies. It honors the Cargo-pinned
Pumas source directly, without modifying pins or relying on the workflow's
older sibling-checkout revision. The required quality-summary gate includes
this job; evidence uploads even when native provisioning or a test fails.
The job becomes executable through the parent's centrally managed PR/push
publication. No PR, review request or hosted run was created here.

The full script was tested in an isolated git fixture with **controlled Rust
commands**, not real host/workflow execution. It used the actual six CPU tests
and installed Torch 2.14.1+cpu/Transformers 4.53.3. The frozen e0293ebf sampler
produces 112 real assertion failures and full-script exit 1 despite tee; the
corrected sampler passes six tests and returns exit 0. This verifies sampler
failure propagation without claiming the controlled Rust statuses as native
results. The same fixture verifies wrong-head, empty-discovery and failing
native-command stops.

Logs: `/tmp/text-controls-native-host.log`,
`/tmp/text-controls-native-standard-host.log`,
`/tmp/text-controls-native-workflow.log`,
`/tmp/text-controls-serialization-tests.log`,
`/tmp/text-controls-wrong-head.log`, `/tmp/text-controls-gate-check.log` and
`/tmp/text-controls-gate-evidence/`. The gate harness is retained at
`/tmp/text-controls-gate-check.py` for review/reproduction. YAML structure,
shell syntax, formatting, whitespace, critical, scheduler-only and staged/range
traceability checks pass. Hosted provisioning and real native execution remain
pending; historical compilation and CPU logs do not close those gaps.

## Next concrete planned feature

The next user-facing milestone in [DA-03](../plan.md) is the desktop-authored
dependent text-to-image graph: a real text model's generated prompt feeds a
real image model, and both retained outputs carry correct run/task identity.
EX-04 already admits the dependent scheduler path; more sampling ports do not
establish that feature. First qualify current owner-produced Pumas identities,
facts, load targets and devices at the existing pin `2243a2b6`, then the saved
desktop graph and Tauri runner. DA-I03 records these unresolved prerequisites.
No model identities or artifact paths are guessed and no model run is claimed.
