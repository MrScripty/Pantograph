# Follow-Up: GUI Smoke Harness

## Objective

Add a repository-owned GUI smoke harness for workbench visual and interaction
verification so future Stage `12` and frontend work can validate Scheduler
default loading, toolbar navigation, active-run propagation, Settings ownership,
and artifact preview lifecycle with screenshots or equivalent browser evidence.

## Scope

In scope:

- Select a browser automation tool such as Playwright or an equivalent harness.
- Add deterministic smoke checks for the workbench shell and core pages.
- Capture or assert visual state for Scheduler default page, page navigation,
  no-active-run states, Settings policy forms, and I/O artifact preview
  lifecycle using mocked backend data.
- Add CI/tooling commands and documentation for running the harness.

Out of scope:

- Replacing unit, presenter, contract, lint, or accessibility gates.
- Testing real model execution, distributed networking, or Node Editor
  authoring.
- Adding broad screenshot coverage for every graph editing interaction in the
  first slice.

## Milestones

1. Choose the harness and dependency boundary.
2. Add mocked workbench boot fixtures for deterministic page data.
3. Add Scheduler-default, navigation, active-run, Settings, and I/O smoke
   checks.
4. Wire a documented npm script and CI-ready verification command.

## Verification

- New GUI smoke command passes locally.
- Existing `npm run test:frontend`, `npm run lint:a11y`, `npm run lint:full`,
  and `npm run build` continue to pass.
- Screenshots or equivalent browser assertions prove the workbench is nonblank,
  correctly framed, and navigable at desktop and mobile-sized viewports.

## Risks And Mitigations

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Browser dependency adds large install cost. | Medium | Keep the harness optional or CI-scoped until release tooling accepts the cost. |
| Smoke tests become flaky because backend data changes. | Medium | Use mocked workflow service responses and stable fixtures. |
| Screenshot assertions overfit styling. | Medium | Prefer structural and visibility assertions, using screenshots for gross layout regressions. |

## Re-Plan Triggers

- The repo adopts a different frontend test runner.
- Tauri desktop integration becomes required for the smoke checks.
- Workbench routing changes from Svelte store navigation to browser routes.

## Completion Criteria

- The repository has a documented command for GUI smoke checks.
- The harness covers Scheduler default loading and workbench page navigation.
- Stage `12` verification can cite the command instead of recording a harness
  gap.
