# Issues

- IMG-1 (blocking, fix now): Graph Image Output interprets typed media references
  as PNG base64; preview/download invalid. Owner: frontend image result component.
- IMG-2 (medium, fix now): frontend input materialization silently omits image-input.
  Scheduler rejection remains authoritative. Owner: submit frontend.
- IMG-3 (deferred): diffusion execution and useful generated images need weights
  and runtime qualification. Owner: runtime integration; revisit when supplied.
- IMG-4 (blocking acceptance): C3 cannot execute because Chromium setuid helper and
  user-namespace sandbox are unavailable. Owner: environment/verification operator.
  Revisit with sandbox-capable environment or explicit scoped test-run authorization.
- IMG-5 (resolved in slice): local unsupported error previously became transport_error
  at Toolbar catch; now uses existing capability_violation with unsupported details.
