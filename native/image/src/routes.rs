use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use danneo_sdk::{models::core_images, rpc::RpcContext, state::AppState};
use sea_orm::EntityTrait;
use serde_json::json;
use std::sync::Arc;

pub async fn serve_image(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let img_opt = core_images::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await
        .unwrap_or(None);

    let Some(img) = img_opt else {
        return (StatusCode::NOT_FOUND, "Image not found").into_response();
    };

    let res = state
        .rpc_registry
        .call(
            "storage",
            "get_url",
            json!({"path": img.original_path}),
            RpcContext::default(),
            state.clone(),
        )
        .await;
    if let Ok(v) = res {
        if let Some(url) = v.get("url").and_then(|u| u.as_str()) {
            return Redirect::temporary(url).into_response();
        }
    }
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to get storage URL",
    )
        .into_response()
}

pub async fn serve_thumb(
    Path((id, size)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let img_opt = core_images::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await
        .unwrap_or(None);

    if let Some(img) = img_opt {
        if let Some(thumb_path) = img.thumbnails.get(&size).and_then(|p| p.as_str()) {
            let res = state
                .rpc_registry
                .call(
                    "storage",
                    "get_url",
                    json!({"path": thumb_path}),
                    RpcContext::default(),
                    state.clone(),
                )
                .await;
            if let Ok(v) = res {
                if let Some(url) = v.get("url").and_then(|u| u.as_str()) {
                    return Redirect::temporary(url).into_response();
                }
            }
        }
    }
    (StatusCode::NOT_FOUND, "Thumb not found").into_response()
}
