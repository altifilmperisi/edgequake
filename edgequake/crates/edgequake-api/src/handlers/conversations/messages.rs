//! Message handlers within conversations.
//!
//! Implements list, create, update, and delete for messages.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::conversations_types::*;
use crate::middleware::TenantContext;
use crate::state::AppState;
use edgequake_core::types::{
    CreateMessageRequest, MessageRole, SetMessageFeedbackRequest, UpdateMessageRequest,
};

fn validate_feedback_rating(rating: Option<String>) -> ApiResult<Option<String>> {
    match rating {
        None => Ok(None),
        Some(r) if r == "up" || r == "down" => Ok(Some(r)),
        Some(_) => Err(ApiError::BadRequest(
            "rating must be 'up', 'down', or null".into(),
        )),
    }
}

/// List messages in a conversation.
#[utoipa::path(
    get,
    path = "/api/v1/conversations/{id}/messages",
    params(
        ("id" = Uuid, Path, description = "Conversation ID"),
        ListMessagesParams
    ),
    responses(
        (status = 200, description = "List of messages", body = PaginatedMessagesResponse),
    ),
    tags = ["conversations"]
)]
pub async fn list_messages(
    State(state): State<AppState>,
    _tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
    Query(params): Query<ListMessagesParams>,
) -> ApiResult<Json<PaginatedMessagesResponse>> {
    let limit = params.limit.min(200);

    let result = state
        .conversation_service
        .list_messages(id, params.cursor, limit)
        .await?;

    Ok(Json(PaginatedMessagesResponse {
        items: result.items.into_iter().map(Into::into).collect(),
        pagination: PaginationMetaResponse {
            next_cursor: result.pagination.next_cursor,
            prev_cursor: result.pagination.prev_cursor,
            total: result.pagination.total,
            has_more: result.pagination.has_more,
        },
    }))
}

/// Create a message in a conversation.
#[utoipa::path(
    post,
    path = "/api/v1/conversations/{id}/messages",
    params(
        ("id" = Uuid, Path, description = "Conversation ID")
    ),
    request_body = CreateMessageApiRequest,
    responses(
        (status = 201, description = "Message created", body = MessageResponse),
    ),
    tags = ["conversations"]
)]
pub async fn create_message(
    State(state): State<AppState>,
    _tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
    Json(request): Json<CreateMessageApiRequest>,
) -> ApiResult<(StatusCode, Json<MessageResponse>)> {
    let role = match request.role.to_lowercase().as_str() {
        "assistant" => MessageRole::Assistant,
        "system" => MessageRole::System,
        _ => MessageRole::User,
    };

    let message = state
        .conversation_service
        .create_message(
            id,
            CreateMessageRequest {
                content: request.content,
                role,
                parent_id: request.parent_id,
                stream: request.stream,
            },
        )
        .await?;

    Ok((StatusCode::CREATED, Json(message.into())))
}

/// Update a message.
#[utoipa::path(
    patch,
    path = "/api/v1/messages/{message_id}",
    params(
        ("message_id" = Uuid, Path, description = "Message ID")
    ),
    request_body = UpdateMessageApiRequest,
    responses(
        (status = 200, description = "Message updated", body = MessageResponse),
    ),
    tags = ["conversations"]
)]
pub async fn update_message(
    State(state): State<AppState>,
    _tenant_ctx: TenantContext,
    Path(message_id): Path<Uuid>,
    Json(request): Json<UpdateMessageApiRequest>,
) -> ApiResult<Json<MessageResponse>> {
    let context = request.context.and_then(|c| serde_json::from_value(c).ok());

    let message = state
        .conversation_service
        .update_message(
            message_id,
            UpdateMessageRequest {
                content: request.content,
                mode: None,
                tokens_used: request.tokens_used,
                duration_ms: request.duration_ms,
                thinking_time_ms: request.thinking_time_ms,
                context,
                is_error: request.is_error,
                llm_provider: None,
                llm_model: None,
                finish_reason: None,
            },
        )
        .await?;

    Ok(Json(message.into()))
}

/// Set thumbs feedback on a message.
#[utoipa::path(
    patch,
    path = "/api/v1/conversations/{conversation_id}/messages/{message_id}/feedback",
    params(
        ("conversation_id" = Uuid, Path, description = "Conversation ID"),
        ("message_id" = Uuid, Path, description = "Message ID"),
    ),
    request_body = SetMessageFeedbackApiRequest,
    responses(
        (status = 200, description = "Feedback updated", body = MessageResponse),
        (status = 404, description = "Message not found"),
    ),
    tags = ["conversations"]
)]
pub async fn set_message_feedback(
    State(state): State<AppState>,
    _tenant_ctx: TenantContext,
    Path((conversation_id, message_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<SetMessageFeedbackApiRequest>,
) -> ApiResult<Json<MessageResponse>> {
    let rating = validate_feedback_rating(request.rating)?;
    let message = state
        .conversation_service
        .set_message_feedback(
            conversation_id,
            message_id,
            SetMessageFeedbackRequest {
                rating,
                reason: request.reason,
            },
        )
        .await?;

    Ok(Json(message.into()))
}

/// Delete a message.
#[utoipa::path(
    delete,
    path = "/api/v1/messages/{message_id}",
    params(
        ("message_id" = Uuid, Path, description = "Message ID")
    ),
    responses(
        (status = 204, description = "Message deleted"),
    ),
    tags = ["conversations"]
)]
pub async fn delete_message(
    State(state): State<AppState>,
    _tenant_ctx: TenantContext,
    Path(message_id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    // WHY: From<edgequake_core::Error> for ApiError maps NotFound → 404,
    // Validation → 422, Llm → 502, etc. The handler just propagates with `?`.
    state
        .conversation_service
        .delete_message(message_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
