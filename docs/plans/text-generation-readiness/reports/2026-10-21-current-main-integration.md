# CPU text integration on current main

The bounded readiness, terminal-outcome, failed-producer recovery and cold-reopen
integration is qualified. Trained-model T3 remains blocked; overall plan
acceptance is not complete. [Draft PR92](https://github.com/MrScripty/Pantograph/pull/92)
contains the integration for review, without a merge to main or deployment.

## Source and preservation

Qualified source: `3de398277e3619fb282766c4ff794c60a2c0d4bd`, tree
`a45b03b46275d96174bafea394df4c35e2901137`, based on freshly fetched main
`9ec7e7f3f4a0db1a1123bdc5504f76f4b79322f6`.

PR80 already reused PR75's adapter (`2f1e38fd` → `7a78625b`); PR82 contains its
terminal-result successor. Normal merge `15b11d48` preserves that complete stack
on current main. New commits `b3219d06` and `3de39827` add recovery and the corrected
cold probe. PR75/80/82 remain untouched. PR84's old transplant is excluded; README
PR79 and unrelated local branches are preserved. No force push or history rewrite.
Custom-node ABI, registry implementation, NodeLab and tool-loop work are excluded.

## Behavior

`TextJobs::drain` now retires only the job it has actually joined. A stream keeps
its original failure, and a stale drain cannot clear a successor. The existing
first-error/no-effects stop and unload contract is preserved; subsequent cleanup
reaches the worker and clears resource metadata only after acknowledgement.
Selected execution can now finish a failed job and perform following cleanup or
replacement without permanently replaying that failure. Regressions cover error,
panic, overlap, cancellation, receiver loss, successor identity and real controlled
PyO3 cleanup effects.

The public probe reopens the actual saved graph and retained model/catalog in a
new process. It verifies unchanged graph/model bytes, current descriptors and
constraints, and the same executable workflow version. It reads the immutable
authorization through public lookup, collects fresh graph validation and
Resolve/Check, and obtains new live readiness through the ordinary producer.
Pumas performs normal startup metadata/index reconciliation. This is not an
untouched-index, crash-recovery or schema-migration claim.

## Results

| Gate | Observed result |
| --- | --- |
| Inference library, supported CPU feature union | 874 passed, zero failed |
| Embedded runtime library, PyTorch plus host inventory | 643 passed, zero failed; four existing qualifications ignored |
| Workflow service library | 1,052 passed, zero failed |
| Public local CPU generation | Nonempty untrained completion with a 24-new-token cap, released owned runtime resources |
| Corrupt-weight loader failure | Public `InternalError` preserves `pytorch_worker_model_load_failed` and header cause; cleanup passes |
| Early active cancellation | Public typed `Cancelled`, matching run/node/attempt diagnostics, empty queue and cleanup |
| Separate-process cold reopen | Same saved graph/model/version and retained authorization, fresh validation/run identities and readiness, nonempty output and cleanup |
| Formatting/tooling | Affected Rust formatting, Bash syntax, workflow YAML, staged/range traceability and whitespace passed; 28 traceability regressions passed |

All four public cases ran locally through Pumas → Resolve/Check → scheduler →
PyTorch → output/terminal projection, with shutdown and resource checks. Every
result retains `t3_qualified: false`. Cancellation precedes generation; no
interruption-after-first-token claim is made.

The [hosted four-scenario run](https://github.com/MrScripty/Pantograph/actions/runs/39659962842)
also passed at direct source `3de39827`. Hosted
[Quality Gates](https://github.com/MrScripty/Pantograph/actions/runs/39659962837),
including strict workspace/all-target/all-feature Clippy with warnings denied,
passed. Clippy checked synthetic merge `a199dbd791c1bdd0dd0a2e3a94ba53ecf599c6ab`;
its tree was independently verified identical to the qualified source. The
hosted artifact upload/digest is recorded; its per-file archive was not
materialized locally. The retained local raw evidence supplies those observations.
[CPU Composition](https://github.com/MrScripty/Pantograph/actions/runs/39659963058),
[Runtime Separation](https://github.com/MrScripty/Pantograph/actions/runs/39659962647)
and [Headless Workflow Contract](https://github.com/MrScripty/Pantograph/actions/runs/39659962708)
also passed: all five implementation-source workflows are green. Final
evidence-only head CI is a separate observation.

## Reproduction and retained failures

Use a clean checkout of the qualified source, the pinned toolchains and a prepared
CPU Python environment. Set `PYO3_PYTHON`, its library directory in
`LD_LIBRARY_PATH`, site-packages in `PYTHONPATH`, and offline Hub/thread limits
as described in the [runbook](../../../cpu-text-generation-workflow-probe.md).
For library tests, set `PUMAS_REGISTRY_DB_PATH` and XDG roots to a new owned
temporary directory, then run:

```bash
cargo test --locked -p inference -p pantograph-embedded-runtime \
  -p pantograph-workflow-service \
  --features pantograph-embedded-runtime/backend-pytorch,pantograph-embedded-runtime/host-dependency-inventory \
  --lib
bash scripts/qualify-cpu-text-workflow.sh \
  3de398277e3619fb282766c4ff794c60a2c0d4bd NEW_ABSOLUTE_EVIDENCE_ROOT
```

Cold compilation/lock waits exhausted initial 600-second budgets before tests ran.
The first executable embedded suite then reported 54 setup failures at the
read-only `/home/agent/.config/pumas` default registry path. That home write was
not retried; supported disposable registry/XDG configuration under the owned
workspace produced the complete passing suites. No product guard or permission
was weakened. A redundant feature-variant build was stopped before execution;
the final three-owner configuration preserves the compiled feature union.

The [initial hosted cold probe](https://github.com/MrScripty/Pantograph/actions/runs/39658398681)
passed three cases but failed when attempting to replace immutable authorization
with a new validation session. The corrected flow reuses that authorization;
it does not bypass publication guards or invent a new semantic version for the
same executable fingerprint. Failed attempts remain in the evidence archive.

See [source/results](source-and-results-2026-10-21.json) and the
[105-file manifest](evidence/2026-10-21/manifest.json). Archives contain logs and
JSON observations, not model weights, catalogs, credentials or private user data.
Independent GPT-6.1 Sol / High source, hosted-evidence and final local-evidence
review found no material issues. The reviewer independently verified all 105
archive entries, 11 unchanged source bindings, test counts, raw cleanup/cancellation
records and cold-reopen identity/proof relationships.

## Remaining required input

T3 was not executed. Supply a verified trained local Transformers causal-LM asset
set (weights, config and tokenizer, at most 600 MiB), authoritative upstream
repository/immutable revision and acquisition checksums, and authoritative
license/notice evidence permitting the intended use. Sanctioned official-host
access is needed if those verified local assets are not supplied.

The historical `distilbert/distilgpt2` candidate revision is
`2290a62682d06624634c1f46a6ad5be0f47f38aa`. Its recorded official config request
failed with proxy CONNECT HTTP403/curl56; no model-host retry, bypass, credential
or network-setting expansion occurred in this task. Untrained output cannot
close T3. GPU, native UI, model quality, performance, allocator-memory and full
text-to-image acceptance remain outside this qualification.
