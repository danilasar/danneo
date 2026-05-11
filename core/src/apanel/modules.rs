use crate::models::core_modules;
use crate::module::DanneoModule;
use crate::state::AppState;
use axum::{
    extract::{Form, Multipart, State, Request, FromRequest},
    http::{StatusCode, Method},
    response::{IntoResponse, Redirect, Response, Html},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower::ServiceExt;

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

fn kernel_module_view(
    module: &Arc<dyn DanneoModule>,
    installed_map: &std::collections::HashMap<String, bool>,
) -> PackageViewModel {
    let id = module.name().to_string();
    PackageViewModel {
        id: id.clone(),
        package_type: "kernel".to_string(),
        name: id.clone(),
        version: "kernel".to_string(),
        description: Some("Native kernel module".to_string()),
        is_installed: installed_map.contains_key(&id),
        is_enabled: installed_map.get(&id).copied().unwrap_or(false),
        entities: vec![],
    }
}

pub async fn list_modules(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut context = tera::Context::new();
    crate::apanel::prepare_admin_context(state.clone(), &mut context).await;

    let installed_modules = match core_modules::Entity::find().all(state.db.as_ref()).await {
        Ok(m) => m,
        Err(_) => vec![],
    };

    let mut installed_map = std::collections::HashMap::new();
    for m in installed_modules {
        installed_map.insert(m.code.clone(), m.enabled);
    }

    // Fetch all entities
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
    let mut added_ids = std::collections::HashSet::new();

    {
        let package_registry = state.packages.read().await;
        for (_, manifest) in package_registry.packages.iter() {
            let id = &manifest.package.id;
            let is_installed = installed_map.contains_key(id);
            let is_enabled = installed_map.get(id).copied().unwrap_or(false);

            packages_view.push(PackageViewModel {
                id: id.clone(),
                package_type: manifest.package.package_type.clone(),
                name: manifest.package.name.clone(),
                version: manifest.package.version.clone(),
                description: manifest.package.description.clone(),
                is_installed,
                is_enabled,
                entities: entity_map.get(id).cloned().unwrap_or_default(),
            });
            added_ids.insert(id.clone());
        }
    }

    {
        let modules = state.modules.read().await;
        let native_modules = modules.native_modules.read().await;
        for (name, module) in native_modules.iter() {
            if !added_ids.contains(name) {
                packages_view.push(kernel_module_view(module, &installed_map));
            }
        }
    }

    context.insert("packages", &packages_view);

    crate::apanel::render_admin_template(state, "admin_menu/default/apanel/modules_list.html", context).await
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
                .render("admin_menu/default/apanel/module_install_preview.html", &context)
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

pub async fn dispatch_admin_clean(
    State(state): State<Arc<AppState>>,
    req: Request,
) -> Response {
    let uri = req.uri().clone();
    let method = req.method().clone();
    let path = uri.path().trim_start_matches("/admin").trim_start_matches('/').to_string();
    
    let match_result = {
        let routes = state.routes.read().await;
        let mut found = None;
        let method_str = method.as_str().to_uppercase();

        for (module_code, descriptor) in &routes.admin_routes {
            if descriptor.method.to_uppercase() == method_str {
                if descriptor.path == format!("/{}", path) || descriptor.path == path {
                     found = Some((module_code.clone(), descriptor.handler.clone()));
                     break;
                }
            }
        }
        found
    };

    if let Some((module_code, handler_name)) = match_result {
        if is_module_enabled(&state, &module_code).await {
            dispatch_admin_internal(state, module_code, handler_name, req).await
        } else {
            (StatusCode::NOT_FOUND, "Module disabled").into_response()
        }
    } else {
        let parts: Vec<&str> = path.split('/').collect();
        if !parts.is_empty() {
             let module_name = parts[0];
             if is_module_enabled(&state, module_name).await {
                 let sub_path = parts[1..].join("/");
                 return dispatch_admin_internal(state, module_name.to_string(), sub_path, req).await;
             }
        }
        (StatusCode::NOT_FOUND, "Admin path not found").into_response()
    }
}

async fn is_module_enabled(state: &AppState, module_code: &str) -> bool {
    core_modules::Entity::find()
        .filter(core_modules::Column::Code.eq(module_code))
        .filter(core_modules::Column::Enabled.eq(true))
        .one(state.db.as_ref())
        .await
        .unwrap_or(None)
        .is_some()
}

async fn dispatch_admin_internal(
    state: Arc<AppState>,
    module_name: String,
    path: String,
    req: Request,
) -> Response {
    let method = req.method().clone();
    
    // 1. Try Native first
    let native_module = {
        let modules_guard = state.modules.read().await;
        modules_guard.native_modules.read().await.get(&module_name).cloned()
    };

    if let Some(native) = native_module {
         let router = native.register_admin_routes();
         let router = router.with_state(state.clone());
         let sub_path = if path.starts_with('/') { path.clone() } else { format!("/{}", path) };
         
         let mut sub_req_builder = Request::builder().uri(sub_path).method(method.clone());
         if let Some(headers) = sub_req_builder.headers_mut() {
             for (key, value) in req.headers() {
                 headers.insert(key.clone(), value.clone());
             }
         }
         let sub_req = sub_req_builder.body(axum::body::Body::empty()).unwrap();
            
         match router.oneshot(sub_req).await {
             Ok(res) => return res,
             Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
         }
    }

    // 2. Try Lua
    let query: std::collections::HashMap<String, String> = req.uri().query()
        .map(|v| serde_urlencoded::from_str(v).unwrap_or_default())
        .unwrap_or_default();

    let mut form_val = serde_json::json!({});
    let mut files_val = serde_json::json!([]);

    if method == Method::POST {
        let content_type = req.headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();

        if content_type.starts_with("multipart/form-data") {
            if let Ok(mut multipart) = Multipart::from_request(req, &state).await {
                let mut form_map = serde_json::Map::new();
                let mut files_vec = Vec::new();
                
                while let Ok(Some(field)) = multipart.next_field().await {
                    let name = field.name().unwrap_or_default().to_string();
                    let file_name = field.file_name().map(|s| s.to_string());
                    
                    if let Some(f_name) = file_name {
                        if let Ok(data) = field.bytes().await {
                            let temp_dir = std::env::temp_dir();
                            let temp_path = temp_dir.join(format!("neodanneo_{}_{}", uuid::Uuid::new_v4(), f_name));
                            if let Ok(_) = std::fs::write(&temp_path, &data) {
                                files_vec.push(serde_json::json!({
                                    "field": name,
                                    "name": f_name,
                                    "size": data.len(),
                                    "temp_path": temp_path.to_string_lossy()
                                }));
                            }
                        }
                    } else {
                        if let Ok(text) = field.text().await {
                            form_map.insert(name, serde_json::Value::String(text));
                        }
                    }
                }
                form_val = serde_json::Value::Object(form_map);
                files_val = serde_json::Value::Array(files_vec);
            }
        } else {
            let body_bytes = axum::body::to_bytes(req.into_body(), 1024 * 1024).await.unwrap_or_default();
            if let Ok(form) = serde_urlencoded::from_bytes::<serde_json::Value>(&body_bytes) {
                 form_val = form;
            }
        }
    }

    let arg = serde_json::json!({
        "path": path,
        "method": method.as_str(),
        "query": query,
        "form": form_val,
        "files": files_val,
    });

    let dynamic_arg = script_rhai::serde::to_dynamic(arg).unwrap();

    match state
        .script_engine
        .call_hook(&module_name, "admin_dispatch", dynamic_arg, state.clone())
        .await
    {
        Ok(res) => {
            if let Some(res_map) = res.clone().try_cast::<script_rhai::Map>() {
                if let Some(redirect) = res_map.get("redirect").and_then(|v| v.clone().into_string().ok()) {
                    return Redirect::to(&redirect).into_response();
                }

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
                    crate::apanel::resolve_module_template(&state, &module_name, &template).await
                };

                let mut response = match state.tera.render(&full_template, &ctx) {
                    Ok(html) => Html(html).into_response(),
                    Err(e) => {
                        tracing::error!("Template rendering error in dispatch_admin: {}", e);
                        Html(format!("<h1>Template Error</h1><pre>{}</pre>", e)).into_response()
                    }
                };

                // Apply custom status if present
                if let Some(status_dyn) = res_map.get("status") {
                    if let Ok(status_code) = status_dyn.clone().as_int() {
                        if let Ok(st) = StatusCode::from_u16(status_code as u16) {
                            *response.status_mut() = st;
                        }
                    }
                }

                // Apply custom headers if present
                if let Some(headers_dyn) = res_map.get("headers") {
                    if let Ok(headers_val) = script_rhai::serde::from_dynamic::<serde_json::Value>(headers_dyn) {
                        if let Some(obj) = headers_val.as_object() {
                            let headers = response.headers_mut();
                            for (k, v) in obj {
                                if let Some(v_str) = v.as_str() {
                                    if let Ok(h_name) = axum::http::header::HeaderName::from_bytes(k.as_bytes()) {
                                        if let Ok(h_val) = axum::http::HeaderValue::from_str(v_str) {
                                            headers.insert(h_name, h_val);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                response
            } else if let Some(s) = res.try_cast::<String>() {
                Html(s).into_response()
            } else {
                "Invalid response from Lua admin_dispatch".into_response()
            }
        }
        Err(e) => {
            tracing::error!("Lua admin_dispatch error for module {}: {}", module_name, e);
            format!("<h1>Module Error</h1><pre>{}</pre>", e).into_response()
        }
    }
}

// dispatch_admin removed in favor of Axum nesting
