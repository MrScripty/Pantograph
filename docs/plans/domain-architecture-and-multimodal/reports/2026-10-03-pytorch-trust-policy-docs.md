# PyTorch trust-policy documentation correction

The PyTorchBackend class comment incorrectly described unconditional trust_remote_code=True. The actual loader chain is policy-driven and defaults closed: ModelRemoteCodePolicy defaults to Deny; PyTorchTransformersTrustPolicy derives allow_remote_code only from explicit Allow; worker_contract.py defaults the missing flag to false; worker.py defaults trust_remote_code=False and rejects required custom code when it remains closed.

Correct only the class documentation to describe that existing behavior. No runtime, policy default, validation, accepted-source, worker, or credential behavior changes. This is a source-grounded documentation correction, not a comprehensive security audit or authorization to load custom code.

Whitespace/formatting checks pass. Runtime tests were not rerun for this comment-only change. Publication receipt pending.

Source review accepted the exact documentation-only checkpoint 52e511f44a5de9113b7ec32c5f2df4f4935cbc34. The review confirms documentation accuracy only; no additional runtime execution is claimed. Publication follows the separately reviewed llama milestone; no runtime implementation changed in this slice.
