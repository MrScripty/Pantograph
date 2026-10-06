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

The successor prepares actual Candle runtime readiness through normal native
configuration/startup commands and fixes two production losses: the startup
builder's foreign llama.cpp device intent, and the descriptor bridge's dropped
owner CPU devices. Runtime status and missing-evidence gates remain unchanged.
The failing CPU bridge regression, nine passing provider tests, 545 passing
affected-runtime tests (one optional ignored), strict all-target Clippy, formatting,
complete feature graphs and source patch are preserved here. The first broader
suite could not launch until its installed Python library directory was supplied
through the command-local loader path; that failure is preserved too. These local
checks do not qualify native CPU submission or output. Native startup regression
tests and the full saved/reopened fixture will run on the successor source.
