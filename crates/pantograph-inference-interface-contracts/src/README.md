# Source Layout

`lib.rs` contains the initial DTO-only inference interface contract surface.
The crate intentionally stays behavior-light: it validates bounds, required
fields, version markers, and dependency-planning model references, but it does
not resolve Pumas facts, select schedulers, execute inference, or mutate graphs.
