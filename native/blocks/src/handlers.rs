use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use danneo_sdk::{
    apanel::render_admin_template,
    auth::Claims,
    models::{core_block_definitions, core_block_posit, core_blocks},
    state::AppState,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::Deserialize;
use std::sync::Arc;
use tera::Context;

#[derive(Deserialize)]
pub struct PositionForm {
    #[serde(default)]
    pub id: Option<i32>,
    pub positname: String,
    pub positcode: String,
    pub pposit: i32,
}

pub async fn list_positions(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let db = state.db.as_ref();

    let positions = match core_block_posit::Entity::find()
        .order_by_asc(core_block_posit::Column::Pposit)
        .all(db)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to fetch positions: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let mut context = Context::new();
    context.insert("positions", &positions);

    render_admin_template(state, "apanel/blocks_positions.html", context).await
}

pub async fn save_position(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    Form(form): Form<PositionForm>,
) -> impl IntoResponse {
    let db = state.db.as_ref();

    let mut active_model = core_block_posit::ActiveModel {
        positname: Set(form.positname),
        positcode: Set(form.positcode),
        pposit: Set(form.pposit),
        ..Default::default()
    };

    if let Some(id) = form.id {
        active_model.id = Set(id);
    }

    match active_model.save(db).await {
        Ok(_) => Redirect::to("/admin/blocks/positions").into_response(),
        Err(e) => {
            tracing::error!("Failed to save position: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn list_blocks(_claims: Claims, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let db = state.db.as_ref();

    let positions = match core_block_posit::Entity::find()
        .order_by_asc(core_block_posit::Column::Pposit)
        .all(db)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to fetch positions: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let blocks = match core_blocks::Entity::find()
        .order_by_asc(core_blocks::Column::BlockWeight)
        .all(db)
        .await
    {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to fetch blocks: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let mut context = Context::new();
    context.insert("positions", &positions);
    context.insert("blocks", &blocks);

    render_admin_template(state, "apanel/blocks_list.html", context).await
}

pub async fn edit_block(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let db = state.db.as_ref();
    let id = params.get("id").and_then(|id| id.parse::<i32>().ok());

    let block = if let Some(id) = id {
        match core_blocks::Entity::find_by_id(id).one(db).await {
            Ok(Some(b)) => Some(b),
            _ => None,
        }
    } else {
        None
    };

    let positions = match core_block_posit::Entity::find().all(db).await {
        Ok(p) => p,
        _ => vec![],
    };

    let block_definitions = match core_block_definitions::Entity::find()
        .filter(core_block_definitions::Column::Enabled.eq(true))
        .all(db)
        .await
    {
        Ok(defs) => defs,
        _ => vec![],
    };

    let mut context = Context::new();
    context.insert("block", &block);
    context.insert("positions", &positions);
    context.insert("block_definitions", &block_definitions);

    render_admin_template(state, "apanel/block_edit.html", context).await
}

#[derive(Deserialize)]
pub struct BlockForm {
    #[serde(default)]
    pub id: Option<i32>,
    pub positcode: String,
    pub block_name: String,
    pub block_file: String,
    pub block_active: Option<String>,
    pub block_weight: i32,
    pub block_setting: String,
}

pub async fn save_block(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    Form(form): Form<BlockForm>,
) -> impl IntoResponse {
    let db = state.db.as_ref();

    let setting_json: Option<serde_json::Value> = serde_json::from_str(&form.block_setting).ok();

    let mut active_model = core_blocks::ActiveModel {
        positcode: Set(form.positcode),
        block_name: Set(form.block_name),
        block_file: Set(form.block_file),
        block_active: Set(form.block_active.is_some()),
        block_weight: Set(form.block_weight),
        block_setting: Set(setting_json),
        block_access: Set("all".to_string()),
        ..Default::default()
    };

    if let Some(id) = form.id {
        active_model.id = Set(id);
    }

    match active_model.save(db).await {
        Ok(_) => Redirect::to("/admin/blocks").into_response(),
        Err(e) => {
            tracing::error!("Failed to save block: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_position(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let db = state.db.as_ref();
    let id = params.get("id").and_then(|id| id.parse::<i32>().ok());

    if let Some(id) = id {
        let _ = core_block_posit::Entity::delete_by_id(id).exec(db).await;
    }

    Redirect::to("/admin/blocks/positions").into_response()
}

pub async fn delete_block(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let db = state.db.as_ref();
    let id = params.get("id").and_then(|id| id.parse::<i32>().ok());

    if let Some(id) = id {
        let _ = core_blocks::Entity::delete_by_id(id).exec(db).await;
    }

    Redirect::to("/admin/blocks").into_response()
}
