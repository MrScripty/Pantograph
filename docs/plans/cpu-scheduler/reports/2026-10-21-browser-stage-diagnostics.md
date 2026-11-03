# Future image-browser failure diagnostics

PR86's first published head, `ecdc27ab9993e2dad60e650b8817022e30c9d3d7`,
failed its hosted image-browser step with exit 1. Available job/check annotations
contain no failing browser assertion. PR85's earlier browser pass does not explain
this failure. The historical redirected log-storage read was denied; no old log,
artifact download, alternate storage route or access-setting change is part of
this investigation. The local Chrome sandbox ownership blocker is separate and
has not been established as a hosted cause.

The authorized diagnostic-only slice publishes future build/start/assertion/cleanup
checkpoints, fixed failure messages, allowlisted error name/code/assertion operator,
and a bounded validated browser version. Its consumers are the CI operator's
existing check annotations and `image-output-browser.log` artifact stream.
Exception messages, stacks, assertion operands, DOM/CDP payloads, paths and URLs
are excluded. Exception fields are read once under a guard, including hostile
getters. Unknown values collapse to fixed vocabulary. Chrome version is probed
before launch with a two-second kill deadline and obtained through the existing
CDP call before assertions; neither version probe output nor arbitrary exceptions
are printed. A failed optional CLI probe reports unavailable. The CDP check remains
required, as it was before this change.

Setup and guarded dependency imports are inside the failure boundary. Primary
and cleanup failures are separately reported with their own stage. Failed
qualification still exits 1; success still waits for owned cleanup. A fixed shell
annotation covers preflight/entrypoint/stream failure when no structured harness
failure is available. Existing artifact name/path and five-minute workflow bound
remain unchanged. Vite's raw logger is silenced in favor of the fixed build-stage
failure projection.

Verification: ten deterministic diagnostic/CDP tests pass, including hostile
payloads, throwing/mutating getters, independent primary/cleanup failures and
version validation. All 731 affected frontend tests pass. New scripts pass
ESLint; the harness passes Node syntax checking; the extracted workflow command
passes `bash -n`; the diff passes whitespace checks. An independent TypeScript
AST comparison confirms all 43 original assertion calls unchanged; source checks
confirm launch flags, existing deadlines and process-group cleanup checks unchanged.
Independent source review approved the exact production/test/workflow hashes before publication. No local browser rerun
or real-model work is used as evidence for this slice.

Future hosted evidence can establish the failing stage, checkpoint and sanitized
error class. It may still be insufficient to establish the underlying cause;
absence of a raw private payload is deliberate. C2's missing authoritative memory
and latency provenance, M4 acceptance, broader M6 and real-model qualification
remain open. No merge, retarget or historical failure explanation is claimed.
