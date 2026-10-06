# Native missing dependency requirements proof

[Run 37505022667](https://github.com/MrScripty/Pantograph/actions/runs/37505022667)
executes `7d89d84892d9265185c53a8bfb8f8fb27b6485c5`, tree
`6768d46f912af7c5cd2ef0afbcb0dc952b40d903`. Native setup/build,
all eight startup tests, cold-owner assertions, save/reopen and wires pass.
Actual owner validation is current/executable with zero diagnostics and matching
revision `aac0d972d0845775`; submit_gate.allowed is true.

The actual GUI rejects submission with:
“Invalid request: dependency requirements proof is missing for executable node infer.”
Snapshot publication retains its proof requirement. The scoped run query returns
zero runs. There is no scheduler execution or actual CPU output. The fixture's
getText initially returned an empty string, but the complete DOM, screenshot and
job log retain the exact GUI error; that empty-string capture is not the cause.

All twenty-one artifact members, original images/JSON, masked job log and source
binding are preserved. ZIP artifact 11431084506 SHA256 is
`26ea52a606c8eb0be1dcca82899e43fbad50bde2f221a43bb7c69bb6dc391898`.
The existing typed dependency Resolve producer requires a valid associated
sidecar, current graph revision and validation session. Producer-to-publication
integration is being verified; no fabricated proof or receipt is supplied.
