# `src/lib.rs` — crate root

Library entry point for the podman-proxy engine.
Declares the internal modules (`http`, `parse`, `secret_detection`, `transport`), the public `proxy` module (which exports `start_proxy`), and the public `types` module (which exports `SandboxMount`).
Re-exports `start_proxy` and `SandboxMount` at the crate root for a clean public API.
Enforces crate-wide deny lints on `unwrap_used`, `expect_used`, `panic`, and `indexing_slicing` (relaxed under `#[cfg(test)]`).
