use crate::models::core_modules;
use crate::state::AppState;
use axum::{
    extract::{Form, State},
    response::{IntoResponse, Redirect},
};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize)]
struct PackageViewModel {
    id: String,
    package_type: String,
    name: String,
    version: String,
    description: Option<String>,
    is_installed: bool,
    is_enabled: bool,
    entities: Vec<String>,
}

pub async fn list_modules(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut context = tera::Context::new();
    let settings = state.settings.read().await;
    context.insert("site_name", &settings.site_name);

    let installed_modules = match core_modules::Entity::find().all(state.db.as_ref()).await {
        Ok(m) => m,
        Err(_) => vec![],
    };

    let installed_blocks = match crate::models::core_block_definitions::Entity::find().all(state.db.as_ref()).await {
        Ok(b) => b,
        Err(_) => vec![],
    };

    let mut installed_map = std::collections::HashMap::new();
    for m in installed_modules {
        installed_map.insert(m.code.clone(), m.enabled);
    }
    for b in installed_blocks {
        installed_map.insert(b.block_code.clone(), b.enabled);
    }

    // Fetch all entities to show in the list
    use crate::models::core_module_entities;
    let all_entities = match core_module_entities::Entity::find().all(state.db.as_ref()).await {
        Ok(e) => e,
        Err(_) => vec![],
    };
    let mut entity_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for e in all_entities {
        entity_map.entry(e.module_code).or_default().push(e.entity_name);
    }

    let mut packages_view = Vec::new();
    {
        let package_registry = state.packages.read().await;
        for (_, manifest) in package_registry.packages.iter() {
            let is_installed = installed_map.contains_key(&manifest.package.id);
            let is_enabled = installed_map
                .get(&manifest.package.id)
                .copied()
                .unwrap_or(false);

            packages_view.push(PackageViewModel {
                id: manifest.package.id.clone(),
                package_type: manifest.package.package_type.clone(),
                name: manifest.package.name.clone(),
                version: manifest.package.version.clone(),
                description: manifest.package.description.clone(),
                is_installed,
                is_enabled,
                entities: entity_map.get(&manifest.package.id).cloned().unwrap_or_default(),
            });
        }
        for (_, manifest) in package_registry.blocks.iter() {
            let is_installed = installed_map.contains_key(&manifest.block.id);
            let is_enabled = installed_map
                .get(&manifest.block.id)
                .copied()
                .unwrap_or(false);

            packages_view.push(PackageViewModel {
                id: manifest.block.id.clone(),
                package_type: "block".to_string(),
                name: manifest.block.name.clone(),
                version: manifest.block.version.clone(),
                description: None,
                is_installed,
                is_enabled,
                entities: vec![],
            });
        }
    }

    context.insert("packages", &packages_view);

    match state.tera.render("apanel/modules_list.html", &context) {
        Ok(html) => axum::response::Html(html).into_response(),
        Err(e) => {
            tracing::error!("Template error: {}", e);
            axum::response::Html(format!("<h1>Template Error</h1><pre>{}</pre>", e)).into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct ModuleActionForm {
    pub package_id: String,
}

pub async fn install_module(
    State(state): State<Arc<AppState>>,
    Form(form): Form<ModuleActionForm>,
) -> impl IntoResponse {
    let installer =
        crate::registry::PackageInstaller::new(state.db.clone(), state.packages.clone());
    if let Err(e) = installer.install(&form.package_id).await {
        tracing::error!("Install error: {}", e);
    }
    Redirect::to("/admin/modules")
}

pub async fn uninstall_module(
    State(state): State<Arc<AppState>>,
    Form(form): Form<ModuleActionForm>,
) -> impl IntoResponse {
    let installer =
        crate::registry::PackageInstaller::new(state.db.clone(), state.packages.clone());
    if let Err(e) = installer.uninstall(&form.package_id).await {
        tracing::error!("Uninstall error: {}", e);
    }
    Redirect::to("/admin/modules")
}

pub async fn enable_module(
    State(state): State<Arc<AppState>>,
    Form(form): Form<ModuleActionForm>,
) -> impl IntoResponse {
    let modules = state.modules.read().await;
    if let Err(e) = modules.enable(&form.package_id).await {
        tracing::error!("Enable error: {}", e);
    }
    Redirect::to("/admin/modules")
}

pub async fn disable_module(
    State(state): State<Arc<AppState>>,
    Form(form): Form<ModuleActionForm>,
) -> impl IntoResponse {
    let modules = state.modules.read().await;
    if let Err(e) = modules.disable(&form.package_id).await {
        tracing::error!("Disable error: {}", e);
    }
    Redirect::to("/admin/modules")
}
