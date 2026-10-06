# Native identity successor reaches dependency readiness pending

[Run 37522366379](https://github.com/MrScripty/Pantograph/actions/runs/37522366379)
executes 5a629f7affd43fd2884899d3a2ed3feb5c3d42ff, tree
70651013a1c8c5c50447a860eec6035536330f51, parent preserved 3b9e731a.
Actual native build and all eight startup tests pass. Cold Candle owner,
save/reopen, three visible wires, current executable validation, typed Resolve
and GUI Submit are exercised. The saved-proof identity rejection is cleared;
the actual GUI now reports runtime dependency readiness pending for infer.
Scoped run run_370d45c5-335d-4858-9817-4dac3e9f331a is queued with resume state
dependency_readiness_pending, no selected runtime/device, no start/completion
and zero retained outputs. Actual CPU execution/output remains unqualified.

All twenty-two artifact members are preserved, along with the complete masked
job log. Original JSON/PNG bytes remain unchanged; logs/text/HTML are losslessly
gzipped with deterministic mtime. ZIP artifact 11440194325 has SHA256
d6c0723c0271deffefc83cd4aa9fb04a19410f9694806b69732ff2d76c4ae22d.
The complete native Cargo graph includes current Pumas 26a84e32, load-dynamic /
disable-linking and no forbidden download features; ORT_SKIP_DOWNLOAD=1 is in
the completed job log. No GPU/pretrained/full-production-loader claim follows.

The GUI test fails on the initial pending response after 9.3 seconds. It does
not capture the provider seed/snapshot result or observe readiness after the
producer's default 60-second poll. Source review identifies a separate cold
bootstrap issue to investigate: hosted composition uses a snapshot-only Resolve
provider, while scheduler dispatch requires a valid requirements payload before
enqueuing snapshot work. The snapshot provider starts empty and its missing
response has no requirements/bindings. The hosted inventory also defaults to
Python probing with other provider kinds not implemented in this build scope.
These are source findings, not captured native provider-result diagnostics;
exact pending cause and eventual readiness remain unqualified. No readiness
receipt, payload or snapshot is invented to make this run pass.
