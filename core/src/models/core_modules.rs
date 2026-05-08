use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "core_modules")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub code: String,
    pub name: String,
    pub version: String,
    pub package_id: i32,
    pub package_path: String,
    pub package_hash: String,
    pub package_signature: Option<String>,
    pub runtime_type: String,
    pub enabled: bool,
    pub installed: bool,
    pub position: i32,
    pub admin_enabled: bool,
    pub sitemap_enabled: bool,
    pub theme: Option<String>,
    pub manifest: Json,
    pub settings: Option<Json>,
    pub capabilities: Option<Json>,
    pub installed_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
