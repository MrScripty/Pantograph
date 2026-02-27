# Compliance Remediation Tracker

Last updated: 2026-02-27 (PR-4 implementation pass 1)

## PR-1 Security Boundary Hardening

Status: In progress

- [x] Add shared canonical path validator utility.
- [x] Apply validator to node-engine read/write file handlers.
- [x] Apply validator to workflow-nodes read/write tasks.
- [x] Apply validator to Tauri `load_workflow` command.
- [x] Apply validator to Tauri sandbox `validate_component` command.
- [x] Replace agent read/write tool path sanitization with canonical root validation.
- [x] Add traversal-focused unit tests in node-engine/workflow-nodes.
- [ ] Add/expand tests for Tauri command/tool path validation.
- [x] Run targeted test suites and fix regressions.

### Verification run (2026-02-27)

- `cargo test -p node-engine test_execute_read_file_rejects_traversal -- --nocapture`
- `cargo test -p node-engine test_execute_write_file_rejects_traversal -- --nocapture`
- `cargo test -p node-engine path_validation::tests -- --nocapture`
- `cargo test -p workflow-nodes test_read_rejects_path_traversal -- --nocapture`
- `cargo test -p workflow-nodes test_write_rejects_path_traversal -- --nocapture`
- `cargo check --manifest-path src-tauri/Cargo.toml`

### Files touched in PR-1

- `crates/node-engine/src/path_validation.rs`
- `crates/node-engine/src/lib.rs`
- `crates/node-engine/src/core_executor.rs`
- `crates/workflow-nodes/src/storage/read_file.rs`
- `crates/workflow-nodes/src/storage/write_file.rs`
- `src-tauri/src/workflow/commands.rs`
- `src-tauri/src/llm/commands/sandbox.rs`
- `src-tauri/src/agent/tools/read.rs`
- `src-tauri/src/agent/tools/write.rs`
- `src-tauri/src/agent/tools/list.rs`

## PR-2 Launcher Contract

Status: Completed

- [x] Implement full required CLI contract in `launcher.sh`.
- [x] Add usage/exit-code coverage tests or smoke checks.

### Verification run (2026-02-27)

- `bash -n launcher.sh`
- `./launcher.sh --help`
- `./launcher.sh --unknown` (expects exit `2`)
- `./launcher.sh --build --run` (expects exit `2`)
- `./launcher.sh --install extra` (expects exit `2`)
- `./launcher.sh -- --foo` (expects exit `2`)
- `./launcher.sh --run-release` (expects exit `4` when artifact missing)

## PR-3 Tooling and Quality Gates

Status: Completed

- [x] Add `.editorconfig`.
- [x] Add `lefthook.yml`.
- [x] Add `lint`, `typecheck`, `test` scripts.
- [x] Tighten `tsconfig` strictness and lint config.

### Verification run (2026-02-27)

- `npm run lint`
- `npm run typecheck`
- `npm test`

### Files touched in PR-3

- `.editorconfig`
- `lefthook.yml`
- `package.json`
- `tsconfig.json`
- `eslint.config.mjs`
- `vite.config.ts`
- `scripts/validate-vite.mjs`

## PR-4 Accessibility Baseline

Status: Completed

- [x] Remove high-risk `svelte-ignore a11y*` suppressions.
- [x] Fix semantic interactive elements and button `type`.

### Verification run (2026-02-27)

- `rg -n "svelte-ignore\\s+a11y" src packages --glob "*.svelte"` (expects no matches)
- `perl -Mstrict -Mwarnings -e 'use File::Find; my @files; find(sub { return unless /\\.svelte\\z/; return unless $File::Find::name =~ m{^(?:src|packages)/}; push @files, $File::Find::name; }, "." ); for my $f (@files) { open my $fh, "<", $f or next; local $/; my $c=<$fh>; while ($c =~ m{<button\\b(.*?)>}sg) { my $attrs=$1; next if $attrs =~ /\\btype\\s*=/s; my $pos = pos($c); my $prefix = substr($c, 0, $pos); my $line = ($prefix =~ tr/\\n//) + 1; print "$f:$line\\n"; } }'` (expects no matches)
- `npm run typecheck`
- `npm test`
- `npm run lint:full` (fails on pre-existing non-PR-4 strict-rule violations in multiple files)

### Files touched in PR-4

- `src/components/WorkflowGraph.svelte`
- `packages/svelte-graph/src/components/ContainerBorder.svelte`
- `src/components/nodes/workflow/NodeGroupNode.svelte`
- `src/components/nodes/workflow/MaskedTextInputNode.svelte`
- `src/components/nodes/workflow/PointCloudOutputNode.svelte`
- Button `type="button"` normalization across interactive Svelte components in `src/` and `packages/svelte-graph/src/` (41 files total changed in this PR)

## PR-5 Documentation Compliance

Status: Not started

- [ ] Add missing source directory `README.md` files.
- [ ] Add root `CHANGELOG.md`.
- [ ] Align root `README.md` with required sections.

## PR-6 Large File Decomposition

Status: Not started

- [ ] Split highest-risk >500 LOC files into smaller modules.
- [ ] Preserve behavior with focused tests.
