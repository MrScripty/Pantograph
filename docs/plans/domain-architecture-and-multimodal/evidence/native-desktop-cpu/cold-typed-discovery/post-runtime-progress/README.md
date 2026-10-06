# Readiness resume must advance downstream non-runtime tasks

Starting published candidate: 9d725702fae26221ee8042621f1923930a4edd3c,
tree 985bdb6adb0e46531c383c058a44e6f6cee47e73. The complete twentieth native
failure is preserved in selected-weight-target/native-attempt before this repair.
Its embedding runtime task and attempt are Completed with three outputs; vectors
remains AwaitingInputs and no vector or metadata artifact is retained.

Source tracing identifies the readiness-resume path dispatching its ready runtime
tasks and immediately checking whole-graph completion. It omits the canonical
progress loop that advances scoped downstream inputs and executes ready non-runtime
tasks. The repair calls that existing loop before finalization, preserving its
vector shape/size/finite-number checks and normal task/result scope validation.
It does not fabricate outputs or loosen readiness, identity or reservation checks.

The new regression saves/reopens the existing embedding graph, withholds readiness,
submits once, rejects a wrong-run resume without consuming the paused candidate,
publishes the existing controlled scoped facts and resumes the same run. Actual
selected-model Candle execution on committed untrained BERT widths 8 and 12 feeds
the real vector sink, and vector/metadata/usage responses match the golden values
and selected identity. Package/readiness/dispatch facts remain controlled scaffolding;
the separate current-pin real-Pumas regression still covers executable-target handoff.
The resumed graph reproduces the exact native AwaitingInputs error before the
one-line progress-loop repair and passes afterward. Immediate-ready coverage remains.

Logs and complete effective Cargo graphs are preserved losslessly in gzip, with
mtime zero and SHA256SUMS for stored bytes. Every Rust build sets ORT_SKIP_DOWNLOAD=1
after auditing current Pumas 26a84e32, dynamic ORT and absence of download-binaries.
Native vector/body qualification remains pending this successor; controlled CPU
tests do not qualify the GUI, production loaders, pretrained models or GPUs.

Final verification: all 539 embedded and 989 workflow tests pass. Strict all-target
selected-crate Clippy, formatting, critical-pattern and staged traceability checks
pass. The default-feature test/Clippy graphs each contain 1552 rows; focused
embedded red/green graphs each contain 1547. Inference source is unchanged from
the preceding repair's passing 597 unit/integration tests and runnable doc test.
