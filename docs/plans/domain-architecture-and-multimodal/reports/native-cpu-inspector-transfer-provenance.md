# Source-only native inspector transfer

This derived commit is based directly on published source
cd5f512c6febc371ba876a27f33b3914b3a26594, tree
03f72df7a7a046253f76915a735dbfbe9c708e9d. Its only source change is the exact
inspector harness patch extracted from the preserved unpublished candidate.
The original two commits remain untouched on qual/native-desktop-cpu-graph:

- abe911fa5d71767ce2fcd1fbc047236d96673d87, tree
  da653ccb1127f32509f0acc42a3110b7ff8bc2a7, parent cd5f512c6febc371ba876a27f33b3914b3a26594:
  complete native numerical evidence. Its runtime files are excluded from this transfer.
- 5e614e98fea5a6e4fa7f95ad156cd2594f638ab8, tree
  dfbe7b7b09ff399a43a0d9b23d70095d1e2e4308, parent abe911fa5d71767ce2fcd1fbc047236d96673d87:
  inspector harness and local-check evidence. Only its harness change is transferred.

The harness selects the exact completed run in Scheduler, opens I/O Inspector,
selects the vector sink, waits for its actual artifact card, clicks Read and
requires the displayed JSON to equal the actual canonical vector body. It records
that preview only when a future native qualification actually executes it.
The harness file is byte-identical to the original unpublished candidate.
The source patch SHA256 is
84b10ade9bd82dfe9b80d49b1fb550c50ad0923e1f23fdcc7e87fdb1ad7b4082.

Published numerical CPU qualification remains at the exact published source above:
[Native run 37547588348](https://github.com/MrScripty/Pantograph/actions/runs/37547588348)
passed real save/reopen/Resolve/single Submit, actual canonical vector-body oracle
verification, a completed Candle CPU attempt for the same run and eight startup
tests. Its original CI artifact is 11451667987, 313602 bytes, SHA256
7cef556b4629600dd4e52f1c5d1dcb03ddc4f35d9d59be4278b8180e8548fcaa.
Fetch that original artifact through GitHub Actions for runtime evidence; its
payload is not duplicated in this source-only transfer. Artifact metadata was
checked through the authorized Actions API during preparation: expired=false,
expires_at=2026-10-20T23:52:15Z. This timestamp is an observation, not a retention
extension or guarantee of future availability.

The derived inspector assertion has local syntax, lint and scope-contract checks
only. It requires a future actual native GUI rerun after source publication is
resolved. Passing published numerical output does not establish this later GUI
display check, internal inference metadata export, pretrained/GPU behavior or
general production-loader qualification. Production source and fixtures are unchanged.

This commit carries only harness source and this sanitized provenance document.
It adds no model weights, credentials, runtime JSON, screenshots or runtime logs.
Its published parent is a required prerequisite, not part of the minimal bundle.
Original candidate commits, complete original evidence and protected checkouts
remain preserved. This packet is preparation only: no public upload, Git push,
credential change or native CI rerun is performed.
