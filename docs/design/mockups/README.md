# Workbench mockups

Interactive mockups of the redesigned workbench, authored for the Claude
Design canvas and viewable locally.

- `boards/*.dc.html`: one artboard per tab (`Revised` is the Scheduler) plus
  the earlier rounds (`Main`, `Studio`, `Instrument`, `Dark`) and the
  `Current` baseline screenshot. `boards/shell.css` holds the shared tokens
  and primitives for the round-3 boards.
- `ctx-markup.html`, `ctx-mixin.js`, `inject.py`: the shared run-context
  picker, expanded into every board by `python3 inject.py` (idempotent).
- `viewer/`: `support.js` is a local stand-in for the canvas runtime;
  `build.sh --serve` builds the boards into `viewer/site/` and serves them on
  http://127.0.0.1:8765/; `cdp.js` drives headless Chromium for scripted
  walkthroughs and screenshots.

The implementation contract derived from these boards is
`../workbench-redesign-spec.md`; the plan is
`../../plans/workbench-redesign/plan.md`.
