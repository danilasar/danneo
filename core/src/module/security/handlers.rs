use crate::{auth::Claims, models::core_admin_groups, models::core_admins, state::AppState};
use axum::{
    extract::{Form, State, Json},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use sea_orm::{ActiveModelTrait, EntityTrait, QueryOrder, Set, ColumnTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tera::Context;
use crate::auth::AuthService;

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
        permissions: Set(Some(serde_json::json!([]))),
        ..Default::default()
    };

    if let Some(id) = form.id {
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
        
        // Use RPC to Casbin module
        state.rpc_registry.call("casbin", "remove_filtered_policy", serde_json::json!({
            "index": 0,
            "value": old_admin.login
        }), crate::rpc::RpcContext::default(), state.clone()).await.ok();
    } else {
        let pass = form.password.unwrap_or_default();
        let hash = bcrypt::hash(pass, 4).unwrap();
        active_model.password_hash = Set(hash);
    }

    match active_model.save(db).await {
        Ok(_saved) => {
            if let Some(gid) = form.group_id {
                if let Some(group) = core_admin_groups::Entity::find_by_id(gid)
                    .one(db)
                    .await
                    .unwrap()
                {
                    state.rpc_registry.call("casbin", "add_role_for_user", serde_json::json!({
                        "user": form.login,
                        "role": format!("role:{}", group.name)
                    }), crate::rpc::RpcContext::default(), state.clone()).await.ok();
                }
            }
            Redirect::to("/admin/security/admins").into_response()
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
                state.rpc_registry.call("casbin", "remove_filtered_policy", serde_json::json!({
                    "index": 0,
                    "value": admin.login
                }), crate::rpc::RpcContext::default(), state.clone()).await.ok();
                let _ = core_admins::Entity::delete_by_id(id).exec(db).await;
            }
        }
    }

    Redirect::to("/admin/security/admins").into_response()
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub login: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}

pub async fn admin_login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let admin = core_admins::Entity::find()
        .filter(core_admins::Column::Login.eq(&payload.login))
        .one(state.db.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let admin = admin.ok_or(StatusCode::UNAUTHORIZED)?;
    let is_valid = bcrypt::verify(&payload.password, &admin.password_hash).unwrap_or(false);

    if !is_valid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let auth_service = AuthService::new(state.jwt_secret.clone());
    let now = chrono::Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + chrono::Duration::hours(24)).timestamp() as usize;

    let token = auth_service
        .create_token(admin.id, exp, iat)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse { token }))
}

pub async fn show_login_page(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let settings = state.settings.read().await;
    let mut context = tera::Context::new();
    context.insert("site_name", &settings.site_name);

    match state.tera.render("apanel/login.html", &context) {
        Ok(html) => axum::response::Html(html),
        Err(e) => {
            tracing::error!("Template error: {}", e);
            axum::response::Html("<h1>Internal Server Error</h1>".to_string())
        }
    }
}
