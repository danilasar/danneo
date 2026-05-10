use crate::state::AppState;
use axum::extract::State;
use axum::response::{Html, IntoResponse};
use sea_orm::ConnectionTrait;
use std::sync::Arc;
use tera::Context;

pub async fn prepare_frontend_context(state: Arc<AppState>, context: &mut Context) {
    let settings = state.settings.read().await;
    let theme = settings.site_temp.clone();
    context.insert("site_name", &settings.site_name);
    context.insert("site_url", &settings.site_url);
    context.insert("site_temp", &theme);
    context.insert("base_template", &format!("frontend/{}/index.html", theme));

    // Render Blocks into "positions"
    let block_ctx = Arc::new(crate::blocks::BlockContext {
        db: state.db.clone(),
        settings: state.settings.clone(),
        state: state.clone(),
    });
    let rendered_blocks = state.block_registry.get_all_positions_html(block_ctx, &state.tera).await;
    context.insert("positions", &rendered_blocks);

    // Fetch site menu via RPC
    let rpc_ctx = crate::rpc::RpcContext::default();
    let top_menu_res = state.rpc_registry.call("mod_menu", "get_menu", serde_json::json!({"position": "top"}), rpc_ctx.clone(), state.clone()).await;
    if let Ok(items) = top_menu_res {
        context.insert("top_menu_items", &items);
    }

    let bot_menu_res = state.rpc_registry.call("mod_menu", "get_menu", serde_json::json!({"position": "bottom"}), rpc_ctx, state.clone()).await;
    if let Ok(items) = bot_menu_res {
        context.insert("bot_menu_items", &items);
    }
}

pub async fn dispatch(
    State(state): State<Arc<AppState>>,
    method: axum::http::Method,
    uri: axum::http::Uri,
) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/').to_string();
    let mut params = std::collections::HashMap::new();
    let match_result = {
        let routes = state.routes.read().await;
        let mut found = None;
        let method_str = method.as_str().to_uppercase();

        for (module_code, descriptor) in &routes.frontend_routes {
            if descriptor.method.to_uppercase() == method_str {
                if let Some(p) = match_route(&descriptor.path, &path) {
                    found = Some((module_code.clone(), descriptor.clone()));
                    params = p;
                    break;
                }
            }
        }
        found
    };

    if let Some((module_code, descriptor)) = match_result {
        if descriptor.handler == "entity.list" {
            return render_entity_list(state, &module_code, &descriptor)
                .await
                .into_response();
        } else {
            return render_script_route(state, &module_code, &descriptor, &path, params)
                .await
                .into_response();
        }
    }

    (axum::http::StatusCode::NOT_FOUND, "Not Found").into_response()
}

fn match_route(pattern: &str, path: &str) -> Option<std::collections::HashMap<String, String>> {
    let pattern_parts: Vec<&str> = pattern
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    let path_parts: Vec<&str> = path
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    if pattern_parts.len() != path_parts.len() {
        return None;
    }

    let mut params = std::collections::HashMap::new();
    for (pat, p) in pattern_parts.iter().zip(path_parts.iter()) {
        if pat.starts_with(':') {
            params.insert(pat[1..].to_string(), p.to_string());
        } else if pat != p {
            return None;
        }
    }
    Some(params)
}

async fn render_script_route(
    state: Arc<AppState>,
    module_code: &str,
    descriptor: &crate::registry::RouteDescriptor,
    path: &str,
    params: std::collections::HashMap<String, String>,
) -> impl IntoResponse {
    let arg = serde_json::json!({
        "path": path,
        "handler": &descriptor.handler,
        "name": &descriptor.name,
        "params": params,
    });

    let dynamic_arg = script_rhai::serde::to_dynamic(arg).unwrap();

    match state
        .script_engine
        .call_hook(module_code, "frontend_dispatch", dynamic_arg, state.clone())
        .await
    {
        Ok(res) => {
            if let Some(res_map) = res.clone().try_cast::<script_rhai::Map>() {
                let template = res_map
                    .get("template")
                    .and_then(|v| v.clone().into_string().ok())
                    .unwrap_or_else(|| "index.html".to_string());

                let context_val = res_map
                    .get("context")
                    .cloned()
                    .unwrap_or_else(|| script_rhai::Dynamic::from(script_rhai::Map::new()));

                let mut ctx = Context::new();
                prepare_frontend_context(state.clone(), &mut ctx).await;

                if let Ok(ctx_json) =
                    script_rhai::serde::from_dynamic::<serde_json::Value>(&context_val)
                {
                    if let Some(obj) = ctx_json.as_object() {
                        for (k, v) in obj {
                            ctx.insert(k, v);
                        }
                    }
                }

                let full_template =
                    crate::apanel::resolve_module_template(&state, module_code, &template).await;
                match state.tera.render(&full_template, &ctx) {
                    Ok(html) => Html(html).into_response(),
                    Err(e) => {
                        tracing::error!("Template rendering error in frontend script: {}", e);
                        Html(format!("<h1>Template Error</h1><pre>{}</pre>", e)).into_response()
                    }
                }
            } else if let Some(s) = res.try_cast::<String>() {
                Html(s).into_response()
            } else {
                "Invalid response from Lua frontend_dispatch".into_response()
            }
        }
        Err(e) => {
            tracing::error!(
                "Lua frontend_dispatch error for module {}: {}",
                module_code,
                e
            );
            format!("<h1>Module Error</h1><pre>{}</pre>", e).into_response()
        }
    }
}

async fn render_entity_list(
    state: Arc<AppState>,
    module_code: &str,
    descriptor: &crate::registry::RouteDescriptor,
) -> impl IntoResponse {
    let entity_name = descriptor.entity.as_ref().unwrap();

    // Fetch entity schema
    use crate::models::core_module_entities;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let entity_record = core_module_entities::Entity::find()
        .filter(core_module_entities::Column::ModuleCode.eq(module_code))
        .filter(core_module_entities::Column::EntityName.eq(entity_name))
        .one(state.db.as_ref())
        .await
        .unwrap();

    if let Some(record) = entity_record {
        let schema: crate::crud::EntitySchema = serde_json::from_value(record.schema).unwrap();

        // Fetch data from the table
        let query = format!("SELECT * FROM {}", schema.table_name);
        let db = state.db.as_ref();
        let conn = sea_orm::ConnectionTrait::get_database_backend(db);
        let stmt = sea_orm::Statement::from_string(conn, query);
        let rows = db.query_all(stmt).await.unwrap();

        let mut context = Context::new();
        prepare_frontend_context(state.clone(), &mut context).await;

        // Convert rows to values
        let mut data = Vec::new();
        for row in rows {
            let mut item = serde_json::Map::new();
            for field in &schema.fields {
                let val: Result<Option<String>, _> = row.try_get("", &field.name);
                if let Ok(Some(v)) = val {
                    item.insert(field.name.clone(), serde_json::Value::String(v));
                }
            }
            data.push(serde_json::Value::Object(item));
        }
        context.insert("items", &data);

        let template_name = descriptor.template.as_deref().unwrap_or("index.html");
        let full_template =
            crate::apanel::resolve_module_template(&state, module_code, template_name).await;

        match state.tera.render(&full_template, &context) {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Template error: {}", e);
                Html(format!("<h1>Template Error</h1><pre>{}</pre>", e)).into_response()
            }
        }
    } else {
        (axum::http::StatusCode::NOT_FOUND, "Entity not found").into_response()
    }
}
