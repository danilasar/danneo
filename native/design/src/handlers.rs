use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use danneo_sdk::{apanel::render_admin_template, auth::Claims, state::AppState, tera::Context};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;

pub async fn list_themes(_claims: Claims, state: State<Arc<AppState>>) -> impl IntoResponse {
    let state_arc = state.0.clone();
    let mut themes = Vec::new();
    let themes_dir = PathBuf::from("templates"); // This might need path resolution too

    if let Ok(entries) = std::fs::read_dir(&themes_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                themes.push(entry.file_name().to_string_lossy().to_string());
            }
        }
    }

    let mut context = Context::new();
    context.insert("themes", &themes);

    render_admin_template(state_arc, "apanel/design_list.html", context).await
}

pub async fn edit_theme(
    _claims: Claims,
    state: State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let state_arc = state.0.clone();
    let theme = params.get("theme");
    let file = params
        .get("file")
        .cloned()
        .unwrap_or_else(|| "index.html".to_string());

    let mut context = Context::new();
    context.insert("theme", &theme);
    context.insert("file", &file);

    if let Some(theme_name) = theme {
        let file_path = PathBuf::from("templates").join(theme_name).join(&file);
        if let Ok(content) = std::fs::read_to_string(&file_path) {
            context.insert("content", &content);
        }
    }

    render_admin_template(state_arc, "apanel/design_edit.html", context).await
}

#[derive(Deserialize)]
pub struct ThemeSaveForm {
    pub theme: String,
    pub file: String,
    pub content: String,
}

pub async fn save_theme(
    _claims: Claims,
    State(_state): State<Arc<AppState>>,
    Form(form): Form<ThemeSaveForm>,
) -> impl IntoResponse {
    // Security check: prevent directory traversal
    if form.file.contains("..") || form.theme.contains("..") {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let file_path = PathBuf::from("templates")
        .join(&form.theme)
        .join(&form.file);

    match std::fs::write(&file_path, &form.content) {
        Ok(_) => {
            let url = format!("/admin/design/edit?theme={}&file={}", form.theme, form.file);
            Redirect::to(&url).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to save theme file: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
