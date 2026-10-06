# Native bootstrap diagnostic capture candidate

Parent is preserved e6ec81121344901ba0e9f8a00ca2cb136276f08b, tree
eca913968689269472de73e5b40a03948b94106c. This successor adds actual-owner
warning records for graph action provider results, rejected requirements seeds,
queued/deferred task transitions and failed producer registry lookups. The
existing provider results, proof checks, enqueue behavior and readiness decisions
are unchanged. Logs contain typed identity/result contracts, not prompt bodies.
The native harness preserves the original Submit failure and observes the same
run through public read-only queries for 70 seconds, spanning the normal producer
poll. It does not resubmit, install, inject payloads or create ready receipts.

Local checks pass: 928 workflow-service unit tests + 60 integration tests, strict
selected-crate all-target Clippy, 537 default-feature embedded tests, affected
harness ESLint, formatting and critical gate. Complete effective feature graphs
were inspected before every build, with Pumas 26a84e32, ORT load-dynamic / ort-sys
disable-linking, no forbidden download features and ORT_SKIP_DOWNLOAD=1.
Native diagnostics and any repair decision remain pending a source-bound run.
