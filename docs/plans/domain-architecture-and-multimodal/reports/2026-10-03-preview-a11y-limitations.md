# Preview accessibility review and disclosure

The run graph's conditional button role and tabindex share the same onSelectNode guard. Existing click and Enter/Space activation remain unchanged; the adjacent review reason explains the static compiler limitation without weakening keyboard behavior.

The artifact projection currently has no caption-track association. Video renderer summaries now disclose that no caption track is provided for this preview; the video preview displays that notice. This does not assert that arbitrary source media has no embedded or burned-in captions, or claim full accessibility. No dummy track is created. Tests cover explicit and inferred video media and non-video families. Current track-association state is absent by contract; adding supported association and validating real accessible playback remain future work.

Local qualification: 27 presenter tests and 547 full frontend tests pass; the eight parser regressions and full accessibility scan pass; TypeScript and full ESLint pass; both changed Svelte components compile without warnings. This is static/controller evidence, not a real-browser accessibility audit. Independent source review and hosted qualification are pending.

Independent source review accepted tree bc2ae922fb12ec0776d7bc6be0a52ff3f6297861. The notice describes this preview's absent track association only, and the role/tabindex rationale matches existing activation behavior. This is not a blanket WCAG conformance claim.
