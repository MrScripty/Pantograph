# Llama server effective settings input

Reuse existing backend-owned LlamaCppRuntimeSettings in the two public inference startup/reuse methods instead of repeating device/context/thread/batch parameters. The backend passes the same normalized settings object it already owns; the server projects the existing DeviceConfig and scalar values without new normalization or validation. Model/mmproj borrows, process spawner, port override/default, startup cleanup, and reuse semantics remain unchanged. Embedding/reranking public APIs are untouched.

A private RuntimeDescriptorInput groups the existing nine snapshot parameters at all three callers. The descriptor construction block, startup behavior block and reuse comparison block are byte-identical after input destructuring/projection. Known call sites are one backend startup/reuse path and the server tests; Tauri's same-named command goes through the gateway. The two public Rust signature changes are recorded explicitly in the changelog.

Existing tests retain exact launch settings/argument boundaries, startup process/PID cleanup, readiness/descriptor identities, port/context/device reuse distinctions and Path-component equality. Hosted commands enable backend-llamacpp, require nonzero discovery, and run both server and backend test modules. The tests use existing mock process infrastructure, not a claim of real model loading.

Formatting, whitespace and unchanged-body comparisons pass locally. No local Rust execution is claimed. Independent source review and fresh hosted startup/reuse/Clippy qualification remain pending. This adds no runtime selection policy or scheduler algorithm.

Independent source review accepted exact tree 47ac0a3557868b5d0847c84e151798c061ca2d8f with no blockers. It verified all callers and unchanged behavior blocks, plus 12 server and 11 backend test attributes in source and feature-enabled nonzero discovery. Those source counts are not execution receipts; fresh hosted tests remain required.
