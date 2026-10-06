# Native desktop CPU qualification evidence

**Result: blocked before native build or launch.** This evidence belongs to
`qual/native-desktop-cpu-graph` source `ecc4194adbcd5a198fb97cf47198427e3002773e`,
tree `53c60a1d27a713d6b2cc3dcd04415859a1c8b953`, directly based on accepted
frozen chat `0d7d573f36d8f25f3ae5c8adb25ab86c42e1dd8c`.

[Run 37480189945](https://github.com/MrScripty/Pantograph/actions/runs/37480189945)
failed read-only native admission with exit 2. The source/fixture checks and
evidence upload passed. Native build and application steps were skipped. No
native screenshots, actual saved/reopened workflow or CPU output were produced.

- `hosted-artifact/`: byte-preserved four files downloaded from artifact
  `11420323376`; source binding, prerequisite result, fixture description and
  proposed graph. ZIP SHA256 is recorded in `qualification.json`.
- `hosted-job.log`, `hosted-jobs.json`, `hosted-artifacts.json`: GitHub run
  evidence; log credential values were already masked by GitHub.
- `local/`: complete effective desktop feature graph compressed without a
  timestamp, local prerequisite/launcher refusal, root gates, traceability and
  five passing existing launcher tests. No local native build was attempted.
- `qualification.json`: exact scope, failed prerequisites and skipped acceptance.
- `SHA256SUMS`: SHA256 of every evidence file except the manifest itself.

The expected vector comes from committed untrained synthetic BERT weights.
Proposed graph metadata is controlled discovery in a separate temporary Pumas
library. It is not native runtime or real Pumas discovery evidence. Pretrained
quality, GPU, broad production loading and post-start cancellation remain
unqualified. See the [report](../../reports/2026-10-06-native-desktop-cpu-qualification.md).
