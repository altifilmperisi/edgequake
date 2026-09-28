//! SPEC-151 — Record per-page parse/figure/entity state during full pipeline.

use std::path::Path;
use std::sync::Arc;

use edgequake_pdf::{is_placeholder, split_sections};
use edgequake_storage::{
    PageStateStorage, UpsertPageEntities, UpsertPageFigures, UpsertPageParse, PAGE_STAGE_FAILED,
    PAGE_STAGE_OK, PAGE_STAGE_PENDING,
};
use sha2::{Digest, Sha256};
use tracing::{debug, warn};
use uuid::Uuid;

/// Build a vision page_result_sink that upserts parse OK rows as pages complete.
pub fn make_parse_page_sink(
    store: Arc<dyn PageStateStorage>,
    document_id: Uuid,
    workspace_id: Uuid,
    model: Option<String>,
    track_id: Option<String>,
) -> edgequake_pdf::VisionPageResultSink {
    Arc::new(move |page: usize, md: &str| {
        let store = Arc::clone(&store);
        let model = model.clone();
        let track_id = track_id.clone();
        let status = if is_placeholder(md) || md.trim().is_empty() {
            PAGE_STAGE_FAILED
        } else {
            PAGE_STAGE_OK
        };
        let error = if status == PAGE_STAGE_FAILED {
            Some("empty or placeholder OCR".into())
        } else {
            None
        };
        let mut hasher = Sha256::new();
        hasher.update(md.as_bytes());
        let raw_sha = format!("{:x}", hasher.finalize());
        let row = UpsertPageParse {
            page_number: page as i32,
            status: status.to_string(),
            error,
            method: Some("vision".into()),
            model,
            raw_markdown: Some(format!("<!-- edgequake-page:{page} -->\n\n{md}")),
            raw_sha256: Some(raw_sha),
            increment_attempts: true,
            track_id,
        };
        // Fire-and-forget via current runtime; ignore join errors.
        let doc = document_id;
        let ws = workspace_id;
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                if let Err(e) = store.upsert_parse_batch(doc, ws, &[row]).await {
                    warn!(error = %e, page, "SPEC-151: upsert_parse_batch failed");
                }
            });
        }
    })
}

/// After a successful convert, mark all markdown sections as parse-ok (covers EdgeParse).
pub async fn record_parse_from_markdown(
    store: &dyn PageStateStorage,
    document_id: Uuid,
    workspace_id: Uuid,
    markdown: &str,
    method: &str,
    model: Option<String>,
    track_id: Option<String>,
) {
    let (sections, _) = split_sections(markdown);
    let rows: Vec<_> = sections
        .iter()
        .map(|s| {
            let status = if is_placeholder(&s.section) {
                PAGE_STAGE_FAILED
            } else {
                PAGE_STAGE_OK
            };
            let mut hasher = Sha256::new();
            hasher.update(s.section.as_bytes());
            UpsertPageParse {
                page_number: s.page as i32,
                status: status.to_string(),
                error: if status == PAGE_STAGE_FAILED {
                    Some("placeholder".into())
                } else {
                    None
                },
                method: Some(method.into()),
                model: model.clone(),
                raw_markdown: Some(s.section.clone()),
                raw_sha256: Some(format!("{:x}", hasher.finalize())),
                increment_attempts: false,
                track_id: track_id.clone(),
            }
        })
        .collect();
    if rows.is_empty() {
        return;
    }
    if let Err(e) = store
        .upsert_parse_batch(document_id, workspace_id, &rows)
        .await
    {
        warn!(error = %e, "SPEC-151: record_parse_from_markdown failed");
    }
}

/// Count figure markers per page section and upsert figures status.
///
/// When `only_pages` is `Some`, only those 1-indexed pages are written (SPEC-151
/// page-scope honesty — do not rewrite health for untouched pages).
pub async fn record_figures_from_markdown(
    store: &dyn PageStateStorage,
    document_id: Uuid,
    workspace_id: Uuid,
    markdown: &str,
    track_id: Option<String>,
) {
    record_figures_from_markdown_scoped(store, document_id, workspace_id, markdown, track_id, None)
        .await;
}

/// Page-scoped figures health upsert (selected pages only).
pub async fn record_figures_from_markdown_scoped(
    store: &dyn PageStateStorage,
    document_id: Uuid,
    workspace_id: Uuid,
    markdown: &str,
    track_id: Option<String>,
    only_pages: Option<&[u32]>,
) {
    let allow: Option<std::collections::HashSet<u32>> =
        only_pages.map(|p| p.iter().copied().collect());
    let (sections, _) = split_sections(markdown);
    let rows: Vec<_> = sections
        .iter()
        .filter(|s| {
            allow
                .as_ref()
                .map(|set| set.contains(&(s.page as u32)))
                .unwrap_or(true)
        })
        .map(|s| {
            let count = s.section.matches("![").count() as i32;
            UpsertPageFigures {
                page_number: s.page as i32,
                status: PAGE_STAGE_OK.to_string(),
                count,
                error: None,
                track_id: track_id.clone(),
            }
        })
        .collect();
    if rows.is_empty() {
        return;
    }
    if let Err(e) = store
        .upsert_figures_batch(document_id, workspace_id, &rows)
        .await
    {
        warn!(error = %e, "SPEC-151: record_figures_from_markdown failed");
    }
}

/// Upsert entities stage: `(page, chunk_count, failed_count)` rows.
pub async fn record_entities_from_chunks(
    store: &dyn PageStateStorage,
    document_id: Uuid,
    workspace_id: Uuid,
    per_page: &[(u32, i32, i32)],
    track_id: Option<String>,
) {
    let rows: Vec<_> = per_page
        .iter()
        .map(|(page, chunks, failed)| {
            let status = if *failed > 0 {
                PAGE_STAGE_FAILED
            } else if *chunks > 0 {
                PAGE_STAGE_OK
            } else {
                PAGE_STAGE_PENDING
            };
            UpsertPageEntities {
                page_number: *page as i32,
                status: status.to_string(),
                chunk_count: *chunks,
                failed_chunk_count: *failed,
                error: if *failed > 0 {
                    Some(format!("{failed} chunk(s) failed"))
                } else {
                    None
                },
                track_id: track_id.clone(),
            }
        })
        .collect();
    if rows.is_empty() {
        return;
    }
    if let Err(e) = store
        .upsert_entities_batch(document_id, workspace_id, &rows)
        .await
    {
        warn!(error = %e, "SPEC-151: record_entities_from_chunks failed");
    }
}

/// Salvage leftover durable vision checkpoint page files after a failed convert.
pub async fn salvage_checkpoint_pages(
    store: &dyn PageStateStorage,
    document_id: Uuid,
    workspace_id: Uuid,
    checkpoint_dir: &Path,
    model: Option<String>,
    track_id: Option<String>,
) {
    if !checkpoint_dir.exists() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(checkpoint_dir) else {
        return;
    };
    let mut rows = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        // Expect `{page}.md` or `page-{n}/result.md` layouts.
        let (page, md) =
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let Ok(page) = stem.parse::<i32>() else {
                    continue;
                };
                let Ok(md) = std::fs::read_to_string(&path) else {
                    continue;
                };
                (page, md)
            } else if path.is_dir() {
                let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                let page = name
                    .strip_prefix("page-")
                    .or_else(|| name.strip_prefix("page_"))
                    .unwrap_or(name)
                    .parse::<i32>()
                    .ok();
                let Some(page) = page else { continue };
                let result = path.join("result.md");
                let alt = path.join(format!("{page}.md"));
                let md_path = if result.exists() { result } else { alt };
                let Ok(md) = std::fs::read_to_string(&md_path) else {
                    continue;
                };
                (page, md)
            } else {
                continue;
            };
        let status = if is_placeholder(&md) || md.trim().is_empty() {
            PAGE_STAGE_FAILED
        } else {
            PAGE_STAGE_OK
        };
        let mut hasher = Sha256::new();
        hasher.update(md.as_bytes());
        rows.push(UpsertPageParse {
            page_number: page,
            status: status.to_string(),
            error: if status == PAGE_STAGE_FAILED {
                Some("salvaged placeholder".into())
            } else {
                None
            },
            method: Some("vision_checkpoint".into()),
            model: model.clone(),
            raw_markdown: Some(md),
            raw_sha256: Some(format!("{:x}", hasher.finalize())),
            increment_attempts: false,
            track_id: track_id.clone(),
        });
    }
    if rows.is_empty() {
        debug!("SPEC-151: no checkpoint pages to salvage");
        return;
    }
    info_salvage(rows.len());
    if let Err(e) = store
        .upsert_parse_batch(document_id, workspace_id, &rows)
        .await
    {
        warn!(error = %e, "SPEC-151: salvage_checkpoint_pages upsert failed");
    }
}

fn info_salvage(n: usize) {
    tracing::info!(
        pages = n,
        "SPEC-151: salvaged checkpoint pages into document_page_states"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_storage::MemoryPageStateStorage;

    #[tokio::test]
    async fn figures_scoped_writes_only_selected_pages() {
        let store = MemoryPageStateStorage::new();
        let doc = Uuid::new_v4();
        let ws = Uuid::new_v4();
        let md = "\
<!-- edgequake-page:1 -->\n![a](a.png)\n\
<!-- edgequake-page:2 -->\ntext only\n\
<!-- edgequake-page:3 -->\n![b](b.png)\n![c](c.png)\n";

        record_figures_from_markdown_scoped(&store, doc, ws, md, Some("track".into()), Some(&[2]))
            .await;

        let rows = store.list_page_states(doc, ws).await.unwrap();
        assert_eq!(rows.len(), 1, "must not upsert untouched pages");
        assert_eq!(rows[0].page_number, 2);
        assert_eq!(rows[0].figures_status, PAGE_STAGE_OK);
        assert_eq!(rows[0].figures_count, 0);
    }
}
