use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "core_admin_groups")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub name: String,
    pub level: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::core_admins::Entity")]
    Admins,
}

impl Related<super::core_admins::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Admins.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
