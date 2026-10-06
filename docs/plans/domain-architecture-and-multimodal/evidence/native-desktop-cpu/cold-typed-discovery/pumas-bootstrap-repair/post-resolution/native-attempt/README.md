# Native seed succeeds; readiness consumes the wrong action

Run 37533284903, job 112507769238 executes
2c750701c3e1e073c1bf07280ec523269c97be03, tree
41ce4b9fe4de7550e35f0c7809009d9c51b3824c. Artifact 11446316351 has
23 members, 447411 bytes, SHA256
b8970532236ac1460a1405a9dd604518fd0a2ee271dd38d09a666a77e77b53ea.
All original JSON/PNG bytes, losslessly gzipped text/log/HTML, the complete
masked job log and 52 extracted actual-owner records are preserved.

Native build and eight startup tests pass. Save/reopen, visible wires, typed
Resolve and the single GUI Submit are exercised. Pumas authoritative empty
resolution succeeds. At 21:35:43.306Z the normal consumer stores the Resolved
requirements seed. At 21:35:43.308Z readiness proof resolution still returns
Resolved with no environment reference; at 21:35:43.309Z infer is paused_deferred
version 13. Retries reach version 19 with no ready task or dispatch selection.
The actual readiness adapter requests Resolve instead of Check, so it reads the
requirements snapshot rather than the separate inventory Check snapshot.

The same run run_b33b58bb-81b3-454c-805f-c7e6e3457cb9, execution session
169f196c-2ee3-4f76-ba2f-34140cdfc515 and saved snapshot
wfvalsnap_142648b4-d7f0-43ee-88b6-88d5cadca593 preserve requirements ID
dependency-requirements-blake3:5106e42ce62356f6f12adcc0419aa6407f44b2a04b443ba559834ec4b6bef01d.
61 observations over 120 seconds retain queued/dependency_readiness_pending,
zero outputs and an explicit empty artifacts array from the canonical query.
The corrected harness fails on missing actual CPU output, not a DTO exception.

This establishes a second shared-boundary defect before further repair. Seed
resolution must continue to use Resolve; readiness evaluation must use Check.
Resolved-only/missing/unavailable/mismatched checks must remain non-ready.
CPU execution/output, GPU, pretrained and full production-loader qualification
remain incomplete; no fake readiness result or loader failure is inferred.
