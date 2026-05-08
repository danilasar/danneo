use axum::{
    extract::{State, Request},
    middleware::Next,
    response::IntoResponse,
    http::StatusCode,
};
use std::sync::Arc;
use crate::state::AppState;
use crate::auth::Claims;
use crate::models::core_admins;
use sea_orm::EntityTrait;

pub async fn admin_acl_middleware(
    claims: Claims,
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, StatusCode> {
    let db = state.db.as_ref();

    // 1. СуперАдмин (ID=1) безусловно имеет доступ ко всему (MAC уровень 100)
    if claims.admin_id == 1 {
        return Ok(next.run(request).await);
    }

    // 2. Получаем админа из базы для получения логина и уровня доступа
    let admin = core_admins::Entity::find_by_id(claims.admin_id)
        .one(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // 3. Определяем запрашиваемый модуль из пути
    let path = request.uri().path();
    // Формат: /admin/module_name/...
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    
    if parts.len() >= 2 && parts[0] == "admin" {
        let module = parts[1];
        
        // Проверяем права через Casbin
        // sub: login, obj: module, act: view (базовый доступ к модулю), level: admin.level
        let has_access = state.acl.enforce(&admin.login, module, "view", admin.level).await;

        if has_access {
            return Ok(next.run(request).await);
        }

        tracing::warn!("Access denied for admin {} to module {}", admin.login, module);
        return Err(StatusCode::FORBIDDEN);
    }

    // Если путь не соответствует формату /admin/module, но мы в админке (например /admin/dashboard)
    Ok(next.run(request).await)
}
