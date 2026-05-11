use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "core_block_definitions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub block_code: String,
    pub module_code: Option<String>,
    pub package_id: i32,
    pub version: String,
    pub enabled: bool,
    pub manifest: Json,
    pub settings_schema: Option<Json>,
    pub template_path: Option<String>,
    pub renderer_type: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
