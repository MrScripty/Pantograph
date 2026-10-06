# Cached numeric refusal invalidates live KV

This separate successor starts at frozen numeric checkpoint
`12980c2f004950cf6434e0e325f3406d0863e4fc`. Review accepted its dtype and overflow
fixes but found that a cached numeric refusal retained inconsistent live state:
continuation mutates the passed KV cache before the checked repetition operation,
while matching token history is published only on success. The original catch
rethrows without invalidating the old history/new cache pair, allowing later reuse
or export. The frozen checkpoint, feature and qualification evidence are unchanged.

The catch now calls the existing live-KV clearing owner before rethrowing the
numeric error. There is no automatic fresh retry and no rollback claim. Later
requests may explicitly establish a fresh cache/history pair. Clearing KV authority
does not acknowledge model shutdown or change ready/resident ownership, resource
uncertainty, full task peaks or source/generation fencing. Scalar bounds, float32
arithmetic, original native processor order and existing numeric refusal behavior
are unchanged.

The regression executes the actual worker catch, continuation sampler, clear,
metadata and export functions with real Torch tensors and Transformers
DynamicCache. Controlled forwards mutate cache before numeric refusal in three
cases: suffix ingestion, a generated-token forward, and empty-suffix replay.
On the old catch all three fail because live state remains published. With the fix,
state and metadata are absent; export refuses before creating a file; no automatic
fresh decode occurs. A later fresh request commits a different cache with history
`[2,3]`; subsequent continuation reuses only that new cache, and the exported payload
has matching history and actual cached key tensors `[2,3,2,3]`.

Twenty-four actual CPU tests pass with Python 3.12.3, Torch 2.14.1+cpu and
Transformers 4.53.3. The full mixed llama.cpp/PyTorch inference package passes 811
checks, with three native/doctest cases ignored. Strict all-target inference Clippy,
format, critical/accessibility gates, scheduler public boundary, staged/range
traceability and nine ONNX no-build-download graphs pass. Read-only source review
found no remaining consequential flaw in this bounded fix; the reviewer did not
execute CPU or Rust tests. Frontend and embedded-runtime sources are unchanged,
and their prior suite results are not counted as reruns.

The private Cargo target is
`/workspace/pantograph-cache/text-repetition-kv-refusal-target`. Every workspace
package's artifacts were removed before compilation; only external dependency
hardlinks were retained, preserving earlier targets. Exact identities, commands,
logs and hashes are recorded in
`/workspace/qualification-evidence/text-repetition-kv-refusal-qualification.json`.
This is controlled SDAR model behavior with actual cache/sampling execution, not
pretrained SDAR loading. Full worker import still lacks soundfile; tests execute
the actual AST-selected functions. Native GTK/WebKit, GPU, pretrained-model and
ONNX execution remain unqualified. The external standards directory is unavailable;
no fresh Core/Router inspection is claimed. Parent owns PR/review/merge. Parent's
merged image controls at `763e8d4b` are separate from this text successor.
