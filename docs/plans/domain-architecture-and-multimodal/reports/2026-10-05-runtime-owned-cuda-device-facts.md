# Runtime-owned CUDA identity observations

This bounded successor preserves RAM repair `43f0a777` and service timing
`0dff986d`. It observes one existing owner: the embedded PyTorch interpreter used
by Pantograph execution. PR55, Pumas pins, timing qualification, ranking and full
peak resource admission are unchanged.

## Existing owners and chosen boundary

The pinned Pumas `a94fd92021f27fdeedb6e2de6e01c41c250ef576` GPU monitor queries
`nvidia-smi`, retains only the first result, and exposes aggregate usage, memory,
temperature and name. It exposes no UUID/ordinal map; probe failures can return
zero-valued default records. Those records cannot establish known-zero capacity
or physical identity for scheduler admission. Pumas runtime profile
`CUDA_VISIBLE_DEVICES` settings describe configuration, not measured placement.
This checkpoint neither modifies that owner nor creates a second host GPU monitor.

Pantograph's existing llama.cpp listing projects backend-local selectors and
reported MiB into canonical selector facts. Those selectors are not stable hardware
identities, and the legacy listing parser defaults malformed/absent memory to zero.
It is therefore not promoted into a physical-capacity authority here. Existing
PyTorch boolean device probes establish runtime-class readiness, not physical
inventory. Automatic candidate discovery continues to advertise CPU only.

The chosen supported path is an explicit gateway call to the PyTorch backend's
one-shot CUDA inventory observation, using the same PyO3 interpreter as execution.
It queries the runtime's visible device count and actual device properties. The
[PyTorch device-count contract](https://docs.pytorch.org/docs/stable/generated/torch.cuda.device_count.html)
reports devices available to that runtime. The [official CUDA property binding](https://github.com/pytorch/pytorch/blob/v2.10.0/torch/csrc/cuda/Module.cpp)
exposes UUID bytes and total memory; both are read directly, without parsing a name
or correlating ordinals with nvidia-smi. No new dependencies or subprocess probes
are added.

## Exact integration contract and unavailable behavior

- `InferenceGateway::observe_pytorch_cuda_inventory()` explicitly delegates to
  `PyTorchBackend::observe_cuda_inventory()` on a blocking Python task. Nothing
  runs in gateway construction, CPU discovery, admission or the timing hot path.
- Supported positive observations require a non-HIP PyTorch CUDA runtime that is
  already initialized. The probe refuses the property API's lazy-initialization path. An uninitialized
  runtime returns a typed unavailable state. The initialized flag is not proof of
  resource custody, and this does not qualify driver/context effects on real GPU
  hardware.
- At most 256 runtime-visible devices are enumerated. A failed property query
  invalidates the entire enumeration. Import/probe/task failures, unsupported HIP,
  invalid counts and unavailable CUDA have distinct typed unavailable reasons.
- `cuda:N` denotes this embedded runtime's visible ordinal, never host/nvidia-smi
  ordinal N. Physical identity comes only from exactly sixteen non-nil UUID bytes.
  Missing, malformed or unsupported UUID properties remain unavailable. Duplicate
  UUIDs make every duplicate ambiguous, rather than collapsing distinct rows.
- Total memory is an optional byte observation; missing/malformed differs from
  observed zero. It does not establish free memory, an admission ceiling, backing
  pool capacity, an external-consumer allowance or a residency discount.
- The facts have no admission or residency authority and are not timing owner
  fingerprints. A timing owner must independently verify its loaded generation,
  implementation, effective configuration and actual execution device before
  using a current UUID in its fingerprint. PyTorch can cache its CUDA runtime view;
  repeated calls are observations of that view, not host hotplug qualification.
- Distinct runtime/device UUIDs, including partitions, do not prove distinct
  physical backing pools. Any future CUDA candidate integration requires the
  owner to resolve loaded placement plus authoritative shared backing and fresh
  capacity facts. No mapping is guessed from GPU classes, names or counts.

This is a reachable owner observation path, not an invented GPU candidate or a
new scheduling policy. It has a meaningful native negative path here, so no
untestable replacement hardware owner or generic sampling framework is added.

## Qualification

Three deterministic owner-probe tests use local Python modules without replacing
`torch` in `sys.modules`. They cover UUID-to-visible-ordinal mapping and reorder,
known-zero versus missing total bytes, malformed/nil/duplicate identity, failed
partial enumeration, unavailable CUDA, initialized-state fencing, HIP rejection,
invalid counts and probe errors. Positive physical identity cases are controlled
fixtures and do not qualify actual GPUs.

A separate explicitly invoked native integration-test executable imports actual
installed PyTorch in the execution interpreter, reaches the public gateway route,
and verifies current backend and CPU candidates are unchanged. The observed
runtime is `2.14.1+cpu`; its CUDA availability is false and device count is zero.
The gateway returns `unavailable/cuda_unavailable`. This qualifies a real negative
owner observation only; it does not prove host GPU absence or known-zero VRAM.
The native test is ignored by default so ordinary portable CI does not pretend
that a missing optional PyTorch installation is qualified.

All 726 serial inference library tests and all 66 focused embedded host tests
pass. The full embedded suite executes 502 tests: 500 pass and the same two
descriptor-count and warmup-timeout baseline failures remain explicit, with their
assertions and timeout values unchanged. Mixed-backend
warning-deny all-target Clippy, default-feature inference check, formatting,
critical/accessibility/traceability gates, 28 traceability tests and nine ONNX
no-build-download graphs pass. The default-feature check retains a dead-field
warning in unchanged `SelectedTextLoad` code with PyTorch disabled. GTK/WebKit,
real GPU UUIDs, GPU execution, per-device capacity,
backing-pool bindings, model performance and cross-platform native execution remain
unqualified. Actual RAM capacity still fails closed in this container's `/..`
cgroup mount layout. Completion-oriented ranking remains subsequent work requiring
comparable qualified timing and physical-feasibility evidence.
