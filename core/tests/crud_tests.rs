//! Tests for dynamic CRUD module

use sea_orm::{Database, DatabaseConnection, DbErr};
use serde_json::json;
use crate::crud::{self, EntitySchema, FieldSchema};

#[tokio::test]
async fn test_create_insert_select() -> Result<(), DbErr> {
    // In‑memory SQLite DB for fast testing
    let db: DatabaseConnection = Database::connect("sqlite::memory:").await?;

    // Define a simple entity schema
    let schema = EntitySchema {
        table_name: "test_items".to_string(),
        fields: vec![
            FieldSchema {
                name: "id".to_string(),
                field_type: "integer".to_string(),
                primary_key: true,
                auto_increment: true,
                nullable: false,
                unique: true,
                default: None,
            },
            FieldSchema {
                name: "title".to_string(),
                field_type: "string".to_string(),
                primary_key: false,
                auto_increment: false,
                nullable: false,
                unique: false,
                default: Some(json!("Untitled")),
            },
            FieldSchema {
                name: "active".to_string(),
                field_type: "boolean".to_string(),
                primary_key: false,
                auto_increment: false,
                nullable: false,
                unique: false,
                default: Some(json!(true)),
            },
        ],
    };

    // 1. Create table
    crud::create_entity_table(&db, &schema).await?;

    // 2. Insert a record
    let record = json!({
        "title": "First item",
        "active": false
    });
    let inserted = crud::insert_record(&db, "test_items", &record).await?;
    assert_eq!(inserted["title"], "First item");

    // 3. Select all rows – should contain the inserted row
    let rows = crud::select_all(&db, "test_items").await?;
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row["title"], "First item");
    assert_eq!(row["active"], false);
    Ok(())
}
