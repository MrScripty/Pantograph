# Issues

- External: PR75 T3 trained-model public text generation remains required and
  blocked in its owning PR. This example's untrained embedding fixture does not
  satisfy it. No private Nearwork or stale Chrema environment is used.
- Dependency checkpoint: Pumas main `ef9eead9` is newer than Pantograph's
  `aa1d79ae` pin. Parent and the Pumas candidate owner must coordinate adoption;
  this integration keeps the consumed pin until exact compatibility is proven.
- Repository instruction discovery: AGENTS.md and `.agents/skills` are absent
  from the selected checkout. Current repository docs and public standards
  supply the available guidance; no missing instructions are invented.
- Verification setup: reduced Candle-only example features do not compile the
  existing embedded-runtime lib test module, which assumes default gateway
  constructors. The supported default-feature suite passes with an isolated
  writable Pumas registry (598 passed, 2 existing ignores). Earlier readonly
  platform-registry failures are retained as failed setup evidence. The example
  itself remains built and exercised with only its declared Candle features.
