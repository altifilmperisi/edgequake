//! Community list + type facets (SPEC-155 W3 / B07 / UI legend).

use axum::{extract::State, Json};

use crate::error::ApiResult;
use crate::handlers::graph::graph_dto::workspace_communities_and_facets;
use crate::handlers::graph_types::{GraphCommunitiesResponse, GraphFacetsResponse};
use crate::middleware::TenantContext;
use crate::services::tenant_guard::{has_full_tenant_context, warn_missing_tenant_context};
use crate::state::StorageRuntime;

/// List workspace communities aggregated from node `community_id` properties.
#[utoipa::path(
    get,
    path = "/api/v1/graph/communities",
    tag = "Graph",
    responses(
        (status = 200, description = "Communities retrieved", body = GraphCommunitiesResponse)
    )
)]
pub async fn get_graph_communities(
    State(storage): State<StorageRuntime>,
    tenant_ctx: TenantContext,
) -> ApiResult<Json<GraphCommunitiesResponse>> {
    if !has_full_tenant_context(&tenant_ctx) {
        warn_missing_tenant_context(&tenant_ctx, "get_graph_communities");
        return Ok(Json(GraphCommunitiesResponse {
            communities: vec![],
        }));
    }

    let (communities, _) =
        workspace_communities_and_facets(&storage.graph_storage, &tenant_ctx).await?;

    Ok(Json(GraphCommunitiesResponse { communities }))
}

/// Entity-type and relationship-type facet counts for Graph Studio legend.
#[utoipa::path(
    get,
    path = "/api/v1/graph/facets",
    tag = "Graph",
    responses(
        (status = 200, description = "Facets retrieved", body = GraphFacetsResponse)
    )
)]
pub async fn get_graph_facets(
    State(storage): State<StorageRuntime>,
    tenant_ctx: TenantContext,
) -> ApiResult<Json<GraphFacetsResponse>> {
    if !has_full_tenant_context(&tenant_ctx) {
        warn_missing_tenant_context(&tenant_ctx, "get_graph_facets");
        return Ok(Json(GraphFacetsResponse {
            entity_types: vec![],
            relationship_types: vec![],
        }));
    }

    let (_, facets) =
        workspace_communities_and_facets(&storage.graph_storage, &tenant_ctx).await?;

    Ok(Json(facets))
}
