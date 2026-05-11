use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use danneo_sdk::{auth::Claims, models::core_settings, state::AppState};
use sea_orm::{EntityTrait, Set};
use serde::Deserialize;
use std::sync::Arc;
use tera::Context;

#[derive(Deserialize)]
pub struct SettingsForm {
    pub site_name: String,
    pub admin_email: String,
    pub site_url: String,
    pub site_temp: String,
}

pub async fn show_settings(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut context = Context::new();
    {
        let settings = state.settings.read().await;
        context.insert("admin_email", &settings.admin_email);
        context.insert("site_url", &settings.site_url);
        context.insert("site_temp", &settings.site_temp);
    }

    let themes = vec!["Soft", "Old", "Clear"];
    context.insert("themes", &themes);

    danneo_sdk::apanel::render_admin_template(state, "apanel/settings.html", context).await
}

pub async fn save_settings(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    Form(form): Form<SettingsForm>,
) -> impl IntoResponse {
    let db = state.db.as_ref();

    let updates = vec![
        ("site_name", &form.site_name),
        ("admin_email", &form.admin_email),
        ("site_url", &form.site_url),
        ("site_temp", &form.site_temp),
    ];

    for (key, value) in updates {
        let active_model = core_settings::ActiveModel {
            key: Set(key.to_string()),
            value: Set(serde_json::json!(value)),
        };

        if let Err(e) = core_settings::Entity::insert(active_model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(core_settings::Column::Key)
                    .update_column(core_settings::Column::Value)
                    .to_owned(),
            )
            .exec(db)
            .await
        {
            tracing::error!("Failed to save setting {}: {:?}", key, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("DB Error: {}", e),
            )
                .into_response();
        }
    }

    {
        let mut settings = state.settings.write().await;
        settings.site_name = form.site_name;
        settings.admin_email = form.admin_email;
        settings.site_url = form.site_url;
        settings.site_temp = form.site_temp;
    }

    Redirect::to("/admin/settings").into_response()
}
