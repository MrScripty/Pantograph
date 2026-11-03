# C3 causal investigation and proposed supplemental evidence

## Status and preserved result

This investigation starts at published head
`938bc89ea381e089ecf4273d578324698959ddb4`. Its production measurement source is
`743e783cad979940bb0eea12fb642d36bd1614f2`; the successor changed test scope and
documentation only. All four ordinary CI workflows passed on `938bc89e`, closing
C6. C3 remains partial and M1 is not fully accepted.

The original fixed release cohort, its exit-1 result, raw baseline/serial exports,
and source manifest remain unchanged in [m1-evidence](m1-evidence/). This document
does not replace that negative result. No additional inference experiment has
been executed by this investigation. The user subsequently authorized the exact
bounded proposal below; its prospective execution authority is the
[plan amendment](../plan.md#m1-c3-supplemental-coverage-amendment-2026-10-12), which
must be published and independently confirmed before execution.

## Cause established from existing source and raw records

Independent recalculation from the compressed release result, proposal and event
exports confirms all eight runs' sample counts, maxima, proposed selection and
actual Start order. Runs 0–2 correctly use FIFO because neither exact workload yet
has three successful samples. Runs 3–7 are ranked:

| Ranked runs | A maximum ms | B maximum ms | B minus A ms | Selected / actual first Start |
| --- | ---: | ---: | ---: | --- |
| 3 | 13.054876 | 13.102873 | 0.047997 | position 0 / infer_a |
| 4–7 | 15.165646 | 16.592732 | 1.427086 | position 0 / infer_a |

The [selector](../../../../crates/pantograph-workflow-service/src/workflow/serial_ready.rs)
chooses position one only when its fresh maximum is strictly below position zero's
maximum. The [worker](../../../../crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs)
uses that selected member and revalidates the authoritative pair before Start.
The raw records satisfy that policy. There was no tied ranked maximum, no hidden
FIFO fallback and no observed proposal-to-Start mismatch. The independent reviewer
also found no selector or Start-order wiring defect.

The immediate cause is therefore **expected selection under the measured costs**.
The remaining acceptance gap is **inadequate coverage of the second-position
case by this workload**, not evidence that a shorter input was required to win.
Both texts use an untrained one-layer BERT-8 with a 16-token limit: the original
240-byte repeated input becomes 16 tokens, and the five-byte input becomes three.
The policy samples dispatch through native drain and matched cleanup, not tensor
execution alone. This audit does not isolate which phase made B's maximum higher,
prove overhead dominance, establish a positional bias or promise speedup.

## Supported, authorized proposal after independent review — not executed

Hypothesis: increasing supported raw input preprocessing demand, while keeping
the tensor recipe and model fixed, may produce a measured service-cost contrast
that exercises position-one selection. Selection still follows the observed raw
maxima; input length is not an oracle and no winner is guaranteed.

Before any new inference, freeze this exact workload:

- A: `"hello world "` repeated 64 times: 768 UTF-8 bytes.
- B: `"hello"`: five UTF-8 bytes.
- Same model, weights, tokenizer, CPU F32, one private CPU thread, graph/task IDs,
  candidate order, ordinary readiness providers, public owner and cleanup policy.
- The narrowest relevant limit is **1,024 bytes for ordinary String embedding
  text**, enforced by `validate_runtime_host_embedding_request` through
  `semantic_text_limit`. A's JSON string payload is 770 bytes; the public
  32,768-byte value limit and serial history's 64 KiB limit also remain unchanged.
  No capability limit is raised. Static review rejected preliminary 49,152-byte
  and 24,576-byte proposals before execution; the latter fit the public payload
  bound but exceeded this stricter host semantic bound.
- Verify token IDs, attention masks, type IDs and output expectations with the
  existing independent local reference before the hosted cohort. Do not download
  dependencies or regenerate weights. If the reference is unavailable or
  disagrees, stop.
- Use one fresh isolated owner/history, ordinary retaining bootstrap, then exactly
  eight baseline and eight serial runs. Keep both compositions/producers alive
  throughout. Stop compilation before measurement. No concurrent model workload.
- Keep the same graph/texts throughout this new cohort. Do not swap positions,
  inject delays/clocks/history, filter samples, choose seeds or alter the minimum
  sample count, estimator, freshness window or acceptance criterion.
- Save all new inputs, source/binary hashes, raw successful per-sample timings,
  proposals, Ready/run identities, both actual Starts, vectors, baseline/serial
  results and shutdown evidence in a separately named supplemental directory.

The extra raw text still truncates to 16 tensor tokens. This is a preprocessing
demand test with real CPU model execution, not a larger tensor-workload test or a
claim of useful semantic embedding quality. It must not replace C5's original
descriptive baseline or support a speedup claim.

The independent reviewer considers this single supplemental cohort defensible
under the prospective evidence-plan amendment. It increases A's raw text only 3.2
times relative to the original 240 bytes. There is no evidence that this increase
will dominate the observed dispatch/drain/cleanup variability, so a second-position
winner must remain a conditional observation. No length sweep, seed search or
automatic retry is admitted by the proposal.

This proposal changes the declared evidence workload; it does not require a
production API, capability, policy or acceptance-criterion change. A stronger
tensor-compute contrast would require a separately reviewed fixture/protocol
decision. Relaxing the String limit or substituting TranscriptText solely to
obtain a favorable ordering is not proposed. Counterbalancing positions would be
a separate position-invariance study, not proof of this preprocessing hypothesis.

## Stopping and interpretation rules

There is at most one additional hosted cohort under this proposal. Finish its
predeclared eight runs per mode regardless of winner, unless ordinary safety,
capability, readiness, oracle or execution failure stops it earlier. Preserve
partial evidence on failure. Do not repeat the cohort or vary input length after
observing a result.

- If raw maxima favor B and the actual first Start is B, report separately scoped
  second-position evidence. Preserve the original C3 negative result and seek the
  independent acceptance disposition before claiming milestone completion.
- If raw maxima favor B but the proposal or Start favors A, stop and investigate
  that concrete defect; any repair requires a targeted regression and review.
- If maxima still favor A, tie, become unusable or lack fresh comparable history,
  report the observed result and leave C3 pending. Further phase instrumentation,
  a different fixture/workload or an acceptance change requires a new design
  decision; it is not authorized by this single experiment.

No model acquisition, paid inference, private Pumas access, default-policy change,
editor/UI claim, GPU/remote expansion, main merge or deployment is included.
