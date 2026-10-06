# PDF to JSON and Markdown behavior specification

This document defines the Java behavior to reproduce in the Rust extraction
engine. It covers local PDF input, structured extraction, JSON and Markdown
outputs, and image output used by those formats. It does not define a command
line interface, C ABI, Python package, or public API. The user will specify
those contracts separately.

The implementation target is the default local Java pipeline in this checkout.
Hybrid or remote processing is excluded. Python is out of scope for the first
implementation. The specification intentionally does not require support for
linearized PDFs or other special layout optimizations.

## Scope

Implement these behaviors:

- Open a local PDF and determine its page count and document metadata.
- Extract text, images, and layout information into page-ordered content.
- Infer paragraphs, headings, lists, tables, captions, and page headers or
  footers where the Java pipeline does so.
- Write the content as the JSON structure defined here.
- Write the content as Markdown using the formatting rules defined here.
- Extract image content to files for JSON and Markdown references.
- Represent embedded images as Base64 data URIs when requested. This is lower
  priority; the first implementation can emit a documented placeholder.

Do not implement hybrid backends, OCR services, HTML, plain text, PDF
modification, tagged-PDF output, PDF/UA validation, Node.js, Python, publishing,
or Java compatibility APIs as part of this specification.

## Processing model

Treat the PDF as a sequence of pages. Preserve the original page count even
when page selection is used. Each selected page produces an ordered list of
content elements; skipped pages produce an empty list internally and contribute
no children to either output.

Use this logical pipeline:

1. Open and parse the PDF. Read page count and document information. Reject an
   unreadable document with a conversion error; do not silently produce an
   empty success result.
2. Extract page text, font information, colors, bounding boxes, image
   references, and drawing information needed for layout analysis.
3. Filter unwanted content such as out-of-page or tiny content according to
   the selected processing configuration. Preserve the Java defaults for the
   first parity implementation.
4. Reconstruct lines and paragraphs from text fragments. Preserve explicit
   line breaks only when the corresponding option is enabled.
5. Infer structured elements: headings and heading levels, lists and nesting,
   tables and cells, captions, and repeating headers or footers.
6. Assign element IDs in page order. Keep each element's page number and
   bounding box associated with the content through all transformations.
7. Sort final page content in reading order. The default reading-order mode is
   `xycut`.
8. Apply configured text sanitization and content filters before serialization.
9. Write requested JSON and/or Markdown. If image output is enabled, write
   referenced image files before writing either format.

The Java implementation has an optional tagged-structure-tree path. Hybrid
processing is out of scope. A Rust implementation can add tagged-structure-tree
support as a separate parity step; the default untagged-PDF path is required.

Use these Java defaults for the first parity run. A public configuration API is
not specified here; these values define the no-options behavior only.

| Behavior | Default |
| --- | --- |
| JSON format | Enabled. |
| Markdown format | Disabled. |
| Page selection | All pages. |
| Reading order | `xycut`. |
| Table detection | Border-based. |
| Image output | External files. |
| Image format | PNG. |
| Image resolution | 144 DPI. |
| Image directory | `<PDF_BASE_NAME>_images` under the output directory. |
| Header/footer output | Excluded from JSON and Markdown. |
| Keep original line breaks | Disabled. |
| Strikethrough detection | Disabled. |
| Worker count | One page-processing worker. |

## JSON output

Write one UTF-8 JSON object per input PDF. Use pretty-printed JSON. Preserve
the Java property names exactly, including spaces and underscores.

The document object contains these fields in this order:

| Field | Type | Meaning |
| --- | --- | --- |
| `file name` | string | Input file name, without its parent directory. |
| `number of pages` | integer | Total page count in the PDF, not the selected-page count. |
| `author` | string or null | PDF author; use XMP creator when the PDF author is absent. |
| `title` | string or null | PDF title; use XMP title when the PDF title is absent. |
| `creation date` | string or null | PDF creation date; use XMP creation date when absent. |
| `modification date` | string or null | PDF modification date; use XMP modification date when absent. |
| `kids` | array | Flat list of serialized content elements in page and reading order. |

Write all six metadata keys, including keys whose values are null. The ordinary
local pipeline does not emit a `hybrid` field.

Each `kids` item is an object with a `type` discriminator. Common fields are:

| Field | Type | Meaning |
| --- | --- | --- |
| `type` | string | Element kind, such as `heading`, `paragraph`, `image`, or `table`. |
| `pdfua_tag` | string, optional | Source or inferred PDF/UA structure tag when available. |
| `id` | integer, optional | Nonzero recognized structure ID when available. |
| `level` | string, optional | Semantic level when the element has one. |
| `page number` | integer | One-based page number. |
| `bounding box` | four numbers | `[left, bottom, right, top]` in PDF page coordinates. |

Coordinates use the PDF page coordinate system. Do not reorder the box into
`[left, top, right, bottom]`. Numeric serialization follows the Java output's
three-decimal precision for measured layout values.

### Element kinds

The JSON writer omits drawing-only `LineArtChunk` elements. It omits semantic
header/footer elements unless `include-header-footer` is enabled. It serializes
the following content kinds when extraction produces them:

| `type` | Kind-specific fields and rules |
| --- | --- |
| `heading` | `content`, `heading level`, and text styling fields when available. Heading level is an integer. |
| `paragraph` | `content` and text styling fields when available. |
| `text` | `content` and text styling fields when the extracted object remains a text node rather than a paragraph. |
| `image` | Image fields described in [Image output](#image-output). |
| `picture` | Semantic picture fields, including caption or description metadata when present, and image fields when an image file or data URI exists. |
| `list` | `list items`; each item carries its text and any nested list information. |
| `table` | `rows`; each row has `type: "row"`, one-based `row number`, `id`, and `cells`. |
| `caption` | Caption text, relation/target metadata when available, and common geometry fields. |
| `formula` | Formula content in the source representation, typically LaTeX. |
| `header` or `footer` | Header/footer content and common geometry fields. These are filtered out by default. |

Text-bearing elements can include `font`, `font size`, and `text color` when
the extractor has those values. Omit absent optional values unless the element
serializer explicitly writes null. Do not invent empty metadata or alt text.

Tables serialize rows and cells in their logical row/column order. A cell can
include `row number`, `column number`, content, and span information when
present in the extracted table model. Each cell appears once even when it spans
multiple rows or columns.

JSON elements are flattened into `kids`; they are not nested under page objects.
The page association comes from each element's one-based `page number`.

### JSON image fields

For external images, include `source` with the relative image path when the
corresponding file exists. For embedded images, include `data` as a data URI
and `format` when image bytes are available; omit `source`. An image may also
include `alt` when a nonempty description exists and `alt_source` to identify
the description origin. Do not synthesize descriptions.

If extraction or image writing fails for one image, keep the rest of the PDF
output. Do not emit a broken `source` or `data` value for that image. Preserve
the image element and its geometry if the JSON serializer can represent it
without image bytes.

The Java serializer can attach element metadata when present, including
`ai_score`, `source label`, `heading inference`, `tsr`, `caption`,
`regionlist resolution`, `word match`, `text source`, and
`stream ocr similarity`. These fields are conditional; the local default path
usually has no hybrid metadata.

## Markdown output

Write UTF-8 Markdown in page order. Do not add a document title or page heading
unless it exists in the extracted content. The default page separator is empty.
After each supported content element, write two line breaks. A configured page
separator is written before that page's content; replace `%page-number%` with
the one-based page number.

Render supported elements as follows:

| Element | Markdown representation |
| --- | --- |
| Heading | Prefix text with `#` repeated for the heading level clamped to 1–6, then one space. |
| Paragraph or text node | Emit the extracted text. |
| Unordered list | Emit one `- ` item per list item; preserve nested list levels. |
| Ordered list | Emit ordered list markers and item text in source order. |
| Table | Emit a pipe table. Put a separator row after the first row. Preserve row and column spans using HTML only when the HTML-in-Markdown mode is enabled. |
| Formula | Emit `$$`, a line break, the formula text, a line break, and `$$`. |
| Picture or image | Emit `![ALT](DESTINATION)` when a usable file or data URI exists. |
| Header or footer | Emit only when `include-header-footer` is enabled. |
| Unsupported extraction object | Omit it. |

For Markdown text, convert null to an empty string, remove U+0000, and escape
ampersand, less-than, and greater-than characters as `&amp;`, `&lt;`, and
`&gt;`. This behavior also escapes existing entities. Preserve or normalize
line breaks according to the `keep-line-breaks` setting.

For an image link to a file, use a relative destination under the image
directory and wrap the destination in angle brackets. Replace `<`, `>`,
backslash, carriage return, and line feed in the destination with spaces before
writing it. Use the extracted image description as alt text when available;
otherwise use an empty alt string.

The default file name is the input PDF base name with the `.md` extension. The
exact output-directory contract is intentionally unspecified here.

## Image output

Run image extraction when JSON or Markdown output is requested and image output
is enabled. The default image mode is external. The default image directory is
`<PDF_BASE_NAME>_images` under the selected output directory; a caller-supplied
image directory overrides it.

Walk pages and their elements in order. Assign image indexes starting at 1.
Use the source PDF image object when one is available. If it is unavailable,
render and crop the image element's page bounding box at the configured image
resolution. Write PNG by default; support JPEG when selected. The reference
name pattern is `imageFile<INDEX>.<EXT>`.

The Markdown and JSON references must point to the same file. Keep the relative
path stable for the same output layout. Do not write an image file when image
output is off. Do not emit a file reference for a file that was not written.

### Embedded image priority

Base64 image output is lower priority than external files. The Java behavior
uses `data:<mime-type>;base64,<bytes>` for JSON and Markdown. It rejects
individual embedded images larger than 10 MiB; omit the failed image reference
without failing the document. A first Rust version may use a fixed placeholder
instead of encoding bytes, but it must not present that placeholder as a valid
data URI. Keep the option and output shape available for later completion.

## Errors and partial output

Treat input-open, parse, and document-level extraction failures as conversion
errors. Do not report success with fabricated empty content. A missing image or
image encoding failure is local to that image and must not discard successfully
extracted text or other images.

If both output formats are requested, derive both from the same extracted page
content and use identical text, page ordering, geometry, and image numbering.
Do not run independent extraction passes for JSON and Markdown.

## Reference fixtures

Use the committed `samples/pdf/lorem.pdf` input with
`samples/json/lorem.json` and `samples/markdown/lorem.md`. The JSON fixture is
the existing Java output. Maven is unavailable in the current Rust checkout, so
the Markdown fixture is derived from the Java writer rules and should be
confirmed against a Java CLI run when that build environment is available.

The `lorem.pdf` fixture exercises document metadata, one heading, and one
paragraph. It does not exercise image output, tables, lists, or page selection.

These fixtures are human-reviewable parity references, not a complete corpus.
Add targeted fixtures for images, tables, lists, headers/footers, formulas,
multiple pages, and malformed PDFs before claiming full parity.

## Java source map

These source files define the behavior this document summarizes:

- `java/opendataloader-pdf-core/src/main/java/org/opendataloader/pdf/processors/DocumentProcessor.java`
  — extraction pipeline, ordering, sanitization, and output dispatch.
- `java/opendataloader-pdf-core/src/main/java/org/opendataloader/pdf/json/JsonWriter.java`
  — document metadata, page flattening, and JSON output.
- `java/opendataloader-pdf-core/src/main/java/org/opendataloader/pdf/json/serializers/`
  — element-specific JSON fields.
- `java/opendataloader-pdf-core/src/main/java/org/opendataloader/pdf/markdown/MarkdownGenerator.java`
  — Markdown type dispatch and formatting.
- `java/opendataloader-pdf-core/src/main/java/org/opendataloader/pdf/utils/ImagesUtils.java`
  — image numbering, extraction, and files.
- `java/opendataloader-pdf-core/src/main/java/org/opendataloader/pdf/utils/Base64ImageUtils.java`
  — embedded image data URIs and size limit.
