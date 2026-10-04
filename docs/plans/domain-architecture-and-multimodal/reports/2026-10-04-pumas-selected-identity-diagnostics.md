# Consumer diagnostic mapping repair

Frozen consumer base: `a5ed074d5e44db13f47ff6f7cd5059e234c183ff`.
Initial minimal repair: `2880a290b743a21874a879f219e839436f251863`.

The native reviewer reproduced two E0004 compiler errors: the new
`SelectedIdentityMismatch` load-target diagnostic was absent from the candidate
provider's exhaustive code and hint matches. The repair classifies it as
`InvalidCandidateEvidence` and supplies the corresponding stable
`embedded_runtime_dispatch_candidate_provider.load_target.selected_identity_mismatch`
hint. The mismatch remains unavailable target evidence; it never creates a
candidate or reservation.

The regression `mismatched_load_target_identity_cannot_reserve_a_candidate`
stages unavailable load-target identity evidence alongside otherwise usable
package/runtime evidence. It requires zero candidates, zero registry
reservations, and the typed invalid-evidence diagnostic with its hint.

Executed in the cloud environment:

- `ORT_SKIP_DOWNLOAD=1 CARGO_BUILD_JOBS=1 cargo check --locked --offline -p pantograph-embedded-runtime --tests`
  passed with the original default/full Pumas features and pinned producer
  `5be6d967dbd5c0ff7449f342e77f05f9cd645a8e`. This is compiler evidence only:
  it checks the new regression's source but does not link or execute tests.
- Rust format, critical anti-pattern lint, diff whitespace and staged decision
  traceability passed.

The parent's exact native full-feature test/check attempts stopped during Pumas
compilation at the RAM floor, before consumer tests. Real IPC, linked runtime
tests and the consumer's native acceptance remain unestablished. No CDN retry,
feature reduction, network change or ORT runtime workaround was made. The
embedding-output feature is on a separate Candle descendant and is absent here.
