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

2026-10-06 — Completed D1. Added `serialize_markdown` with heading, text,
formula, recursive list, pipe-table, and external/embedded image rendering,
Java-compatible text escaping, destination sanitization, and exact two-LF
element separators. Added focused renderer tests; `cargo fmt --all -- --check`,
`cargo test -p opendataloader_core` (18 passed), and `cargo check --workspace`
pass. Existing C-library warnings remain; full output orchestration is deferred
to O1.

2026-10-06 — Completed O1. Connected `convert` to parse each local PDF once,
optionally write per-PDF PNG images, and write requested JSON and Markdown
files with contextual errors. Added an oracle-backed raster end-to-end test
that verifies shared image references and unchanged source input. `cargo fmt
--all`, `cargo check --workspace`, and `cargo test --workspace` pass; existing
C-library warnings remain.

2026-10-06 — Completed V1. Added checklist traceability to
`samples/oracle/manifest.toml`, mapping all nine first-round conformance items
to Rust entry points, committed fixtures, focused tests, and concrete parity
blockers. Deferred gaps remain explicit for truncated/XMP fixtures, exact
XY-Cut++ parity, unsupported semantic inputs, isolated image-failure fixtures,
and Base64 images.

2026-10-06 — Completed X1. Added a Rust stage-to-specification and fixture map
to `docs/specs/pdf-json-markdown-reimplementation.md` and documented the
self-contained pipeline in `crates/opendataloader_core/src/lib.rs`. Verified
that Rust tests and builds use committed samples and do not require Java
sources, Maven, generated JARs, or decompilation output.

2026-10-06 — Completed B1. Added core `ConversionOptions` with sequential
defaults and Rayon-backed processing for multiple PDFs when `parallel` is
enabled; single-PDF conversion remains serial. Added colocated tests comparing
serial and parallel JSON, Markdown, and image outputs plus enabled single-PDF
conversion. `cargo fmt --all`, `cargo test -p opendataloader_core` (20 passed),
and `cargo check --workspace` pass; existing C-library warnings remain.

2026-10-06 — Completed C2. Moved JSON, Markdown, and external-image output
switches into `ConversionOptions` alongside `parallel`; updated the core batch
conversion path, CLI caller, and colocated tests. `cargo fmt --all`,
`cargo test --workspace` (20 core tests passed), and `cargo check --workspace`
pass; existing C-library warnings remain.
