use crate::state::AppState;
use async_trait::async_trait;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Полезная нагрузка токена (то, что будет храниться внутри JWT)
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Claims {
    pub admin_id: i32,
    pub exp: usize,  // Timestamp окончания жизни токена
    pub iat: usize,  // Timestamp выпуска токена
    pub jti: String, // Уникальный идентификатор токена (JWT ID)
}

use jsonwebtoken::{EncodingKey, Header, encode};
use uuid::Uuid;

/// Сервис для работы с JWT
#[derive(Clone)]
pub struct AuthService {
    secret: String,
}

impl AuthService {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }

    /// Генерация токена для администратора
    pub fn create_token(
        &self,
        admin_id: i32,
        exp: usize,
        iat: usize,
    ) -> jsonwebtoken::errors::Result<String> {
        let jti = Uuid::new_v4().to_string();
        let claims = Claims {
            admin_id,
            exp,
            iat,
            jti,
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )
    }

    /// Проверка валидности токена и извлечение данных
    pub fn verify_token(&self, token: &str) -> jsonwebtoken::errors::Result<Claims> {
        let token_data = jsonwebtoken::decode::<Claims>(
            token,
            &jsonwebtoken::DecodingKey::from_secret(self.secret.as_ref()),
            &Validation::default(),
        )?;
        Ok(token_data.claims)
    }
}

// Для извлечения Claims из заголовка Authorization или Cookies
#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    Arc<AppState>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
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
            let token_data = decode::<Claims>(
                &token,
                &DecodingKey::from_secret(state.jwt_secret.as_ref()),
                &Validation::default(),
            )
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired token"))?;

            return Ok(token_data.claims);
        }

        Err((
            StatusCode::UNAUTHORIZED,
            "Missing Authorization header or cookie",
        ))
    }
}
