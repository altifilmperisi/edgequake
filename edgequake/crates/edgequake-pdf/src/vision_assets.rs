//! SPEC-151 — Page-scoped vision asset facade (SRP around Pass-A writers).
//!
//! Full `VisionPdfConverter::convert` remains the SSOT for OCR+assets in the
//! happy path. This module exposes the same writers for figures-only reprocess
//! without re-running OCR when raw page markdown already exists.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use edgequake_llm::LLMProvider;
use tracing::{info, warn};

use crate::chart_crop::{
    chart_residual_alongside_fig_pages, chart_residual_candidate_pages,
    filter_chart_pages_by_page_png_ink, promote_fig_as_chart_when_ink_empty,
    write_chart_crop_assets, CropCoverageReport, CHART_CROP_RENDER,
};
use crate::embedded_images::{figures_by_page, write_embedded_figure_assets, WrittenFigureAsset};
use crate::figure_filter::{
    apply_filter_result_or_keep, collect_filter_candidates, prune_chart_crop_paths, write_manifest,
    FigureFilter, FigureFilterResult,
};
use crate::page_assets::{write_page_png_assets, PageAssetRenderConfig};
use crate::page_layout;
use crate::region_assets::{tables_by_page, write_caption_region_assets, WrittenTableAsset};
use crate::PageModality;

/// Bundle of Pass-A assets for a selected page set.
#[derive(Debug, Default)]
pub struct PageAssetBundle {
    pub figure_map: HashMap<usize, Vec<WrittenFigureAsset>>,
    pub table_map: HashMap<usize, Vec<WrittenTableAsset>>,
    pub chart_crop_paths: HashMap<usize, String>,
    pub filter_results: Vec<FigureFilterResult>,
    pub crop_coverage_comment: Option<String>,
}

/// Options for [`build_page_asset_bundle`].
pub struct PageAssetBuildConfig<'a> {
    pub assets_root: &'a Path,
    pub pages: &'a [usize],
    pub dpi: u32,
    pub max_rendered_pixels: u32,
    pub write_figures: bool,
    pub write_page_pngs: bool,
    pub write_charts: bool,
    pub promote_fig_as_chart: bool,
    pub page_modality: Option<PageModality>,
    pub figure_filter_provider: Option<Arc<dyn LLMProvider>>,
}

/// Build figure/chart/page PNG assets for `pages` only (SPEC-151 figures stage).
pub async fn build_page_asset_bundle(
    pdf_bytes: &[u8],
    cfg: PageAssetBuildConfig<'_>,
) -> PageAssetBundle {
    let mut bundle = PageAssetBundle::default();
    let page_as_unit = cfg.page_modality.is_some_and(|m| m.is_manuscript_like());
    let render = PageAssetRenderConfig {
        dpi: cfg.dpi,
        max_rendered_pixels: cfg.max_rendered_pixels,
    };
    let page_numbers = cfg.pages.to_vec();

    if cfg.write_figures {
        match write_embedded_figure_assets(pdf_bytes, cfg.assets_root, Some(&page_numbers)).await {
            Ok(written) => {
                bundle.figure_map = figures_by_page(&written);
                info!(
                    figures = written.len(),
                    "SPEC-151: embedded figures written"
                );
            }
            Err(e) => warn!(error = %e, "SPEC-151: embedded figure extract failed"),
        }
        if !page_as_unit {
            match write_caption_region_assets(
                pdf_bytes,
                cfg.assets_root,
                &bundle.figure_map,
                Some(&page_numbers),
            )
            .await
            {
                Ok((figs, tables)) => {
                    for fig in figs {
                        bundle.figure_map.entry(fig.page_num).or_default().push(fig);
                    }
                    bundle.table_map = tables_by_page(&tables);
                }
                Err(e) => warn!(error = %e, "SPEC-151: caption region extract failed"),
            }
        }
    }

    if cfg.write_page_pngs {
        match write_page_png_assets(pdf_bytes, cfg.assets_root, &page_numbers, render).await {
            Ok(written) => info!(pages = written.len(), "SPEC-151: page PNGs written"),
            Err(e) => warn!(error = %e, "SPEC-151: page PNG write failed"),
        }
    }

    let mut coverage =
        CropCoverageReport::from_pages(&page_numbers, &bundle.figure_map, &bundle.table_map);
    if cfg.write_charts && !page_as_unit {
        let candidates =
            chart_residual_candidate_pages(&page_numbers, &bundle.figure_map, &bundle.table_map);
        let chart_pages = if cfg.write_page_pngs {
            filter_chart_pages_by_page_png_ink(cfg.assets_root, &candidates)
        } else {
            candidates
        };
        coverage = coverage.with_ink_filter_count(chart_pages.len());
        if !chart_pages.is_empty() {
            match write_chart_crop_assets(
                pdf_bytes,
                cfg.assets_root,
                &chart_pages,
                CHART_CROP_RENDER,
            )
            .await
            {
                Ok(paths) => bundle.chart_crop_paths = paths,
                Err(e) => warn!(error = %e, "SPEC-151: chart crop failed"),
            }
        }
        if cfg.promote_fig_as_chart {
            let alongside = chart_residual_alongside_fig_pages(
                &page_numbers,
                &bundle.figure_map,
                &bundle.table_map,
            );
            let promoted = promote_fig_as_chart_when_ink_empty(
                cfg.assets_root,
                &alongside,
                &bundle.chart_crop_paths,
            );
            bundle.chart_crop_paths.extend(promoted);
        }
    }
    coverage = coverage.with_crops_written(bundle.chart_crop_paths.len());
    bundle.crop_coverage_comment = Some(coverage.to_html_comment());

    if let Some(ref provider) = cfg.figure_filter_provider {
        let candidates = collect_filter_candidates(
            cfg.assets_root,
            &bundle.figure_map,
            &bundle.chart_crop_paths,
        );
        if !candidates.is_empty() {
            let filter = FigureFilter::new(Arc::clone(provider));
            let run = filter.run(&candidates).await;
            if let Ok(ref results) = run {
                let _ = write_manifest(cfg.assets_root, results);
                page_layout::write_sidecar_from_assets(
                    cfg.assets_root,
                    pdf_bytes,
                    &bundle.figure_map,
                    &bundle.table_map,
                    Some(results),
                );
                bundle.filter_results = results.clone();
            }
            bundle.figure_map =
                apply_filter_result_or_keep(bundle.figure_map, run, cfg.assets_root, true);
            bundle.chart_crop_paths =
                prune_chart_crop_paths(bundle.chart_crop_paths, &bundle.filter_results);
        }
    }
    if !page_layout::sidecar_exists(cfg.assets_root) {
        page_layout::write_sidecar_from_assets(
            cfg.assets_root,
            pdf_bytes,
            &bundle.figure_map,
            &bundle.table_map,
            None,
        );
    }
    bundle
}
