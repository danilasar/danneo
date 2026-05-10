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

    let path = request.uri().path();
    let module_code = get_module_from_path(&state, path).await;

    if let Some(module) = module_code {
        if matches!(module.as_str(), "dashboard" | "modules" | "login") {
            return Ok(next.run(request).await);
        }

        let has_access = state
            .acl
            .enforce(&admin.login, &module, "view", admin.level)
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
    let path = request.uri().path();
    let module_code = get_module_from_path(&state, path).await;

    if let Some(module) = module_code {
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

async fn get_module_from_path(state: &AppState, path: &str) -> Option<String> {
    // 1. Remove /admin prefix if present
    let clean_path = if path.starts_with("/admin") {
        &path[6..]
    } else {
        path
    };
    
    let parts: Vec<&str> = clean_path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() { 
        tracing::debug!("get_module_from_path: empty path parts for {}", path);
        return None; 
    }

    // 2. Check technical routes /admin/m/:module/...
    if parts[0] == "m" && parts.len() >= 2 {
        let m = parts[1].to_string();
        tracing::debug!("get_module_from_path: technical route for module {}", m);
        return Some(m);
    }

    // 3. Check for match in RouteRegistry
    {
        let routes = state.routes.read().await;
        // Normalize clean_path to always start with /
        let normalized_path = if clean_path.starts_with('/') { clean_path.to_string() } else { format!("/{}", clean_path) };
        
        for (module_code, descriptor) in &routes.admin_routes {
            if descriptor.path == normalized_path || descriptor.path == clean_path {
                tracing::debug!("get_module_from_path: matched route {} to module {}", normalized_path, module_code);
                return Some(module_code.clone());
            }
        }
    }

    // 4. Default: first segment is module ID (for native modules or simple paths)
    let m = parts[0].to_string();
    tracing::debug!("get_module_from_path: default to first segment {} for path {}", m, path);
    Some(m)
}
