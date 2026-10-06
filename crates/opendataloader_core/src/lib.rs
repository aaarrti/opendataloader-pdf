mod cleanup;
mod images;
mod model;
mod parser;
mod reading_order;
mod semantics;

pub use cleanup::normalize_page_chunks;
pub use images::write_external_images;
pub use model::*;
pub use reading_order::ReadingOrder;
pub use semantics::reconstruct_semantics;

use std::path::{Path, PathBuf};

/// Convert local PDFs using the extraction stages exposed by this crate.
pub fn convert(
    pdf_paths: Vec<PathBuf>,
    output_dir: PathBuf,
    json_enabled: bool,
    markdown_enabled: bool,
    image_output_enabled: bool,
) -> anyhow::Result<()> {
    let _ = (
        pdf_paths,
        output_dir,
        json_enabled,
        markdown_enabled,
        image_output_enabled,
    );
    todo!()
}

/// Parse one local PDF into the primitive document model and supported semantic elements.
pub fn parse_pdf(path: &Path) -> Result<Document, ConversionError> {
    let mut document = parser::parse_document(path)?;
    for page in &mut document.pages {
        normalize_page_chunks(page);
    }
    reconstruct_semantics(&mut document);
    reading_order::apply(&mut document, ReadingOrder::Xycut);
    Ok(document)
}

fn page_box(width: f64, height: f64) -> BoundingBox {
    parser::page_box(width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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
    fn external_images_write_png_and_reference_written_file() -> anyhow::Result<()> {
        let pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/pdf/chinese_scan.pdf");
        let image_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/i1-test/images");
        fs::create_dir_all(&image_dir)?;
        let mut document = parse_pdf(&pdf_path).map_err(|error| anyhow::anyhow!(error))?;
        write_external_images(&pdf_path, &image_dir, &mut document, "png")?;

        let image_path = image_dir.join("imageFile1.png");
        assert!(image_path.is_file());
        assert_eq!(fs::read(&image_path)?.get(..8), Some(b"\x89PNG\r\n\x1a\n".as_slice()));
        assert!(matches!(
            document.elements.first(),
            Some(SemanticElement::Image { reference, .. })
                if reference.source.as_deref() == Some("images/imageFile1.png")
                    && reference.format.as_deref() == Some("png")
        ));
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
