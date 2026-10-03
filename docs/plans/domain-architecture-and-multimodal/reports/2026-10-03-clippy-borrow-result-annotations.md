# Redundant borrow and Result annotation repair

Two independent warning-deny Clippy findings require no behavioral change. Pass the existing &Path directly to fs::read_to_string instead of borrowing it again. Remove a redundant bare must_use annotation from select_scheduler_dispatch because its Result return type already carries that obligation; the validated successful payload also remains must_use.

Error formatting, deserialization, dispatch policy, signatures, and tests are unchanged. Whitespace validation passes. No local Rust workspace compilation was performed; exact-head hosted qualification and independent source review are pending. These two changes do not address the separate AsRef migration, public runtime enum layout, or other aggregate findings.

Independent source review accepted the two-line repair at tree b35f32903f85288265a68ffbe57e9d61bf021b80. Publication is stacked on the separately reviewed AsRef milestone to retain cumulative hosted qualification; this report does not imply that aggregate Clippy already passes.
