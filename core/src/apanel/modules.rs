use crate::models::core_modules;
use crate::state::AppState;
use axum::{
    extract::{Form, Multipart, State},
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
    crate::apanel::prepare_admin_context(state.clone(), &mut context).await;

    let installed_modules = match core_modules::Entity::find().all(state.db.as_ref()).await {
        Ok(m) => m,
        Err(_) => vec![],
    };

    let installed_blocks = match crate::models::core_block_definitions::Entity::find()
        .all(state.db.as_ref())
        .await
    {
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
    let all_entities = match core_module_entities::Entity::find()
        .all(state.db.as_ref())
        .await
    {
        Ok(e) => e,
        Err(_) => vec![],
    };
    let mut entity_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for e in all_entities {
        entity_map
            .entry(e.module_code)
            .or_default()
            .push(e.entity_name);
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
                entities: entity_map
                    .get(&manifest.package.id)
                    .cloned()
                    .unwrap_or_default(),
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
            tracing::error!("Template rendering error: {}", e);
            "Internal Server Error".into_response()
        }
    }
}

#[derive(Serialize)]
pub struct VerificationViewModel {
    pub manifest: crate::registry::PackageManifest,
    pub staging_path: String,
    pub is_upgrade: bool,
    pub current_version: Option<String>,
    pub issues: Vec<String>,
}

pub async fn upload_module(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut zip_bytes = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or_default().to_string();
        if name == "package" {
            if let Ok(data) = field.bytes().await {
                zip_bytes = data.to_vec();
            }
        }
    }

    if zip_bytes.is_empty() {
        return "No file uploaded".into_response();
    }

    let installed_versions = {
        let mut map = std::collections::HashMap::new();
        if let Ok(modules) = core_modules::Entity::find().all(state.db.as_ref()).await {
            for m in modules {
                map.insert(m.code, m.version);
            }
        }
        map
    };

    let package_registry = state.packages.read().await;
    match package_registry.extract_and_verify(&zip_bytes, &installed_versions) {
        Ok(result) => {
            let mut context = tera::Context::new();
            crate::apanel::prepare_admin_context(state.clone(), &mut context).await;
            context.insert(
                "result",
                &VerificationViewModel {
                    manifest: result.manifest,
                    staging_path: result.staging_path.to_string_lossy().to_string(),
                    is_upgrade: result.is_upgrade,
                    current_version: result.current_version,
                    issues: result.issues,
                },
            );

            match state
                .tera
                .render("apanel/module_install_preview.html", &context)
            {
                Ok(html) => axum::response::Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Template rendering error: {}", e);
                    "Internal Server Error".into_response()
                }
            }
        }
        Err(e) => format!("Verification failed: {}", e).into_response(),
    }
}

#[derive(Deserialize)]
pub struct ModuleActionForm {
    pub package_id: String,
}

#[derive(Deserialize)]
pub struct StagingInstallForm {
    pub package_id: String,
    pub staging_path: String,
}

pub async fn install_from_staging_handle(
    State(state): State<Arc<AppState>>,
    Form(form): Form<StagingInstallForm>,
) -> impl IntoResponse {
    let installer = crate::registry::PackageInstaller::new(
        state.db.clone(),
        state.packages.clone(),
        state.modules.clone(),
        state.routes.clone(),
        state.script_engine.clone(),
        state.clone(),
    );
    let staging_path = std::path::PathBuf::from(&form.staging_path);
    match installer
        .install_from_staging(&form.package_id, &staging_path)
        .await
    {
        Ok(_) => Redirect::to("/admin/modules").into_response(),
        Err(e) => {
            tracing::error!("Staging install error: {}", e);
            format!("Installation failed: {}", e).into_response()
        }
    }
}

pub async fn install_module(
    State(state): State<Arc<AppState>>,
    Form(form): Form<ModuleActionForm>,
) -> impl IntoResponse {
    let installer = crate::registry::PackageInstaller::new(
        state.db.clone(),
        state.packages.clone(),
        state.modules.clone(),
        state.routes.clone(),
        state.script_engine.clone(),
        state.clone(),
    );
    if let Err(e) = installer.install(&form.package_id).await {
        tracing::error!("Install error: {}", e);
    }
    Redirect::to("/admin/modules")
}

pub async fn uninstall_module(
    State(state): State<Arc<AppState>>,
    Form(form): Form<ModuleActionForm>,
) -> impl IntoResponse {
    let installer = crate::registry::PackageInstaller::new(
        state.db.clone(),
        state.packages.clone(),
        state.modules.clone(),
        state.routes.clone(),
        state.script_engine.clone(),
        state.clone(),
    );
    if let Err(e) = installer.uninstall(&form.package_id).await {
        tracing::error!("Uninstall error: {}", e);
    }
    Redirect::to("/admin/modules")
}

pub async fn enable_module(
    State(state): State<Arc<AppState>>,
    Form(form): Form<ModuleActionForm>,
) -> impl IntoResponse {
    let installer = crate::registry::PackageInstaller::new(
        state.db.clone(),
        state.packages.clone(),
        state.modules.clone(),
        state.routes.clone(),
        state.script_engine.clone(),
        state.clone(),
    );

    let res = {
        let modules = state.modules.read().await;
        modules.enable(&form.package_id).await
    };

    match res {
        Ok(_) => {
            installer.refresh_registries().await;
        }
        Err(e) => {
            tracing::error!("Enable error: {}", e);
        }
    }
    Redirect::to("/admin/modules")
}

pub async fn disable_module(
    State(state): State<Arc<AppState>>,
    Form(form): Form<ModuleActionForm>,
) -> impl IntoResponse {
    let installer = crate::registry::PackageInstaller::new(
        state.db.clone(),
        state.packages.clone(),
        state.modules.clone(),
        state.routes.clone(),
        state.script_engine.clone(),
        state.clone(),
    );

    let res = {
        let modules = state.modules.read().await;
        modules.disable(&form.package_id).await
    };

    match res {
        Ok(_) => {
            installer.refresh_registries().await;
        }
        Err(e) => {
            tracing::error!("Disable error: {}", e);
        }
    }
    Redirect::to("/admin/modules")
}

pub async fn dispatch_admin(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((module, path)): axum::extract::Path<(String, String)>,
    method: axum::http::Method,
    axum::extract::Query(query): axum::extract::Query<std::collections::HashMap<String, String>>,
    form: Option<axum::extract::Form<serde_json::Value>>,
) -> impl IntoResponse {
    let mut form_val = serde_json::json!({});
    if let Some(f) = form {
        form_val = f.0;
    }

    let arg = serde_json::json!({
        "path": path,
        "method": method.as_str(),
        "query": query,
        "form": form_val,
    });

    let dynamic_arg = script_rhai::serde::to_dynamic(arg).unwrap();

    match state
        .script_engine
        .call_hook(&module, "admin_dispatch", dynamic_arg, state.clone())
        .await
    {

        Ok(res) => {
            if let Some(res_map) = res.clone().try_cast::<script_rhai::Map>() {
                let template = res_map
                    .get("template")
                    .and_then(|v| v.clone().into_string().ok())
                    .unwrap_or_else(|| "admin.html".to_string());

                let context_val = res_map
                    .get("context")
                    .cloned()
                    .unwrap_or_else(|| script_rhai::Dynamic::from(script_rhai::Map::new()));

                let mut ctx = tera::Context::new();
                crate::apanel::prepare_admin_context(state.clone(), &mut ctx).await;

                if let Ok(ctx_json) =
                    script_rhai::serde::from_dynamic::<serde_json::Value>(&context_val)
                {
                    if let Some(obj) = ctx_json.as_object() {
                        for (k, v) in obj {
                            ctx.insert(k, v);
                        }
                    }
                }

                let full_template = if template.starts_with("apanel/") {
                    template
                } else {
                    crate::apanel::resolve_module_template(&state, &module, &template).await
                };

                match state.tera.render(&full_template, &ctx) {
                    Ok(html) => axum::response::Html(html).into_response(),
                    Err(e) => {
                        tracing::error!("Template rendering error in dispatch_admin: {}", e);
                        format!("<h1>Template Error</h1><pre>{}</pre>", e).into_response()
                    }
                }
            } else if let Some(s) = res.try_cast::<String>() {
                axum::response::Html(s).into_response()
            } else {
                "Invalid response from Lua admin_dispatch".into_response()
            }
        }
        Err(e) => {
            tracing::error!("Lua admin_dispatch error for module {}: {}", module, e);
            format!("<h1>Module Error</h1><pre>{}</pre>", e).into_response()
        }
    }
}
