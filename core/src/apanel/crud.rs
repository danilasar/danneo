use crate::state::AppState;
use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::json;
use std::sync::Arc;
use tera::Context;

/// List records for a generic entity.
pub async fn list_page(
    State(state): State<Arc<AppState>>,
    Path((module, entity)): Path<(String, String)>,
) -> Response {
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
    let columns: Vec<String> = schema.fields.iter().map(|f| f.name.clone()).collect();

    let data = crate::crud::select_all(db, &entity_meta.table_name, &columns)
        .await
        .unwrap_or_default();

    let mut context = Context::new();
    context.insert("module", &module);
    context.insert("entity", &entity);
    context.insert("schema", &schema);
    context.insert("data", &data);

    crate::apanel::render_admin_template(state, "apanel/crud_list.html", context).await
}

/// Show edit form for a generic entity.
pub async fn edit_page(
    State(state): State<Arc<AppState>>,
    Path((module, entity)): Path<(String, String)>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
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
    let id = params.get("id");

    let record = if let Some(id) = id {
        let primary_key = schema
            .fields
            .iter()
            .find(|f| f.primary_key.unwrap_or(false))
            .map(|f| f.name.clone())
            .unwrap_or_default();
        let columns: Vec<String> = schema.fields.iter().map(|f| f.name.clone()).collect();
        crate::crud::select_by_pk(db, &entity_meta.table_name, &columns, &primary_key, id)
            .await
            .unwrap_or(None)
    } else {
        None
    };

    let mut context = Context::new();
    context.insert("module", &module);
    context.insert("entity", &entity);
    context.insert("schema", &schema);
    context.insert("record", &record);

    crate::apanel::render_admin_template(state, "apanel/crud_edit.html", context).await
}

/// Dispatch actions.
pub async fn handle(
    state: State<Arc<AppState>>,
    Path((module, entity, action)): Path<(String, String, String)>,
    form: Option<Form<std::collections::HashMap<String, String>>>,
) -> Response {
    match action.as_str() {
        "save" => {
            if let Some(Form(payload)) = form {
                save_handle(state, Path((module, entity)), Form(payload)).await
            } else {
                StatusCode::BAD_REQUEST.into_response()
            }
        }
        "delete" => {
            // ID usually comes from query for delete
            StatusCode::NOT_IMPLEMENTED.into_response()
        }
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Handle save action (insert or update).
pub async fn save_handle(
    State(state): State<Arc<AppState>>,
    Path((module, entity)): Path<(String, String)>,
    Form(payload): Form<std::collections::HashMap<String, String>>,
) -> Response {
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

    let mut payload_val = json!(payload);

    // Convert types based on schema
    if let Some(obj) = payload_val.as_object_mut() {
        for field in &schema.fields {
            if let Some(val_str) = obj.get(&field.name).and_then(|v| v.as_str()) {
                match field.field_type.as_str() {
                    "integer" => {
                        if let Ok(i) = val_str.parse::<i64>() {
                            obj.insert(field.name.clone(), json!(i));
                        }
                    }
                    "boolean" => {
                        let b = val_str == "true" || val_str == "on" || val_str == "1";
                        obj.insert(field.name.clone(), json!(b));
                    }
                    _ => {}
                }
            } else if field.field_type == "boolean" {
                // Checkboxes are not sent if unchecked
                obj.insert(field.name.clone(), json!(false));
            }
        }
    }

    // --- HOOK: before_save ---
    let arg = script_rhai::serde::to_dynamic(json!({
        "entity": &entity,
        "data": payload_val
    }))
    .unwrap_or(script_rhai::Dynamic::UNIT);

    if let Ok(res) = state
        .script_engine
        .call_hook(&module, "before_save", arg, state.clone())
        .await
    {
        // Если скрипт вернул данные, используем их
        if let Ok(new_data) = script_rhai::serde::from_dynamic::<serde_json::Value>(&res) {
            if let Some(d) = new_data.get("data") {
                payload_val = d.clone();
            }
        }
    }

    let id_val = payload_val.get(&primary_key);
    let res = if let Some(id) = id_val {
        let id = match id {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => "".to_string(),
        };

        if !id.is_empty() && id != "0" {
            crate::crud::update_record(db, &entity_meta.table_name, &primary_key, &id, &payload_val)
                .await
        } else {
            crate::crud::insert_record(db, &entity_meta.table_name, &payload_val).await
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
            .call_hook(&module, "after_save", arg_after, state.clone())
            .await;
    }

    match res {
        Ok(_) => Redirect::to(&format!("/admin/crud/{}/{}/list", module, entity)).into_response(),
        Err(e) => format!("Error saving: {}", e).into_response(),
    }
}

/// Handle delete action.
pub async fn delete_handle(
    State(state): State<Arc<crate::state::AppState>>,
    Path((module, entity, id)): Path<(String, String, String)>,
) -> Response {
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
        Ok(_) => Redirect::to(&format!("/admin/crud/{}/{}/list", module, entity)).into_response(),
        Err(e) => format!("Error deleting: {}", e).into_response(),
    }
}
