use crate::state::AppState;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use std::sync::Arc;
use tera::Context;

pub async fn resolve_module_template(
    state: &AppState,
    module_code: &str,
    template_name: &str,
) -> String {
    let settings = state.settings.read().await;
    let theme = &settings.site_temp;

    // 1. Try to find template in current theme
    let themed_path = format!("{}/{}/{}", module_code, theme, template_name);
    if state.tera.get_template_names().any(|n| n == themed_path) {
        return themed_path;
    }

    // 2. Fallback to 'default' theme
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
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template Error ({}): {}", template_name, e),
            )
                .into_response()
        }
    }
}

pub async fn prepare_admin_context(state: Arc<AppState>, context: &mut Context) {
    let settings = state.settings.read().await;
    context.insert("site_name", &settings.site_name);

    // Collect module menus using RPC
    let ctx = crate::rpc::RpcContext::default();
    let menu_res = state
        .rpc_registry
        .call(
            "admin_menu",
            "get_tree",
            serde_json::json!({}),
            ctx,
            state.clone(),
        )
        .await;

    if let Ok(menu_json) = menu_res {
        if let Ok(menu) = serde_json::from_value::<crate::registry::AdminMenu>(menu_json) {
            context.insert("admin_menu", &menu.supercategories);
        }
    }
}
