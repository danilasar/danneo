use crate::crud::{self};
use axum::{
    Json as AxumJson,
    extract::{Form, Json, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::json;
use std::sync::Arc;

/// Handles dynamic CRUD actions for admin routes.
/// Expected path: /admin/crud/:module/:entity/:action
/// - `list`  – returns all rows as JSON array.
/// - `create` – expects JSON body with record data, inserts and returns the record.
pub async fn handle(
    State(state): State<Arc<crate::state::AppState>>,
    Path((module, entity, action)): Path<(String, String, String)>,
    payload: Json<serde_json::Value>,
) -> impl IntoResponse {
    let db = state.db.as_ref();
    // Verify entity belongs to module
    use crate::models::core_module_entities::Entity as EntEntity;
    let entity_meta = match EntEntity::find()
        .filter(crate::models::core_module_entities::Column::ModuleCode.eq(&module))
        .filter(crate::models::core_module_entities::Column::EntityName.eq(&entity))
        .one(db)
        .await
    {
        Ok(Some(m)) => m,
        _ => return AxumJson(json!({"error": "Entity not found for module"})).into_response(),
    };
    let table_name = entity_meta.table_name;
    match action.as_str() {
        "list" => match crud::select_all(db, &table_name, &[]).await {
            Ok(rows) => AxumJson(json!({"data": rows})).into_response(),
            Err(e) => AxumJson(json!({"error": e.to_string()})).into_response(),
        },
        "create" => match crud::insert_record(db, &table_name, &payload.0).await {
            Ok(rec) => AxumJson(json!({"data": rec})).into_response(),
            Err(e) => AxumJson(json!({"error": e.to_string()})).into_response(),
        },
        _ => AxumJson(json!({"error": "Unsupported action"})).into_response(),
    }
}

/// Render HTML list page for a dynamic entity (admin UI).
pub async fn list_page(
    State(state): State<Arc<crate::state::AppState>>,
    Path((module, entity)): Path<(String, String)>,
) -> impl IntoResponse {
    let db = state.db.as_ref();
    // Load entity metadata
    use crate::models::core_module_entities::Entity as EntEntity;
    let entity_meta = match EntEntity::find()
        .filter(crate::models::core_module_entities::Column::ModuleCode.eq(&module))
        .filter(crate::models::core_module_entities::Column::EntityName.eq(&entity))
        .one(db)
        .await
    {
        Ok(Some(m)) => m,
        _ => return format!("<h1>Entity not found</h1>").into_response(),
    };
    // Parse schema for column names
    let schema: crate::crud::EntitySchema = match serde_json::from_value(entity_meta.schema.clone())
    {
        Ok(s) => s,
        Err(_) => return format!("<h1>Invalid schema</h1>").into_response(),
    };
    let columns: Vec<String> = schema.fields.iter().map(|f| f.name.clone()).collect();
    // Fetch rows
    let rows = match crate::crud::select_all(db, &entity_meta.table_name, &columns).await {
        Ok(r) => r,
        Err(_) => vec![],
    };
    // Determine primary key for edit links (first pk or first column)
    let primary_key = schema
        .fields
        .iter()
        .find(|f| f.primary_key.unwrap_or(false))
        .map(|f| f.name.clone())
        .unwrap_or_else(|| columns.get(0).cloned().unwrap_or_default());
    // Build Tera context
    let mut ctx = tera::Context::new();
    crate::apanel::prepare_admin_context(state.clone(), &mut ctx).await;
    ctx.insert("module", &module);
    ctx.insert("entity", &entity);
    ctx.insert("entity_name", &entity);
    ctx.insert("columns", &columns);
    ctx.insert("primary_key", &primary_key);
    ctx.insert("rows", &rows);
    // Render template
    match state.tera.render("apanel/crud_list.html", &ctx) {
        Ok(html) => Html(html).into_response(),
        Err(e) => Html(format!("<h1>Template error: {}</h1>", e)).into_response(),
    }
}

/// Render HTML edit page for a dynamic entity.
pub async fn edit_page(
    State(state): State<Arc<crate::state::AppState>>,
    Path(params): Path<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let db = state.db.as_ref();
    let module = match params.get("module") {
        Some(m) => m,
        None => return (StatusCode::BAD_REQUEST, "Missing module parameter").into_response(),
    };
    let entity = match params.get("entity") {
        Some(e) => e,
        None => return (StatusCode::BAD_REQUEST, "Missing entity parameter").into_response(),
    };
    let id = params.get("id");

    // Load entity metadata
    use crate::models::core_module_entities::Entity as EntEntity;
    let entity_meta = match EntEntity::find()
        .filter(crate::models::core_module_entities::Column::ModuleCode.eq(module))
        .filter(crate::models::core_module_entities::Column::EntityName.eq(entity))
        .one(db)
        .await
    {
        Ok(Some(m)) => m,
        _ => return (StatusCode::NOT_FOUND, "Entity not found").into_response(),
    };

    let schema: crate::crud::EntitySchema = match serde_json::from_value(entity_meta.schema.clone())
    {
        Ok(s) => s,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid schema").into_response(),
    };

    let columns: Vec<String> = schema.fields.iter().map(|f| f.name.clone()).collect();
    let primary_key = schema
        .fields
        .iter()
        .find(|f| f.primary_key.unwrap_or(false))
        .map(|f| f.name.clone())
        .unwrap_or_default();

    let mut record = json!({});
    if let Some(id_val) = id {
        if let Ok(Some(r)) =
            crate::crud::select_by_pk(db, &entity_meta.table_name, &columns, &primary_key, id_val)
                .await
        {
            record = r;
        }
    }

    let mut ctx = tera::Context::new();
    crate::apanel::prepare_admin_context(state.clone(), &mut ctx).await;
    ctx.insert("module", module);
    ctx.insert("entity", entity);
    ctx.insert("schema", &schema);
    ctx.insert("primary_key", &primary_key);
    ctx.insert("record", &record);

    match state.tera.render("apanel/crud_edit.html", &ctx) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Template rendering error in apanel/crud_edit.html: {}", e);
            Html(format!("<h1>Template error: {}</h1>", e)).into_response()
        }
    }
}

/// Handle form save (POST).
pub async fn save_handle(
    State(state): State<Arc<crate::state::AppState>>,
    Path((module, entity)): Path<(String, String)>,
    Form(payload): Form<serde_json::Value>,
) -> impl IntoResponse {
    let db = state.db.as_ref();
    use crate::models::core_module_entities::Entity as EntEntity;
    let entity_meta = match EntEntity::find()
        .filter(crate::models::core_module_entities::Column::ModuleCode.eq(&module))
        .filter(crate::models::core_module_entities::Column::EntityName.eq(&entity))
        .one(db)
        .await
    {
        Ok(Some(m)) => m,
        _ => return "Entity not found".into_response(),
    };

    let schema: crate::crud::EntitySchema =
        serde_json::from_value(entity_meta.schema.clone()).unwrap();
    let primary_key = schema
        .fields
        .iter()
        .find(|f| f.primary_key.unwrap_or(false))
        .map(|f| f.name.clone())
        .unwrap_or_default();

    let mut payload_val = payload;

    // --- TYPE CASTING based on schema ---
    if let Some(obj) = payload_val.as_object_mut() {
        for field in &schema.fields {
            if let Some(val) = obj.get_mut(&field.name) {
                if let Some(s) = val.as_str() {
                    match field.field_type.as_str() {
                        "boolean" | "bool" => {
                            *val = json!(s == "true" || s == "on" || s == "1");
                        }
                        "integer" | "int" | "bigint" => {
                            if let Ok(n) = s.parse::<i64>() {
                                *val = json!(n);
                            }
                        }
                        "float" | "double" => {
                            if let Ok(n) = s.parse::<f64>() {
                                *val = json!(n);
                            }
                        }
                        _ => {}
                    }
                }
            } else if field.field_type == "boolean" || field.field_type == "bool" {
                // Checkboxes are not sent if unchecked
                obj.insert(field.name.clone(), json!(false));
            }
        }
    }

    // --- HOOK: before_save ---
    let arg = script_rhai::serde::to_dynamic(json!({
        "entity": entity,
        "data": payload_val
    }))
    .unwrap_or(script_rhai::Dynamic::UNIT);

    if let Ok(res) = state
        .script_engine
        .call_hook(&module, "before_save", arg)
        .await
    {
        // Если скрипт вернул данные, используем их
        if let Ok(new_data) = script_rhai::serde::from_dynamic::<serde_json::Value>(&res) {
            if let Some(d) = new_data.get("data") {
                payload_val = d.clone();
            }
        }
    } else {
        // Если скрипт вернул ошибку, можно прервать сохранение
        // tracing::warn!("Hook before_save returned error or not found");
    }

    let id_val = payload_val
        .get(&primary_key)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .or_else(|| {
            payload_val
                .get(&primary_key)
                .and_then(|v| v.as_i64().map(|i| i.to_string()))
        });

    let res = if let Some(id) = id_val {
        if id.is_empty() {
            crate::crud::insert_record(db, &entity_meta.table_name, &payload_val).await
        } else {
            crate::crud::update_record(db, &entity_meta.table_name, &primary_key, &id, &payload_val)
                .await
        }
    } else {
        crate::crud::insert_record(db, &entity_meta.table_name, &payload_val).await
    };

    if res.is_ok() {
        // --- HOOK: after_save ---
        let arg_after = script_rhai::serde::to_dynamic(json!({
            "entity": entity,
            "data": payload_val
        }))
        .unwrap_or(script_rhai::Dynamic::UNIT);
        let _ = state
            .script_engine
            .call_hook(&module, "after_save", arg_after)
            .await;
    }

    match res {
        Ok(_) => axum::response::Redirect::to(&format!("/admin/crud/{}/{}/list", module, entity))
            .into_response(),
        Err(e) => format!("Error saving: {}", e).into_response(),
    }
}

/// Handle delete action.
pub async fn delete_handle(
    State(state): State<Arc<crate::state::AppState>>,
    Path((module, entity, id)): Path<(String, String, String)>,
) -> impl IntoResponse {
    let db = state.db.as_ref();
    use crate::models::core_module_entities::Entity as EntEntity;
    let entity_meta = match EntEntity::find()
        .filter(crate::models::core_module_entities::Column::ModuleCode.eq(&module))
        .filter(crate::models::core_module_entities::Column::EntityName.eq(&entity))
        .one(db)
        .await
    {
        Ok(Some(m)) => m,
        _ => return "Entity not found".into_response(),
    };

    let schema: crate::crud::EntitySchema =
        serde_json::from_value(entity_meta.schema.clone()).unwrap();
    let primary_key = schema
        .fields
        .iter()
        .find(|f| f.primary_key.unwrap_or(false))
        .map(|f| f.name.clone())
        .unwrap_or_default();

    match crate::crud::delete_record(db, &entity_meta.table_name, &primary_key, &id).await {
        Ok(_) => axum::response::Redirect::to(&format!("/admin/crud/{}/{}/list", module, entity))
            .into_response(),
        Err(e) => format!("Error deleting: {}", e).into_response(),
    }
}
