# Native runtime failure capture

Fetched candidate before edits: ff55e5f199872255e3d1787ac50494e24c2fcba1,
tree 271eb49689b524bc978bf38dd30726cfcd66ea21. Worktree was clean;
all earlier failed native artifacts and bootstrap repairs remain ancestors.
No workspace/repository AGENTS.md or applicable .agents skill file was present.

The preserved native artifact's 16 JSON files contain 1198 structured failure
field occurrences, 134 nonempty, with only eight unique values. They identify
initial missing requirements, the normal deferred bootstrap, capacity metadata
and the final generic TerminalFailed message. None identifies the subsequent
runtime cause. All original fields remain intact in the earlier evidence.

Independent review and source tracing locate the generic message in
ensure_all_scheduler_tasks_completed. An Ok task result with Failed/Unavailable/
Invalid status records failure diagnostics and sets terminal scheduler state,
but finalization still returns Completed before the completion check. Selection
errors on this path would return a different CapabilityViolation. No specific
runtime cause or source repair is inferred yet.

Qualification-only diagnostics now capture actual returned task status and
all diagnostics (without outputs), plus the exact terminal attempt event payload
and its error_summary. If selection/dispatch instead returns an Err, typed error
and source-chain details are captured; no-selection logs omit task intent/traits.
The PANTOGRAPH_NATIVE_CPU_EVIDENCE_DIR gate limits this capture to qualification.
Requests, prompt/input/output bodies and trait values are excluded. Existing
proof validation, transition logic and event recording remain unchanged.

Acceptance for this diagnostic successor: capture the native task result and
terminal attempt event before proposing a cause-specific repair. The final goal
still requires the real GUI save/reopen/typed Resolve/single Submit path to retain
actual finite CPU output and selection metadata. Preserve every failure first.
No successful CPU, pretrained, GPU or broad loader qualification is inferred.
No model/credential/authentication changes or ONNX build downloads are allowed.
Every Rust build follows a complete effective Cargo graph audit with current
Pumas 26a84e32, dynamic ORT and ORT_SKIP_DOWNLOAD=1; download features stay absent.

Local checks pass: 989 workflow tests, strict selected-crate all-target Clippy,
formatting, critical checks and staged traceability. Traceability has no mapped
impact; semantic contract review remains required. No runtime repair is included.
