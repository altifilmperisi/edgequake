//! SPEC-151 — Page section split / replace SSOT (LAW-151-8).
//!
//! Durable PDF markdown is a sequence of `<!-- edgequake-page:N -->` sections
//! plus an optional trailing "tail" (crop-coverage comment, multimodal-chunks).
//! All splice / never-downgrade logic must go through this module.

use crate::drawing_tags::EMPTY_VISION_PAGE_PLACEHOLDER;
use crate::page_marker::{PageMarkerWriter, PAGE_MARKER_PREFIX, PAGE_MARKER_SUFFIX};
use std::collections::BTreeMap;

/// One page section: 1-indexed page number + full section text (marker + body).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageSectionText {
    pub page: usize,
    pub section: String,
}

/// Byte offsets for a page section inside the source markdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageSectionSpan {
    pub page: usize,
    pub start: usize,
    pub end: usize,
}

/// Locate page markers and return half-open spans `[start, end)`.
pub fn section_spans(markdown: &str) -> Vec<PageSectionSpan> {
    let starts = marker_starts(markdown);
    starts
        .iter()
        .enumerate()
        .map(|(i, (start, num))| {
            let end = starts.get(i + 1).map(|(s, _)| *s).unwrap_or(markdown.len());
            PageSectionSpan {
                page: *num,
                start: *start,
                end,
            }
        })
        .collect()
}

/// Split markdown into ordered page sections and an optional unmarked tail.
pub fn split_sections(markdown: &str) -> (Vec<PageSectionText>, String) {
    let spans = section_spans(markdown);
    if spans.is_empty() {
        return (Vec::new(), markdown.to_string());
    }
    let mut sections = Vec::with_capacity(spans.len());
    for (i, span) in spans.iter().enumerate() {
        let raw = &markdown[span.start..span.end];
        let is_last = i + 1 == spans.len();
        if is_last {
            let (section, peeled) = peel_known_tail(raw);
            sections.push(PageSectionText {
                page: span.page,
                section,
            });
            let after = markdown[span.end..].trim_matches(|c: char| c == '\n' || c == '\r');
            let mut tail = peeled;
            if !after.trim().is_empty() {
                if !tail.is_empty() {
                    tail.push_str("\n\n");
                }
                tail.push_str(after.trim_end());
            }
            return (sections, tail);
        }
        sections.push(PageSectionText {
            page: span.page,
            section: raw.trim_end().to_string(),
        });
    }
    (sections, String::new())
}

/// Known document-level tails that must not ride inside the last page section.
const KNOWN_TAIL_PREFIXES: &[&str] = &[
    "<!-- edgequake-crop-coverage:",
    "<!-- multimodal-chunks -->",
];

fn peel_known_tail(section: &str) -> (String, String) {
    let mut cut = None;
    for prefix in KNOWN_TAIL_PREFIXES {
        if let Some(idx) = section.find(prefix) {
            cut = Some(cut.map_or(idx, |c: usize| c.min(idx)));
        }
    }
    match cut {
        Some(idx) => {
            let head = section[..idx].trim_end().to_string();
            let tail = section[idx..].trim_end().to_string();
            (head, tail)
        }
        None => (section.trim_end().to_string(), String::new()),
    }
}

/// Convenience: page → section map (last write wins for duplicate markers).
pub fn sections_by_page(markdown: &str) -> BTreeMap<usize, String> {
    let (sections, _) = split_sections(markdown);
    let mut map = BTreeMap::new();
    for s in sections {
        map.insert(s.page, s.section);
    }
    map
}

/// True when the section body is empty or the canonical empty-page placeholder.
pub fn is_placeholder(section: &str) -> bool {
    let body = match section.find('\n') {
        Some(idx) => section[idx + 1..].trim(),
        None => "",
    };
    body.is_empty() || body == EMPTY_VISION_PAGE_PLACEHOLDER
}

/// Prefer a non-placeholder section over a placeholder (stable on ties).
/// Used when stitching ambiguous convert groups (keep first real section).
pub fn prefer_section(existing: &str, incoming: &str) -> String {
    match (is_placeholder(existing), is_placeholder(incoming)) {
        (false, true) => existing.to_string(),
        (true, false) => incoming.to_string(),
        _ => existing.to_string(),
    }
}

/// Apply a reprocess result under LAW-151-2 (never downgrade).
///
/// Successful non-placeholder incoming always wins. Placeholder / empty
/// incoming does not overwrite a good existing section when `never_downgrade`.
pub fn apply_reprocess_section(
    existing: Option<&str>,
    incoming: &str,
    never_downgrade: bool,
) -> String {
    match existing {
        Some(prev) if never_downgrade && is_placeholder(incoming) && !is_placeholder(prev) => {
            prev.to_string()
        }
        _ => incoming.to_string(),
    }
}

/// Replace selected page sections. Preserves unmarked tails.
///
/// When `never_downgrade` is true, an incoming placeholder / empty section does
/// not overwrite a non-placeholder existing section (LAW-151-2).
pub fn replace_sections(
    markdown: &str,
    replacements: &BTreeMap<usize, String>,
    never_downgrade: bool,
) -> String {
    let (existing, tail) = split_sections(markdown);
    let mut by_page: BTreeMap<usize, String> =
        existing.into_iter().map(|s| (s.page, s.section)).collect();

    for (page, incoming) in replacements {
        let page = *page;
        if page == 0 {
            continue;
        }
        let normalized = ensure_marker(page, incoming);
        let next = apply_reprocess_section(
            by_page.get(&page).map(|s| s.as_str()),
            &normalized,
            never_downgrade,
        );
        by_page.insert(page, next);
    }

    let mut out = by_page.into_values().collect::<Vec<_>>().join("\n\n");
    if !tail.is_empty() {
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(&tail);
    }
    out
}

/// Ensure a section starts with the correct page marker.
pub fn ensure_marker(page: usize, section: &str) -> String {
    let page_u32 = page as u32;
    if PageMarkerWriter::parse(section.lines().next().unwrap_or("")) == Some(page_u32) {
        return section.trim_end().to_string();
    }
    PageMarkerWriter::strip_before_restamp(section, page_u32)
}

fn marker_starts(markdown: &str) -> Vec<(usize, usize)> {
    let mut starts = Vec::new();
    let mut rest_idx = 0usize;
    while let Some(rel) = markdown[rest_idx..].find(PAGE_MARKER_PREFIX) {
        let idx = rest_idx + rel;
        let after = &markdown[idx + PAGE_MARKER_PREFIX.len()..];
        if let Some(end) = after.find(PAGE_MARKER_SUFFIX) {
            if let Ok(n) = after[..end].trim().parse::<usize>() {
                if n > 0 {
                    starts.push((idx, n));
                }
            }
            rest_idx = idx + PAGE_MARKER_PREFIX.len() + end + PAGE_MARKER_SUFFIX.len();
        } else {
            break;
        }
    }
    starts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_and_replace_preserves_tail() {
        let md = "<!-- edgequake-page:1 -->\n\nA\n\n<!-- edgequake-page:2 -->\n\nB\n\n<!-- edgequake-crop-coverage: x -->\n";
        let (secs, tail) = split_sections(md);
        assert_eq!(secs.len(), 2);
        assert!(tail.contains("edgequake-crop-coverage"));
        let mut repl = BTreeMap::new();
        repl.insert(2, "<!-- edgequake-page:2 -->\n\nB2".into());
        let out = replace_sections(md, &repl, true);
        assert!(out.contains("B2"));
        assert!(out.contains("edgequake-crop-coverage"));
        assert!(out.contains("<!-- edgequake-page:1 -->"));
    }

    #[test]
    fn never_downgrade_keeps_good_section() {
        let md = "<!-- edgequake-page:1 -->\n\nGood text\n";
        let mut repl = BTreeMap::new();
        repl.insert(
            1,
            format!("<!-- edgequake-page:1 -->\n\n{EMPTY_VISION_PAGE_PLACEHOLDER}"),
        );
        let out = replace_sections(md, &repl, true);
        assert!(out.contains("Good text"));
        assert!(!out.contains(EMPTY_VISION_PAGE_PLACEHOLDER));
    }

    #[test]
    fn placeholder_may_be_replaced() {
        let md = format!("<!-- edgequake-page:1 -->\n\n{EMPTY_VISION_PAGE_PLACEHOLDER}\n");
        let mut repl = BTreeMap::new();
        repl.insert(1, "<!-- edgequake-page:1 -->\n\nRecovered".into());
        let out = replace_sections(&md, &repl, true);
        assert!(out.contains("Recovered"));
    }

    #[test]
    fn inserts_missing_page_in_order() {
        let md = "<!-- edgequake-page:1 -->\n\nA\n\n<!-- edgequake-page:3 -->\n\nC\n";
        let mut repl = BTreeMap::new();
        repl.insert(2, "<!-- edgequake-page:2 -->\n\nB".into());
        let out = replace_sections(md, &repl, true);
        let p1 = out.find("page:1").unwrap();
        let p2 = out.find("page:2").unwrap();
        let p3 = out.find("page:3").unwrap();
        assert!(p1 < p2 && p2 < p3);
    }
}
