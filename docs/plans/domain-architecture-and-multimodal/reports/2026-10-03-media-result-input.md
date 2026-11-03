# Named media conversion result input

The next aggregate warning-deny Clippy failure after the scheduler layout repair was the eight-argument MediaConversionResult::try_new constructor. Replace its positional arguments with MediaConversionResultInput carrying exactly the existing eight fields. The constructor destructures that input and retains the existing validation statements, ordering, normalization, errors, and result representation. Migrate all three repository callers: the managed executor and two existing tests.

This is an explicit public Rust construction change despite the crate being publish=false at workspace development version 0.1.0. The changelog records the new call shape; unseen external Git/path consumers are not assumed compatible. No serialized DTO, conversion routing, process execution, or dependency policy changes are included.

Four regressions cover all successful fields and existing command-only normalization, result-field round-trip, ordered failures for body/command/stderr/dependency holder, and text bounds. Existing executor success, failure, timeout and cancellation tests remain. The focused hosted Rust job now runs the media conversion crate tests explicitly.

Formatting and whitespace checks pass locally. No local workspace compilation or executed new Rust tests are claimed. Independent source review and fresh hosted media/scheduler/aggregate Clippy qualification are pending. Existing gates remain intact, with no lint suppression.

Independent source review accepted final tree 443829480fb96b4164eb068adfa51ee9e8d14d92. This confirms the bounded source/API migration and regression coverage; actual Rust execution remains hosted qualification.
