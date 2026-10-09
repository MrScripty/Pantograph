# Svelte role-button scanner correction

## Decision and scope

The role-button scanner truncated opening tags at JavaScript arrow operators and quoted greater-than signs. SavedGraphInspectionSnapshot therefore failed keyboard activation checks despite declaring onkeydown. Parse Svelte with the existing locked compiler and inspect complete attributes. Keep native-control exemptions and require explicit tabindex, a keyboard handler, and accessible-name evidence. Rendered descendant text/expressions count as name evidence; callback source and comments do not. Preserve the existing native-button and reviewed-ignore policies.

No dependency change or accessibility suppression is included. Dynamic role expressions remain outside this literal-role check. This is a static guard, not a complete runtime accessibility audit. Malformed Svelte stops the gate.

## Verification

- Eight parser regression tests pass: modern arrow handlers, quoted greater-than signs, legacy directives, real missing keyboard activation, independent required attributes and source lines, nested markup, comments/scripts, malformed syntax, and descendant names versus callback source.
- The lint:a11y entry point runs these regressions before scanning.
- Full repository scan now reports only the two existing reviewed-a11y-ignore findings in IoInspectorPage and RunGraphSnapshot. Those remain unresolved rather than waived.
- Independent review pending. No hosted qualification yet.

Independent source review accepted tree 9ac66e91ff8ee2e5e1e9bc0393b22df5b27a6e00. Full ESLint also passes. Name evidence is intentionally static rather than a complete accessible-name computation: conditional or hidden content and dynamically empty labels still require UI/accessibility validation. Hosted qualification remains pending publication.
