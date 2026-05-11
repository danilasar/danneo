use crate::{auth::Claims, models::core_settings, state::AppState};
use axum::{
    Form,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use sea_orm::{EntityTrait, Set};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{collections::HashMap, sync::Arc};
use tera::Context;

const SEO_SETTINGS_KEY: &str = "seo_settings";
const SEO_SITEMAP_KEY: &str = "seo_sitemap";
const SEO_SOCIAL_KEY: &str = "seo_social";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeoModule {
    pub code: &'static str,
    pub title: &'static str,
    pub default_prefix: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct SeoModuleView {
    pub code: &'static str,
    pub title: &'static str,
    pub prefix: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeoSettings {
    pub rewrite: bool,
    pub cpu: bool,
    pub social_bookmark: bool,
    pub anchor: bool,
    pub prefixes: HashMap<String, String>,
}

impl Default for SeoSettings {
    fn default() -> Self {
        Self {
            rewrite: true,
            cpu: true,
            social_bookmark: true,
            anchor: false,
            prefixes: default_prefixes(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SitemapModuleSetting {
    pub code: String,
    pub title: String,
    pub add: bool,
    pub freq: String,
    pub prio: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SocialBookmark {
    pub posit: i32,
    pub link: String,
    pub icon: String,
    pub alt: String,
}

#[derive(Deserialize)]
pub struct SeoSettingsForm {
    pub rewrite: Option<String>,
    pub cpu: Option<String>,
    pub social_bookmark: Option<String>,
    pub anchor: Option<String>,
    pub prefix_news: String,
    pub prefix_article: String,
    pub prefix_down: String,
    pub prefix_info: String,
    pub prefix_photos: String,
    pub prefix_link: String,
    pub prefix_faq: String,
}

#[derive(Deserialize)]
pub struct SitemapForm {
    pub add_news: Option<String>,
    pub freq_news: String,
    pub prio_news: String,
    pub add_article: Option<String>,
    pub freq_article: String,
    pub prio_article: String,
    pub add_down: Option<String>,
    pub freq_down: String,
    pub prio_down: String,
    pub add_info: Option<String>,
    pub freq_info: String,
    pub prio_info: String,
    pub add_photos: Option<String>,
    pub freq_photos: String,
    pub prio_photos: String,
    pub add_link: Option<String>,
    pub freq_link: String,
    pub prio_link: String,
    pub add_faq: Option<String>,
    pub freq_faq: String,
    pub prio_faq: String,
}

#[derive(Deserialize)]
pub struct SocialForm {
    #[serde(
        default,
        deserialize_with = "crate::apanel::utils::empty_string_as_none"
    )]
    pub id: Option<usize>,
    pub link: String,
    pub icon: String,
    pub alt: String,
    pub posit: Option<i32>,
}

#[derive(Deserialize)]
pub struct SocialDeleteQuery {
    pub id: usize,
}

pub async fn show_settings(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let settings = load_json_setting::<SeoSettings>(&state, SEO_SETTINGS_KEY).await;
    let modules = seo_modules()
        .into_iter()
        .map(|module| SeoModuleView {
            code: module.code,
            title: module.title,
            prefix: settings
                .prefixes
                .get(module.code)
                .cloned()
                .unwrap_or_else(|| module.default_prefix.to_string()),
        })
        .collect::<Vec<_>>();

    let mut context = Context::new();
    insert_common(&mut context, &state, "settings");
    context.insert("seo_settings", &settings);
    context.insert("modules", &modules);

    crate::apanel::render_admin_template(state, "apanel/seo_settings.html", context).await
}

pub async fn save_settings(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    Form(form): Form<SeoSettingsForm>,
) -> impl IntoResponse {
    let settings = SeoSettings {
        rewrite: form.rewrite.is_some(),
        cpu: form.cpu.is_some(),
        social_bookmark: form.social_bookmark.is_some(),
        anchor: form.anchor.is_some(),
        prefixes: HashMap::from([
            (
                "news".to_string(),
                sanitize_prefix(&form.prefix_news, "news"),
            ),
            (
                "article".to_string(),
                sanitize_prefix(&form.prefix_article, "article"),
            ),
            (
                "down".to_string(),
                sanitize_prefix(&form.prefix_down, "down"),
            ),
            (
                "info".to_string(),
                sanitize_prefix(&form.prefix_info, "info"),
            ),
            (
                "photos".to_string(),
                sanitize_prefix(&form.prefix_photos, "photos"),
            ),
            (
                "link".to_string(),
                sanitize_prefix(&form.prefix_link, "link"),
            ),
            ("faq".to_string(), sanitize_prefix(&form.prefix_faq, "faq")),
        ]),
    };

    match save_json_setting(&state, SEO_SETTINGS_KEY, &settings).await {
        Ok(()) => Redirect::to("/admin/seo").into_response(),
        Err(e) => {
            tracing::error!("Failed to save SEO settings: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn show_sitemap(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let sitemap = load_sitemap_settings(&state).await;
    let mut context = Context::new();
    insert_common(&mut context, &state, "sitemap");
    context.insert("sitemap", &sitemap);
    context.insert("changefreq", &changefreq_values());
    context.insert("priorities", &priority_values());

    crate::apanel::render_admin_template(state, "apanel/seo_sitemap.html", context).await
}

pub async fn save_sitemap(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    Form(form): Form<SitemapForm>,
) -> impl IntoResponse {
    let sitemap = vec![
        sitemap_item(
            "news",
            "Новости",
            form.add_news,
            form.freq_news,
            form.prio_news,
        ),
        sitemap_item(
            "article",
            "Статьи",
            form.add_article,
            form.freq_article,
            form.prio_article,
        ),
        sitemap_item(
            "down",
            "Загрузки",
            form.add_down,
            form.freq_down,
            form.prio_down,
        ),
        sitemap_item(
            "info",
            "Инфостраницы",
            form.add_info,
            form.freq_info,
            form.prio_info,
        ),
        sitemap_item(
            "photos",
            "Фотографии",
            form.add_photos,
            form.freq_photos,
            form.prio_photos,
        ),
        sitemap_item(
            "link",
            "Ссылки",
            form.add_link,
            form.freq_link,
            form.prio_link,
        ),
        sitemap_item("faq", "FAQ", form.add_faq, form.freq_faq, form.prio_faq),
    ];

    if let Err(e) = save_json_setting(&state, SEO_SITEMAP_KEY, &sitemap).await {
        tracing::error!("Failed to save sitemap settings: {}", e);
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    if let Err(e) = write_sitemap_file(&state, &sitemap).await {
        tracing::error!("Failed to write sitemap.xml: {}", e);
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    Redirect::to("/admin/seo/sitemap").into_response()
}

pub async fn show_social(_claims: Claims, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let social = load_social(&state).await;
    let mut context = Context::new();
    insert_common(&mut context, &state, "social");
    context.insert("social", &social);

    crate::apanel::render_admin_template(state, "apanel/seo_social.html", context).await
}

pub async fn save_social(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    Form(form): Form<SocialForm>,
) -> impl IntoResponse {
    let mut social = load_social(&state).await;
    let item = SocialBookmark {
        posit: form.posit.unwrap_or(0),
        link: sanitize_social_link(&form.link),
        icon: strip_quotes(&form.icon),
        alt: strip_quotes(&form.alt),
    };

    if !item.link.contains("{link}") || item.icon.len() < 2 {
        return Redirect::to("/admin/seo/social").into_response();
    }

    if let Some(id) = form.id {
        if let Some(existing) = social.get_mut(id) {
            *existing = item;
        }
    } else {
        social.push(item);
    }

    social.sort_by_key(|item| item.posit);
    match save_json_setting(&state, SEO_SOCIAL_KEY, &social).await {
        Ok(()) => Redirect::to("/admin/seo/social").into_response(),
        Err(e) => {
            tracing::error!("Failed to save social bookmarks: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_social(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    Query(query): Query<SocialDeleteQuery>,
) -> impl IntoResponse {
    let mut social = load_social(&state).await;
    if query.id < social.len() {
        social.remove(query.id);
        if let Err(e) = save_json_setting(&state, SEO_SOCIAL_KEY, &social).await {
            tracing::error!("Failed to delete social bookmark: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
    Redirect::to("/admin/seo/social").into_response()
}

fn insert_common(context: &mut Context, _state: &AppState, active: &str) {
    context.insert("active_tab", active);
}

async fn load_json_setting<T>(state: &AppState, key: &str) -> T
where
    T: DeserializeOwned + Default,
{
    match core_settings::Entity::find_by_id(key.to_string())
        .one(state.db.as_ref())
        .await
    {
        Ok(Some(model)) => serde_json::from_value(model.value).unwrap_or_default(),
        Ok(None) => T::default(),
        Err(e) => {
            tracing::error!("Failed to load setting {}: {}", key, e);
            T::default()
        }
    }
}

async fn save_json_setting<T>(state: &AppState, key: &str, value: &T) -> Result<(), sea_orm::DbErr>
where
    T: Serialize,
{
    let active_model = core_settings::ActiveModel {
        key: Set(key.to_string()),
        value: Set(serde_json::to_value(value).unwrap_or(serde_json::Value::Null)),
    };

    core_settings::Entity::insert(active_model)
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(core_settings::Column::Key)
                .update_column(core_settings::Column::Value)
                .to_owned(),
        )
        .exec(state.db.as_ref())
        .await
        .map(|_| ())
}

async fn load_sitemap_settings(state: &AppState) -> Vec<SitemapModuleSetting> {
    let saved = load_json_setting::<Vec<SitemapModuleSetting>>(state, SEO_SITEMAP_KEY).await;
    if saved.is_empty() {
        return seo_modules()
            .into_iter()
            .map(|module| SitemapModuleSetting {
                code: module.code.to_string(),
                title: module.title.to_string(),
                add: false,
                freq: "never".to_string(),
                prio: "0.5".to_string(),
            })
            .collect();
    }
    saved
}

async fn load_social(state: &AppState) -> Vec<SocialBookmark> {
    load_json_setting::<Vec<SocialBookmark>>(state, SEO_SOCIAL_KEY).await
}

async fn write_sitemap_file(
    state: &AppState,
    sitemap: &[SitemapModuleSetting],
) -> Result<(), std::io::Error> {
    let settings = state.settings.read().await;
    let site_url = settings.site_url.trim_end_matches('/').to_string();
    drop(settings);

    let seo_settings = load_json_setting::<SeoSettings>(state, SEO_SETTINGS_KEY).await;
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" ?><urlset xmlns="http://www.google.com/schemas/sitemap/0.84">"#,
    );

    for item in sitemap.iter().filter(|item| item.add) {
        let prefix = seo_settings
            .prefixes
            .get(&item.code)
            .cloned()
            .unwrap_or_else(|| item.code.clone());
        let loc = if seo_settings.rewrite {
            format!("{}/{}/", site_url, prefix.trim_matches('/'))
        } else {
            format!("{}/index.php?dn={}", site_url, item.code)
        };
        xml.push_str(&format!(
            "<url><loc>{}</loc><changefreq>{}</changefreq><priority>{}</priority></url>",
            escape_xml(&loc),
            escape_xml(&item.freq),
            escape_xml(&item.prio)
        ));
    }

    xml.push_str("</urlset>");
    tokio::fs::write("sitemap.xml", xml).await
}

fn sitemap_item(
    code: &str,
    title: &str,
    add: Option<String>,
    freq: String,
    prio: String,
) -> SitemapModuleSetting {
    SitemapModuleSetting {
        code: code.to_string(),
        title: title.to_string(),
        add: add.is_some(),
        freq: normalize_choice(freq, &changefreq_values(), "never"),
        prio: normalize_choice(prio, &priority_values(), "0.5"),
    }
}

fn normalize_choice(value: String, allowed: &[&str], default: &str) -> String {
    if allowed.contains(&value.as_str()) {
        value
    } else {
        default.to_string()
    }
}

fn seo_modules() -> Vec<SeoModule> {
    vec![
        SeoModule {
            code: "news",
            title: "Новости",
            default_prefix: "news",
        },
        SeoModule {
            code: "article",
            title: "Статьи",
            default_prefix: "article",
        },
        SeoModule {
            code: "down",
            title: "Загрузки",
            default_prefix: "down",
        },
        SeoModule {
            code: "info",
            title: "Инфостраницы",
            default_prefix: "info",
        },
        SeoModule {
            code: "photos",
            title: "Фотографии",
            default_prefix: "photos",
        },
        SeoModule {
            code: "link",
            title: "Ссылки",
            default_prefix: "link",
        },
        SeoModule {
            code: "faq",
            title: "FAQ",
            default_prefix: "faq",
        },
    ]
}

fn default_prefixes() -> HashMap<String, String> {
    seo_modules()
        .into_iter()
        .map(|module| (module.code.to_string(), module.default_prefix.to_string()))
        .collect()
}

fn changefreq_values() -> Vec<&'static str> {
    vec![
        "always", "hourly", "daily", "weekly", "monthly", "yearly", "never",
    ]
}

fn priority_values() -> Vec<&'static str> {
    vec![
        "0.0", "0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "1.0",
    ]
}

fn sanitize_prefix(value: &str, fallback: &str) -> String {
    let slug = crate::utils::seo::generate_slug(value);
    if slug.is_empty() {
        fallback.to_string()
    } else {
        slug
    }
}

fn sanitize_social_link(value: &str) -> String {
    value.replace(['"', '\''], "")
}

fn strip_quotes(value: &str) -> String {
    value.replace(['"', '\''], "")
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('\'', "&#039;")
        .replace('"', "&quot;")
        .replace('>', "&gt;")
        .replace('<', "&lt;")
}
