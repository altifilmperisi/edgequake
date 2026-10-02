//! SPEC-047 MV-22 — preserve atomic blocks during recursive chunking.
//!
//! First principle: chart/table labels and numeric values must land in the same
//! chunk for retrieval. Internal `\n\n` separators must not split:
//! - LightRAG mm blocks (`[Chart Name]` …)
//! - Fenced code / table fences
//! - Pipe markdown tables
//!
//! WHY no `regex::Regex` here: LazyLock `Regex::new` on the eq-ingest worker
//! stack overflows in debug builds when nested under the large pipeline FSM
//! (crash: `regex_automata::Builder::build_many_from_hir` inside `MM_HEAD_RE`).
//! Prefix checks are exact for these fixed headers and keep init off the stack.

use super::page_marker::PAGE_MARKER_PREFIX;

/// LightRAG multimodal chunk headers (`[Image Name]`, …).
const MM_HEAD_PREFIXES: &[&str] = &[
    "[Image Name]",
    "[Chart Name]",
    "[Figure Name]",
    "[Table Name]",
    "[Equation Name]",
];

/// Kind of indivisible content region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomicKind {
    MultimodalChunk,
    FencedCode,
    PipeTable,
}

/// Plain or atomic segment of source markdown with byte span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentRegion {
    pub text: String,
    pub atomic: Option<AtomicKind>,
    pub start: usize,
    pub end: usize,
}

/// True when `line` starts a LightRAG mm chunk header.
pub fn is_mm_chunk_header(line: &str) -> bool {
    let t = line.trim();
    MM_HEAD_PREFIXES.iter().any(|prefix| t.starts_with(prefix))
}

/// True when `line` is a VLM inline analyzed block title (`# name`).
pub fn is_vlm_analyzed_heading(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("# ") && t.len() > 2
}

/// True when `line` is the VLM type marker (`**Type:** Chart`).
pub fn is_vlm_type_marker(line: &str) -> bool {
    line.trim().starts_with("**Type:**")
}

/// True when line at `idx` begins a VLM inline block (`# title` then `**Type:**`).
pub fn is_vlm_analyzed_block_start(lines: &[(usize, &str)], idx: usize) -> bool {
    let Some((_, line)) = lines.get(idx) else {
        return false;
    };
    if !is_vlm_analyzed_heading(line) {
        return false;
    }
    let mut j = idx + 1;
    while j < lines.len() && j <= idx + 4 {
        let t = lines[j].1.trim();
        if t.is_empty() {
            j += 1;
            continue;
        }
        return is_vlm_type_marker(t);
    }
    false
}

fn is_hard_region_boundary(line: &str) -> bool {
    let t = line.trim();
    is_mm_chunk_header(t)
        || is_page_marker(t)
        || t == "<!-- multimodal-chunks -->"
        || fence_opener(t).is_some()
        || is_pipe_table_row(t)
}

fn is_region_boundary_line(line: &str, lines: &[(usize, &str)], idx: usize) -> bool {
    is_hard_region_boundary(line) || is_vlm_analyzed_block_start(lines, idx)
}

fn fence_opener(trimmed: &str) -> Option<&'static str> {
    if trimmed.starts_with("```") {
        Some("```")
    } else if trimmed.starts_with("~~~") {
        Some("~~~")
    } else {
        None
    }
}

fn is_page_marker(line: &str) -> bool {
    line.trim().starts_with(PAGE_MARKER_PREFIX)
}

/// True when `text` is a single HTML comment (page marker, mm fence, empty).
pub fn is_html_comment_only(text: &str) -> bool {
    let t = text.trim();
    t.starts_with("<!--") && t.ends_with("-->") && t.matches("<!--").count() == 1
}

fn is_pipe_table_row(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('|') && t.ends_with('|')
}

fn iter_lines_with_offsets(content: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    let mut rest = content;
    while !rest.is_empty() {
        let Some(newline) = rest.find('\n') else {
            if !rest.is_empty() {
                out.push((offset, rest));
            }
            break;
        };
        let line = &rest[..newline];
        out.push((offset, line));
        offset += newline + 1;
        rest = &rest[newline + 1..];
    }
    out
}

fn region_end(content: &str, lines: &[(usize, &str)], end_idx: usize) -> usize {
    if end_idx >= lines.len() {
        content.len()
    } else {
        lines[end_idx].0
    }
}

fn push_region(
    regions: &mut Vec<ContentRegion>,
    content: &str,
    start: usize,
    end: usize,
    atomic: Option<AtomicKind>,
) {
    if start >= end {
        return;
    }
    let text = content[start..end].trim_end().to_string();
    if text.trim().is_empty() {
        return;
    }
    // LAW-135-6: HTML comments are control plane, never extract units.
    if is_html_comment_only(&text) {
        return;
    }
    regions.push(ContentRegion {
        text,
        atomic,
        start,
        end,
    });
}

/// Split markdown into plain vs atomic regions (mm chunks, fences, pipe tables).
pub fn split_preserving_atomic_regions(content: &str) -> Vec<ContentRegion> {
    if content.trim().is_empty() {
        return Vec::new();
    }

    let lines = iter_lines_with_offsets(content);
    if lines.is_empty() {
        return vec![ContentRegion {
            text: content.to_string(),
            atomic: None,
            start: 0,
            end: content.len(),
        }];
    }

    let mut regions = Vec::new();
    let mut idx = 0usize;

    while idx < lines.len() {
        let (start, line) = lines[idx];
        let trimmed = line.trim();

        if is_mm_chunk_header(trimmed) || is_vlm_analyzed_block_start(&lines, idx) {
            idx += 1;
            while idx < lines.len() && !is_region_boundary_line(lines[idx].1, &lines, idx) {
                idx += 1;
            }
            push_region(
                &mut regions,
                content,
                start,
                region_end(content, &lines, idx),
                Some(AtomicKind::MultimodalChunk),
            );
        } else if let Some(opener) = fence_opener(trimmed) {
            idx += 1;
            while idx < lines.len() {
                if lines[idx].1.trim().starts_with(opener) {
                    idx += 1;
                    break;
                }
                idx += 1;
            }
            push_region(
                &mut regions,
                content,
                start,
                region_end(content, &lines, idx),
                Some(AtomicKind::FencedCode),
            );
        } else if is_pipe_table_row(trimmed) {
            idx += 1;
            while idx < lines.len() {
                let t = lines[idx].1.trim();
                if t.is_empty() {
                    idx += 1;
                    continue;
                }
                if !is_pipe_table_row(t) {
                    break;
                }
                idx += 1;
            }
            push_region(
                &mut regions,
                content,
                start,
                region_end(content, &lines, idx),
                Some(AtomicKind::PipeTable),
            );
        } else if is_hard_region_boundary(trimmed) {
            // Page markers / multimodal section headers stay in plain flow as single-line regions.
            idx += 1;
            push_region(
                &mut regions,
                content,
                start,
                region_end(content, &lines, idx),
                None,
            );
        } else {
            let plain_start = start;
            idx += 1;
            while idx < lines.len() && !is_region_boundary_line(lines[idx].1, &lines, idx) {
                idx += 1;
            }
            push_region(
                &mut regions,
                content,
                plain_start,
                region_end(content, &lines, idx),
                None,
            );
        }
    }

    if regions.is_empty() && !is_html_comment_only(content) && !content.trim().is_empty() {
        regions.push(ContentRegion {
            text: content.to_string(),
            atomic: None,
            start: 0,
            end: content.len(),
        });
    }

    regions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_mm_block_as_single_atomic_region() {
        let md = "Intro.\n\n[Chart Name]rev\n[Image Type]Chart\n\nQ4: 42\n\nTail.";
        let regions = split_preserving_atomic_regions(md);
        let mm = regions
            .iter()
            .find(|r| r.atomic == Some(AtomicKind::MultimodalChunk))
            .expect("mm region");
        assert!(mm.text.contains("[Chart Name]"));
        assert!(mm.text.contains("42"));
        assert!(!mm.text.contains("Intro"));
    }

    #[test]
    fn pipe_table_is_atomic() {
        let md = "| A | B |\n| --- | --- |\n| 1 | 2 |\n\nAfter";
        let regions = split_preserving_atomic_regions(md);
        assert!(regions
            .iter()
            .any(|r| r.atomic == Some(AtomicKind::PipeTable) && r.text.contains("| 1 |")));
    }

    #[test]
    fn vlm_inline_chart_block_is_atomic() {
        let block = "# rev q4\n\n**Type:** Chart\n\n**Key values:**\n- Q4: 42\n\nRevenue rose.";
        let md = format!("Intro.\n\n{block}\n\nTail.");
        let regions = split_preserving_atomic_regions(&md);
        let mm = regions
            .iter()
            .find(|r| r.atomic == Some(AtomicKind::MultimodalChunk))
            .expect("vlm atomic region");
        assert!(mm.text.contains("**Type:** Chart"));
        assert!(mm.text.contains("42"));
        assert!(!mm.text.contains("Intro"));
    }

    #[test]
    fn fenced_block_is_atomic() {
        let md = "Before\n\n```\n| a | b |\n| 1 | 2 |\n```\n\nAfter";
        let regions = split_preserving_atomic_regions(md);
        assert!(regions
            .iter()
            .any(|r| r.atomic == Some(AtomicKind::FencedCode) && r.text.contains("```")));
    }

    #[test]
    fn tilde_fence_is_atomic() {
        let md = "Before\n\n~~~\n# not a heading\n~~~\n\nAfter";
        let regions = split_preserving_atomic_regions(md);
        assert!(regions.iter().any(
            |r| r.atomic == Some(AtomicKind::FencedCode) && r.text.contains("# not a heading")
        ));
    }

    #[test]
    fn html_comment_only_is_not_a_region() {
        let md = "Hello.\n\n<!-- multimodal-chunks -->\n\n<!-- edgequake-page:3 -->\n\nWorld.";
        let regions = split_preserving_atomic_regions(md);
        assert!(
            regions.iter().all(|r| !is_html_comment_only(&r.text)),
            "comment-only regions leaked: {:?}",
            regions.iter().map(|r| r.text.as_str()).collect::<Vec<_>>()
        );
        let joined = regions.iter().map(|r| r.text.as_str()).collect::<String>();
        assert!(joined.contains("Hello"));
        assert!(joined.contains("World"));
    }
}
