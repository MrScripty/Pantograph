# Issues

- External: PR75 T3 trained-model public text generation remains required and
  blocked in its owning PR. This example's untrained embedding fixture does not
  satisfy it. No private Nearwork or stale Chrema environment is used.
- Dependency checkpoint: Pumas main `ef9eead9` is newer than Pantograph's
  `aa1d79ae` pin. Parent and the Pumas candidate owner must coordinate adoption;
  isolated candidate source 039cee07 passed the bounded local consumer edge.
  PR78 retains its consumed baseline until parent coordinates adoption; hosted
  candidate, migration and pretrained-model gates remain separate.
- Repository instruction discovery: AGENTS.md and `.agents/skills` are absent
  from the selected checkout. Current repository docs and public standards
  supply the available guidance; no missing instructions are invented.
- Verification setup: reduced Candle-only example features do not compile the
  existing embedded-runtime lib test module, which assumes default gateway
  constructors. The supported default-feature suite passes with an isolated
  writable Pumas registry (598 passed, 2 existing ignores). Earlier readonly
  platform-registry failures are retained as failed setup evidence. The example
  itself remains built and exercised with only its declared Candle features.
- Evidence download limitation: actual hosted logs/metadata were retrieved, but
  signed artifact ZIP download was denied by proxy CONNECT 403 Forbidden. The
  hosted ZIP digest is advertised metadata only; complete local archives were
  independently inspected and remain available.
