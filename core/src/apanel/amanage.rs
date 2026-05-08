use crate::{auth::Claims, models::core_admin_groups, models::core_admins, state::AppState};
use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use sea_orm::{ActiveModelTrait, EntityTrait, QueryOrder, Set};
use serde::Deserialize;
use std::sync::Arc;
use tera::Context;

#[derive(Deserialize)]
pub struct AdminForm {
    #[serde(
        default,
        deserialize_with = "crate::apanel::utils::empty_string_as_none"
    )]
    pub id: Option<i32>,
    pub login: String,
    pub email: String,
    pub password: Option<String>,
    pub group_id: Option<i32>,
    pub level: i32,
}

pub async fn list_admins(_claims: Claims, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let db = state.db.as_ref();

    let admins = match core_admins::Entity::find()
        .order_by_asc(core_admins::Column::Login)
        .all(db)
        .await
    {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Failed to fetch admins: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let mut context = Context::new();
    context.insert("admins", &admins);

    crate::apanel::render_admin_template(state, "apanel/amanage_list.html", context).await
}

pub async fn edit_admin(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let db = state.db.as_ref();
    let id = params.get("id").and_then(|id| id.parse::<i32>().ok());

    let admin = if let Some(id) = id {
        match core_admins::Entity::find_by_id(id).one(db).await {
            Ok(Some(a)) => Some(a),
            _ => None,
        }
    } else {
        None
    };

    let groups = core_admin_groups::Entity::find()
        .all(db)
        .await
        .unwrap_or_default();

    let mut context = Context::new();
    context.insert("admin", &admin);
    context.insert("groups", &groups);

    crate::apanel::render_admin_template(state, "apanel/amanage_edit.html", context).await
}

pub async fn save_admin(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    Form(form): Form<AdminForm>,
) -> impl IntoResponse {
    let db = state.db.as_ref();

    let mut active_model = core_admins::ActiveModel {
        login: Set(form.login.clone()),
        email: Set(Some(form.email)),
        group_id: Set(form.group_id),
        level: Set(form.level),
        permissions: Set(serde_json::json!([])), // Теперь пусто, всё в Casbin
        ..Default::default()
    };

    if let Some(id) = form.id {
        // Редактирование
        active_model.id = Set(id);
        if let Some(pass) = form.password {
            if !pass.is_empty() {
                let hash = bcrypt::hash(pass, 4).unwrap();
                active_model.password_hash = Set(hash);
            }
        }

        let old_admin = core_admins::Entity::find_by_id(id)
            .one(db)
            .await
            .unwrap()
            .unwrap();
        // Удаляем старые роли в Casbin для этого пользователя
        state.acl.remove_filtered_policy(0, &old_admin.login).await;
        // В Casbin (g, sub, role) sub - это login.
    } else {
        // Создание
        let pass = form.password.unwrap_or_default();
        let hash = bcrypt::hash(pass, 4).unwrap();
        active_model.password_hash = Set(hash);
    }

    match active_model.save(db).await {
        Ok(_saved) => {
            // Привязываем к группе в Casbin
            if let Some(gid) = form.group_id {
                if let Some(group) = core_admin_groups::Entity::find_by_id(gid)
                    .one(db)
                    .await
                    .unwrap()
                {
                    state
                        .acl
                        .add_role_for_user(&form.login, &format!("role:{}", group.name))
                        .await;
                }
            }
            Redirect::to("/admin/amanage").into_response()
        }
        Err(e) => {
            tracing::error!("Failed to save admin: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_admin(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let db = state.db.as_ref();
    let id = params.get("id").and_then(|id| id.parse::<i32>().ok());

    if let Some(id) = id {
        if id != 1 {
            if let Some(admin) = core_admins::Entity::find_by_id(id).one(db).await.unwrap() {
                // Удаляем роли в Casbin
                state.acl.remove_filtered_policy(0, &admin.login).await;
                // Удаляем из БД
                let _ = core_admins::Entity::delete_by_id(id).exec(db).await;
            }
        }
    }

    Redirect::to("/admin/amanage").into_response()
}
