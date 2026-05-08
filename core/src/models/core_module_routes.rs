use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "core_module_routes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub module_code: String,
    pub route_name: String,
    pub method: String,
    pub path: String,
    pub handler: String,
    pub permission: Option<String>,
    pub descriptor: Json,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
