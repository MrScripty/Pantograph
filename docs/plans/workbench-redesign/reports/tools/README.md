# Walkthrough tools

Evidence tooling for the headless-Chromium walkthroughs recorded in the ledger.

- `fixture.js`: a stand-in for the Tauri host injected before the page loads.
  It answers `workflow_run_list_query` with fixture runs and rejects every
  other command the way an unreachable host would. Page milestones add their
  own `fixture-<page>.js` that loads this file's runs and answers the commands
  their page needs; the base file stays unchanged.
- `cdp-init.cjs`: drives the headless shell over the DevTools protocol, injects
  a fixture with `Page.addScriptToEvaluateOnNewDocument`, runs the steps in a
  JSON file and captures screenshots. Run it with `node cdp-init.cjs <steps.json>`;
  read its header for the step shapes.
- `steps.json`: the M0 picker walkthrough.
- Page fixtures, each loaded after `fixture.js`: `fixture-scheduler.js`, `fixture-diagnostics.js`, `fixture-io.js`, `fixture-network.js`, `fixture-graph.js`, `fixture-node-editor.js`, `fixture-library.js`, `fixture-sessions.js`, `fixture-settings.js`.
