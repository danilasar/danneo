use crate::{
    auth::Claims,
    models::{core_block_posit, core_blocks},
    state::AppState,
};
use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::Deserialize;
use std::sync::Arc;
use tera::Context;

#[derive(Deserialize)]
pub struct PositionForm {
    #[serde(
        default,
        deserialize_with = "crate::apanel::utils::empty_string_as_none"
    )]
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
    let settings = state.settings.read().await;
    context.insert("site_name", &settings.site_name);
    context.insert("positions", &positions);

    match state.tera.render("apanel/blocks_positions.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Template rendering error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
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
    let settings = state.settings.read().await;
    context.insert("site_name", &settings.site_name);
    context.insert("positions", &positions);
    context.insert("blocks", &blocks);

    match state.tera.render("apanel/blocks_list.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Template rendering error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
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

    let positions = match crate::models::core_block_posit::Entity::find().all(db).await {
        Ok(p) => p,
        _ => vec![],
    };

    let block_definitions = match crate::models::core_block_definitions::Entity::find()
        .filter(crate::models::core_block_definitions::Column::Enabled.eq(true))
        .all(db)
        .await
    {
        Ok(defs) => defs,
        _ => vec![],
    };

    let mut context = Context::new();
    let settings = state.settings.read().await;
    context.insert("site_name", &settings.site_name);
    context.insert("block", &block);
    context.insert("positions", &positions);
    context.insert("block_definitions", &block_definitions);

    match state.tera.render("apanel/block_edit.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Template rendering error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct BlockForm {
    #[serde(
        default,
        deserialize_with = "crate::apanel::utils::empty_string_as_none"
    )]
    pub id: Option<i32>,
    pub positcode: String,
    pub block_name: String,
    pub block_file: String,
    pub block_active: Option<String>, // Checkbox sends "true" or nothing
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
        block_access: Set("all".to_string()), // Пока хардкодим
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthService;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use sea_orm::Database;
    use tower::ServiceExt;

    async fn setup_test_db() -> sea_orm::DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        use sea_orm_migration::MigratorTrait;
        migration::Migrator::up(&db, None).await.unwrap();
        db
    }

    #[tokio::test]
    async fn test_list_positions() {
        let db = setup_test_db().await;
        let state = Arc::new(AppState::new(db).await.unwrap());
        let jwt_secret = state.jwt_secret.clone();

        let app = axum::Router::new()
            .route(
                "/admin/blocks/positions",
                axum::routing::get(list_positions),
            )
            .with_state(state);

        let auth_service = AuthService::new(jwt_secret);
        let token = auth_service.create_token(1, 9999999999).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/admin/blocks/positions")
                    .header("Cookie", format!("danneo_token={}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_save_position() {
        let db = setup_test_db().await;
        let state = Arc::new(AppState::new(db).await.unwrap());
        let jwt_secret = state.jwt_secret.clone();

        let app = axum::Router::new()
            .route(
                "/admin/blocks/positions/save",
                axum::routing::post(save_position),
            )
            .with_state(state);

        let auth_service = AuthService::new(jwt_secret);
        let token = auth_service.create_token(1, 9999999999).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/admin/blocks/positions/save")
                    .header("Cookie", format!("danneo_token={}", token))
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(Body::from("positname=Right&positcode=RIGHT&pposit=2"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get("location").unwrap(),
            "/admin/blocks/positions"
        );
    }

    #[tokio::test]
    async fn test_list_blocks() {
        let db = setup_test_db().await;
        let state = Arc::new(AppState::new(db).await.unwrap());
        let jwt_secret = state.jwt_secret.clone();

        let app = axum::Router::new()
            .route("/admin/blocks", axum::routing::get(list_blocks))
            .with_state(state);

        let auth_service = AuthService::new(jwt_secret);
        let token = auth_service.create_token(1, 9999999999).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/admin/blocks")
                    .header("Cookie", format!("danneo_token={}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
