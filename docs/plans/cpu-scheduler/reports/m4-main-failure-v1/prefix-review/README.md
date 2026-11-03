# Preserved M4 prefix review

`review_prefix.py` is a Python standard-library, read-only analysis of the original
failed main attempt. It does not invoke a model, rerun a workflow, refresh an
original projection, or modify original evidence. It reads frozen source from Git
objects and opens SQLite only on a temporary byte-copy. Outputs use exclusive
creation and must be outside the preserved input directory.

Reproduce with a fresh output path:

```sh
python3 /workspace/m4-prefix-review-v1/review_prefix.py --output /tmp/m4-prefix-reproduced.json
```

`final-report.json` includes original-archive verification. `report.json` retains
the initial successful analysis before the archive check was added. Both preserve
the original attempt's exit 1 and mark M4-C4 Partial/Blocked. They verify one actual
completed native call and its retained vector, not the unexecuted 3-versus-5
protocol. Separately declared cancellation evidence is outside this review.

The original 54 main files match the earlier independent fingerprint. Two later
diagnostic SQLite sidecars are transparently recorded; both already occur in the
original 71-file archive. This script neither created nor modified those files.
The original archive remains unchanged.

The vector body and public return agree and match the committed independent
golden within declared tolerance. The raw SQLite/events contain the completed
run and vector facts, while all inspected projection tables and projection state
are empty. Frozen source inspection explains the producer's projection-dependent
missing-artifact error. This causal conclusion is source-assisted analysis, not
a replay or promotion of the original failed oracle.
