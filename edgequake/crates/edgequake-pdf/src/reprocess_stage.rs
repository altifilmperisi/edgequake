//! SPEC-151 — Reprocess stages and page scope (LAW-151-3, LAW-151-8).

use crate::error::PdfConversionError;
use crate::page_selection::parse_page_list;
use serde::{Deserialize, Serialize};

/// Named product stages for partial reprocess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReprocessStage {
    Parse,
    Figures,
    Entities,
}

impl ReprocessStage {
    /// Parse a stage token (`parse` / `figures` / `entities`).
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "parse" | "parsing" | "ocr" | "convert" => Ok(Self::Parse),
            "figures" | "figure" | "charts" | "assets" => Ok(Self::Figures),
            "entities" | "entity" | "extract" | "kg" => Ok(Self::Entities),
            other => Err(format!("unknown reprocess stage '{other}'")),
        }
    }

    /// Stage order index (Parse=0 … Entities=2).
    pub fn order(self) -> u8 {
        match self {
            Self::Parse => 0,
            Self::Figures => 1,
            Self::Entities => 2,
        }
    }

    /// Closure: selecting a stage includes all downstream stages (LAW-151-3).
    /// Entities alone returns only Entities.
    pub fn closure(self) -> Vec<Self> {
        match self {
            Self::Parse => vec![Self::Parse, Self::Figures, Self::Entities],
            Self::Figures => vec![Self::Figures, Self::Entities],
            Self::Entities => vec![Self::Entities],
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parse => "parse",
            Self::Figures => "figures",
            Self::Entities => "entities",
        }
    }
}

impl std::fmt::Display for ReprocessStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Effective stage set from a user request (union of closures, sorted).
pub fn effective_stages(requested: &[ReprocessStage]) -> Vec<ReprocessStage> {
    let mut set = std::collections::BTreeSet::new();
    for stage in requested {
        for s in stage.closure() {
            set.insert(s.order());
        }
    }
    set.into_iter()
        .filter_map(|o| match o {
            0 => Some(ReprocessStage::Parse),
            1 => Some(ReprocessStage::Figures),
            2 => Some(ReprocessStage::Entities),
            _ => None,
        })
        .collect()
}

/// If figures is requested but raw OCR is missing for some pages, add Parse.
pub fn ensure_parse_when_figures_need_raw(
    stages: &[ReprocessStage],
    pages_missing_raw: bool,
) -> Vec<ReprocessStage> {
    let mut out = effective_stages(stages);
    if pages_missing_raw
        && out.contains(&ReprocessStage::Figures)
        && !out.contains(&ReprocessStage::Parse)
    {
        out.insert(0, ReprocessStage::Parse);
    }
    out
}

/// Page-scoped reprocess intent attached to [`PdfProcessingData`](edgequake tasks).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageScope {
    /// 1-indexed page numbers (deduped, sorted).
    pub pages: Vec<u32>,
    /// Requested stages (before closure).
    pub stages: Vec<ReprocessStage>,
}

impl PageScope {
    /// Build from page numbers + stages; validates non-empty pages.
    pub fn new(pages: Vec<u32>, stages: Vec<ReprocessStage>) -> Result<Self, PdfConversionError> {
        let pages = normalize_pages(&pages, None)?;
        if stages.is_empty() {
            return Err(PdfConversionError::Backend(
                "at least one reprocess stage is required".into(),
            ));
        }
        Ok(Self { pages, stages })
    }

    /// Parse pages from a list or a range string (`1-3,7`).
    pub fn from_pages_spec(
        pages: PagesSpec,
        stages: Vec<ReprocessStage>,
        page_count: Option<u32>,
    ) -> Result<Self, PdfConversionError> {
        let raw = match pages {
            PagesSpec::List(v) => v,
            PagesSpec::Range(s) => parse_page_list(&s)?,
        };
        let pages = normalize_pages(&raw, page_count)?;
        if stages.is_empty() {
            return Err(PdfConversionError::Backend(
                "at least one reprocess stage is required".into(),
            ));
        }
        Ok(Self { pages, stages })
    }

    pub fn effective_stages(&self) -> Vec<ReprocessStage> {
        effective_stages(&self.stages)
    }

    pub fn includes_parse(&self) -> bool {
        self.effective_stages().contains(&ReprocessStage::Parse)
    }

    pub fn includes_figures(&self) -> bool {
        self.effective_stages().contains(&ReprocessStage::Figures)
    }

    pub fn includes_entities(&self) -> bool {
        self.effective_stages().contains(&ReprocessStage::Entities)
    }
}

/// Flexible pages field for JSON APIs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PagesSpec {
    List(Vec<u32>),
    Range(String),
}

/// Dedupe, sort, reject empty / zero / out-of-range.
pub fn normalize_pages(
    pages: &[u32],
    page_count: Option<u32>,
) -> Result<Vec<u32>, PdfConversionError> {
    let mut set = std::collections::BTreeSet::new();
    for &p in pages {
        if p < 1 {
            return Err(PdfConversionError::Backend(format!(
                "Pages are 1-indexed, minimum is 1 (got {p})"
            )));
        }
        if let Some(max) = page_count {
            if p > max {
                return Err(PdfConversionError::Backend(format!(
                    "Page {p} is out of range (document has {max} pages)"
                )));
            }
        }
        set.insert(p);
    }
    if set.is_empty() {
        return Err(PdfConversionError::Backend(
            "at least one page is required".into(),
        ));
    }
    Ok(set.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closure_parse_includes_all() {
        assert_eq!(
            ReprocessStage::Parse.closure(),
            vec![
                ReprocessStage::Parse,
                ReprocessStage::Figures,
                ReprocessStage::Entities
            ]
        );
    }

    #[test]
    fn closure_entities_alone() {
        assert_eq!(
            ReprocessStage::Entities.closure(),
            vec![ReprocessStage::Entities]
        );
    }

    #[test]
    fn effective_union() {
        let e = effective_stages(&[ReprocessStage::Figures, ReprocessStage::Entities]);
        assert_eq!(e, vec![ReprocessStage::Figures, ReprocessStage::Entities]);
    }

    #[test]
    fn figures_missing_raw_adds_parse() {
        let out = ensure_parse_when_figures_need_raw(&[ReprocessStage::Figures], true);
        assert!(out.contains(&ReprocessStage::Parse));
    }

    #[test]
    fn page_scope_rejects_empty() {
        assert!(PageScope::new(vec![], vec![ReprocessStage::Entities]).is_err());
    }

    #[test]
    fn page_scope_from_range_string() {
        let scope = PageScope::from_pages_spec(
            PagesSpec::Range("1-3,7".into()),
            vec![ReprocessStage::Parse],
            Some(10),
        )
        .unwrap();
        assert_eq!(scope.pages, vec![1, 2, 3, 7]);
    }
}
