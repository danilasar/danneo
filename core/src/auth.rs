pub use danneo_sdk::auth::{AuthService, Claims};

#[derive(serde::Deserialize)]
pub struct LoginRequest {
    pub login: String,
    pub password: String,
}

#[derive(serde::Serialize)]
pub struct LoginResponse {
    pub token: String,
}

use crate::models::core_admins;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
pub use danneo_sdk::state::AppState;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::sync::Arc;

pub async fn admin_login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // 1. Ищем админа по логину
    let admin = core_admins::Entity::find()
        .filter(core_admins::Column::Login.eq(&payload.login))
        .one(state.db.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let admin = admin.ok_or(StatusCode::UNAUTHORIZED)?;

    // 2. Проверяем хеш пароля
    let is_valid = bcrypt::verify(&payload.password, &admin.password_hash).unwrap_or(false);

    if !is_valid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // 3. Генерируем токен
    let auth_service = AuthService::new(state.jwt_secret.clone());
    // Даем токен на 24 часа
    let now = chrono::Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + chrono::Duration::hours(24)).timestamp() as usize;

    let token = auth_service
        .create_token(admin.id, exp, iat)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse { token }))
}

// Обработчик для страницы входа
pub async fn show_login_page(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let settings = state.settings.read().await;
    let mut context = tera::Context::new();
    context.insert("site_name", &settings.site_name);

    match state.tera.render("apanel/login.html", &context) {
        Ok(html) => axum::response::Html(html),
        Err(e) => {
            tracing::error!("Template error: {}", e);
            axum::response::Html("<h1>Internal Server Error</h1>".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_creation_and_verification() {
        let auth_service = AuthService::new("super_secret_key".to_string());

        let admin_id = 42;
        // Токен живет до timestamp 10000000000 (далекое будущее)
        let now = 1000000000;
        let token_res = auth_service.create_token(admin_id, 10000000000, now);
        assert!(token_res.is_ok(), "Token should be created successfully");

        let token = token_res.unwrap();

        let claims_res = auth_service.verify_token(&token);
        assert!(claims_res.is_ok(), "Token should be verified successfully");

        let claims = claims_res.unwrap();
        assert_eq!(claims.admin_id, admin_id);
        assert_eq!(claims.iat, now);
        assert!(!claims.jti.is_empty());
    }

    #[test]
    fn test_jwt_invalid_token() {
        let auth_service = AuthService::new("super_secret_key".to_string());
        let res = auth_service.verify_token("invalid.token.string");
        assert!(res.is_err(), "Invalid token should return an error");
    }

    #[tokio::test]
    async fn test_admin_login_success() {
        use crate::models::core_admins;
        use crate::state::init_state;
        use sea_orm::{ActiveModelTrait, ConnectionTrait, Database, Set, Statement};

        let db = Database::connect("sqlite::memory:").await.unwrap();
        // В тестах теперь используем init_state, так как AppState::new больше нет
        let state = init_state(db).await.unwrap();

        // Debug: check tables
        let tables = state
            .db
            .query_all(Statement::from_string(
                state.db.get_database_backend(),
                "SELECT name FROM sqlite_master WHERE type='table'",
            ))
            .await
            .unwrap();
        let table_names: Vec<String> = tables
            .into_iter()
            .map(|r| r.try_get("", "name").unwrap())
            .collect();
        eprintln!("Test: existing tables: {:?}", table_names);

        let password = "my_secure_password";
        let password_hash = bcrypt::hash(password, 4).unwrap();

        core_admins::ActiveModel {
            login: Set("test_admin".to_string()),
            password_hash: Set(password_hash),
            ..Default::default()
        }
        .insert(state.db.as_ref())
        .await
        .unwrap();

        let payload = Json(LoginRequest {
            login: "test_admin".to_string(),
            password: password.to_string(),
        });

        let response = admin_login(State(state), payload).await;
        assert!(
            response.is_ok(),
            "Admin login should succeed with correct credentials"
        );
    }
}
