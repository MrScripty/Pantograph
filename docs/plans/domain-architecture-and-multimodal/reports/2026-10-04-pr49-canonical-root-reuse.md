# PR49 canonical-root reuse correction

This bounded descendant starts at independently reviewed
`deac37b20f8136f4fd13f0da6603d7d889d16d1a`, tree
`ada885216e27b6f0534b1f7a1577e352f53b8012`, on branch
`fix/pr49-candle-canonical-root`. It preserves the complete stack and excludes
the separate pooling change `495efea606620153fe2f0363b104e56518436a06`.

## Demonstrated gap and repair

`existing_directory` admits an absolute symlink or noncanonical model root and
preserves its spelling. Model loading compares canonical paths correctly, but
the reuse check compared a canonical Normalize path with the raw load-plan root.
An unchanged admitted model therefore loaded successfully and reloaded on every
request. The new regression fails on the exact reviewed loader: its second load
returns `runtime_reused = false` instead of `true`.

The reuse check now canonicalizes the model root and compares the canonical
Normalize path against it. Canonicalization failure still declines reuse. The
complete selected target, raw load plan, exact input bytes and physical checkpoint
identity comparisons are retained. No selected identity field is rewritten.
The worker join, cancellation, publication and directional revision code are
unchanged.

## Executed evidence and limits

The Unix regression executes actual untrained width-8 BERT checkpoints through
both a symlink root and an absolute `2_Normalize/..` root. Repeated loading must
reuse the same model Arc and produce identical actual vectors. An empty Normalize
directory redirected outside the selected model root is still rejected, and the
previous model remains usable for real inference.

`cargo test --locked --offline -p inference --features backend-candle --lib`:
455 passed, zero failed or ignored. This includes both widths' independent Torch
goldens, identity/revision rejection, dropped-caller and host cancellation, and
retained-worker cleanup/replacement coverage from the reviewed base. Formatting
and whitespace checks pass. Candle-enabled inference Clippy with all targets
completed with only the existing selected-text unused-fields warning. The staged
critical, accessibility and traceability gates passed. Frozen fixtures,
dependencies, audit gates and
traceability configuration are unchanged.

Full candidate native IPC/workflow acceptance, GUI, GPU and pretrained inference
are not executed here. The previously documented pinned ONNX Runtime artifact
restriction remains; no dependency feature reduction or network workaround was
used. Base receipts are not descendant acceptance. The parent coordinates
independent review and exact-head hosted qualification. This lane publishes only
the separate branch and handoff; PR49 is not advanced and no external review is
requested.
