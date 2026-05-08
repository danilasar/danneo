use axum::{
    extract::{State, Form},
    response::{Html, IntoResponse, Redirect},
    http::StatusCode,
};
use std::sync::Arc;
use crate::{state::AppState, auth::Claims, models::core_admins};
use tera::Context;
use serde::Deserialize;
use sea_orm::{EntityTrait, Set, ActiveModelTrait, QueryOrder};

#[derive(Deserialize)]
pub struct AdminForm {
    #[serde(default, deserialize_with = "crate::apanel::utils::empty_string_as_none")]
    pub id: Option<i32>,
    pub login: String,
    pub email: String,
    pub password: Option<String>,
    pub permissions: Vec<String>,
}

pub async fn list_admins(
    _claims: Claims,
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

pub async fn edit_admin(
    _claims: Claims,
    State(_state): State<Arc<AppState>>,
    axum::extract::Query(_params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

pub async fn save_admin(
    _claims: Claims,
    State(_state): State<Arc<AppState>>,
    Form(_form): Form<AdminForm>,
) -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

pub async fn delete_admin(
    _claims: Claims,
    State(_state): State<Arc<AppState>>,
    axum::extract::Query(_params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
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
    use serde_json::json;

    #[tokio::test]
    async fn test_list_admins_page() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([
                vec![
                    core_admins::Model { 
                        id: 1, 
                        login: "admin".to_string(), 
                        password_hash: "hash".to_string(),
                        email: Some("admin@test.com".to_string()),
                        permissions: json!(["all"]),
                    },
                ]
            ])
            .into_connection();
        let state = Arc::new(AppState::new(db).await.unwrap());
        let jwt_secret = state.jwt_secret.clone();
        
        let app = axum::Router::new()
            .route("/admin/amanage", axum::routing::get(list_admins))
            .with_state(state);

        let auth_service = AuthService::new(jwt_secret);
        let token = auth_service.create_token(1, 9999999999).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/admin/amanage")
                    .header("Cookie", format!("danneo_token={}", token))
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_save_admin_logic() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([
                // Для загрузки настроек в AppState::new
                vec![],
            ])
            .append_exec_results([
                sea_orm::MockExecResult { last_insert_id: 0, rows_affected: 1 },
            ])
            .into_connection();
        let state = Arc::new(AppState::new(db).await.unwrap());
        let jwt_secret = state.jwt_secret.clone();
        
        let app = axum::Router::new()
            .route("/admin/amanage/save", axum::routing::post(save_admin))
            .with_state(state);

        let auth_service = AuthService::new(jwt_secret);
        let token = auth_service.create_token(1, 9999999999).unwrap();

        let form_data = "login=newadmin&email=new@test.com&password=secret&permissions=news&permissions=settings";
        
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/admin/amanage/save")
                    .header("Cookie", format!("danneo_token={}", token))
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(Body::from(form_data))
                    .unwrap()
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
    }
}
