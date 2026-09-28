//! SPEC-151 — Derive page health for legacy documents without M160 rows.

use edgequake_pdf::{is_placeholder, split_sections, EMPTY_VISION_PAGE_PLACEHOLDER};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Per-stage status payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct StageHealth {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_chunk_count: Option<i32>,
}

/// One page's health across stages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct PageHealth {
    pub page_number: u32,
    pub parse: StageHealth,
    pub figures: StageHealth,
    pub entities: StageHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct PageHealthSummary {
    pub parse_failed: usize,
    pub figures_failed: usize,
    pub entities_failed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct PageHealthResponse {
    pub document_id: String,
    pub page_count: u32,
    pub pages: Vec<PageHealth>,
    pub summary: PageHealthSummary,
    /// `stored` when rows come from `document_page_states`; else `derived`.
    pub source: String,
}

fn stage(status: &str, error: Option<String>) -> StageHealth {
    StageHealth {
        status: status.to_string(),
        error,
        count: None,
        chunk_count: None,
        failed_chunk_count: None,
    }
}

/// Derive health from markdown markers + optional figure/entity hints.
pub fn derive_page_health(
    document_id: &str,
    page_count: u32,
    markdown: &str,
    figures_by_page: &[(u32, i32)],
    failed_chunk_pages: &[u32],
    chunk_counts_by_page: &[(u32, i32)],
) -> PageHealthResponse {
    let (sections, _) = split_sections(markdown);
    let mut pages = Vec::new();
    let max_page = page_count.max(sections.iter().map(|s| s.page as u32).max().unwrap_or(0));
    for n in 1..=max_page.max(1) {
        let section = sections.iter().find(|s| s.page as u32 == n);
        let parse = match section {
            None => stage("pending", Some("missing page marker".into())),
            Some(s)
                if is_placeholder(&s.section)
                    || s.section.contains(EMPTY_VISION_PAGE_PLACEHOLDER) =>
            {
                stage("failed", Some("empty or placeholder OCR".into()))
            }
            Some(_) => stage("ok", None),
        };
        let fig_count = figures_by_page
            .iter()
            .find(|(p, _)| *p == n)
            .map(|(_, c)| *c)
            .unwrap_or(0);
        let mut figures = if fig_count > 0 {
            stage("ok", None)
        } else {
            stage("pending", None)
        };
        figures.count = Some(fig_count);

        let chunks = chunk_counts_by_page
            .iter()
            .find(|(p, _)| *p == n)
            .map(|(_, c)| *c)
            .unwrap_or(0);
        let failed = failed_chunk_pages.iter().filter(|p| **p == n).count() as i32;
        let mut entities = if failed > 0 {
            stage("failed", Some(format!("{failed} failed chunk(s)")))
        } else if chunks > 0 {
            stage("ok", None)
        } else {
            stage("pending", None)
        };
        entities.chunk_count = Some(chunks);
        entities.failed_chunk_count = Some(failed);

        pages.push(PageHealth {
            page_number: n,
            parse,
            figures,
            entities,
        });
    }

    let summary = PageHealthSummary {
        parse_failed: pages.iter().filter(|p| p.parse.status == "failed").count(),
        figures_failed: pages
            .iter()
            .filter(|p| p.figures.status == "failed")
            .count(),
        entities_failed: pages
            .iter()
            .filter(|p| p.entities.status == "failed")
            .count(),
    };

    PageHealthResponse {
        document_id: document_id.to_string(),
        page_count: max_page,
        pages,
        summary,
        source: "derived".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_failed_placeholder() {
        let md = format!(
            "<!-- edgequake-page:1 -->\n\n{EMPTY_VISION_PAGE_PLACEHOLDER}\n\n<!-- edgequake-page:2 -->\n\nOk\n"
        );
        let resp = derive_page_health("doc", 2, &md, &[], &[], &[(2, 1)]);
        assert_eq!(resp.pages[0].parse.status, "failed");
        assert_eq!(resp.pages[1].parse.status, "ok");
        assert_eq!(resp.pages[1].entities.status, "ok");
    }
}
