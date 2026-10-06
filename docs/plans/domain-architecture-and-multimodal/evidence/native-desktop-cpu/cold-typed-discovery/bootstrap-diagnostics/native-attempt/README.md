# Actual native bootstrap diagnostics: first blocked transition captured

[Run 37526479578](https://github.com/MrScripty/Pantograph/actions/runs/37526479578)
executes 7ec336032a4e9910a26f65ef2e34d930a786d0bb, tree
adc9932f9215cbf5d6b9b7a81f8e3b850b8f250b. Actual app build and eight native
startup tests pass. The unchanged saved/reopened graph reaches typed Resolve
and GUI Submit. The owner logs capture the first blocking transition:

1. Graph Resolve's actual provider result is Missing/Unavailable with failure
   RequirementsUnavailable: No fresh dependency readiness snapshot matches the
   request. It carries no requirements/bindings while the intent is request_ready.
2. Scheduler consumes the same requirements ID, descriptor, graph revision,
   validation session and saved snapshot. Registry seed rejects the actual
   Missing result: only Resolved/Ready results can seed payloads.
3. The task transitions to paused_deferred, state version 3, and queues one probe.
4. The normal producer poll rejects lookup with MissingPayload for the same ID.
5. Actual automatic retries preserve the saved snapshot and identity, reaching
   deferred state versions 5/7/9/11. Read-only samples over 70 seconds retain the
   same queued run with dependency_readiness_pending and zero outputs.

The scoped run is run_845edcb4-8b01-4cbf-b14b-6438fb4922da. The saved validation
snapshot is wfvalsnap_211237cd-4b7d-4488-92cd-55fb36f94665. This is actual native
provider/consumer evidence, not inferred fixture success. All 23 artifact members,
complete masked job log and 12 verbatim-derived structured diagnostic records
are preserved. Original JSON/PNG bytes are unchanged; text/log/HTML are losslessly
gzipped. Artifact 11443605285 SHA256:
50163fcaf05389a2e1a55bb459988150a9587902631eaefa35eea15cd677cf89.

This establishes the cold requirements seed/snapshot circular dependency. It
also corrects the earlier queue hypothesis: rejected seeding does enqueue work,
and a real owner loop retries it. No repairs, payloads or ready receipts have
been substituted in this diagnostic run. Actual CPU output remains unqualified;
no GPU/pretrained/full production-loader claim follows.
