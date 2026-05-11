use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "core_blocks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub positcode: String,
    pub block_name: String,
    pub block_file: String,
    pub block_active: bool,
    pub block_weight: i32,
    pub block_temp: Option<String>,
    pub block_mods: Option<Json>,
    pub block_access: String,
    pub block_setting: Option<Json>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::core_block_posit::Entity",
        from = "Column::Positcode",
        to = "super::core_block_posit::Column::Positcode"
    )]
    Posit,
}

impl Related<super::core_block_posit::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Posit.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
