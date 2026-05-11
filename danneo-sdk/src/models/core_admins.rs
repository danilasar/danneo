use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "core_admins")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub login: String,
    pub password_hash: String,
    pub email: Option<String>,
    pub permissions: Option<Json>,
    pub group_id: Option<i32>,
    pub level: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::core_admin_groups::Entity",
        from = "Column::GroupId",
        to = "super::core_admin_groups::Column::Id"
    )]
    Group,
}

impl Related<super::core_admin_groups::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Group.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
