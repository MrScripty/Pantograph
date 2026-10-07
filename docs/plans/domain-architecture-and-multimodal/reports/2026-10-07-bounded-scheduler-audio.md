# Bounded scheduler audio slice

This additive route connects a scheduler-selected `audio_transcription` task to
an existing local Hugging Face compatible ASR directory through `pytorch.cpu`.
It retains the direct audio route because the selected route covers only the
small inline WAV contract below. It does not choose, acquire or download a model.
Lanternwake's STT preference remains Cohere Transcribe; the synthetic fixture and
this backend compatibility check do not establish Cohere native support or set
a different product default.

## Admission and outputs

Inline base64 strings and encoded-audio objects are admitted. WAV must have an
exact RIFF/WAVE header, one PCM16 `fmt ` chunk of 16 bytes followed directly by
`data`, one or two channels, and a sample rate between 8,000 and 48,000 Hz.
Lengths, block alignment, byte rate and optional sample-rate metadata must
agree. There must be at least one complete frame, at most one second of frames,
at most 48 KiB decoded and at most 64 KiB of encoded data. Extra chunks, trailing
bytes, alternate MIME types, audio references and unsupported formats are
refused. The route does not truncate or convert an input.

Optional language, prompt, ASR task and finite positive chunk length are passed
to the worker. Chunk length is also passed during actual ASR pipeline loading.
Absent/null and empty-map backend options both mean no controls and become
`null` at the selected worker boundary. Nonempty options and streaming are
refused. The response retains the eight direct-route ports: `response`,
`stream`, `text`, `language`, `duration_seconds`, `segments`, `metadata` and
`diagnostics`, including nullable values.

## Identity and custody

Request, scheduler decision, package and separate executable load target must
agree on the model, artifact, canonical task and explicit/known revisions.
Only a valid local directory with current package facts, known content
fingerprint, closed remote-code policy and concrete matching CPU decisions is
supported. Logical package paths are not executable load paths. Both load and
forward envelopes carry the scheduler execution request ID.

Each selected audio owner has a private embedded module using the existing
worker source; its ASR globals cannot be replaced or stopped by another live
embedded runtime. Blocking jobs retain that module until actual completion,
and retirement attempts cleanup and removes its registration. Explicit successful
shutdown supplies the acknowledged release boundary; destructor cleanup is
best effort. Generic/direct
worker globals keep their inherited behavior. Private readiness and release
state are separate from generic readiness, so selected-only stop does not
shut down another runtime's shared generic worker.

The public direct route can also run after a selected-only load. It records
possible generic ASR residency before worker execution, separately from the
private selected module, and retains direct job completion after caller loss.
Stop and model replacement await those jobs. Stop clears generic ASR ownership
only after acknowledged all-family shutdown; failed shutdown retains it for
retry. A selected-only owner that has never invoked direct ASR still leaves the
generic worker untouched. Deterministic public gateway regressions cover the
successful selected-to-direct transition, shutdown failure/retry, and caller
abort while direct inference is blocked.

Selected reuse requires a recorded exact target (including revision, content,
storage and path), CPU device and chunk length. Changed or unknown residency,
including a new Rust owner, requires an
acknowledged all-family shutdown before loading. Direct audio and other
effectful Transformers loading invalidate selected reuse. A failed shutdown
cannot authorize a replacement load.

The gateway moves its exclusive backend guard into an owned operation which
awaits actual blocking Python load/forward completion. Cancellation and caller
loss suppress publication but do not release custody early. This establishes
observed completion, not bounded interruption of arbitrary Python execution.
Batch envelopes execute sequentially with each member's request identity; this
does not advertise backend-native audio batching.

## Evidence limits

The checked-in 52-byte WAV contains four synthetic PCM samples. Controlled
backend and Python fixtures test input refusal, eight-port direct-route parity,
physical target and CPU envelopes, actual pipeline chunk/policy projection,
reuse fences, cancellation/caller loss during load and forward, sequential
member identity, and a saved/reopened graph through the production scheduler.
No real model or ORT runtime is downloaded or run by those fixtures. These tests
do not establish transcription quality, arbitrary recording support, GUI
Inspector display or the proposed research-v2 scheduling algorithm.

Scheduling uses the existing production queue/admission and eligible-candidate
selection implementation. The migration supplies selected execution and custody
contracts; completion-oriented lookahead planning remains separate research.
