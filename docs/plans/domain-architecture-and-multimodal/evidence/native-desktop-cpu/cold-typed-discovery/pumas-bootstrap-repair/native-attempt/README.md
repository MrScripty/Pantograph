# Native authoritative bootstrap attempt

Run 37530472028, job 112498248639 executes
`fffe5342a29f903136186f79de67c99cd32d7711`, tree
`5adc052e54363307373dc43e915cc21055ea1c8a`, parent preserved `69e6b416`.
Artifact 11445275701 has 23 members, 440985 bytes, SHA256
`235b9d8ad91030c375eccf1b6124f9a031907664be2cef379148a44f07e53d3a`.
All original JSON/PNG bytes are retained; text/log/HTML is losslessly gzipped.
The complete masked job log and 13 extracted actual-owner records are included.

Actual native build and all eight startup tests pass. Save/reopen, three visible
wires, executable current validation, typed Resolve and GUI Submit are exercised.
Initial Resolve remains Missing before the async producer has a queued request.
At 21:10:44.529Z and 21:10:44.537Z the actual Pumas owner reports current
contract version 1, model embedding/qualification/synthetic-bert-8, platform
linux-x86_64, backend candle, validation resolved, no errors and no bindings.
This establishes authoritative native resolution, not scheduler admission.

Run `run_37c0f197-82f9-478a-8836-42cc1e509bfa`, session
`6a072083-76b7-4769-ada8-81aa5680f869`, saved snapshot
`wfvalsnap_a7b4488e-9373-43a0-bee6-7b6423c1ce6f` retains requirements ID
dependency-requirements-blake3:5106e42ce62356f6f12adcc0419aa6407f44b2a04b443ba559834ec4b6bef01d.
61 public read-only observations over 120 seconds report queued,
dependency_readiness_pending and zero output artifacts, even after resolution.
No post-resolution seed/admission result is captured, so its next blocking
transition remains unknown.

The GUI test fails because it incorrectly reads io_artifacts on the inspection
response. The actual inspection DTO contains run_graph, run and projection
states; artifact rows require workflow_io_artifact_query. The reported error is
Cannot read properties of undefined (reading 'some'). This harness error does
not establish successful execution or explain the still-zero output count.
It must be corrected without converting pending or empty output into success.

CPU output and loading remain unqualified. No GPU, pretrained or broad loader
claim follows from the successful build, startup or owner resolution.

Contract correction after capture: the inspection DTO does define io_artifacts,
but omits that field when the list is empty. The preceding observed response
therefore represents an empty list, not absence of an artifact capability.
The harness successor uses workflow_io_artifact_query's explicit artifacts array
and retains zero outputs as failure. Original captured bytes are unchanged.
