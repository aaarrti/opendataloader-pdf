use super::model::*;

pub fn reconstruct_semantics(document: &mut Document) {
    document.elements.clear();
    for page in &document.pages {
        document.elements.extend(page.chunks.iter().filter_map(|chunk| {
            let ParserChunk::Image(image) = chunk else { return None };
            Some(SemanticElement::Image {
                common: ElementCommon {
                    id: None,
                    page_index: image.page_index,
                    bounds: image.bounds,
                    pdfua_tag: image.pdfua_tag.clone(),
                },
                reference: ImageReference {
                    source: None,
                    data: None,
                    format: None,
                },
            })
        }));
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

pub(crate) fn element_bounds(element: &SemanticElement) -> BoundingBox {
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

pub(crate) fn contains_bounds(outer: BoundingBox, inner: BoundingBox) -> bool {
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
