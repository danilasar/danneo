use axum::{extract::State, response::IntoResponse};
use std::sync::Arc;
use crate::state::AppState;
use serde::Serialize;

#[derive(Serialize)]
struct PackageViewModel {
    id: String,
    package_type: String,
    name: String,
    version: String,
    description: Option<String>,
}

pub async fn list_modules(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut context = tera::Context::new();
    let settings = state.settings.read().await;
    context.insert("site_name", &settings.site_name);

    let mut packages_view = Vec::new();
    {
        let package_registry = state.packages.read().await;
        for (_, manifest) in package_registry.packages.iter() {
            packages_view.push(PackageViewModel {
                id: manifest.package.id.clone(),
                package_type: manifest.package.package_type.clone(),
                name: manifest.package.name.clone(),
                version: manifest.package.version.clone(),
                description: manifest.package.description.clone(),
            });
        }
        for (_, manifest) in package_registry.blocks.iter() {
            packages_view.push(PackageViewModel {
                id: manifest.block.id.clone(),
                package_type: "block".to_string(),
                name: manifest.block.name.clone(),
                version: manifest.block.version.clone(),
                description: None,
            });
        }
    }

    context.insert("packages", &packages_view);

    match state.tera.render("apanel/modules_list.html", &context) {
        Ok(html) => axum::response::Html(html),
        Err(e) => {
            tracing::error!("Template error: {}", e);
            axum::response::Html(format!("<h1>Template Error</h1><pre>{}</pre>", e))
        }
    }
}
