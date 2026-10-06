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
