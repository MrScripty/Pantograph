# Private embedded source payload layout

Fresh PR42 Clippy reports large variants in the package-facts bridge outcome and dispatch candidate source. Box only Projected.facts and Snapshot, keeping raw projection/snapshot DTOs and public APIs unchanged. The package bridge has one production constructor and five fixture constructors across three files. Candidate source has two constructors (default and explicit snapshot) and one raw-cloning reader. Borrowed readers retain deref coercion; the owned reader explicitly clones the raw snapshot rather than returning a box.

The regression compares complete raw package facts and snapshots, mutates a returned nested fact and snapshot version, then proves provider state remains isolated. Existing source-owner, stale/missing/blocked, resolver, and candidate-selection tests remain in the full embedded unit gate. Size bounds apply only to these private enums; no measured throughput claim is made.

Prior artifact-error head b73c5d1 has actual hosted concrete source/downcast and artifact-write tests passing. Its full embedded unit run has 449 passed and three failures: image dispatch gets RetryDeferred instead of RuntimeHostCompleted; resource-backed validation gets Blocked instead of Executable; publication dependency readiness gets Blocked instead of RequestReady with DriftDetected. Those failures remain separate and are not waived or classified as baseline without exact comparison. Format and workspace default/all-feature compilation pass at that head.

Root source review accepted all four files at frozen tree 614eb5f7f61ee8ffa96d4d07cdaa238f7275e1a2. Fresh hosted execution remains required. Local Rust execution is not attempted under the shared disk constraint; full pinned formatting is checked separately.
