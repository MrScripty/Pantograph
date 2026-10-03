# Node-engine test-only lint repair

Fresh strict Clippy at PR43 head e2dbe87 clears the UniFFI library and reaches two node-engine lib-test findings. Both exact source expressions also exist at main 4938e405. This milestone only names the captured embedding request tuple (Vec<String>, String) with a private feature-gated alias and expresses absent stream membership with !contains_key instead of get().is_none(). All tuple ordering, values, test actions and assertions are unchanged; there is no production API or behavior change.

The embedding mock has two constructors and two existing test readers, plus one push site. Existing tests assert captured input/model and compatibility lifecycle/output behavior. Because the default node-engine suite does not enable inference-nodes, add a feature-enabled gate requiring both exact existing canonical embedding test names and an individual execution receipt of 1 passed / 0 failed / 0 ignored for each, retaining the full default suite and strict aggregate lint.

Root source review accepted the mechanical repairs and the corrected exact-name/execution-count gate at frozen tree 2b5bc9252887e86bed791b4c759923592ff9435d. Fresh hosted execution remains pending. Full pinned formatting passes locally; no local Rust build is claimed. PR43 shutdown success/failure and regenerated C# qualification continue independently on its exact published head.
