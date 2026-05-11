use axum::{
    extract::{Form, Json, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
};
use danneo_sdk::{
    apanel::render_admin_template,
    auth::{AuthService, Claims},
    models::{core_admin_groups, core_admins},
    state::AppState,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tera::Context;

#[derive(Deserialize)]
pub struct AdminForm {
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

    render_admin_template(state, "apanel/amanage_list.html", context).await
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

    render_admin_template(state, "apanel/amanage_edit.html", context).await
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
    } else if let Some(pass) = form.password {
        let hash = bcrypt::hash(pass, 4).unwrap();
        active_model.password_hash = Set(hash);
    }

    match active_model.save(db).await {
        Ok(_) => Redirect::to("/admin/security/admins").into_response(),
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
            let _ = core_admins::Entity::delete_by_id(id).exec(db).await;
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
    let mut context = Context::new();
    context.insert("site_name", &settings.site_name);

    match state.tera.render("apanel/login.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Template error: {}", e);
            Html("<h1>Internal Server Error</h1>".to_string()).into_response()
        }
    }
}
