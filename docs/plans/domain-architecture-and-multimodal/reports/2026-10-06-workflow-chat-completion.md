# Canonical single-prompt chat graphs

The registry and causal-LM profile already recognized `chat_completion`, but its
descriptor exposed only a prompt and the selected host route admitted only
`text_generation`. Saved single-prompt chat graphs now expose the existing text
controls and execute through the same selected native text owner while retaining
their exact canonical task identity.

## Source and acceptance

Normal Git fetch confirmed main
`c75fa2379a730833709ea8976075bf17a117040f`, tree
`6c8c4bb1cbed0527b58c1048ebf49fa0e243d56b`, before edits/builds. Isolated branch
`feat/workflow-chat-completion` composes the two separately preserved candidates
with ordinary merge commits:

- `fcc24a70756920b70fd3957c737b07f60ff1d9d1` has parents main and stop repair
  `e570302d688517f26125ca37bd406603e9b7cf7f`.
- `fff03a153ff26b5a38c01be8a3cc65ddb28be07e` has parents `fcc24a70` and embedding
  revision repair `7be490d7e403a95e2efff7e5a8f21a68d40a42fd`; its pre-feature tree
  is `d5e041fc306e8c9c168c05f3d8f8e64e32358582`.

Acceptance requires canonical task agreement, explicit requested-revision
preservation, a local synthetic causal LM and explicit chat template through
saved/reopened public scheduler graphs, deterministic scope isolation,
precancellation and retained text-generation controls. The composed stop and
embedding repairs retain their own independent review status. No main/PR change
or resident/ranking work belongs to this candidate.

## Behavior

Chat descriptors share prompt, optional system prompt, length, sampling,
repetition, seed and scalar stop controls with text generation. Optional defaults
and validation remain shared. This is one prompt plus an optional system string;
messages ports, persistent conversation history and tools are not added.

Single requests and compatible sequential envelopes admit the canonical chat
task. Request, selected decision and package evidence must agree exactly between
`chat_completion` and `text_generation`; no task relabeling is allowed. Backend
capability facts now include the existing causal-LM chat route. The existing
worker owns tokenizer template formatting and model execution.

The shared text projector rejects an explicit requested revision that differs
from, or is absent on, the selected model before resolving packages/targets. The
typed request retains its original revision, including omission; the selected
owner may refine an omitted revision and requires explicit selected/package/target
revisions to agree. Tests cover the task matrix, revision matrix, unsupported
shape, blank prompt and package-task refusal before loading.

## Qualification and limits

Two committed, locally initialized GPT-2 fixtures each have 5,392 untrained
parameters. Independent installed native `model.generate` oracles cover two
prompt/system contexts and seeds 0, 42, 43 and `u64::MAX`. No external weights or
privileges were acquired. The full worker qualifier makes 32 actual CPU requests,
checks streaming/nonstreaming text, explicit template, real first-forward token
IDs and unchanged ambient RNG. Worker imports and model forwards are real.

The explicitly run ignored Rust qualification saves/reopens four graphs and
makes 28 public scheduler graph runs plus eight separately attributed native
envelope members. It changes model 11 → 37 → 11 and task chat → text → chat,
interleaves/replays seeds and prompt contexts, checks exact inference/sink text,
unique graph run identities and scoped reservation events, and follows cancelled
single/envelope requests with healthy runs. Alternate host loading/execution
panics if called. These are **precancellation** checks; interruption after a native
forward begins is not qualified here. Envelope execution remains sequential,
without backend-native batching or throughput claims.

The deciding ordinary Rust run passes 2,486 tests and one doctest; five native
tests and two doctests are ignored by that default run. The new native graph test
is explicitly run and passes separately. All 78 existing Python worker tests and
670 frontend tests pass. Typecheck, frontend build, formatting and affected strict
Clippy are recorded with their final statuses in the evidence packet.

Before building, the complete effective all-target Cargo feature graph for the
five affected packages was inspected: current Pumas `26a84e32`, ORT `load-dynamic`
and `disable-linking`, no `download-binaries`. Every Cargo command uses
`ORT_SKIP_DOWNLOAD=1` and offline resolution. No denied binary download was
retried. SoundFile 0.14.0 and its ordinary Python dependencies were installed to
satisfy the existing full-worker import; Torch/Transformers were unchanged.

Package/target/readiness/dispatch facts and host graph discovery are controlled.
The selected local loader, gateway, full worker and GPT-2 CPU forwards are actual.
This does not qualify live Pumas discovery/owner facts, pretrained quality, GPU,
desktop authoring/execution, arbitrary architectures, segment-ID tokenizers,
template fallback or full production loading. Desktop numeric precision remains
bounded by the existing safe-integer contract; full-u64 coverage here is Rust/API.

Initial compile and harness/fixture failures are retained alongside deciding
logs: missing test imports, single cancellation-state expectation, unbound
external controls and nonstandard tokenizer segment inputs were corrected in the
test fixtures/harness. No worker production hardening was introduced for those
fixture issues. Repository evidence and source binding are under
[`evidence/workflow-chat-completion`](../evidence/workflow-chat-completion/README.md).
