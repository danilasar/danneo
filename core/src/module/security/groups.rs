use crate::{models::core_admin_groups, state::AppState};
use axum::{
    Form,
    extract::State,
    response::{IntoResponse, Redirect},
};
use sea_orm::{ActiveModelTrait, EntityTrait, QueryOrder, Set};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct GroupSaveForm {
    pub id: Option<i32>,
    pub name: String,
    pub level: i32,
    #[serde(deserialize_with = "deserialize_vec_or_string")]
    pub permissions: Vec<String>,
}

fn deserialize_vec_or_string<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct VecOrString;

    impl<'de> serde::de::Visitor<'de> for VecOrString {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("string or list of strings")
        }

        fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(vec![s.to_owned()])
        }

        fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
        where
            S: serde::de::SeqAccess<'de>,
        {
            let mut v = Vec::new();
            while let Some(s) = seq.next_element()? {
                v.push(s);
            }
            Ok(v)
        }
    }

    deserializer.deserialize_any(VecOrString)
}

/// Список групп
pub async fn list_groups(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let groups = core_admin_groups::Entity::find()
        .order_by_asc(core_admin_groups::Column::Id)
        .all(state.db.as_ref())
        .await
        .unwrap_or_default();

    let mut context = tera::Context::new();
    context.insert("groups", &groups);

    crate::apanel::render_admin_template(state, "apanel/agroups_list.html", context).await
}

/// Форма редактирования/создания группы
pub async fn edit_group(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let id = params.get("id").and_then(|id| id.parse::<i32>().ok()).unwrap_or(0);
    let group = if id > 0 {
        core_admin_groups::Entity::find_by_id(id)
            .one(state.db.as_ref())
            .await
            .unwrap_or(None)
    } else {
        None
    };

    // Список доступных системных модулей
    let available_modules = vec![
        "dashboard",
        "settings",
        "design",
        "blocks",
        "menu",
        "security",
    ];

    // Получаем текущие политики группы из Casbin через RPC
    let mut current_permissions = Vec::new();
    if let Some(ref g) = group {
        let res = state.rpc_registry.call("casbin", "get_filtered_policy", serde_json::json!({
            "index": 0,
            "value": format!("role:{}", g.name)
        }), crate::rpc::RpcContext::default(), state.clone()).await;

        if let Ok(policies) = res {
            if let Some(policies_array) = policies.as_array() {
                for p in policies_array {
                    if let Some(p_array) = p.as_array() {
                        if p_array.len() >= 2 {
                            if let Some(obj) = p_array[1].as_str() {
                                current_permissions.push(obj.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    let mut context = tera::Context::new();
    context.insert("group", &group);
    context.insert("available_modules", &available_modules);
    context.insert("current_permissions", &current_permissions);

    crate::apanel::render_admin_template(state, "apanel/agroups_edit.html", context).await
}

/// Сохранение группы
pub async fn save_group(
    State(state): State<Arc<AppState>>,
    Form(form): Form<GroupSaveForm>,
) -> impl IntoResponse {
    let _group_id = if let Some(id) = form.id {
        // Обновление
        let group_res = core_admin_groups::Entity::find_by_id(id)
            .one(state.db.as_ref())
            .await
            .unwrap();
        
        if let Some(group_model) = group_res {
            let mut active_model: core_admin_groups::ActiveModel = group_model.into();
            let old_name = active_model.name.clone().unwrap();
            active_model.name = Set(form.name.clone());
            active_model.level = Set(form.level);
            let updated = active_model.update(state.db.as_ref()).await.unwrap();

            // Обновляем политики в Casbin через RPC
            state.rpc_registry.call("casbin", "remove_filtered_policy", serde_json::json!({
                "index": 0,
                "value": format!("role:{}", old_name)
            }), crate::rpc::RpcContext::default(), state.clone()).await.ok();

            updated.id
        } else {
            0
        }
    } else {
        // Создание
        let group = core_admin_groups::ActiveModel {
            name: Set(form.name.clone()),
            level: Set(form.level),
            ..Default::default()
        };
        let created = group.insert(state.db.as_ref()).await.unwrap();
        created.id
    };

    // 2. Добавляем новые политики через RPC
    for module in form.permissions {
        state.rpc_registry.call("casbin", "add_policy", serde_json::json!({
            "params": [format!("role:{}", form.name), module, "*", form.level.to_string()]
        }), crate::rpc::RpcContext::default(), state.clone()).await.ok();
    }

    Redirect::to("/admin/security/groups")
}

/// Удаление группы
pub async fn delete_group(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let id = params.get("id").and_then(|id| id.parse::<i32>().ok()).unwrap_or(0);
    if id == 1 {
        return Redirect::to("/admin/security/groups").into_response();
    }

    if let Some(group) = core_admin_groups::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await
        .unwrap()
    {
        // Удаляем политики из Casbin через RPC
        state.rpc_registry.call("casbin", "remove_filtered_policy", serde_json::json!({
            "index": 0,
            "value": format!("role:{}", group.name)
        }), crate::rpc::RpcContext::default(), state.clone()).await.ok();

        // Удаляем из БД
        let _ = core_admin_groups::Entity::delete_by_id(id)
            .exec(state.db.as_ref())
            .await;
    }

    Redirect::to("/admin/security/groups").into_response()
}
