//! Graph tools: entity search / get / neighborhood (SPEC-152).

use serde_json::{json, Value};

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::entity_neighborhood::build_entity_neighborhood;
use crate::state::AppState;

use super::budget::{apply_budget, BudgetClass};
use super::envelope::EnvelopeBuilder;
use super::errors::{eq_error, ErrorCode};
use super::ids::{
    agent_entity_id, is_artifact_type, map_entity_type, resolve_entity_lookup, title_case_label,
    truncate_chars,
};

const DEFAULT_NEIGHBOR_CAP: usize = 16;

pub async fn eq_entity_search(
    state: &AppState,
    tenant_ctx: &TenantContext,
    args: &Value,
) -> ApiResult<Value> {
    let budget = BudgetClass::parse(args.get("budget").and_then(|v| v.as_str()));
    let q = args
        .get("q")
        .or_else(|| args.get("query"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if q.trim().is_empty() {
        return Ok(eq_error(ErrorCode::InvalidId, "q is required", None));
    }
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(10)
        .clamp(1, 50) as usize;
    let include_artifacts = args
        .get("include_artifacts")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let type_filter = args.get("type").and_then(|v| v.as_str());
    let type_filter_storage = type_filter.map(|t| t.to_ascii_uppercase());
    let workspace = tenant_ctx.workspace_id.as_deref().unwrap_or("default");

    let nodes = state
        .storage
        .graph_storage
        .search_nodes(
            &q,
            limit * 3,
            type_filter_storage.as_deref(),
            tenant_ctx.tenant_id.as_deref(),
            tenant_ctx.workspace_id.as_deref(),
        )
        .await
        .map_err(|e| ApiError::Internal(format!("entity search: {e}")))?;

    let mut entities = Vec::new();
    for (node, degree) in nodes {
        let etype = node
            .properties
            .get("entity_type")
            .and_then(|v| v.as_str())
            .unwrap_or("CONCEPT")
            .to_string();
        if !include_artifacts && is_artifact_type(&etype) {
            continue;
        }
        let mapped = map_entity_type(&etype);
        if let Some(tf) = type_filter {
            if !mapped.eq_ignore_ascii_case(tf) {
                continue;
            }
        }
        let desc = node
            .properties
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        entities.push(json!({
            "id": agent_entity_id(workspace, &node.id),
            "name": title_case_label(&node.id),
            "slug": resolve_entity_lookup(&node.id),
            "type": mapped,
            "one_liner": truncate_chars(&desc, budget.max_one_liner()),
            "degree": degree,
            "document_ids": [],
        }));
        if entities.len() >= limit {
            break;
        }
    }

    let env = EnvelopeBuilder::new("entity_search", budget)
        .insert("entities", json!(entities))
        .build();
    let (capped, over) = apply_budget(env, budget);
    if over {
        return Ok(eq_error(
            ErrorCode::TruncateInvalid,
            "structuredContent exceeded budget",
            None,
        ));
    }
    Ok(capped)
}

pub async fn eq_entity_get(
    state: &AppState,
    tenant_ctx: &TenantContext,
    args: &Value,
) -> ApiResult<Value> {
    let budget = BudgetClass::parse(args.get("budget").and_then(|v| v.as_str()));
    let entity_id = args
        .get("entity_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("entity_id required".into()))?;
    let lookup = resolve_entity_lookup(entity_id);
    let workspace = tenant_ctx.workspace_id.as_deref().unwrap_or("default");

    let node = crate::handlers::isolation::load_node_for_tenant_context(
        state.storage.graph_storage.as_ref(),
        &lookup,
        tenant_ctx,
    )
    .await
    .map_err(|_| ApiError::NotFound(format!("Entity not found: {entity_id}")))?;

    let etype = node
        .properties
        .get("entity_type")
        .and_then(|v| v.as_str())
        .unwrap_or("CONCEPT")
        .to_string();
    let desc = node
        .properties
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let uri = format!(
        "eq://{}/entities/{}",
        workspace,
        agent_entity_id(workspace, &node.id)
    );

    Ok(EnvelopeBuilder::new("entity_get", budget)
        .insert(
            "entities",
            json!([{
                "id": agent_entity_id(workspace, &node.id),
                "name": title_case_label(&node.id),
                "slug": resolve_entity_lookup(&node.id),
                "type": map_entity_type(&etype),
                "one_liner": truncate_chars(&desc, budget.max_one_liner()),
                "description": truncate_chars(&desc, budget.max_one_liner().saturating_mul(3)),
                "document_ids": [],
            }]),
        )
        .insert("lineage_resources", json!([uri]))
        .build())
}

pub async fn eq_neighborhood(
    state: &AppState,
    tenant_ctx: &TenantContext,
    args: &Value,
) -> ApiResult<Value> {
    let budget = BudgetClass::parse(args.get("budget").and_then(|v| v.as_str()));
    let entity_id = args
        .get("entity_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("entity_id required".into()))?;
    let mut max_hops = args
        .get("max_hops")
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .clamp(1, 3) as u32;
    if max_hops == 3 && budget != BudgetClass::Deep {
        max_hops = 2;
    }
    let include_artifacts = args
        .get("include_artifacts")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let include_weak_edges = args
        .get("include_weak_edges")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let workspace = tenant_ctx.workspace_id.as_deref().unwrap_or("default");
    let lookup = resolve_entity_lookup(entity_id);

    let (nodes, edges) =
        build_entity_neighborhood(&state.storage.graph_storage, tenant_ctx, &lookup, max_hops)
            .await?;

    let mut entities = Vec::new();
    for n in &nodes {
        if !include_artifacts && is_artifact_type(&n.entity_type) {
            continue;
        }
        entities.push(json!({
            "id": agent_entity_id(workspace, &n.id),
            "name": title_case_label(&n.id),
            "slug": resolve_entity_lookup(&n.id),
            "type": map_entity_type(&n.entity_type),
            "one_liner": truncate_chars(&n.description, budget.max_one_liner()),
            "degree": n.degree,
            "document_ids": [],
        }));
    }
    entities.sort_by(|a, b| {
        let da = a.get("degree").and_then(|v| v.as_u64()).unwrap_or(0);
        let db = b.get("degree").and_then(|v| v.as_u64()).unwrap_or(0);
        db.cmp(&da)
    });
    let omitted = entities.len().saturating_sub(DEFAULT_NEIGHBOR_CAP);
    entities.truncate(DEFAULT_NEIGHBOR_CAP);

    let mut relationships = Vec::new();
    for e in &edges {
        if !include_weak_edges && e.relation_type.eq_ignore_ascii_case("RELATED_TO") {
            continue;
        }
        relationships.push(json!({
            "id": format!("rel:{}:{}:{}", e.source, e.relation_type, e.target),
            "type": e.relation_type,
            "source": agent_entity_id(workspace, &e.source),
            "target": agent_entity_id(workspace, &e.target),
            "score": e.weight,
        }));
    }

    let path: Vec<String> = entities
        .iter()
        .filter_map(|e| e.get("id").and_then(|v| v.as_str()).map(str::to_string))
        .collect();

    let mut env = EnvelopeBuilder::new("neighborhood", budget)
        .insert("entities", json!(entities))
        .insert("relationships", json!(relationships))
        .insert("path", json!(path));
    if omitted > 0 {
        env = env.truncation(json!({
            "truncated": true,
            "omitted_entities": omitted,
        }));
    }
    let env = env.build();
    let (capped, over) = apply_budget(env, budget);
    if over {
        return Ok(eq_error(
            ErrorCode::TruncateInvalid,
            "structuredContent exceeded budget",
            None,
        ));
    }
    Ok(capped)
}
