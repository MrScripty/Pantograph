# Native text ingress reaches dependency readiness admission; CPU output remains blocked

[Run 37517106108](https://github.com/MrScripty/Pantograph/actions/runs/37517106108)
executes ce6b7bc6e3c88c9c59f66a899a203c1715363ce9, tree
a8571236894c4bba3135d309559aa61fffc03bd9. Native build/startup tests,
cold-owner, save/reopen, wires, typed Resolve and GUI Submit pass their earlier
boundaries. The owner records queued run
run_5dda0631-cb25-4504-89ea-c6913f76e6d5 with execution session resume state
dependency_readiness_pending. Actual GUI error: scheduler dependency readiness
admission failed: workflow service operation failed. There is no selected
runtime/device, no start/completion timestamps and zero retained output artifacts.
No CPU execution/output or broad loader/GPU/pretrained qualification is established.

A separate local diagnostic uses the actual public requirements producer and
readiness lifecycle to reproduce canonical identity rejection. Graph production
sets task_type None and platform_context Some(host OS/arch); scheduler readiness
sets task_type Some(task kind) and platform_context None. Both participate in the
requirements hash. The focused test returns the nested invalid-request message
that the requirements ID does not match the saved validation proof. This is a
local reproduction and source comparison, not an observed native inner-error
capture. The temporary diagnostic test is removed from production source; its
exact replayable patch, audited feature graph and successful guard-rejection log
are preserved. Apply the patch to the executed source to rerun the diagnostic.
No proof/readiness guard is bypassed and no readiness receipt is manufactured.
Carrying the producer-owned canonical planning identity into admission remains
the next unresolved integration boundary. No successor run is active.

All twenty-two members, original images/JSON and masked job log are preserved.
Artifact 11438650692 SHA256 is
8575ede33b077dde517a0c8dfa77aa0883442b443cdd9f383cfdd11a0f89b080.
