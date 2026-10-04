# Private projection tuple and summary error names

Name the existing five-element schedulable projection tuple with the private SchedulableIntentProjection alias; tuple members, order, function body and caller destructuring are unchanged. Shorten only the crate-private summary error variants from MissingTaskState/MismatchedTaskState/UnexpectedTaskState to Missing/Mismatched/Unexpected, retaining every task_id field and exact thiserror Display string. Neither type has a new public or serialized representation.

The source inventory finds two production summary consumers, both using Display interpolation before admission/resume; all variant matches are local tests. Derived Debug variant labels intentionally change with the private names and may appear in test failures, but no production Debug consumer was found. Real missing/unexpected summary tests now assert exact Display, and a real mismatched-run correlation test covers the third branch. CI runs the complete summary module with nonzero discovery, retaining projection tests for the tuple alias.

Root approved this bounded cleanup. Root source review accepted all four files at frozen tree c3df7026f434651b5b57876d967110d55f3150c6. Hosted execution remains pending. No local Rust execution is claimed.
