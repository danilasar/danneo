use axum::{
    extract::{State, Form},
    response::{Html, IntoResponse, Redirect},
    http::StatusCode,
};
use std::sync::Arc;
use crate::{state::AppState, auth::Claims, models::{core_blocks, core_block_posit}};
use tera::Context;
use serde::Deserialize;
use sea_orm::{EntityTrait, Set, ActiveModelTrait, QueryOrder};

#[derive(Deserialize)]
pub struct PositionForm {
    #[serde(default, deserialize_with = "crate::apanel::utils::empty_string_as_none")]
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
        .await {
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

pub async fn list_blocks(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let db = state.db.as_ref();
    
    let positions = match core_block_posit::Entity::find()
        .order_by_asc(core_block_posit::Column::Pposit)
        .all(db)
        .await {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Failed to fetch positions: {}", e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

    let blocks = match core_blocks::Entity::find()
        .order_by_asc(core_blocks::Column::BlockWeight)
        .all(db)
        .await {
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

    let positions = match core_block_posit::Entity::find().all(db).await {
        Ok(p) => p,
        _ => vec![],
    };

    let block_files = vec!["b-News", "b-Sample", "b-Auth", "b-Menu"];

    let mut context = Context::new();
    let settings = state.settings.read().await;
    context.insert("site_name", &settings.site_name);
    context.insert("block", &block);
    context.insert("positions", &positions);
    context.insert("block_files", &block_files);

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
    #[serde(default, deserialize_with = "crate::apanel::utils::empty_string_as_none")]
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
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use sea_orm::{DatabaseBackend, MockDatabase};
    use crate::auth::AuthService;
    use crate::models::core_settings;
    use serde_json::json;

    #[tokio::test]
    async fn test_list_positions() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([
                vec![
                    core_settings::Model { key: "site_name".to_string(), value: json!("Danneo Test") },
                    core_settings::Model { key: "admin_email".to_string(), value: json!("admin@test.com") },
                    core_settings::Model { key: "site_url".to_string(), value: json!("http://localhost") },
                    core_settings::Model { key: "site_temp".to_string(), value: json!("Soft") },
                ]
            ])
            .append_query_results([
                vec![
                    core_block_posit::Model { id: 1, positname: "Left".to_string(), positcode: "LEFT".to_string(), pposit: 1 },
                ]
            ])
            .into_connection();
        let state = Arc::new(AppState::new(db).await.unwrap());
        let jwt_secret = state.jwt_secret.clone();
        
        let app = axum::Router::new()
            .route("/admin/blocks/positions", axum::routing::get(list_positions))
            .with_state(state);

        let auth_service = AuthService::new(jwt_secret);
        let token = auth_service.create_token(1, 9999999999).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/admin/blocks/positions")
                    .header("Cookie", format!("danneo_token={}", token))
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_save_position() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([
                vec![
                    core_settings::Model { key: "site_name".to_string(), value: json!("Danneo Test") },
                    core_settings::Model { key: "admin_email".to_string(), value: json!("admin@test.com") },
                    core_settings::Model { key: "site_url".to_string(), value: json!("http://localhost") },
                    core_settings::Model { key: "site_temp".to_string(), value: json!("Soft") },
                ]
            ])
            .append_query_results([
                vec![
                    core_block_posit::Model { id: 1, positname: "Left".to_string(), positcode: "LEFT".to_string(), pposit: 1 },
                ]
            ])
            .into_connection();
        let state = Arc::new(AppState::new(db).await.unwrap());
        let jwt_secret = state.jwt_secret.clone();
        
        let app = axum::Router::new()
            .route("/admin/blocks/positions/save", axum::routing::post(save_position))
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
                    .unwrap()
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers().get("location").unwrap(), "/admin/blocks/positions");
    }

    #[tokio::test]
    async fn test_list_blocks() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([
                vec![
                    core_settings::Model { key: "site_name".to_string(), value: json!("Danneo Test") },
                    core_settings::Model { key: "admin_email".to_string(), value: json!("admin@test.com") },
                    core_settings::Model { key: "site_url".to_string(), value: json!("http://localhost") },
                    core_settings::Model { key: "site_temp".to_string(), value: json!("Soft") },
                ]
            ])
            .append_query_results([
                vec![
                    core_block_posit::Model { id: 1, positname: "Left".to_string(), positcode: "LEFT".to_string(), pposit: 1 },
                ]
            ])
            .append_query_results([
                vec![
                    core_blocks::Model { 
                        id: 1, 
                        positcode: "LEFT".to_string(), 
                        block_name: "News".to_string(), 
                        block_file: "b-News".to_string(), 
                        block_active: true, 
                        block_weight: 1,
                        block_temp: None,
                        block_mods: None,
                        block_access: "all".to_string(),
                        block_setting: None,
                    },
                ]
            ])
            .into_connection();
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
                    .unwrap()
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
