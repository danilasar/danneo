pub mod agroups;
pub mod amanage;
pub mod blocks;
pub mod crud;
pub mod dashboard;
pub mod design;
pub mod menu;
pub mod middleware;
pub mod modules;
pub mod seo;
pub mod settings;
pub mod utils;

use crate::state::AppState;
use std::sync::Arc;
use tera::Context;

use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};

pub async fn resolve_module_template(
    state: &AppState,
    module_code: &str,
    template_name: &str,
) -> String {
    let settings = state.settings.read().await;
    let theme = &settings.site_temp;

    // 1. Пытаемся найти шаблон в текущей теме
    let themed_path = format!("{}/{}/{}", module_code, theme, template_name);
    if state.tera.get_template_names().any(|n| n == themed_path) {
        return themed_path;
    }

    // 2. Fallback на 'default' тему
    format!("{}/default/{}", module_code, template_name)
}

pub async fn render_admin_template(
    state: Arc<AppState>,
    template_name: &str,
    mut context: Context,
) -> axum::response::Response {
    prepare_admin_context(state.clone(), &mut context).await;
    match state.tera.render(template_name, &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Template rendering error for {}: {}", template_name, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn prepare_admin_context(state: Arc<AppState>, context: &mut Context) {
    let settings = state.settings.read().await;
    context.insert("site_name", &settings.site_name);

    // Collect module menus using AdminMenu module
    let menu = state.admin_menu.build_menu(None, None).await;
    context.insert("admin_menu", &menu.supercategories);
}
