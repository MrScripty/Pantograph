# Twentieth native attempt: embedding completes, downstream awaits input

[Run 37543486549](https://github.com/MrScripty/Pantograph/actions/runs/37543486549),
job 112541805258, executes 55e6d076571a683a3c5b87b3b48f813e72c8f60a,
tree d914f4bafec81c900cf9319246851b647057d05d. Native build and eight startup
tests pass, followed by real GUI save/reopen, visible wires, typed Resolve and
one Submit. The runtime task now returns Completed with three outputs. The exact
terminal attempt event is Completed and identifies candle/candle.cpu/cpu.

The same run subsequently fails with invalid_request: scheduler task 'vectors'
did not complete; final state was AwaitingInputs. All 61 public artifact-query
observations retain an empty array. No actual vector or metadata body is retained
for numerical verification. Runtime completion establishes progress past the
previous executable-target rejection, but does not qualify native vector output
or graph completion. Inspect the actual output propagation/finalization boundary
before proposing another source repair; the cause of AwaitingInputs is unproven.

Artifact 11450490195 is 446033 bytes, SHA256
a8f1decbc40f1427e84038fee4faea589186f68b3d5fa3360be155ce9c655507.
All 23 original members, full masked job log and 38 actual-owner records are
preserved. archive-members.json verifies original bytes, with text/log/HTML stored
in lossless gzip. This failure is preserved before another production repair.
No fixture or proof change, model/binary download, credential change, PR or merge.
GPU, pretrained and broad production-loader qualification remain incomplete.
