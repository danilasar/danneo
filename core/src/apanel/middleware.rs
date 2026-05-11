use crate::auth::Claims;
use crate::models::core_admins;
use crate::state::AppState;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::IntoResponse,
};
use sea_orm::EntityTrait;
use std::sync::Arc;

pub async fn admin_acl_middleware(
    claims: Claims,
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, StatusCode> {
    let db = state.db.as_ref();

    if claims.admin_id == 1 {
        return Ok(next.run(request).await);
    }

    let admin = core_admins::Entity::find_by_id(claims.admin_id)
        .one(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let module_info = request.extensions().get::<crate::module::ModuleInfo>();

    if let Some(info) = module_info {
        let module = &info.code;
        if matches!(module.as_str(), "dashboard" | "modules" | "login") {
            return Ok(next.run(request).await);
        }

        let has_access = state
            .acl
            .enforce(&admin.login, module, "view", admin.level)
            .await;

        if has_access {
            return Ok(next.run(request).await);
        }

        tracing::warn!("Access denied for admin {} to module {}", admin.login, module);
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}

/// Middleware для проверки включен ли модуль в БД
pub async fn module_enabled_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, StatusCode> {
    let module_info = request.extensions().get::<crate::module::ModuleInfo>();

    if let Some(info) = module_info {
        let module = &info.code;
        if matches!(module.as_str(), "dashboard" | "modules" | "login" | "crud" | "menu_system") {
             return Ok(next.run(request).await);
        }

        use crate::models::core_modules;
        use sea_orm::{ColumnTrait, QueryFilter};
        let is_enabled = core_modules::Entity::find()
            .filter(core_modules::Column::Code.eq(module))
            .filter(core_modules::Column::Enabled.eq(true))
            .one(state.db.as_ref())
            .await
            .unwrap_or(None)
            .is_some();

        if !is_enabled {
            return Err(StatusCode::NOT_FOUND);
        }
    }

    Ok(next.run(request).await)
}
