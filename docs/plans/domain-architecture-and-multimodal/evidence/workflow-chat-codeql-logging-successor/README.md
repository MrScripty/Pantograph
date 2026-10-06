# CodeQL logging repair for native CPU graph assertion

Frozen predecessor `e04a10610ec838c0bff8ddfff1a2126592fcafc1`, tree
`b60fae63778a8fa5a1d6d8ef9ce9747c05fd4cd3`, is preserved with all prior source
and evidence. Its 20 execution/analysis check runs passed, but its distinct
[CodeQL security check](https://github.com/MrScripty/Pantograph/runs/112393631140)
failed on [alert 31](https://github.com/MrScripty/Pantograph/security/code-scanning/31).
The [review thread](https://github.com/MrScripty/Pantograph/pull/62#discussion_r4198327326)
reports three event-client-session-ID flows reaching the test's assertion log.
`previous-checks.json` records exact predecessor check statuses independently of
the successful Rust analysis job. No suppression or manual thread resolution is used.

## Narrow source repair

Tested repair `dbd59248cd193f80da483fa96a37c4e86e2a369b`, tree
`67b92853a53576e2bfbbd0f158fc2a50891a1c67`, changes only
`crates/pantograph-embedded-runtime/src/lib_tests/cpu_chat_graph_tests.rs`.
The exact `output.value == case["text"]` equality remains asserted for every
inference/sink output. The assertion now uses `assert!` with only fixed fixture
model/case identifiers in its failure message. Both the output structure's Debug
formatting and `assert_eq!`'s automatic actual/expected value logging are removed.
The comparison is neither waived nor weakened; test execution and fixture scope
are unchanged. No other runtime, test, CI or dependency behavior is changed.

`source-binding.json` and `source.patch.gz` bind the one-file, three-line repair
and preserved predecessor. Subsequent evidence commits change only this packet,
so application/test/CI source matches the tested repair.

## Fresh local validation

- Explicit actual saved/reopened CPU graph qualification passes: 28 public runs
  comprise **21 chat_completion + 7 text_generation controls**; eight sequential
  native envelope members comprise **6 chat + 2 text controls**.
- Affected five-package suite passes 2,486 Rust tests plus one doctest; seven
  default ignored checks include doctests.
- Expanded eight-package no-fail-fast suite passes 3,170 Rust tests plus one
  doctest, with zero failures and 23 default ignored checks including doctests.
- Strict Clippy for all targets of all eight selected packages, formatting,
  critical/accessibility/explicit-range traceability and scheduler-only public
  source guardrail pass.

All-target Cargo features were inspected before builds: pinned Pumas `26a84e32`,
ORT `load-dynamic`/`disable-linking`, no `download-binaries`. Builds/tests/Clippy
use locked offline Cargo, `ORT_SKIP_DOWNLOAD=1`, and the existing dynamic/no-download
configuration. No ONNX/ORT artifacts or dependencies are installed, and manifests,
lockfiles, credentials, global identity, permissions and CI pins remain unchanged.
Repository-local identity remains `MrScripty <TheEnvironmentGuy@protonmail.com>`.

## Hosted security result and limits

This packet records pre-publication local qualification. After a normal push,
the exact published head's CodeQL security check and completed Rust analysis must
be checked separately. Hosted security status is recorded in the PR description
and separate publication evidence without committing another source head. A
successful analysis job alone does not establish a passing security check.
Independent recheck belongs to the parent/reviewer. No CodeRabbit retry, merge,
alert suppression or manual review-thread resolution is performed.

Scope remains synthetic untrained 5,392-parameter GPT-2 CPU models with explicit
standard tokenizer templates and controlled package/target/readiness/dispatch/
graph discovery. Selected loader, gateway, full worker imports and CPU forwards
are actual. Cancellation coverage remains precancellation; envelopes sequential.
Production-model quality, GPU, usage accuracy, post-start cancellation, native
desktop, live Pumas discovery and arbitrary model/tokenizer/template-fallback
qualification remain outside this slice. The separate desktop worker's unpublished
changes are not incorporated or its gate reported passed.

Complete commands, compressed logs and exact statuses are preserved here.
`manifest.sha256.json` verifies every packet member; `preservation.json` verifies
all three prior packets unchanged, including the original main failure reproduction.
