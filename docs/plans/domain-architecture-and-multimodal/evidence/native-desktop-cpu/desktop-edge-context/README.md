# Native wires verified; descriptor availability remains blocked

[Desktop edge run 37490528758](https://github.com/MrScripty/Pantograph/actions/runs/37490528758)
executed `6eb84e949986799d70c5e7949a470cfb3bf86a52`, tree
`41d750d5dd9419519e4ab68a34b95ff539c8a78c`. The real native screenshot visibly
shows both connection wires; both constant-Y edge paths pass computed paint and
geometry assertions. Native setup/build/launch/save/reopen pass. The old fixture
still blocks submission with validation diagnostics. Its 17-member artifact
`11425847888` has ZIP SHA256
`7847a9060a1b9ec4bc1e74b9e3cdb1866596be9fb323d5f93b3f6260484b9a33`.

[Runtime-context run 37490714747](https://github.com/MrScripty/Pantograph/actions/runs/37490714747)
executed `ef2d0df6b0c78adf8aa884637efec9e8d08b4381`, tree
`9eb99c5b45c81ff9e84592e08e50986b0aeb6038`. It passes native launch/save/reopen
and edge assertions, then reports **Inference descriptor is unavailable**.
No public submission or CPU output is produced. Its complete 17-member artifact
`11426082322` has ZIP SHA256
`6aa84f4f339e754bb9c9d27fc72612beb1158406022a949c430ec5882c32244f`.

Both attempts retain their original screenshots, JSON, complete masked job logs,
and feature/dependency records. Text/HTML/log files use lossless timestamp-free
gzip; JSON and PNG bytes remain unchanged. Their owner-observation JSON is empty:
Tauri's official `invoke` property is read-only, so the attempted wrapper was
ineffective. This evidence does not claim a captured owner response.

A bounded offline Rust checkpoint runs the actual graph-resolution function on
both generated fixtures: exactly one missing-runtime-context diagnostic for the
old graph, then one valid embedding request with no resolution diagnostics for
the corrected graph. Another bounded probe uses the real current Pumas owner API,
matching native's HF/process-manager-disabled builder, on an isolated fixture.
It returns a valid HF directory, selected artifact `main`, 8065-byte logical
size, embedding task evidence and accepted Candle backend hints. These are
controlled fixture diagnostics, not native CPU output or real-user discovery
acceptance. The temporary checkpoint test sources and complete feature graphs
are included; neither temporary test remains in product source. The desktop
frontend build and critical/accessibility/traceability gates passed.

`SHA256SUMS` binds this folder. Earlier attempts, PR61 and frozen source branches
remain preserved. Descriptor availability, submission, CPU output, pretrained
quality, GPU and full production qualification remain unresolved.
