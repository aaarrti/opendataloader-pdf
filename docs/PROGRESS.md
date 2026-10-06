2026-10-06 — Completed F1. Built the Java CLI, generated JSON/Markdown oracle
outputs for baseline, multipage, table, raster/image, sanitization, and invalid
character PDFs, and recorded invalid-input/password outcomes. Added checksums,
provenance, and task coverage to `samples/oracle/manifest.toml`.

2026-10-06 — Completed M1. Added the Rust document model, parser chunk types,
semantic element variants, image references, and typed conversion errors with
fixture-derived unit tests. `cargo test -p opendataloader_core` and
`cargo check --workspace` pass.
