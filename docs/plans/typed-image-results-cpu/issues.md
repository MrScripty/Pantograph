# Issues

- IMG-1 (blocking, fix now): Graph Image Output interprets typed media references
  as PNG base64; preview/download invalid. Owner: frontend image result component.
- IMG-2 (medium, fix now): frontend input materialization silently omits image-input.
  Scheduler rejection remains authoritative. Owner: submit frontend.
- IMG-3 (deferred): diffusion execution and useful generated images need weights
  and runtime qualification. Owner: runtime integration; revisit when supplied.
- IMG-4 (blocking acceptance): local sandbox startup is unavailable; hosted ordinary
  sandbox startup is proven. Owner: fixture verification. Final-source hosted
  browser assertions and cleanup must pass before C3 is accepted.
- IMG-5 (resolved in slice): local unsupported error previously became transport_error
  at Toolbar catch; now uses existing capability_violation with unsupported details.
- IMG-6 (P1, fixed in follow-up): inherited capture-phase click suppression prevented
  Svelte delegated Download/Expand actions from running. Actual installed compiler
  and event-dispatch source prove the failure. This image node's controls now use
  a single direct capture action and capture-phase pointer/mouse isolation; browser
  assertions must verify both activation and ancestor isolation. Other node owners
  are outside this bounded image-control family and unchanged.
- IMG-7 (fixed in follow-up, acceptance pending): hosted fixture teardown raced Chrome profile writers and
  masked the original assertion with ENOTEMPTY. Owner: fixture lifecycle; Linux
  process-group shutdown, primary-error preservation and bounded rm retry replace
  parent-only termination. Final hosted cleanup must pass before acceptance.
- IMG-8 (fixed in follow-up, acceptance pending): fixture Enter lacked CDP character
  text, suppressing native keypress/button activation. Independent source review
  established the driver defect. Enter now supplies CR text/unmodifiedText and
  Tab/Escape use rawKeyDown; native keyboard/modal/focus assertions stay required.
