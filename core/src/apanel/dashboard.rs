use crate::{auth::Claims, state::AppState};
use axum::{
    extract::State,
    response::IntoResponse,
};
use std::sync::Arc;
use tera::Context;

pub async fn render_dashboard(
    claims: Claims, // Extractor проверяет JWT токен
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("admin_id", &claims.admin_id);

    crate::apanel::render_admin_template(state, "apanel/dashboard.html", context).await
}
