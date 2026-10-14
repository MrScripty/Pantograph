# Candidate issues

- Resolved: inherited PR78 ownership/isolation findings are repaired in baseline
  0dc45263 and incorporated by normal history-preserving merge 039cee07.
- Resolved: initial Cargo update reselected three unrelated existing socket2
  edges. Restored their original bindings; `--locked` build and metadata checks
  passed. Parsed lock comparison confirms only the Pumas package changed.
- Resolved tooling setup: first staged traceability attempt could not find
  `commonmark` in the new worktree. Linked the existing same-lock node_modules
  verification cache, then reran the gate successfully. No hook bypass or npm
  dependency update. Removing this task worktree removes only that link.
- No API, compilation or runtime failure observed on the selected CPU edge.
- Remaining outside-scope gates: pretrained T3, historical-library migration,
  other targets, hosted candidate acceptance and parent release promotion.
