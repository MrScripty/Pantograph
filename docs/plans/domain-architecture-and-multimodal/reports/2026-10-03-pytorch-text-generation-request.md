# Named PyTorch text generation inputs

The existing seven scalar inputs are grouped in public PyTorchTextGenerationRequest for the two top_k entry points and the private envelope helper. No existing request type represented exactly these inputs: the private worker DTO includes additional denoising/block controls and arbitrary kwargs, and general generation options carry broader policy. The new input type has no defaults or serialization contract and is re-exported beside PyTorchBackend under the existing feature gate.

All three repository callers of the two public methods are migrated. Legacy generate/generate_stream signatures remain unchanged and wrap the same values. Strings move through the new container without added production clones; top_k still maps only to transformers_kwargs and absent values remain absent. The private worker DTO, request IDs, cancellation defaults, validation, stream/job behavior and Python worker are unchanged. The two changed public Rust call shapes are explicitly recorded in the changelog.

Tests assert exact generated and streaming wire envelopes for all seven inputs, including the existing cancellation default, and compare legacy/named early validation errors and bounded stream closure. Existing top_k/unknown-policy envelope tests remain. Hosted commands explicitly enable backend-pytorch and require nonzero new-test discovery; no actual model inference is claimed by the validation-only tests.

Formatting/whitespace pass locally. No local Rust execution is claimed. Independent source review and fresh hosted PyTorch contract/Clippy qualification remain pending. Public llama-server requests and other error owners are outside this slice.

Independent source review accepted tree 6f40ff092f2b12780fb2152d0755ae3b7c531c23: exact owned inputs, unchanged legacy defaults/worker mapping, and bounded validation/stream tests. Fresh hosted execution remains required. The existing remote-code documentation/policy wording is outside this arity slice.
