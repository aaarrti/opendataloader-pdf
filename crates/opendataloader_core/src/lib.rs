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
pub use reading_order::ReadingOrder;
pub use semantics::reconstruct_semantics;

use std::{
    fs,
    path::{Path, PathBuf},
};

/// Convert local PDFs using the extraction stages exposed by this crate.
pub fn convert(
    pdf_paths: Vec<PathBuf>,
    output_dir: PathBuf,
    json_enabled: bool,
    markdown_enabled: bool,
    image_output_enabled: bool,
) -> anyhow::Result<()> {
    if !json_enabled && !markdown_enabled {
        return Ok(());
    }
    fs::create_dir_all(&output_dir).with_context(|| format!("create output directory {}", output_dir.display()))?;

    for pdf_path in pdf_paths {
        let mut document = parse_pdf(&pdf_path)
            .map_err(|error| anyhow::anyhow!(error))
            .with_context(|| format!("parse PDF {}", pdf_path.display()))?;
        let stem = pdf_path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| anyhow::anyhow!("PDF has no valid file name: {}", pdf_path.display()))?;

        if image_output_enabled {
            write_external_images(
                &pdf_path,
                &output_dir.join(format!("{stem}_images")),
                &mut document,
                "png",
            )?;
        }
        if json_enabled {
            fs::write(output_dir.join(format!("{stem}.json")), serialize_document(&document)?)
                .with_context(|| format!("write JSON output for {}", pdf_path.display()))?;
        }
        if markdown_enabled {
            fs::write(output_dir.join(format!("{stem}.md")), serialize_markdown(&document)?)
                .with_context(|| format!("write Markdown output for {}", pdf_path.display()))?;
        }
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
        convert(vec![pdf_path.clone()], output_dir.clone(), true, true, true)?;

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
}
