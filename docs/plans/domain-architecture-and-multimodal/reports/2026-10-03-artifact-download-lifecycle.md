# Artifact download anchor and URL lifecycle

## Decision

The critical anti-pattern gate identified imperative insertion of a temporary download anchor in IoInspectorPage. Render pending anchors through Svelte and activate them after mounting. A small download lifecycle owner retains each object URL until its own 30-second release or activation failure. Successful activations retain their grace timer after component disposal; unactivated URLs are reclaimed immediately. Pending completion promises settle on disposal, late URL admission is rejected and reclaimed, and activation after disposal cannot click a detached anchor. The original click error reaches the existing artifact error formatter.

Concurrent requests retain distinct IDs, URLs, and filenames. Existing filename/media derivation, read authorization, loading state, and rel protection remain. Lifecycle-generation checks prevent late read completion from creating a download after unmount. The scheduler and artifact backend are unchanged.

## Verification

- Six direct lifecycle tests cover concurrent downloads and independent delayed release; original activation error; unmount with pending and active downloads; late admission; and unmount during activation.
- The lifecycle and existing inspector presenter suite pass 32 tests; all frontend tests pass 546/546.
- TypeScript, full ESLint, critical anti-pattern gate, and Svelte client compilation (zero warnings) pass locally.
- This is controller execution plus component compilation, not a real-browser download receipt. Browser download policy and actual saved bytes remain outside this local qualification.
- Independent review and exact-head hosted qualification are pending. The two a11y suppression findings, Clippy, and dependency audit remain separate.


## Independent review repair

Review of initial tree a367c62b8b30729e1d5fa4a95d6e247337ca6756 found that immediate revocation of already-clicked URLs on unmount shortened the pre-existing browser grace period. The revised owner keeps those timers after disposal and clears component publication/completion callbacks. Tests distinguish clicked-then-unmount and successful versus failed activation that synchronously unmounts. Browser timer/revocation defaults live in the controller, so retained timers do not need component callbacks. Narrow re-review is pending.

Independent narrow re-review accepted repair tree 1521cf361523f32ab5a127955cfed3fb87f8ee0c. The grace timer confirms URL release only; it cannot confirm that the browser saved a file. Hosted and real-browser qualification remain distinct.
