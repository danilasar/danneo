use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "core_menu_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub group_id: i32,
    pub parent_id: i32,
    pub title: String,
    pub link: String,
    pub target: String,
    pub css: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub posit: i32,
    pub acc: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::core_menu_groups::Entity",
        from = "Column::GroupId",
        to = "super::core_menu_groups::Column::Id"
    )]
    Group,
}

impl Related<super::core_menu_groups::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Group.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
