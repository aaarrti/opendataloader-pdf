use super::model::*;

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
