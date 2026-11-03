# Service timing correlation retention review repair

This narrow successor preserves frozen service timing `0dff986d` and accepted RAM
repair `43f0a777`. CUDA observation checkpoint `4f189dd9e506205c0e5d8dca5fd403f57f4dfa32` remains
separate.
It closes the timing review finding that caller request IDs were copied verbatim
before gateway validation. Direct gateway callers can supply unbounded IDs, and
host ID validation checks trimmed contents while retaining original whitespace.
A bounded number of records therefore did not bound retained telemetry bytes.

The capture now emits optional `execution_request_id_digest`: exactly 64 lowercase
BLAKE3 hexadecimal characters computed from the original request-ID bytes. The
wire field name explicitly identifies the correlation representation. An absent
request ID remains absent; supplied IDs, including rejected blank values, have a
digest. There is no trimming, truncation or normalization of actual execution
identity. The digest is for telemetry correlation only, not authorization,
admission, request identity or a model/runtime timing profile. No raw request-ID
payload is retained in the timing record. Hashing runs only with opt-in capture.
This bounds retained correlation bytes, not the cost of hashing an arbitrary input
or other existing execution/host journals.

A test-only regression is first executed against frozen pre-repair production
source. A one-MiB padded valid ID executes successfully but its timing row retains
1,049,163 bytes, failing the byte-bound assertion. After repair the same test
passes for ordinary, padded and oversized IDs while the backend receives the
original bytes unchanged. Additional rejected-call regressions cover absent,
blank, oversized and padded IDs: failed attempts retain bounded digest metadata,
all phases remain not reached, and no phase clocks or backend loads run. The
existing disabled-default test now uses a padded one-MiB ID and still reads no
phase clock or owner facts.

The existing actual selected-text host-port regression also uses the padded ID.
It verifies that the response preserves the original execution identity, the
captured correlation is 64 characters, the serialized timing record stays below
4096 bytes, all four observed phases remain available and the full peak claim
remains held until custody release. Request validation and host limits are not
changed or weakened. The 4096-byte test is an emitted-record regression bound,
not a new limit on execution IDs or arbitrary externally constructed DTOs.

Six portable timing-contract tests, ten focused timing lifecycle tests and the
padded real-host regression pass. All 725 inference library tests pass after a
clean local-core rebuild; all 66 focused host tests pass. The full embedded suite
executes 502 tests: 500 pass and the same descriptor-count and warmup-timeout
baseline failures remain. Warning-deny all-target mixed-backend Clippy, formatting,
critical/accessibility/staged traceability gates, 28 traceability tests and nine
ONNX no-build-download graphs pass.

Two earlier serial inference runs retain a failure in the unchanged fresh-backend
lifecycle fixture; its isolated run passes. Source inspection identifies an
existing alias collision: registration keeps both production `PyTorch` and fixture
`pytorch`, while canonical lookup walks a HashMap and can select the production
factory. The failed runs then attempt a real PyTorch load against the suite's
Transformers stub, which lacks AutoModelForCausalLM. Both registry and fixture
blobs are unchanged by this repair. A separate clean frozen `0dff986d` replay
passes all 723 tests, and the clean successor run passes all 725. Earlier failures
remain in evidence; this does not claim deterministic reliability of that existing
fixture or repair the unrelated alias registration contract. Parent reports approved main `4153772634269e342a8b0cca797f1cd6716f18a5`
contains descriptor/sampler repairs; eventual integration will preserve history
with that main under parent coordination, rather than duplicating those fixes.

Configured estimates, successful observed spans, unavailable identity, measured
zero, runtime generation/source fencing, full peak claims and lifecycle accounting
are unchanged. Native exact-model timings, GTK/WebKit, physical GPU execution and
unsupported native RAM mapping remain unqualified. No ranking or PR changes are
made.
