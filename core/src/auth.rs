use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};

/// Полезная нагрузка токена (то, что будет храниться внутри JWT)
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Claims {
    pub admin_id: i32,
    pub exp: usize, // Timestamp окончания жизни токена
}

/// Сервис для работы с JWT
#[derive(Clone)]
pub struct AuthService {
    secret: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub login: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}

use axum::{
    extract::{State, Json},
    response::IntoResponse,
};
use std::sync::Arc;
use crate::state::AppState;
use crate::models::core_admins;
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};

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
    let is_valid = bcrypt::verify(&payload.password, &admin.password_hash)
        .unwrap_or(false);

    if !is_valid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // 3. Генерируем токен
    let auth_service = AuthService::new(state.jwt_secret.clone());
    // Даем токен на 24 часа
    let exp = (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize;
    
    let token = auth_service.create_token(admin.id, exp)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse { token }))
}

impl AuthService {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }

    /// Генерация токена для администратора
    pub fn create_token(&self, admin_id: i32, exp: usize) -> jsonwebtoken::errors::Result<String> {
        let claims = Claims { admin_id, exp };
        encode(&Header::default(), &claims, &EncodingKey::from_secret(self.secret.as_ref()))
    }

    /// Проверка валидности токена и извлечение данных
    pub fn verify_token(&self, token: &str) -> jsonwebtoken::errors::Result<Claims> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &Validation::default(),
        )?;
        Ok(token_data.claims)
    }
}

use axum::{
    async_trait,
    extract::{FromRequestParts, FromRef},
    http::{request::Parts, StatusCode},
};

// Обработчик для страницы входа
pub async fn show_login_page(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
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

// Для извлечения Claims из заголовка Authorization или Cookies
#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    Arc<AppState>: axum::extract::FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> std::result::Result<Self, Self::Rejection> {
        let state = Arc::<AppState>::from_ref(state);
        
        // 1. Пытаемся взять из заголовка Authorization
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|header| header.to_str().ok());

        let mut token = None;

        if let Some(auth_header) = auth_header {
            if auth_header.starts_with("Bearer ") {
                token = Some(auth_header[7..].to_string());
            }
        }

        // 2. Если в заголовке нет, ищем в Cookies
        if token.is_none() {
            if let Some(cookie_header) = parts.headers.get("Cookie").and_then(|h| h.to_str().ok()) {
                for cookie in cookie_header.split(';') {
                    let cookie = cookie.trim();
                    if cookie.starts_with("danneo_token=") {
                        token = Some(cookie["danneo_token=".len()..].to_string());
                        break;
                    }
                }
            }
        }

        if let Some(token) = token {
            let auth_service = AuthService::new(state.jwt_secret.clone());
            return auth_service.verify_token(&token).map_err(|_| {
                (StatusCode::UNAUTHORIZED, "Invalid or expired token")
            });
        }

        Err((StatusCode::UNAUTHORIZED, "Missing Authorization header or cookie"))
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
        let token_res = auth_service.create_token(admin_id, 10000000000);
        assert!(token_res.is_ok(), "Token should be created successfully");
        
        let token = token_res.unwrap();
        
        let claims_res = auth_service.verify_token(&token);
        assert!(claims_res.is_ok(), "Token should be verified successfully");
        
        let claims = claims_res.unwrap();
        assert_eq!(claims.admin_id, admin_id);
    }

    #[test]
    fn test_jwt_invalid_token() {
        let auth_service = AuthService::new("super_secret_key".to_string());
        let res = auth_service.verify_token("invalid.token.string");
        assert!(res.is_err(), "Invalid token should return an error");
    }

    #[tokio::test]
    async fn test_admin_login_success() {
        use sea_orm::{DatabaseBackend, MockDatabase, Value};
        use crate::models::core_admins;
        use std::collections::BTreeMap;
        
        let password = "my_secure_password";
        let password_hash = bcrypt::hash(password, 4).unwrap();

        let mut row = BTreeMap::new();
        row.insert("id".to_string(), Value::Int(Some(1)));
        row.insert("login".to_string(), Value::String(Some(Box::new("admin".to_string()))));
        row.insert("password_hash".to_string(), Value::String(Some(Box::new(password_hash.clone()))));
        row.insert("email".to_string(), Value::String(Some(Box::new("admin@test.com".to_string()))));
        row.insert("permissions".to_string(), Value::Json(Some(Box::new(serde_json::json!(["all"])))));

        // Мокаем базу данных для возврата админа
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // Возвращаем пустые настройки для AppState::new
            .append_query_results([Vec::<BTreeMap<String, Value>>::new()])
            // Возвращаем админа при запросе admin_login
            .append_query_results([vec![row]])
            .into_connection();

        let state = Arc::new(AppState::new(db).await.unwrap());
        
        let payload = Json(LoginRequest {
            login: "admin".to_string(),
            password: password.to_string(),
        });

        let response = admin_login(State(state), payload).await;
        assert!(response.is_ok(), "Admin login should succeed with correct credentials");
    }
}
