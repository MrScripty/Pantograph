# Authorized setup successor: actual native UI, blocked submission

[Hosted run 37482087035](https://github.com/MrScripty/Pantograph/actions/runs/37482087035)
executed source `df6498274c0c49670765c5a01d8536c416465201`, tree
`1883fe84310d8c11b64029a02506344134393a8a`, parent
`1af2d7376d460aea33d10f0b40f59805551f2ac0`.

Official Ubuntu dependencies and pinned tauri-driver `2.1.0` installed normally.
The complete feature checks passed before builds, with `ORT_SKIP_DOWNLOAD=1`.
The actual Tauri application built, launched on hosted Xvfb, saved/reopened the
graph through real native IPC, and displayed three nodes and typed ports.
The raw configured/failure screenshots are actual WebKit/Tauri captures.

The production Submit control remained disabled for 120 seconds with
**“Inference validation is stale for the current graph”**. No interface update
control was applied and no native CPU output was produced. Two edges are retained
in the saved graph JSON, but their wires are not visible in the screenshot.
The owner also logged `NonCanonicalLayout` for the controlled synthetic library;
the root cause of stale validation is not established by this attempt. No frozen
product source or submit gate was changed to force acceptance.

`hosted-artifact/` preserves all 15 downloaded artifact members. Text, HTML and
logs use lossless timestamp-free gzip; JSON and PNG bytes are unchanged.
Artifact ZIP SHA256 is
`fb338ae8e3f386ecd96238e959823750bfae88b2c8e8fb84a4ea7b1623523bcf`.
`hosted-job.log.gz` preserves GitHub's already-masked job log.
`setup-source.patch.gz` binds the four-file setup successor to its parent;
`local-desktop-features.txt.gz` preserves the local complete feature audit.
`qualification.json` records scope; `SHA256SUMS` binds every file in this folder.
The preceding attempt's evidence and hash manifest remain unchanged.

Native public scheduler execution/CPU output, real Pumas discovery, pretrained
quality, GPU, post-start cancellation and full desktop release acceptance remain
unqualified. See the [report](../../../reports/2026-10-06-native-desktop-cpu-qualification.md).
