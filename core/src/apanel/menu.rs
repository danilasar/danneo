use crate::{
    auth::Claims,
    models::{core_menu_groups, core_menu_items},
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
pub struct GroupForm {
    #[serde(
        default,
        deserialize_with = "crate::apanel::utils::empty_string_as_none"
    )]
    pub id: Option<i32>,
    pub code: String,
    pub title: String,
}

pub async fn list_groups(_claims: Claims, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let db = state.db.as_ref();

    let groups = match core_menu_groups::Entity::find()
        .order_by_asc(core_menu_groups::Column::Id)
        .all(db)
        .await
    {
        Ok(g) => g,
        Err(e) => {
            tracing::error!("Failed to fetch menu groups: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let mut context = Context::new();
    crate::apanel::prepare_admin_context(state.clone(), &mut context).await;
    context.insert("groups", &groups);

    match state.tera.render("apanel/menu_list.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Template rendering error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn save_group(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    Form(form): Form<GroupForm>,
) -> impl IntoResponse {
    let db = state.db.as_ref();

    let mut active_model = core_menu_groups::ActiveModel {
        code: Set(form.code),
        title: Set(form.title),
        ..Default::default()
    };

    if let Some(id) = form.id {
        active_model.id = Set(id);
    }

    match active_model.save(db).await {
        Ok(_) => Redirect::to("/admin/menu").into_response(),
        Err(e) => {
            tracing::error!("Failed to save menu group: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn list_items(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let db = state.db.as_ref();
    let group_id = params
        .get("group_id")
        .and_then(|id| id.parse::<i32>().ok())
        .unwrap_or(1);

    let group = match core_menu_groups::Entity::find_by_id(group_id).one(db).await {
        Ok(Some(g)) => g,
        _ => return StatusCode::NOT_FOUND.into_response(),
    };

    let items = match core_menu_items::Entity::find()
        .filter(core_menu_items::Column::GroupId.eq(group_id))
        .order_by_asc(core_menu_items::Column::Posit)
        .all(db)
        .await
    {
        Ok(i) => i,
        Err(e) => {
            tracing::error!("Failed to fetch menu items: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let mut context = Context::new();
    crate::apanel::prepare_admin_context(state.clone(), &mut context).await;
    context.insert("group", &group);
    context.insert("items", &items);

    match state.tera.render("apanel/menu_items.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Template rendering error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct ItemForm {
    #[serde(
        default,
        deserialize_with = "crate::apanel::utils::empty_string_as_none"
    )]
    pub id: Option<i32>,
    pub group_id: i32,
    pub parent_id: i32,
    pub title: String,
    pub link: String,
    pub target: String,
    pub css: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub posit: i32,
    pub acc: String,
}

pub async fn save_item(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    Form(form): Form<ItemForm>,
) -> impl IntoResponse {
    let db = state.db.as_ref();

    let mut active_model = core_menu_items::ActiveModel {
        group_id: Set(form.group_id),
        parent_id: Set(form.parent_id),
        title: Set(form.title),
        link: Set(form.link),
        target: Set(form.target),
        css: Set(form.css),
        before: Set(form.before),
        after: Set(form.after),
        posit: Set(form.posit),
        acc: Set(form.acc),
        ..Default::default()
    };

    if let Some(id) = form.id {
        active_model.id = Set(id);
    }

    let redirect_url = format!("/admin/menu/items?group_id={}", form.group_id);

    match active_model.save(db).await {
        Ok(_) => Redirect::to(&redirect_url).into_response(),
        Err(e) => {
            tracing::error!("Failed to save menu item: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_group(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let db = state.db.as_ref();
    let id = params.get("id").and_then(|id| id.parse::<i32>().ok());

    if let Some(id) = id {
        // Каскадное удаление пунктов должно сработать на уровне БД, но для надежности можно и здесь
        let _ = core_menu_groups::Entity::delete_by_id(id).exec(db).await;
    }

    Redirect::to("/admin/menu").into_response()
}

pub async fn delete_item(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let db = state.db.as_ref();
    let id = params.get("id").and_then(|id| id.parse::<i32>().ok());
    let group_id = params
        .get("group_id")
        .cloned()
        .unwrap_or_else(|| "1".to_string());

    if let Some(id) = id {
        let _ = core_menu_items::Entity::delete_by_id(id).exec(db).await;
    }

    Redirect::to(&format!("/admin/menu/items?group_id={}", group_id)).into_response()
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
    async fn test_list_groups() {
        let db = setup_test_db().await;
        let state = Arc::new(AppState::new(db).await.unwrap());
        let jwt_secret = state.jwt_secret.clone();

        let app = axum::Router::new()
            .route("/admin/menu", axum::routing::get(list_groups))
            .with_state(state);

        let auth_service = AuthService::new(jwt_secret);
        let token = auth_service.create_token(1, 9999999999, 1000000000).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/admin/menu")
                    .header("Cookie", format!("danneo_token={}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_list_items() {
        let db = setup_test_db().await;
        let state = Arc::new(AppState::new(db).await.unwrap());
        let jwt_secret = state.jwt_secret.clone();

        let app = axum::Router::new()
            .route("/admin/menu/items", axum::routing::get(list_items))
            .with_state(state);

        let auth_service = AuthService::new(jwt_secret);
        let token = auth_service.create_token(1, 9999999999, 1000000000).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/admin/menu/items?group_id=1")
                    .header("Cookie", format!("danneo_token={}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
