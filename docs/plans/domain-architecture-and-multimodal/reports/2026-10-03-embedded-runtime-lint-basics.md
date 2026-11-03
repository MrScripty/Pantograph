# Embedded runtime mechanical lint basics

Fresh PR #40 Clippy clears runtime-registry and exposes thirteen embedded-runtime library findings. This first area milestone addresses only four mechanical findings: derive the candidate provider's identical default, remove a redundant inventory-reason borrow and selector-access borrow, and express the Python runtime node predicate with matches!. Payload/error layout and arity findings remain separate.

Regressions prove the default still uses an empty snapshot with no resource facts and four fail-closed diagnostics, and the helper accepts exactly audio-generation/onnx-inference while rejecting other, differently cased and padded node identifiers. CI runs the complete embedded-runtime unit suite with its default backend features. Public APIs, runtime policy and errors are unchanged.

The same checkout is reused on an embedded-runtime branch, preserving prior refs. The actual full pinned cargo fmt --all -- --check passes locally; root source review accepted frozen tree 405aea674557289e585587c1b23b48e381acbb0d; hosted tests remain required. No local Rust execution or heavy build is claimed.
