use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
};

use lopdf::{Document as PdfDocument, Object, content::Operation};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub left: f64,
    pub bottom: f64,
    pub right: f64,
    pub top: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentMetadata {
    pub author: Option<String>,
    pub title: Option<String>,
    pub creation_date: Option<String>,
    pub modification_date: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FontInfo {
    pub name: Option<String>,
    pub size: Option<f64>,
    pub style: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextChunk {
    pub page_index: usize,
    pub bounds: BoundingBox,
    pub text: String,
    pub glyph_order: Vec<usize>,
    pub character_spacing: Option<f64>,
    pub font: FontInfo,
    pub parser_order: usize,
    pub structure_id: Option<u64>,
    pub pdfua_tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageChunk {
    pub page_index: usize,
    pub bounds: BoundingBox,
    pub object_reference: Option<String>,
    pub parser_order: usize,
    pub structure_id: Option<u64>,
    pub pdfua_tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LineArtChunk {
    pub page_index: usize,
    pub bounds: BoundingBox,
    pub parser_order: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParserChunk {
    Text(TextChunk),
    Image(ImageChunk),
    LineArt(LineArtChunk),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub index: usize,
    pub width: f64,
    pub height: f64,
    pub chunks: Vec<ParserChunk>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageReference {
    pub source: Option<String>,
    pub data: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElementCommon {
    pub id: Option<u64>,
    pub page_index: usize,
    pub bounds: BoundingBox,
    pub pdfua_tag: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListStyle {
    Ordered,
    Unordered,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SemanticElement {
    Heading {
        common: ElementCommon,
        level: u8,
        text: String,
        font: FontInfo,
    },
    Paragraph {
        common: ElementCommon,
        text: String,
        font: FontInfo,
    },
    TextChunk {
        common: ElementCommon,
        text: String,
        font: FontInfo,
    },
    TextBlock {
        common: ElementCommon,
        text: String,
        font: FontInfo,
    },
    Formula {
        common: ElementCommon,
        content: String,
    },
    Image {
        common: ElementCommon,
        reference: ImageReference,
    },
    Caption {
        common: ElementCommon,
        text: String,
        linked_content_id: Option<u64>,
    },
    List {
        common: ElementCommon,
        style: ListStyle,
        items: Vec<SemanticElement>,
    },
    ListItem {
        common: ElementCommon,
        children: Vec<SemanticElement>,
        text: Option<String>,
    },
    Table {
        common: ElementCommon,
        rows: Vec<SemanticElement>,
    },
    TableRow {
        common: ElementCommon,
        row_number: usize,
        cells: Vec<SemanticElement>,
    },
    TableCell {
        common: ElementCommon,
        row_number: usize,
        column_number: usize,
        row_span: usize,
        column_span: usize,
        is_header: bool,
        children: Vec<SemanticElement>,
    },
    Toc {
        common: ElementCommon,
        items: Vec<SemanticElement>,
    },
    TocItem {
        common: ElementCommon,
        text: String,
        children: Vec<SemanticElement>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub file_name: String,
    pub page_count: usize,
    pub metadata: DocumentMetadata,
    pub pages: Vec<Page>,
    pub elements: Vec<SemanticElement>,
}

#[derive(Debug)]
pub enum ConversionError {
    InvalidInput { path: PathBuf, reason: String },
    PasswordProtected { path: PathBuf },
    Io { path: PathBuf, source: io::Error },
    Processing { path: PathBuf, reason: String },
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput { path, reason } => write!(f, "invalid PDF input {}: {reason}", path.display()),
            Self::PasswordProtected { path } => write!(f, "password-protected PDF: {}", path.display()),
            Self::Io { path, source } => write!(f, "I/O error for {}: {source}", path.display()),
            Self::Processing { path, reason } => write!(f, "PDF processing error for {}: {reason}", path.display()),
        }
    }
}

impl std::error::Error for ConversionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub fn convert(
    pdf_paths: Vec<PathBuf>,
    output_dir: PathBuf,
    json_enabled: bool,
    markdown_enabled: bool,
    image_output_enabled: bool,
) -> anyhow::Result<()> {
    todo!()
}

/// Parse one local PDF into the primitive document model used by later stages.
pub fn parse_pdf(path: &Path) -> Result<Document, ConversionError> {
    let bytes = fs::read(path).map_err(|source| ConversionError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if !bytes.starts_with(b"%PDF-") {
        return Err(ConversionError::InvalidInput {
            path: path.to_path_buf(),
            reason: "missing %PDF- header".into(),
        });
    }

    let pdf = PdfDocument::load_mem(&bytes).map_err(|error| {
        let reason = error.to_string();
        if reason.to_ascii_lowercase().contains("password") || reason.to_ascii_lowercase().contains("encrypt") {
            ConversionError::PasswordProtected {
                path: path.to_path_buf(),
            }
        } else {
            ConversionError::InvalidInput {
                path: path.to_path_buf(),
                reason,
            }
        }
    })?;
    if pdf.is_encrypted() {
        return Err(ConversionError::PasswordProtected {
            path: path.to_path_buf(),
        });
    }

    let pages = pdf.get_pages();
    let metadata = read_metadata(&pdf);
    let mut parsed_pages = Vec::with_capacity(pages.len());
    for (number, page_id) in pages {
        let (width, height) = page_bounds(&pdf, page_id).map_err(|reason| ConversionError::Processing {
            path: path.to_path_buf(),
            reason,
        })?;
        let chunks = page_chunks(&pdf, page_id, number as usize - 1, width, height).map_err(|reason| {
            ConversionError::Processing {
                path: path.to_path_buf(),
                reason,
            }
        })?;
        let mut page = Page {
            index: number as usize - 1,
            width,
            height,
            chunks,
        };
        normalize_page_chunks(&mut page);
        parsed_pages.push(page);
    }

    let mut document = Document {
        file_name: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .into(),
        page_count: parsed_pages.len(),
        metadata,
        pages: parsed_pages,
        elements: Vec::new(),
    };
    reconstruct_semantics(&mut document);
    Ok(document)
}

/// Reconstruct the text elements that can be supported by the parser boundary.
pub fn reconstruct_semantics(document: &mut Document) {
    document.elements.clear();
    for page in &document.pages {
        let table = table_from_page(page);
        let table_bounds = table.as_ref().map(element_bounds);
        if let Some(table) = table {
            document.elements.push(table);
        }
        let lines = text_lines(page)
            .into_iter()
            .filter(|line| table_bounds.is_none_or(|bounds| !contains_bounds(bounds, line.common.bounds)))
            .collect::<Vec<_>>();
        let mut index = 0;
        while index < lines.len() {
            if let Some((style, _)) = list_marker(&lines[index].text) {
                let start = index;
                let mut items = Vec::new();
                while index < lines.len() {
                    let Some((item_style, item_text)) = list_marker(&lines[index].text) else {
                        break;
                    };
                    if item_style != style {
                        break;
                    }
                    let line = &lines[index];
                    items.push(SemanticElement::ListItem {
                        common: line.common.clone(),
                        children: vec![SemanticElement::Paragraph {
                            common: line.common.clone(),
                            text: item_text,
                            font: line.font.clone(),
                        }],
                        text: None,
                    });
                    index += 1;
                }
                let common = common_for_lines(&lines[start..index]);
                document.elements.push(SemanticElement::List { common, style, items });
                continue;
            }

            let start = index;
            index += 1;
            while index < lines.len() && joins_paragraph(&lines[index - 1], &lines[index]) {
                index += 1;
            }
            let group = &lines[start..index];
            let common = common_for_lines(group);
            let text = group
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let font = group[0].font.clone();
            if let Some(level) = heading_level(group) {
                document.elements.push(SemanticElement::Heading {
                    common,
                    level,
                    text,
                    font,
                });
            } else {
                document
                    .elements
                    .push(SemanticElement::Paragraph { common, text, font });
            }
        }
    }
}

fn table_from_page(page: &Page) -> Option<SemanticElement> {
    let epsilon = 0.01;
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    for chunk in &page.chunks {
        let ParserChunk::LineArt(line) = chunk else {
            continue;
        };
        let width = line.bounds.right - line.bounds.left;
        let height = line.bounds.top - line.bounds.bottom;
        if width.abs() <= epsilon && height > epsilon {
            columns.push(line.bounds.left);
        } else if height.abs() <= epsilon && width > epsilon {
            rows.push(line.bounds.bottom);
        } else if width > epsilon && height > epsilon {
            columns.extend([line.bounds.left, line.bounds.right]);
            rows.extend([line.bounds.bottom, line.bounds.top]);
        }
    }
    deduplicate_coordinates(&mut columns);
    deduplicate_coordinates(&mut rows);
    if columns.len() < 2 || rows.len() < 2 {
        return None;
    }

    let text_lines = positioned_text_lines(page);
    let bounds = BoundingBox {
        left: *columns.first()?,
        bottom: *rows.first()?,
        right: *columns.last()?,
        top: *rows.last()?,
    };
    if !text_lines
        .iter()
        .any(|line| contains_bounds(bounds, line.common.bounds))
    {
        return None;
    }
    let mut table_rows = Vec::new();
    for (row_index, pair) in rows.windows(2).rev().enumerate() {
        let row_bounds = BoundingBox {
            left: bounds.left,
            bottom: pair[0],
            right: bounds.right,
            top: pair[1],
        };
        let mut cells = Vec::new();
        for (column_index, pair) in columns.windows(2).enumerate() {
            let cell_bounds = BoundingBox {
                left: pair[0],
                bottom: row_bounds.bottom,
                right: pair[1],
                top: row_bounds.top,
            };
            let children = text_lines
                .iter()
                .filter(|line| contains_bounds(cell_bounds, line.common.bounds))
                .map(|line| SemanticElement::Paragraph {
                    common: line.common.clone(),
                    text: line.text.clone(),
                    font: line.font.clone(),
                })
                .collect();
            cells.push(SemanticElement::TableCell {
                common: ElementCommon {
                    id: None,
                    page_index: page.index,
                    bounds: cell_bounds,
                    pdfua_tag: Some(if row_index == 0 { "TH" } else { "TD" }.into()),
                },
                row_number: row_index + 1,
                column_number: column_index + 1,
                row_span: 1,
                column_span: 1,
                is_header: row_index == 0,
                children,
            });
        }
        table_rows.push(SemanticElement::TableRow {
            common: ElementCommon {
                id: None,
                page_index: page.index,
                bounds: row_bounds,
                pdfua_tag: Some("TR".into()),
            },
            row_number: row_index + 1,
            cells,
        });
    }
    Some(SemanticElement::Table {
        common: ElementCommon {
            id: None,
            page_index: page.index,
            bounds,
            pdfua_tag: Some("Table".into()),
        },
        rows: table_rows,
    })
}

fn deduplicate_coordinates(values: &mut Vec<f64>) {
    values.sort_by(f64::total_cmp);
    values.dedup_by(|left, right| (*left - *right).abs() <= 0.01);
}

fn positioned_text_lines(page: &Page) -> Vec<TextLine> {
    page.chunks
        .iter()
        .filter_map(|chunk| match chunk {
            ParserChunk::Text(text) => Some(TextLine {
                common: ElementCommon {
                    id: None,
                    page_index: text.page_index,
                    bounds: text.bounds,
                    pdfua_tag: None,
                },
                text: text.text.clone(),
                font: text.font.clone(),
            }),
            _ => None,
        })
        .collect()
}

fn element_bounds(element: &SemanticElement) -> BoundingBox {
    match element {
        SemanticElement::Table { common, .. }
        | SemanticElement::TableRow { common, .. }
        | SemanticElement::TableCell { common, .. }
        | SemanticElement::Heading { common, .. }
        | SemanticElement::Paragraph { common, .. }
        | SemanticElement::TextChunk { common, .. }
        | SemanticElement::TextBlock { common, .. }
        | SemanticElement::Formula { common, .. }
        | SemanticElement::Image { common, .. }
        | SemanticElement::Caption { common, .. }
        | SemanticElement::List { common, .. }
        | SemanticElement::ListItem { common, .. }
        | SemanticElement::Toc { common, .. }
        | SemanticElement::TocItem { common, .. } => common.bounds,
    }
}

fn contains_bounds(outer: BoundingBox, inner: BoundingBox) -> bool {
    inner.left >= outer.left && inner.right <= outer.right && inner.bottom >= outer.bottom && inner.top <= outer.top
}

#[derive(Debug, Clone)]
struct TextLine {
    common: ElementCommon,
    text: String,
    font: FontInfo,
}

fn text_lines(page: &Page) -> Vec<TextLine> {
    let mut lines = Vec::new();
    for chunk in page.chunks.iter().filter_map(|chunk| match chunk {
        ParserChunk::Text(text) => Some(text),
        _ => None,
    }) {
        if let Some(line) = lines
            .iter_mut()
            .find(|line: &&mut TextLine| same_line(&line.common.bounds, &chunk.bounds))
        {
            line.text.push(' ');
            line.text.push_str(&chunk.text);
            line.common.bounds = union_bounds(line.common.bounds, chunk.bounds);
        } else {
            lines.push(TextLine {
                common: ElementCommon {
                    id: None,
                    page_index: chunk.page_index,
                    bounds: chunk.bounds,
                    pdfua_tag: None,
                },
                text: chunk.text.clone(),
                font: chunk.font.clone(),
            });
        }
    }
    lines.sort_by(|left, right| {
        right
            .common
            .bounds
            .top
            .total_cmp(&left.common.bounds.top)
            .then_with(|| left.common.bounds.left.total_cmp(&right.common.bounds.left))
    });
    lines
}

fn same_line(left: &BoundingBox, right: &BoundingBox) -> bool {
    let tolerance = (left.top - left.bottom).max(right.top - right.bottom) * 0.5;
    ((left.top + left.bottom) - (right.top + right.bottom)).abs() <= tolerance
}

fn joins_paragraph(previous: &TextLine, current: &TextLine) -> bool {
    let gap = previous.common.bounds.bottom - current.common.bounds.top;
    previous.common.page_index == current.common.page_index
        && previous.font.size == current.font.size
        && (0.0..=previous.font.size.unwrap_or(12.0) * 2.5).contains(&gap)
}

fn heading_level(lines: &[TextLine]) -> Option<u8> {
    let size = lines.first()?.font.size?;
    (size >= 14.0).then(|| ((24.0 - size) / 2.0).round().clamp(1.0, 6.0) as u8)
}

fn list_marker(text: &str) -> Option<(ListStyle, String)> {
    let trimmed = text.trim_start();
    let (style, rest) = if let Some(rest) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
        (ListStyle::Unordered, rest)
    } else {
        let digits = trimmed.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0
            || trimmed.as_bytes().get(digits) != Some(&b'.')
            || trimmed.as_bytes().get(digits + 1) != Some(&b' ')
        {
            return None;
        }
        (ListStyle::Ordered, &trimmed[digits + 2..])
    };
    (!rest.is_empty()).then(|| (style, rest.to_owned()))
}

fn common_for_lines(lines: &[TextLine]) -> ElementCommon {
    let first = &lines[0].common;
    ElementCommon {
        id: None,
        page_index: first.page_index,
        bounds: lines
            .iter()
            .skip(1)
            .fold(first.bounds, |bounds, line| union_bounds(bounds, line.common.bounds)),
        pdfua_tag: None,
    }
}

fn union_bounds(left: BoundingBox, right: BoundingBox) -> BoundingBox {
    BoundingBox {
        left: left.left.min(right.left),
        bottom: left.bottom.min(right.bottom),
        right: left.right.max(right.right),
        top: left.top.max(right.top),
    }
}

fn read_metadata(pdf: &PdfDocument) -> DocumentMetadata {
    let info = pdf
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|object| object.as_reference().ok())
        .and_then(|id| pdf.get_dictionary(id).ok());
    let value = |key| info.and_then(|dict| dict.get(key).ok()).and_then(pdf_text);
    let mut metadata = DocumentMetadata {
        author: value(b"Author"),
        title: value(b"Title"),
        creation_date: value(b"CreationDate"),
        modification_date: value(b"ModDate"),
    };
    if let Some(xmp) = xmp_metadata(pdf) {
        metadata.author = metadata
            .author
            .or_else(|| xmp_value(&xmp, "pdf:Author").or_else(|| xmp_value(&xmp, "dc:creator")));
        metadata.title = metadata.title.or_else(|| xmp_value(&xmp, "dc:title"));
        metadata.creation_date = metadata.creation_date.or_else(|| xmp_value(&xmp, "xmp:CreateDate"));
        metadata.modification_date = metadata.modification_date.or_else(|| xmp_value(&xmp, "xmp:ModifyDate"));
    }
    metadata
}

fn xmp_metadata(pdf: &PdfDocument) -> Option<String> {
    let catalog = pdf.catalog().ok()?;
    let metadata_id = catalog.get(b"Metadata").ok()?.as_reference().ok()?;
    let stream = pdf.get_object(metadata_id).ok()?.as_stream().ok()?;
    let bytes = stream.decompressed_content().ok()?;
    String::from_utf8(bytes).ok()
}

fn xmp_value(xml: &str, tag: &str) -> Option<String> {
    let start = xml.find(&format!("<{tag}"))?;
    let content_start = xml[start..].find('>')? + start + 1;
    let end = xml[content_start..].find(&format!("</{tag}>"))? + content_start;
    let value = xml[content_start..end].replace(|character: char| character == '<' || character == '>', "");
    (!value.trim().is_empty()).then(|| value.trim().to_owned())
}

fn pdf_text(object: &Object) -> Option<String> {
    object
        .as_str()
        .ok()
        .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
}

fn page_bounds(pdf: &PdfDocument, page_id: lopdf::ObjectId) -> Result<(f64, f64), String> {
    let page = pdf.get_dictionary(page_id).map_err(|error| error.to_string())?;
    let box_object = page.get_deref(b"MediaBox", pdf).map_err(|error| error.to_string())?;
    let values = box_object.as_array().map_err(|error| error.to_string())?;
    if values.len() != 4 {
        return Err("MediaBox must contain four numbers".into());
    }
    let left = values[0].as_float().map_err(|error| error.to_string())? as f64;
    let bottom = values[1].as_float().map_err(|error| error.to_string())? as f64;
    let right = values[2].as_float().map_err(|error| error.to_string())? as f64;
    let top = values[3].as_float().map_err(|error| error.to_string())? as f64;
    Ok((right - left, top - bottom))
}

fn page_chunks(
    pdf: &PdfDocument,
    page_id: lopdf::ObjectId,
    page_index: usize,
    width: f64,
    height: f64,
) -> Result<Vec<ParserChunk>, String> {
    let content = pdf
        .get_and_decode_page_content(page_id)
        .map_err(|error| error.to_string())?;
    let mut chunks = Vec::new();
    for (parser_order, operation) in content.operations.iter().enumerate() {
        match operation.operator.as_str() {
            "Tj" | "'" | "\"" => {
                if let Some(text) = operation.operands.last().and_then(pdf_text) {
                    if !text.is_empty() {
                        chunks.push(ParserChunk::Text(TextChunk {
                            page_index,
                            bounds: page_box(width, height),
                            text,
                            glyph_order: Vec::new(),
                            character_spacing: None,
                            font: FontInfo::default(),
                            parser_order,
                            structure_id: None,
                            pdfua_tag: None,
                        }));
                    }
                }
            }
            "TJ" => {
                let text = operation
                    .operands
                    .first()
                    .and_then(|object| object.as_array().ok())
                    .map(|items| items.iter().filter_map(pdf_text).collect::<String>());
                if let Some(text) = text.filter(|text| !text.is_empty()) {
                    chunks.push(ParserChunk::Text(TextChunk {
                        page_index,
                        bounds: page_box(width, height),
                        text,
                        glyph_order: Vec::new(),
                        character_spacing: None,
                        font: FontInfo::default(),
                        parser_order,
                        structure_id: None,
                        pdfua_tag: None,
                    }));
                }
            }
            "Do" if is_image_invocation(pdf, page_id, operation) => chunks.push(ParserChunk::Image(ImageChunk {
                page_index,
                bounds: page_box(width, height),
                object_reference: operation
                    .operands
                    .first()
                    .and_then(|object| object.as_name().ok())
                    .map(|name| String::from_utf8_lossy(name).into_owned()),
                parser_order,
                structure_id: None,
                pdfua_tag: None,
            })),
            "m" | "l" | "re" => {
                if let Some(bounds) = line_bounds(operation) {
                    chunks.push(ParserChunk::LineArt(LineArtChunk {
                        page_index,
                        bounds,
                        parser_order,
                    }));
                }
            }
            _ => {}
        }
    }
    if !chunks.iter().any(|chunk| matches!(chunk, ParserChunk::Text(_))) {
        let text = pdf
            .extract_text(&[(page_index + 1) as u32])
            .map_err(|error| error.to_string())?;
        if !text.trim().is_empty() {
            chunks.push(ParserChunk::Text(TextChunk {
                page_index,
                bounds: page_box(width, height),
                text,
                glyph_order: Vec::new(),
                character_spacing: None,
                font: FontInfo::default(),
                parser_order: content.operations.len(),
                structure_id: None,
                pdfua_tag: None,
            }));
        }
    }
    Ok(chunks)
}

fn page_box(width: f64, height: f64) -> BoundingBox {
    BoundingBox {
        left: 0.0,
        bottom: 0.0,
        right: width,
        top: height,
    }
}

/// Apply the default page-local cleanup before semantic reconstruction.
pub fn normalize_page_chunks(page: &mut Page) {
    let mut seen_text = Vec::new();
    page.chunks.retain_mut(|chunk| match chunk {
        ParserChunk::Text(text) => {
            text.text = clean_text(&text.text);
            let duplicate = seen_text
                .iter()
                .any(|(value, bounds): &(String, BoundingBox)| value == &text.text && *bounds == text.bounds);
            let keep = !text.text.is_empty()
                && text.font.size.is_none_or(|size| size >= 1.0)
                && intersects_page(text.bounds, page.width, page.height)
                && !duplicate;
            if keep {
                seen_text.push((text.text.clone(), text.bounds));
            }
            keep
        }
        ParserChunk::Image(image) => intersects_page(image.bounds, page.width, page.height),
        ParserChunk::LineArt(line) => {
            intersects_page(line.bounds, page.width, page.height)
                && !is_page_background(line.bounds, page.width, page.height)
        }
    });
}

fn clean_text(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character == '\0' || character == '\u{fffd}' {
                ' '
            } else {
                character
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn intersects_page(bounds: BoundingBox, width: f64, height: f64) -> bool {
    bounds.right > 0.0 && bounds.top > 0.0 && bounds.left < width && bounds.bottom < height
}

fn is_page_background(bounds: BoundingBox, width: f64, height: f64) -> bool {
    bounds.left <= 0.0 && bounds.bottom <= 0.0 && bounds.right >= width && bounds.top >= height
}

fn is_image_invocation(pdf: &PdfDocument, page_id: lopdf::ObjectId, operation: &Operation) -> bool {
    let Some(name) = operation.operands.first().and_then(|object| object.as_name().ok()) else {
        return false;
    };
    let Ok((resources, _)) = pdf.get_page_resources(page_id) else {
        return false;
    };
    let Some(resources) = resources else { return false };
    let Ok(xobjects) = resources.get_deref(b"XObject", pdf).and_then(Object::as_dict) else {
        return false;
    };
    let Ok(reference) = xobjects.get(name).and_then(Object::as_reference) else {
        return false;
    };
    let Ok(stream) = pdf.get_object(reference).and_then(Object::as_stream) else {
        return false;
    };
    stream
        .dict
        .get(b"Subtype")
        .and_then(Object::as_name)
        .is_ok_and(|subtype| subtype == b"Image")
}

fn line_bounds(operation: &Operation) -> Option<BoundingBox> {
    let numbers: Vec<f64> = operation
        .operands
        .iter()
        .filter_map(|object| object.as_float().ok().map(f64::from))
        .collect();
    let (left, bottom, right, top) = match operation.operator.as_str() {
        "m" | "l" if numbers.len() >= 2 => (numbers[0], numbers[1], numbers[0], numbers[1]),
        "re" if numbers.len() >= 4 => (numbers[0], numbers[1], numbers[0] + numbers[2], numbers[1] + numbers[3]),
        _ => return None,
    };
    Some(BoundingBox {
        left: left.min(right),
        bottom: bottom.min(top),
        right: left.max(right),
        top: bottom.max(top),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_preserves_fixture_metadata_and_page_geometry() {
        let document = Document {
            file_name: "lorem.pdf".into(),
            page_count: 1,
            metadata: DocumentMetadata {
                author: Some("anonymous".into()),
                title: Some("untitled".into()),
                ..Default::default()
            },
            pages: vec![Page {
                index: 0,
                width: 595.276,
                height: 841.89,
                chunks: vec![ParserChunk::Text(TextChunk {
                    page_index: 0,
                    bounds: BoundingBox {
                        left: 72.0,
                        bottom: 766.698,
                        right: 378.11,
                        top: 783.358,
                    },
                    text: "Lorem".into(),
                    glyph_order: vec![0, 1, 2, 3, 4],
                    character_spacing: None,
                    font: FontInfo {
                        name: Some("Helvetica-Bold".into()),
                        size: Some(14.0),
                        ..Default::default()
                    },
                    parser_order: 0,
                    structure_id: None,
                    pdfua_tag: None,
                })],
            }],
            elements: Vec::new(),
        };

        assert_eq!(document.page_count, document.pages.len());
        assert_eq!(document.metadata.author.as_deref(), Some("anonymous"));
        assert_eq!(document.pages[0].index, 0);
        assert_eq!(document.pages[0].chunks.len(), 1);
    }

    #[test]
    fn reconstructs_heading_paragraph_and_unordered_list() {
        let text = |text: &str, top: f64, size: f64| {
            ParserChunk::Text(TextChunk {
                page_index: 0,
                bounds: BoundingBox {
                    left: 50.0,
                    bottom: top - 10.0,
                    right: 250.0,
                    top,
                },
                text: text.into(),
                glyph_order: Vec::new(),
                character_spacing: None,
                font: FontInfo {
                    size: Some(size),
                    ..Default::default()
                },
                parser_order: 0,
                structure_id: None,
                pdfua_tag: None,
            })
        };
        let mut document = Document {
            file_name: "semantic.pdf".into(),
            page_count: 1,
            metadata: DocumentMetadata::default(),
            pages: vec![Page {
                index: 0,
                width: 300.0,
                height: 800.0,
                chunks: vec![
                    text("Title", 700.0, 18.0),
                    text("First paragraph line", 650.0, 10.0),
                    text("continues here", 635.0, 10.0),
                    text("- one", 580.0, 10.0),
                    text("- two", 565.0, 10.0),
                ],
            }],
            elements: Vec::new(),
        };

        reconstruct_semantics(&mut document);

        assert!(matches!(
            document.elements[0],
            SemanticElement::Heading { level: 3, .. }
        ));
        assert!(
            matches!(document.elements[1], SemanticElement::Paragraph { ref text, .. } if text == "First paragraph line continues here")
        );
        assert!(
            matches!(document.elements[2], SemanticElement::List { style: ListStyle::Unordered, ref items, .. } if items.len() == 2)
        );
    }

    #[test]
    fn reconstructs_border_table_rows_cells_and_headers() {
        let text = |value: &str, bounds: BoundingBox| {
            ParserChunk::Text(TextChunk {
                page_index: 0,
                bounds,
                text: value.into(),
                glyph_order: Vec::new(),
                character_spacing: None,
                font: FontInfo::default(),
                parser_order: 0,
                structure_id: None,
                pdfua_tag: None,
            })
        };
        let line = |bounds: BoundingBox| {
            ParserChunk::LineArt(LineArtChunk {
                page_index: 0,
                bounds,
                parser_order: 0,
            })
        };
        let mut document = Document {
            file_name: "table.pdf".into(),
            page_count: 1,
            metadata: DocumentMetadata::default(),
            pages: vec![Page {
                index: 0,
                width: 120.0,
                height: 120.0,
                chunks: vec![
                    line(BoundingBox {
                        left: 10.0,
                        bottom: 10.0,
                        right: 10.0,
                        top: 100.0,
                    }),
                    line(BoundingBox {
                        left: 60.0,
                        bottom: 10.0,
                        right: 60.0,
                        top: 100.0,
                    }),
                    line(BoundingBox {
                        left: 110.0,
                        bottom: 10.0,
                        right: 110.0,
                        top: 100.0,
                    }),
                    line(BoundingBox {
                        left: 10.0,
                        bottom: 10.0,
                        right: 110.0,
                        top: 10.0,
                    }),
                    line(BoundingBox {
                        left: 10.0,
                        bottom: 55.0,
                        right: 110.0,
                        top: 55.0,
                    }),
                    line(BoundingBox {
                        left: 10.0,
                        bottom: 100.0,
                        right: 110.0,
                        top: 100.0,
                    }),
                    text(
                        "Name",
                        BoundingBox {
                            left: 15.0,
                            bottom: 80.0,
                            right: 45.0,
                            top: 90.0,
                        },
                    ),
                    text(
                        "Value",
                        BoundingBox {
                            left: 65.0,
                            bottom: 80.0,
                            right: 95.0,
                            top: 90.0,
                        },
                    ),
                    text(
                        "A",
                        BoundingBox {
                            left: 15.0,
                            bottom: 25.0,
                            right: 25.0,
                            top: 35.0,
                        },
                    ),
                    text(
                        "1",
                        BoundingBox {
                            left: 65.0,
                            bottom: 25.0,
                            right: 75.0,
                            top: 35.0,
                        },
                    ),
                ],
            }],
            elements: Vec::new(),
        };

        reconstruct_semantics(&mut document);

        let SemanticElement::Table { rows, .. } = &document.elements[0] else {
            panic!("expected a table element");
        };
        assert_eq!(rows.len(), 2);
        assert!(matches!(
            &rows[0],
            SemanticElement::TableRow { cells, .. }
                if matches!(&cells[0], SemanticElement::TableCell { is_header: true, children, .. } if matches!(&children[0], SemanticElement::Paragraph { text, .. } if text == "Name"))
        ));
        assert!(matches!(
            &rows[1],
            SemanticElement::TableRow { cells, .. }
                if matches!(&cells[1], SemanticElement::TableCell { is_header: false, children, .. } if matches!(&children[0], SemanticElement::Paragraph { text, .. } if text == "1"))
        ));
        assert_eq!(document.elements.len(), 1);
    }

    #[test]
    fn model_represents_raster_image_reference() {
        let reference = ImageReference {
            source: Some("images/imageFile1.png".into()),
            data: None,
            format: Some("png".into()),
        };
        assert_eq!(reference.source.as_deref(), Some("images/imageFile1.png"));
        assert!(reference.data.is_none());
    }

    #[test]
    fn conversion_errors_remain_distinguishable() {
        let path = PathBuf::from("input.pdf");
        let invalid = ConversionError::InvalidInput {
            path: path.clone(),
            reason: "missing header".into(),
        };
        let password = ConversionError::PasswordProtected { path };
        assert!(invalid.to_string().contains("invalid PDF input"));
        assert!(password.to_string().contains("password-protected"));
    }

    #[test]
    fn parser_reads_lorem_metadata_pages_and_text() -> anyhow::Result<()> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/pdf/lorem.pdf");
        let document = parse_pdf(&path).map_err(|error| anyhow::anyhow!(error))?;
        assert_eq!(document.page_count, 1);
        assert_eq!(document.metadata.author.as_deref(), Some("leebd-public"));
        assert!(document.metadata.title.is_none());
        assert!(document.pages[0].width > 594.0);
        assert!(
            document.pages[0]
                .chunks
                .iter()
                .any(|chunk| { matches!(chunk, ParserChunk::Text(text) if !text.text.is_empty()) })
        );
        Ok(())
    }

    #[test]
    fn parser_distinguishes_invalid_and_password_inputs() {
        let invalid_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/pdf/fake-jpg.pdf");
        assert!(matches!(
            parse_pdf(&invalid_path),
            Err(ConversionError::InvalidInput { .. })
        ));

        let password_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/pdf/password-protected.pdf");
        assert!(matches!(
            parse_pdf(&password_path),
            Err(ConversionError::PasswordProtected { .. })
        ));
    }

    #[test]
    fn parser_preserves_raster_image_chunk_without_text() -> anyhow::Result<()> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/pdf/chinese_scan.pdf");
        let document = parse_pdf(&path).map_err(|error| anyhow::anyhow!(error))?;
        assert!(
            document
                .pages
                .iter()
                .flat_map(|page| page.chunks.iter())
                .any(|chunk| { matches!(chunk, ParserChunk::Image(_)) })
        );
        assert!(
            !document
                .pages
                .iter()
                .flat_map(|page| page.chunks.iter())
                .any(|chunk| { matches!(chunk, ParserChunk::Text(_)) })
        );
        Ok(())
    }

    #[test]
    fn parser_applies_fixture_text_cleanup() -> anyhow::Result<()> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/pdf/invalid-chars.pdf");
        let document = parse_pdf(&path).map_err(|error| anyhow::anyhow!(error))?;
        let text = document.pages[0]
            .chunks
            .iter()
            .find_map(|chunk| match chunk {
                ParserChunk::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .ok_or_else(|| anyhow::anyhow!("invalid-chars fixture has no text chunk"))?;
        assert!(!text.contains('\u{fffd}'));
        assert!(!text.contains("  "));
        Ok(())
    }

    #[test]
    fn cleanup_removes_default_noise_and_normalizes_text() {
        let bounds = BoundingBox {
            left: 10.0,
            bottom: 10.0,
            right: 100.0,
            top: 30.0,
        };
        let mut page = Page {
            index: 0,
            width: 100.0,
            height: 100.0,
            chunks: vec![
                ParserChunk::Text(TextChunk {
                    page_index: 0,
                    bounds,
                    text: "  hello\0  world\u{fffd} ".into(),
                    glyph_order: Vec::new(),
                    character_spacing: None,
                    font: FontInfo::default(),
                    parser_order: 0,
                    structure_id: None,
                    pdfua_tag: None,
                }),
                ParserChunk::Text(TextChunk {
                    page_index: 0,
                    bounds,
                    text: "hello world ".into(),
                    glyph_order: Vec::new(),
                    character_spacing: None,
                    font: FontInfo::default(),
                    parser_order: 1,
                    structure_id: None,
                    pdfua_tag: None,
                }),
                ParserChunk::Text(TextChunk {
                    page_index: 0,
                    bounds,
                    text: "tiny".into(),
                    glyph_order: Vec::new(),
                    character_spacing: None,
                    font: FontInfo {
                        size: Some(0.5),
                        ..Default::default()
                    },
                    parser_order: 2,
                    structure_id: None,
                    pdfua_tag: None,
                }),
                ParserChunk::Text(TextChunk {
                    page_index: 0,
                    bounds: BoundingBox {
                        left: 120.0,
                        bottom: 10.0,
                        right: 130.0,
                        top: 20.0,
                    },
                    text: "outside".into(),
                    glyph_order: Vec::new(),
                    character_spacing: None,
                    font: FontInfo::default(),
                    parser_order: 3,
                    structure_id: None,
                    pdfua_tag: None,
                }),
                ParserChunk::LineArt(LineArtChunk {
                    page_index: 0,
                    bounds: page_box(100.0, 100.0),
                    parser_order: 4,
                }),
            ],
        };

        normalize_page_chunks(&mut page);

        assert_eq!(page.chunks.len(), 1);
        assert!(matches!(&page.chunks[0], ParserChunk::Text(text) if text.text == "hello world"));
    }

    #[test]
    fn cleanup_does_not_apply_sensitive_data_sanitization() {
        let mut page = Page {
            index: 0,
            width: 100.0,
            height: 100.0,
            chunks: vec![ParserChunk::Text(TextChunk {
                page_index: 0,
                bounds: page_box(100.0, 100.0),
                text: "alice@example.org".into(),
                glyph_order: Vec::new(),
                character_spacing: None,
                font: FontInfo::default(),
                parser_order: 0,
                structure_id: None,
                pdfua_tag: None,
            })],
        };

        normalize_page_chunks(&mut page);

        assert!(matches!(&page.chunks[0], ParserChunk::Text(text) if text.text == "alice@example.org"));
    }
}
