use crate::models::core_admin_groups;
use crate::state::AppState;
use axum::{
    Form,
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect},
};
use casbin::{CoreApi, MgmtApi};
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
    let settings = state.settings.read().await;
    context.insert("site_name", &settings.site_name);
    context.insert("groups", &groups);

    let html = state
        .tera
        .render("apanel/agroups_list.html", &context)
        .unwrap();
    Html(html)
}

/// Форма редактирования/создания группы
pub async fn edit_group(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
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
        "amanage",
        "agroups",
    ];

    // Получаем текущие политики группы из Casbin
    let mut current_permissions = Vec::new();
    if let Some(ref g) = group {
        let enforcer = state.acl.enforcer();
        let e = enforcer.read().await;
        let policies = e.get_filtered_policy(0, vec![format!("role:{}", g.name)]);
        for p in policies {
            if p.len() >= 2 {
                current_permissions.push(p[1].clone()); // obj
            }
        }
    }

    let mut context = tera::Context::new();
    let settings = state.settings.read().await;
    context.insert("site_name", &settings.site_name);
    context.insert("group", &group);
    context.insert("available_modules", &available_modules);
    context.insert("current_permissions", &current_permissions);

    let html = state
        .tera
        .render("apanel/agroups_edit.html", &context)
        .unwrap();
    Html(html)
}

/// Сохранение группы
pub async fn save_group(
    State(state): State<Arc<AppState>>,
    Form(form): Form<GroupSaveForm>,
) -> impl IntoResponse {
    let _group_id = if let Some(id) = form.id {
        // Обновление
        let mut group: core_admin_groups::ActiveModel = core_admin_groups::Entity::find_by_id(id)
            .one(state.db.as_ref())
            .await
            .unwrap()
            .unwrap()
            .into();

        let old_name = group.name.clone().unwrap();
        group.name = Set(form.name.clone());
        group.level = Set(form.level);
        let updated = group.update(state.db.as_ref()).await.unwrap();

        // Обновляем политики в Casbin
        // 1. Удаляем старые
        state
            .acl
            .remove_filtered_policy(0, &format!("role:{}", old_name))
            .await;

        updated.id
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

    // 2. Добавляем новые политики
    for module in form.permissions {
        // p, role:name, module, *, level
        state
            .acl
            .add_policy(&format!("role:{}", form.name), &module, "*", form.level)
            .await;
    }

    Redirect::to("/admin/agroups")
}

/// Удаление группы
pub async fn delete_group(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    if id == 1 {
        // Запрет удаления группы SuperAdmins (ID=1)
        return Redirect::to("/admin/agroups");
    }

    if let Some(group) = core_admin_groups::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await
        .unwrap()
    {
        // Удаляем политики из Casbin
        state
            .acl
            .remove_filtered_policy(0, &format!("role:{}", group.name))
            .await;

        // Удаляем из БД
        let _ = core_admin_groups::Entity::delete_by_id(id)
            .exec(state.db.as_ref())
            .await;
    }

    Redirect::to("/admin/agroups")
}
