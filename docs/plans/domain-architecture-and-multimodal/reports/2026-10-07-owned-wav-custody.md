# Owned WAV source and completion custody

A typed audio_wav MediaArtifactRef can supply the scheduler-selected CPU audio
route from the workflow-owned artifact store. Host byte ingress accepts canonical
PCM16 RIFF/WAV, mono or stereo,8–48kHz, up to five minutes AND16MiB including
headers. The original bounded inline route and direct audio handlers remain.
No path, URL, unbounded base64 recording or opaque option bag is admitted.

The immutable source identity binds workflow, original source-run and actual
BLAKE3 body hash. Later execution runs of the same workflow retain this source
identity. Resolution checks the managed canonical body, retained/readable state,
TTL, format/role, cap+1 bounded read and actual hash under the artifact writer
lock. Existing artifact budgets and retention policy remain authoritative.

A globally shared two-slot admission precedes disk read. Its permit moves into
the real blocking read/hash job, then into the immutable snapshot. Caller loss
does not stop that job or release admission while physical bytes remain in use.
Queued cancellation is checked after admission and before any new read. Selected
load/forward retain the snapshot and backend custody through actual worker
completion, including Python PyBytes jobs. Cancellation suppresses outputs; it
does not promise bounded interruption of native or Python execution.

The deterministic regression pauses the actual shared store after its bounded
read and before hash verification. Weak-Arc evidence observes those exact bytes,
not an unrelated sentinel. Success, signalled cancellation, caller abort and
hash rejection prove the third admission remains blocked until completion while
an independent second permit is held. A queued request cancelled during that
pause is rejected with the cancellation diagnostic before another disk read or
model effect. Per-store barriers are cfg(test); the storage observer seam is
available only with test-support, enabled by the embedded runtime dev dependency.
No new package or dependency version is introduced; production builds use the
same bounded reader and verifier with a no-op observer.

TranscriptText carries bounded64KiB UTF-8 output through saved/reopened results
and semantic text, prompt/system, embedding and rerank-query consumers. Ordinary
String and control limits remain unchanged. Unknown/overbound formats, durations,
options, bytes, transcripts and timing fail closed; no truncation or fabricated
ASR timestamps is supplied. The private owned-WAV worker uses real SoundFile
and NumPy decoding through the existing selected CPU ASR seam.

Tests use controlled owners and existing synthetic fixtures. Native ASR model
compatibility and Cohere compatibility remain unqualified. No models/ORT runtime
are downloaded and no dependency/runtime upgrade is required. Hostile concurrent
filesystem replacement is outside the trusted managed-store threat model.
