use sea_orm::{Database, EntityTrait, ActiveModelTrait, Set, QueryFilter, ColumnTrait};
use danneo_core::models::{core_menu_groups, core_menu_items, core_blocks, core_block_posit};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = Database::connect(&db_url).await?;

    println!("Seeding menus...");
    
    // 1. Убедимся, что есть группа top_menu
    let group = if let Some(g) = core_menu_groups::Entity::find()
        .filter(core_menu_groups::Column::Code.eq("top_menu"))
        .one(&db).await? {
            g
        } else {
            let new_group = core_menu_groups::ActiveModel {
                code: Set("top_menu".to_string()),
                title: Set("Верхнее меню".to_string()),
                ..Default::default()
            };
            new_group.insert(&db).await?
        };

    // 2. Добавим пункты меню
    let items = vec![
        ("Главная", "/", 1),
        ("Новости", "/news", 2),
        ("О компании", "/about", 3),
        ("Контакты", "/contacts", 4),
    ];

    for (title, link, posit) in items {
        let exists = core_menu_items::Entity::find()
            .filter(core_menu_items::Column::GroupId.eq(group.id))
            .filter(core_menu_items::Column::Title.eq(title))
            .one(&db).await?;
            
        if exists.is_none() {
            let item = core_menu_items::ActiveModel {
                group_id: Set(group.id),
                parent_id: Set(0),
                title: Set(title.to_string()),
                link: Set(link.to_string()),
                target: Set("_self".to_string()),
                posit: Set(posit),
                acc: Set("all".to_string()),
                ..Default::default()
            };
            item.insert(&db).await?;
        }
    }

    println!("Seeding blocks...");

    // 3. Добавим блок меню в leftblock
    let exists = core_blocks::Entity::find()
        .filter(core_blocks::Column::BlockFile.eq("b-Menu"))
        .filter(core_blocks::Column::Positcode.eq("leftblock"))
        .one(&db).await?;

    if exists.is_none() {
        let block = core_blocks::ActiveModel {
            positcode: Set("leftblock".to_string()),
            block_name: Set("Навигация".to_string()),
            block_file: Set("b-Menu".to_string()),
            block_active: Set(true),
            block_weight: Set(1),
            block_setting: Set(Some(json!({"group_code": "top_menu"}))),
            block_access: Set("all".to_string()),
            ..Default::default()
        };
        block.insert(&db).await?;
    }

    // 4. Добавим Sample блок в rightblock
    let exists = core_blocks::Entity::find()
        .filter(core_blocks::Column::BlockFile.eq("sample_block"))
        .one(&db).await?;

    if exists.is_none() {
        let block = core_blocks::ActiveModel {
            positcode: Set("rightblock".to_string()),
            block_name: Set("Информация".to_string()),
            block_file: Set("sample_block".to_string()),
            block_active: Set(true),
            block_weight: Set(1),
            block_access: Set("all".to_string()),
            ..Default::default()
        };
        block.insert(&db).await?;
    }

    println!("Seeding completed!");
    Ok(())
}
