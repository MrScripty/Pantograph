# Session owner graph on workflow load

The native glow qualification at `732b4824` stopped before Submit with stale
inference validation and no lifecycle events. The saved embedding graph had
revision `dde190203c4f499b`. Creating its edit session hydrates
`emit_metadata=false`, changing the owner revision to `157f87306e2aa806`.
The editor rendered the saved file instead of the owner's session graph.
Its refresh therefore returned stale before validation publication or Pumas fact
lookup. This evidence does not implicate the separately reported Pumas IPC
cancellation/reply-correlation defect.

Workflow loading now reads the canonical graph through the existing
`getExecutionGraph` backend method before activating the session. New workflows
also use the owner's graph and revision. Superseded or failed snapshot reads
close their candidate sessions, preserve the active graph, and cannot publish
late errors over a newer transition. No new status API or validation bypass is
introduced; the C# status projection candidate is outside this source change.

Fourteen session-store tests pass, including canonical semantic data/revision,
late snapshot rejection, cancelled read rejection, candidate cleanup after
failure, owner revision for empty graphs, and a new workflow superseded during old-session cleanup. The new regressions fail against
the prior source. All 680 frontend tests, typecheck, full ESLint, critical checks,
27 accessibility tests and production build pass. Traceability is checked with
an explicit change scope; an initial unscoped invocation was rejected and its
log retained. Complete effective desktop Cargo graphs were audited before
builds: dynamic ORT with disabled linking, no binary-download features, Pumas
`26a84e32`; `ORT_SKIP_DOWNLOAD=1` is enforced defensively.

This is controlled frontend evidence. The existing native assertions, glow
correction, PR64/65/67 source scopes and original failed artifacts are preserved.
Native scheduler admission, CPU execution, retained artifact Read and inspector
display require the separately bounded native continuation. No GPU, pretrained
model, real-user discovery or full production-loader qualification is implied.
