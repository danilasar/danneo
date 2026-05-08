use danneo_core::models::core_blocks;
use sea_orm::{ActiveModelTrait, Database, EntityTrait, Set};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = Database::connect(&db_url)
        .await
        .expect("Failed to connect to database");

    let test_block = core_blocks::ActiveModel {
        positcode: Set("leftblock".to_string()),
        block_name: Set("Тестовый блок".to_string()),
        block_file: Set("sample_block".to_string()),
        block_active: Set(true),
        block_weight: Set(1),
        block_access: Set("all".to_string()),
        ..Default::default()
    };

    test_block
        .insert(&db)
        .await
        .expect("Failed to insert test block");
    println!("Test block inserted successfully!");
}
