use axum::{
    extract::{State, Form},
    response::{Html, IntoResponse, Redirect},
    http::StatusCode,
};
use std::sync::Arc;
use crate::{state::AppState, auth::Claims, models::core_settings};
use tera::Context;
use serde::Deserialize;
use sea_orm::{EntityTrait, Set, ActiveModelTrait};

#[derive(Deserialize)]
pub struct SettingsForm {
    pub site_name: String,
    pub admin_email: String,
    pub site_url: String,
    pub site_temp: String,
}

pub async fn show_settings(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut context = Context::new();
    let settings = state.settings.read().await;
    context.insert("site_name", &settings.site_name);
    context.insert("admin_email", &settings.admin_email);
    context.insert("site_url", &settings.site_url);
    context.insert("site_temp", &settings.site_temp);
    
    // Список доступных тем (в будущем сканировать папку)
    let themes = vec!["Soft", "Old", "Clear"]; 
    context.insert("themes", &themes);

    match state.tera.render("apanel/settings.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            eprintln!("Tera error: {:?}", e);
            tracing::error!("Template rendering error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn save_settings(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    Form(form): Form<SettingsForm>,
) -> impl IntoResponse {
    let db = state.db.as_ref();

    // Обновляем в БД
    let updates = vec![
        ("site_name", &form.site_name),
        ("admin_email", &form.admin_email),
        ("site_url", &form.site_url),
        ("site_temp", &form.site_temp),
    ];

    for (key, value) in updates {
        let active_model = core_settings::ActiveModel {
            key: Set(key.to_string()),
            value: Set(serde_json::json!(value)),
        };
        
        if let Err(e) = core_settings::Entity::insert(active_model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(core_settings::Column::Key)
                    .update_column(core_settings::Column::Value)
                    .to_owned()
            )
            .exec(db)
            .await {
            tracing::error!("Failed to save setting {}: {}", key, e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    // Обновляем в кэше AppState
    {
        let mut settings = state.settings.write().await;
        settings.site_name = form.site_name;
        settings.admin_email = form.admin_email;
        settings.site_url = form.site_url;
        settings.site_temp = form.site_temp;
    }

    Redirect::to("/admin/settings").into_response()
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
    use crate::models::core_settings;
    use serde_json::json;

    use crate::auth::AuthService;

    #[tokio::test]
    async fn test_show_settings_page() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([
                vec![
                    core_settings::Model { key: "site_name".to_string(), value: json!("Test Site") },
                    core_settings::Model { key: "admin_email".to_string(), value: json!("admin@test.com") },
                ]
            ])
            .into_connection();

        let state = Arc::new(AppState::new(db).await.unwrap());
        let jwt_secret = state.jwt_secret.clone();
        
        let app = axum::Router::new()
            .route("/admin/settings", axum::routing::get(show_settings))
            .with_state(state);

        // Создаем токен для теста
        let auth_service = AuthService::new(jwt_secret);
        let token = auth_service.create_token(1, 9999999999).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/admin/settings")
                    .header("Cookie", format!("danneo_token={}", token))
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_save_settings_redirect() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([
                vec![
                    core_settings::Model { key: "site_name".to_string(), value: json!("Test Site") },
                    core_settings::Model { key: "admin_email".to_string(), value: json!("admin@test.com") },
                    core_settings::Model { key: "site_url".to_string(), value: json!("http://localhost") },
                    core_settings::Model { key: "site_temp".to_string(), value: json!("Soft") },
                ]
            ])
            .append_exec_results([
                sea_orm::MockExecResult { last_insert_id: 0, rows_affected: 1 },
                sea_orm::MockExecResult { last_insert_id: 0, rows_affected: 1 },
                sea_orm::MockExecResult { last_insert_id: 0, rows_affected: 1 },
                sea_orm::MockExecResult { last_insert_id: 0, rows_affected: 1 },
            ])
            .into_connection();

        let state = Arc::new(AppState::new(db).await.unwrap());
        let jwt_secret = state.jwt_secret.clone();
        
        let app = axum::Router::new()
            .route("/admin/settings/save", axum::routing::post(save_settings))
            .with_state(state);

        let auth_service = AuthService::new(jwt_secret);
        let token = auth_service.create_token(1, 9999999999).unwrap();

        let form_data = "site_name=New+Name&admin_email=new@test.com&site_url=http://new.com&site_temp=Old";
        
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/admin/settings/save")
                    .header("Cookie", format!("danneo_token={}", token))
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(Body::from(form_data))
                    .unwrap()
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers().get("location").unwrap(), "/admin/settings");
    }
}
