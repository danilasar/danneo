use crate::{auth::Claims, state::AppState};
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
};
use std::sync::Arc;
use tera::Context;

pub async fn render_dashboard(
    claims: Claims, // Extractor проверяет JWT токен
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, StatusCode> {
    let settings = state.settings.read().await;
    let mut context = Context::new();
    context.insert("admin_id", &claims.admin_id);
    context.insert("site_name", &settings.site_name);

    match state.tera.render("apanel/dashboard.html", &context) {
        Ok(html) => Ok(Html(html)),
        Err(e) => {
            tracing::error!("Template rendering error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
