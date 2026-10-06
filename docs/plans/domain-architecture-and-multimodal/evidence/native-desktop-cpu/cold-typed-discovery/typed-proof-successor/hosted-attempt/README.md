# Actual native Resolve succeeds; fixture diagnostics assertion fails

[Run 37509685511](https://github.com/MrScripty/Pantograph/actions/runs/37509685511)
executes 77b9704920697ff01c9fa5b0bb49dd5b585c885f, tree
fae84ac50e846f25928c73b3fc5bb40eba5589ed. Native build, all eight startup
regressions, cold-owner assertions, save/reopen and all three visible wires pass.
Actual current-session typed Resolve returns request_ready. The fixture then
asserts an explicit empty diagnostics array, but the contract serializes empty
collections by omission. That fixture assertion fails before GUI Submit.
No scheduler run or CPU output is established. No publication guard is weakened.

All twenty-one members, original JSON/PNG and masked job log are preserved.
Artifact 11434358121 SHA256 is
ebf7a5ee7f1eaef3d57fc791ff47064e414ae0b387acd8c6a226119897935f8c.
