# PR58 omitted generation controls repair

This separate candidate starts at fetched PR58 head
`1b9fd0bdaf201c92a54165ffab08cc6c6863a640`, tree
`c9124b2f1b9cba9f0db8b9d736f35a95212abb99`, on
`fix/pr58-omitted-generation-controls`. Main was independently confirmed at
`763e8d4b13ba311ff245c75a370e56dafb417d8e`, tree
`7caa32b549397bfc6250a141e3a383742f90dd55`. Reviewed composition `9e8cd64`,
its evidence successor, PR58's harness successor and their branches are preserved.
No resident-accounting branch is included. Graph-authored seed work remains
paused in its separate worktree; its tracked patch and untracked seed-test hashes
still match the frozen snapshot. No seed source enters this repair.

The two review findings were reproduced before production edits against exact
main and PR58 source: six regression methods reported 13 failed subtests and eight
errors on PR58. Manual/SDAR decoding now stops only on tokenizer EOS, as main did.
Minimum suppression uses the union of tokenizer and model EOS IDs; model chat
delimiters do not acquire stop semantics when controls are omitted or zero.

Inherited manual minima are capped by the request budget. Native omission forwards
no minimum override and retains Transformers' inherited minimum processors,
warning and forced-EOS precedence. Authored minima still receive strict u32/budget
validation and refusal when later processors violate the authored floor. This
restores native behavior without replacing model defaults with a clamped kwarg.

The empty-SDAR retry retains main's `min(max_tokens, 24)` delimiter safeguard when
the minimum is omitted. An authored floor can raise that internal floor, but the
strict guard enforces only the actual authored value. Zero and already-met floors
permit forced EOS; the internal retry floor is not promoted into a user promise.
A regression caught two errors in an intermediate boolean-guard implementation;
that implementation was corrected before publication. The empty SDAR snapshot is
cleared before retry setup, so explicit-minimum rejection cannot leave failed KV
available to the next request. Replaying the original retry function reproduced
two stale-KV failures; repaired streaming and nonstreaming paths start fresh.

Acceptance evidence is in the [hashed archive](../evidence/pr58-omitted-controls/README.md):

- All 40 actual Python CPU methods pass, including eight new compatibility/lifecycle
  methods. Six exact-main comparison methods also pass on the final source.
- Inference library with `backend-pytorch` passes 747 tests; strict all-target
  inference Clippy passes with `-D warnings`.
- Rust format, critical anti-pattern, scheduler public boundary, diff whitespace
  and staged decision traceability pass. No frontend/product UI changed.
- Complete all-target effective feature graphs were inspected before building.
  The combined graph uses current Pumas `26a84e32`, dynamic ORT and disable-linking
  with no `download-binaries`. The exact inference-only test/Clippy graph contains
  no ORT or Pumas dependency. Every Cargo build used `ORT_SKIP_DOWNLOAD=1`, locked
  offline operation, and the existing dynamic configuration. No binary download
  was attempted or retried; manifests and lockfile are unchanged.

Independent Astra medium source review found no remaining blocker after reviewing
both compatibility repairs, retry-floor distinction and KV cleanup. Full production
loader, GPU, pretrained/custom model, desktop and ONNX execution remain unqualified.
Torch 2.14.1+cpu and Transformers 4.53.3 execute fixed CPU forwards and actual native
processors; SDAR cache behavior uses controlled fixtures. Worker entry functions
are selected from actual source through AST because full audio dependencies are
unavailable. Other packages, frontend and native Diffusers checks are not rerun or
claimed as fresh evidence. Parent owns PR58 integration, hosted CI and merge.
