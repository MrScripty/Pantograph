# Issues

- IMG-1 (resolved): Graph Image Output now validates typed media references, reads
  retained bytes and gates preview/download on real raster decoding and MIME.
- IMG-2 (resolved): unsupported image-input submission is rejected explicitly;
  scheduler rejection remains authoritative and authored data is preserved.
- IMG-3 (deferred): diffusion execution and useful generated images need weights
  and runtime qualification. Owner: runtime integration; revisit when supplied.
- IMG-4 (resolved by hosted acceptance): local sandbox startup is unavailable;
  ordinary hosted sandbox acceptance passed on 051b08ed, including all browser
  assertions and cleanup. Final committed-head checks remain required for merge.
- IMG-5 (resolved in slice): local unsupported error previously became transport_error
  at Toolbar catch; now uses existing capability_violation with unsupported details.
- IMG-6 (resolved): inherited capture-phase click suppression prevented
  Svelte delegated Download/Expand actions from running. Actual installed compiler
  and event-dispatch source prove the failure. This image node's controls now use
  a single direct capture action and capture-phase pointer/mouse isolation; browser
  assertions verified both activation and ancestor isolation. Other node owners
  are outside this bounded image-control family and unchanged.
- IMG-7 (resolved): hosted fixture teardown raced Chrome profile writers and
  masked the original assertion with ENOTEMPTY. Owner: fixture lifecycle; Linux
  process-group shutdown, primary-error preservation and bounded rm retry replace
  parent-only termination. Hosted acceptance on 051b08ed confirms completed cleanup.
- IMG-8 (resolved): fixture Enter lacked CDP character
  text, suppressing native keypress/button activation. Independent source review
  established the driver defect. Enter now supplies CR text/unmodifiedText and
  Tab/Escape use rawKeyDown; actual hosted keyboard/modal/focus assertions passed.
- IMG-9 (blocking qualification, fix now): d9e52e51 browser job 115613091802
  timed out reopening via pointer Expand after Escape/focus checks passed. The
  fixture waits for native open=false, which precedes queued close-event delivery
  and component updates. Observe real close plus Svelte completion before the next
  action; preserve all native input/focus/reopen assertions. Exact old event order
  and a production defect are not established by that log. Owner: browser fixture.
