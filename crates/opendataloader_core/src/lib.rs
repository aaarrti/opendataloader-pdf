//! Local PDF extraction pipeline.
//!
//! The stage order follows the extraction stages in
//! `docs/specs/pdf-json-markdown-reimplementation.md`: parse, clean, rebuild
//! semantics, order and assign IDs, then serialize or write images. The
//! committed oracle and stage-to-fixture mapping live in
//! `samples/oracle/manifest.toml`; this crate does not invoke the Java
//! reference implementation.

mod cleanup;
mod images;
mod json;
mod markdown;
mod model;
mod parser;
mod reading_order;
mod semantics;

use anyhow::Context;
pub use cleanup::normalize_page_chunks;
pub use images::write_external_images;
pub use json::serialize_document;
pub use markdown::serialize_markdown;
pub use model::*;
use rayon::prelude::*;
pub use reading_order::ReadingOrder;
pub use semantics::reconstruct_semantics;

use std::{
    fs,
    path::{Path, PathBuf},
};

/// Options for the core conversion pipeline.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct ConversionOptions {
    /// Write JSON output files.
    pub json_enabled: bool,
    /// Write Markdown output files.
    pub markdown_enabled: bool,
    /// Write external image files.
    pub image_output_enabled: bool,
    /// Process multiple PDFs concurrently with Rayon.
    pub parallel: bool,
}

/// Convert local PDFs with explicit core conversion options.
pub fn convert_with_options(
    pdf_paths: &[PathBuf],
    output_dir: &PathBuf,
    options: ConversionOptions,
) -> anyhow::Result<()> {
    if !options.json_enabled && !options.markdown_enabled {
        return Ok(());
    }
    fs::create_dir_all(&output_dir).with_context(|| format!("create output directory {}", output_dir.display()))?;

    let convert_one = |pdf_path: &PathBuf| -> anyhow::Result<()> {
        let mut document = parse_pdf(&pdf_path)
            .map_err(|error| anyhow::anyhow!(error))
            .with_context(|| format!("parse PDF {}", pdf_path.display()))?;
        let stem = pdf_path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| anyhow::anyhow!("PDF has no valid file name: {}", pdf_path.display()))?;

        if options.image_output_enabled {
            write_external_images(
                &pdf_path,
                &output_dir.join(format!("{stem}_images")),
                &mut document,
                "png",
            )?;
        }
        if options.json_enabled {
            fs::write(output_dir.join(format!("{stem}.json")), serialize_document(&document)?)
                .with_context(|| format!("write JSON output for {}", pdf_path.display()))?;
        }
        if options.markdown_enabled {
            fs::write(output_dir.join(format!("{stem}.md")), serialize_markdown(&document)?)
                .with_context(|| format!("write Markdown output for {}", pdf_path.display()))?;
        }
        Ok(())
    };

    if options.parallel && pdf_paths.len() > 1 {
        pdf_paths.into_par_iter().try_for_each(convert_one)?;
    } else {
        pdf_paths.into_iter().try_for_each(convert_one)?;
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn convert_writes_requested_outputs_from_one_document() -> anyhow::Result<()> {
        let pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/pdf/chinese_scan.pdf");
        let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/o1-test");
        fs::create_dir_all(&output_dir)?;
        let source = fs::read(&pdf_path)?;
        convert_with_options(
            vec![pdf_path.clone()],
            output_dir.clone(),
            ConversionOptions {
                json_enabled: true,
                markdown_enabled: true,
                image_output_enabled: true,
                ..ConversionOptions::default()
            },
        )?;

        assert!(output_dir.join("chinese_scan.json").is_file());
        assert!(output_dir.join("chinese_scan.md").is_file());
        assert!(output_dir.join("chinese_scan_images/imageFile1.png").is_file());
        assert_eq!(fs::read(&pdf_path)?, source);
        let actual_json =
            serde_json::from_str::<serde_json::Value>(&fs::read_to_string(output_dir.join("chinese_scan.json"))?)?;
        let mut expected_json = serde_json::from_str::<serde_json::Value>(&fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/oracle/raster/chinese_scan.json"),
        )?)?;
        expected_json["kids"][0]["source"] = "chinese_scan_images/imageFile1.png".into();
        assert_eq!(actual_json, expected_json);
        assert_eq!(
            fs::read_to_string(output_dir.join("chinese_scan.md"))?,
            fs::read_to_string(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/oracle/raster/chinese_scan.md"),
            )?
            .replace("images/", "chinese_scan_images/"),
        );
        Ok(())
    }

    #[test]
    fn parallel_conversion_matches_sequential_conversion() -> anyhow::Result<()> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let source_pdf = manifest_dir.join("../../samples/pdf/chinese_scan.pdf");
        let fixture_dir = manifest_dir.join("../../target/b1-test-inputs");
        let sequential_dir = manifest_dir.join("../../target/b1-sequential");
        let parallel_dir = manifest_dir.join("../../target/b1-parallel");
        fs::create_dir_all(&fixture_dir)?;
        fs::create_dir_all(&sequential_dir)?;
        fs::create_dir_all(&parallel_dir)?;
        let first_pdf = fixture_dir.join("first.pdf");
        let second_pdf = fixture_dir.join("second.pdf");
        fs::copy(&source_pdf, &first_pdf)?;
        fs::copy(&source_pdf, &second_pdf)?;

        convert_with_options(
            vec![first_pdf.clone(), second_pdf.clone()],
            sequential_dir.clone(),
            ConversionOptions {
                json_enabled: true,
                markdown_enabled: true,
                image_output_enabled: true,
                ..ConversionOptions::default()
            },
        )?;
        convert_with_options(
            vec![first_pdf.clone(), second_pdf.clone()],
            parallel_dir.clone(),
            ConversionOptions {
                json_enabled: true,
                markdown_enabled: true,
                image_output_enabled: true,
                parallel: true,
            },
        )?;

        for stem in ["first", "second"] {
            assert_eq!(
                fs::read(sequential_dir.join(format!("{stem}.json")))?,
                fs::read(parallel_dir.join(format!("{stem}.json")))?
            );
            assert_eq!(
                fs::read(sequential_dir.join(format!("{stem}.md")))?,
                fs::read(parallel_dir.join(format!("{stem}.md")))?
            );
            assert_eq!(
                fs::read(sequential_dir.join(format!("{stem}_images/imageFile1.png")))?,
                fs::read(parallel_dir.join(format!("{stem}_images/imageFile1.png")))?
            );
        }

        let single_dir = manifest_dir.join("../../target/b1-single");
        convert_with_options(
            vec![first_pdf],
            single_dir.clone(),
            ConversionOptions {
                json_enabled: true,
                parallel: true,
                ..ConversionOptions::default()
            },
        )?;
        assert!(single_dir.join("first.json").is_file());
        Ok(())
    }
}
