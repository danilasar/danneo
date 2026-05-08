use sea_orm::entity::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "core_module_entities")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub module_code: String,
    pub entity_name: String,
    pub table_name: String,
    pub schema: Json,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
