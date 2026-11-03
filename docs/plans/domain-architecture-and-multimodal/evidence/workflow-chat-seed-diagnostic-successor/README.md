# Seed diagnostic and CI coverage successor

This narrow successor preserves reviewed draft source
`6eceb28997fe4c8d0ef6400e598f093d2bffa252`, tree
`5e16ca5f33f21fd2b46d8c51067fe8da54418423`, and its complete 78-member integration
packet unchanged. The original exact-main reproduction and failed expanded run
remain in [the prior packet](../workflow-chat-main-integration/README.md).
`preservation.json` verifies their hashes. No failure is waived or overwritten.

## Contract decision and change

The current public chat edge forwards `sampling.seed` to `ChatRequest.seed`
(`crates/inference/src/gateway.rs:3009`). Its support diagnostic at lines 3080–3091
is `Mapped` for PyTorch and `RequiresBackendSupport` for mock/other backends.
`Unsupported` applies to unmapped options. Existing gateway tests at lines
2504–2507 and 6115–6141 and the accepted
[seed report](../../reports/2026-10-06-workflow-text-seed.md) agree. Commit
`6cf549dd` introduced this deliberate contract. The parent also reports independent
reviewer confirmation of this decision. Runtime semantics are unchanged.

Test assertion commit `cf1be8cee288968c1e67601d75458c4ef7720dcd` replaces the
obsolete `unsupported` expectation with `requires_backend_support`, additionally
requiring diagnostic backend key `mock` and captured request seed `42`. All response,
usage-fixture, cache and other diagnostic assertions are retained. This strengthens
coverage of forwarding and attribution without claiming mock seed execution.

CI commit `91d2657a57a5226448507c1b5e8b205dc1112a9c`, tree
`14dae3e1c9f5c5da8e75abe314640016a51caa5c`, is the final combined implementation
source qualified here. It adds the exact feature-enabled seed regression to the
existing typed embedding capture loop. Both embedding checks remain. All three
selected tests must be discovered and each must report exactly one passed,
zero failed, zero ignored test. Broadening discovery lists names only; no full
expensive suite is duplicated. The block sets `ORT_SKIP_DOWNLOAD=1` defensively.
Workflow permissions, dependency/toolchain pins and all other gate blocks are
byte-identical to frozen source.

`source-binding.json` and `source.patch.gz` bind both source commits and the
complete two-file implementation delta. Subsequent evidence commits change only
this directory; application and CI source remain identical to tested `91d2657a`.

## Deciding qualification

| Check | Result |
| --- | --- |
| Focused repaired seed regression | One pass; forwarded seed and backend attribution asserted. |
| Final affected five-package Rust suite | 2,486 tests plus one doctest pass; seven default ignored including doctests. |
| Final expanded eight-package suite, no-fail-fast | 3,170 tests plus one doctest pass; zero failures; 23 ignored including doctests. |
| Exact modified CI capture block | Three passes: both existing embedding tests and the seed regression, with discovery and exactly-one-pass guards executed. |
| Explicit saved/reopened native CPU graph test | One pass; 28 public runs comprise **21 chat_completion + 7 text_generation controls**. Eight sequential envelope members comprise **6 chat + 2 text controls**. |
| Actual full worker CPU fixture qualifier | 32 requests pass. |
| Python worker regressions | 78 pass. |
| Frontend | 670 tests, typecheck and production build pass. |
| Expanded strict Clippy | All targets of all eight packages pass with `-D warnings`. |
| Source gates | Formatting, critical/accessibility/explicit-range traceability and scheduler-only guardrail pass. Modified workflow YAML parses; extracted actual CI shell block executes. |

The affected/expanded/native/CI/Clippy deciding runs execute the final combined
`91d2657a` source. Frontend/Python reruns execute the assertion successor whose
application/frontend/Python code is identical in `91d2657a`; the later CI change
is qualified by the actual extracted block. Complete commands and raw compressed
logs preserve both initial successor passes and final deciding passes.

Before builds, complete all-target feature graphs were inspected for the expanded
selection and the minimal CI `node-engine/inference-nodes` selection. No
`download-binaries` feature is effective; expanded ORT retains `load-dynamic` and
`disable-linking`. All qualification builds/tests/Clippy use `ORT_SKIP_DOWNLOAD=1`
with the existing dynamic/no-download configuration. Main suites use locked
offline Cargo; the actual CI block uses its exact existing Cargo invocations with
only the added defensive environment variable. No ONNX/ORT artifact downloads,
dependency installation or manifest/lockfile changes occur in this successor.
Repository-local author remains `MrScripty <TheEnvironmentGuy@protonmail.com>`.

## Limits and review

Native acceptance remains two untrained 5,392-parameter GPT-2 CPU fixtures with
explicit standard tokenizer templates and controlled package/target/readiness/
dispatch/graph discovery. Actual selected loading, gateway, full worker imports
and CPU forwards are exercised. Cancellation checks are precancellation, and
envelopes remain sequential. Production-model quality, GPU, usage accuracy,
post-start cancellation, native desktop, live Pumas discovery and general
architecture/tokenizer/template-fallback claims remain unqualified.

The original inherited seed gate is now repaired and the final expanded suite is
clean. Fresh final-head independent/external review remains parent-owned; hosted
CI on the updated head is separate from locally executed gates. No native desktop
worker changes are incorporated or its separately owned gate claimed passed.
No CodeRabbit request or merge belongs to this successor.

`manifest.sha256.json` verifies every packet member. Original source, failed logs
and main-only reproduction remain separately frozen and reviewable.
