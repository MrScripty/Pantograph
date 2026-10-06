# Actual native descriptor availability diagnostics

[Run 37493778399](https://github.com/MrScripty/Pantograph/actions/runs/37493778399)
executes `218423c6a314f5cf8ecdbcee1afb7ed20edfd360`, tree
`3eb266c8885626ab60318ea3635178e32755068c`. Official setup, no-download checks,
native build/launch, save/reopen and visible wire assertions pass.

The supported native lifecycle subscription and read-only session projection
capture exact owner results. The normal interface Apply and Save execute; both
requested/current graph revisions equal `80fbe18c51d2f00f`. The owner rejects
submission with **invalid_runtime_constraint** and **invalid_device_constraint**:
no available runtime/device satisfies the explicit Candle/CPU selection. Model
facts and typed embedding ports resolve. No scheduler submission or actual CPU
output is produced. This is runtime/device availability, not stale revision or
missing inference snapshot. The fixture had only installed model files.

All 18 artifact members, raw screenshots and JSON are preserved; text/HTML/logs
use lossless timestamp-free gzip. Artifact `11426653550` ZIP SHA256 is
`3bb3173979735162ddf6b8624c7baff545d94737d64c4e45a2021b50f999abcc`.
The masked job log and source record bind this attempt. `SHA256SUMS` binds this
folder. Every earlier failure remains preserved separately.

The attempted readiness successor at `7fcd990b` used generic native startup.
Independent review identified its typed-contract rejection; that approach is
withdrawn. Its [separate failure evidence](../startup-regression/README.md) records
the actual upstream selector-test failure and absence of a native app session.
The device bridge repair remains valid. The failing regression, nine passing
provider tests, 545 passing affected-runtime tests (one optional ignored), strict
Clippy, formatting, complete graphs and attempted source patch remain preserved.
The initial missing-Python-library launch is preserved too. These checks do not
qualify native CPU submission or output. The current successor uses compiled
cold-owner registration and the existing typed scheduler loader; see the report.
