2026-10-06 — Completed F1. Built the Java CLI, generated JSON/Markdown oracle
outputs for baseline, multipage, table, raster/image, sanitization, and invalid
character PDFs, and recorded invalid-input/password outcomes. Added checksums,
provenance, and task coverage to `samples/oracle/manifest.toml`.

2026-10-06 — Completed M1. Added the Rust document model, parser chunk types,
semantic element variants, image references, and typed conversion errors with
fixture-derived unit tests. `cargo test -p opendataloader_core` and
`cargo check --workspace` pass.

2026-10-06 — Completed P1. Added the local `lopdf` parser for PDF validation,
password classification, page count and geometry, Info metadata, text/image/
line-art chunks, plus fixture-backed tests for baseline, raster, invalid, and
password-protected PDFs. `cargo test --workspace` and `cargo check --workspace`
pass.

2026-10-06 — Completed C1. Added deterministic page-local cleanup for duplicate,
empty, tiny, out-of-page, page-background, whitespace, NUL, and U+FFFD content;
sanitization remains disabled by default. Added focused and invalid-character
fixture tests. `cargo test -p opendataloader_core` and `cargo check --workspace`
pass.

2026-10-06 — Completed S1. Added deterministic semantic reconstruction for
positioned text lines, paragraphs, font-size headings, and ordered/unordered
lists, and wired it into `parse_pdf`. Added focused semantic tests;
nested/cross-page list joining remains deferred because the current parser
boundary does not expose reliable indentation and line geometry. `cargo test
-p opendataloader_core` and `cargo check --workspace` pass.

2026-10-06 — Completed the observable portion of T1. Added border-grid table
reconstruction with ordered rows/cells, first-row headers, cell bounds, text
children, and line-art omission, plus a focused geometry-backed test. Full
table-fixture parity remains deferred because the current parser does not yet
preserve reliable text coordinates or line segments for the committed PDF.
`cargo test -p opendataloader_core` and `cargo check --workspace` pass.

2026-10-06 — Completed I1. Added semantic image elements, deterministic PNG
external-image writing from embedded PDF streams, JPEG passthrough support,
stable relative references, and isolated per-image write failures. Added a
raster-fixture test; `cargo test -p opendataloader_core` and `cargo check
--workspace` pass.

2026-10-06 — Completed M2. Verified the Rust core is split into focused
`model`, `parser`, `cleanup`, `semantics`, and `images` modules, with `lib.rs`
limited to module declarations, public exports, and thin parsing orchestration.
Preserved public symbols and behavior; `cargo test -p opendataloader_core`
(12 passed) and `cargo check --workspace` pass.

2026-10-06 — Completed R1. Added deterministic default reading-order sorting,
explicit parser-order mode, and recursive stable nonzero IDs in
`crates/opendataloader_core/src/reading_order.rs`; wired the default into
`parse_pdf` and added focused tests. Full XY-Cut++ parity remains deferred
until the parser preserves reliable per-chunk coordinates. `cargo test
-p opendataloader_core` (14 passed) and `cargo check --workspace` pass.

2026-10-06 — Completed M3. Moved the centralized Rust unit tests into the
`model`, `parser`, `cleanup`, `semantics`, and `images` modules, leaving
`lib.rs` without a centralized test module. `cargo fmt --all`, `cargo test
-p opendataloader_core` (14 passed), and `cargo check --workspace` pass.

2026-10-06 — Completed J1. Added serde_json serialization for the specified
document root and semantic element shapes, including mandatory metadata nulls,
canonical PDF/UA tags, rounded bounds, image references, nested lists/tables,
formulas, captions, and TOC nodes. Added focused serializer tests. `cargo fmt
--all -- --check`, `cargo check --workspace`, and `cargo test
-p opendataloader_core` (16 passed) pass; existing C-library warnings remain.
