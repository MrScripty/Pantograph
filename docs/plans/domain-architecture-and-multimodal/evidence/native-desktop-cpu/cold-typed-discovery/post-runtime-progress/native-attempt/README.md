# Twenty-first native attempt: completed graph, wrong artifact expectation

[Run 37545703782](https://github.com/MrScripty/Pantograph/actions/runs/37545703782),
job 112549066558, executes 6a7d9b1aaa866371695a1b68596c8495faf48880,
tree 20cfa27b46d7405f870af5193651dcba4fab5fc3. Native build and eight startup
tests pass, then real save/reopen/Resolve/single Submit. The embedding runtime task
and terminal attempt are Completed with three outputs and candle/candle.cpu/cpu.
The same whole graph now completes with no terminal error. The public artifact
query retains the prompt input and two roles for the same vectors.vector payload
(workflow_output and node_output), each vector record reporting 160 bytes.

The harness times out looking for infer.embedding and infer.metadata. The normal
GUI submits output_targets:null; the authoritative host workflow I/O exposes the
vector sink as the default workflow output, not those internal inference ports.
The source's default output projection and actual retained rows agree. This is
a harness expectation defect, not proof of another runtime failure or missing
vector artifact. No vector body was read, so numerical native CPU acceptance
still awaits the corrected public artifact/body check. Internal metadata is not
exported by this GUI default and must not be described as a retained metadata body.
Read actual CPU selection from the public scoped scheduler-attempt timeline.

Artifact 11450853556 is 495563 bytes, SHA256
1f223e064f087db553a65f6be237668da1540ae11d89553b6ab3c624a30a2250.
All 23 original members, full masked job log and 38 actual-owner records are
preserved. archive-members.json verifies original bytes, with text/log/HTML stored
in lossless gzip. All 61 observations and the incorrect harness failure remain
intact before correction. No model, fixture, proof, production default-output,
authentication or download change follows. GPU, pretrained and broad loader
qualification remain incomplete.
