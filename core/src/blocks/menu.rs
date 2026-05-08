use crate::blocks::{BlockContext, DanneoBlock};
use crate::models::{core_menu_groups, core_menu_items};
use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use serde_json::Value;
use std::sync::Arc;

pub struct MenuBlock;

#[async_trait]
impl DanneoBlock for MenuBlock {
    fn identifier(&self) -> &'static str {
        "b-Menu"
    }

    async fn render(&self, ctx: Arc<BlockContext>, settings: Option<Value>) -> String {
        let group_code = settings
            .as_ref()
            .and_then(|s| s.get("group_code"))
            .and_then(|v| v.as_str())
            .unwrap_or("top_menu");

        render_menu(ctx.db.as_ref(), group_code).await
    }
}

pub async fn render_menu(db: &DatabaseConnection, group_code: &str) -> String {
    // Находим группу
    let group = match core_menu_groups::Entity::find()
        .filter(core_menu_groups::Column::Code.eq(group_code))
        .one(db)
        .await
    {
        Ok(Some(g)) => g,
        _ => return format!("<!-- Menu group '{}' not found -->", group_code),
    };

    // Находим пункты
    let items = match core_menu_items::Entity::find()
        .filter(core_menu_items::Column::GroupId.eq(group.id))
        .order_by_asc(core_menu_items::Column::Posit)
        .all(db)
        .await
    {
        Ok(i) => i,
        Err(_) => return "<!-- Failed to fetch menu items -->".to_string(),
    };

    if items.is_empty() {
        return "<!-- Menu is empty -->".to_string();
    }

    // Строим простое дерево
    let mut html = String::from("<ul class=\"menu\">\n");
    for item in items.iter().filter(|i| i.parent_id == 0) {
        html.push_str(&format!(
            "  <li class=\"menu-item\"><a href=\"{}\" target=\"{}\">{}</a>",
            item.link, item.target, item.title
        ));

        let children: Vec<_> = items.iter().filter(|i| i.parent_id == item.id).collect();
        if !children.is_empty() {
            html.push_str("\n    <ul class=\"sub-menu\">\n");
            for child in children {
                html.push_str(&format!(
                    "      <li class=\"sub-menu-item\"><a href=\"{}\" target=\"{}\">{}</a></li>\n",
                    child.link, child.target, child.title
                ));
            }
            html.push_str("    </ul>\n  ");
        }
        html.push_str("</li>\n");
    }
    html.push_str("</ul>");
    html
}
