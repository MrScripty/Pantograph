# Authoritative Pumas requirements bootstrap repair

Parent: preserved `69e6b4163642349436373781cce730010d1536f6`, tree
`c9384336fb4d5dd6f1bfa07e9e4d9df5b53339be`. Its ancestors `7ec33603` and
`e6ec8112` remain intact. The actual diagnostic run established the cold
requirements seed/snapshot cycle before this repair.

Hosted composition now supplies its existing Pumas selector access to the async
producer. Only a missing registry entry permits authoritative resolution. The
producer checks the canonical requirements ID and host platform, reuses the
selected package model/revision/artifact guards, and accepts only the current
Pumas owner's valid resolved result with no declared bindings. It publishes a
Resolved requirements snapshot for the existing seed consumer and uses inventory
for the Check snapshot. It does not insert registry data itself. The registry
accepts a complete empty validated set, retaining Missing/Unavailable and partial
set rejection. Stale/mismatched registry entries are not replaced by bootstrap.

The real-Pumas regression starts with an actual graph-authored saved/reopened
proof and runs producer -> snapshot provider -> normal seed consumer -> registry
lookup -> Check/preflight. Negatives cover stale IDs and model/revision/artifact/
runtime/device changes, fresh proofs for unavailable model/revision/artifact
facts, read-only access, and real indexed declared bindings unresolved for the
requested backend. Existing registry/producer stale and mismatch tests remain.
No success provider fixture or model metadata change substitutes for resolution.

Local checks pass: 21 dependency-environment tests, 928 workflow-service unit +
60 integration tests, 537 default-feature embedded tests, strict selected-crate
all-target Clippy for all three crates, affected harness ESLint, formatting and
critical gate. Effective Cargo graphs precede each build: current Pumas
26a84e32, ORT load-dynamic, ort-sys disable-linking, no download features, and
ORT_SKIP_DOWNLOAD=1. The initial focused failure used the executor's read-only
default config directory; its retry uses the established writable XDG directories.
The next focused failure records rejection of an incorrectly shaped negative
profile fixture; the final fixture uses Pumas' current python_packages schema.
Both failures are preserved alongside the final successful test and full suites.

Native acceptance still requires the actual save/reopen, typed Resolve, GUI
Submit, automatic scheduling and retained CPU vector/selection metadata. The
harness accepts only a verified scoped dependency-pending response as nonterminal
and waits for output from the same single run; it does not resubmit or create
readiness receipts. Native evidence is pending the exact-source run.

Limitations: current Pumas local-client access has no exposed requirements
resolution operation; cold bootstrap through that route stays unavailable.
Declared dependency profile projection, explicit selected bindings, overrides
and trait intents remain unsupported by this first bootstrap. Empty resolution
is the current owner's indexed-binding result, not inferred from metadata.
Runtime capability, device/resource admission and actual loader checks retain
their existing owners. GPU, pretrained models and full production-loader
qualification are not implied.
