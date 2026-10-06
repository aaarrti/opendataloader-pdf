use std::{fs, path::Path};

use lopdf::{Document as PdfDocument, Object, content::Operation};

use super::model::*;

pub(crate) fn parse_document(path: &Path) -> Result<Document, ConversionError> {
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
        let page = Page {
            index: number as usize - 1,
            width,
            height,
            chunks,
        };
        parsed_pages.push(page);
    }

    let document = Document {
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
    Ok(document)
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

pub(crate) fn page_box(width: f64, height: f64) -> BoundingBox {
    BoundingBox {
        left: 0.0,
        bottom: 0.0,
        right: width,
        top: height,
    }
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
    use std::path::PathBuf;

    #[test]
    fn parser_reads_lorem_metadata_pages_and_text() -> anyhow::Result<()> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/pdf/lorem.pdf");
        let document = crate::parse_pdf(&path).map_err(|error| anyhow::anyhow!(error))?;
        assert_eq!(document.page_count, 1);
        assert_eq!(document.metadata.author.as_deref(), Some("leebd-public"));
        assert!(document.metadata.title.is_none());
        assert!(document.pages[0].width > 594.0);
        assert!(
            document.pages[0]
                .chunks
                .iter()
                .any(|chunk| matches!(chunk, ParserChunk::Text(text) if !text.text.is_empty()))
        );
        Ok(())
    }

    #[test]
    fn parser_distinguishes_invalid_and_password_inputs() {
        let invalid_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/pdf/fake-jpg.pdf");
        assert!(matches!(
            crate::parse_pdf(&invalid_path),
            Err(ConversionError::InvalidInput { .. })
        ));
        let password_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/pdf/password-protected.pdf");
        assert!(matches!(
            crate::parse_pdf(&password_path),
            Err(ConversionError::PasswordProtected { .. })
        ));
    }

    #[test]
    fn parser_preserves_raster_image_chunk_without_text() -> anyhow::Result<()> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/pdf/chinese_scan.pdf");
        let document = crate::parse_pdf(&path).map_err(|error| anyhow::anyhow!(error))?;
        assert!(
            document
                .pages
                .iter()
                .flat_map(|page| page.chunks.iter())
                .any(|chunk| matches!(chunk, ParserChunk::Image(_)))
        );
        assert!(
            !document
                .pages
                .iter()
                .flat_map(|page| page.chunks.iter())
                .any(|chunk| matches!(chunk, ParserChunk::Text(_)))
        );
        Ok(())
    }
}
