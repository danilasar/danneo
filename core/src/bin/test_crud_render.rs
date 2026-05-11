use sea_orm::Database;
use serde_json::json;
use tera::Context;

#[tokio::main]
async fn main() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    use sea_orm_migration::MigratorTrait;
    migration::Migrator::up(&db, None).await.unwrap();
    let state = danneo_core::state::init_state(db).await.unwrap();

    let mut ctx = Context::new();
    danneo_core::apanel::prepare_admin_context(state.clone(), &mut ctx).await;

    let schema = json!({
        "table_name": "test",
        "fields": [
            {
                "name": "id",
                "field_type": "integer",
                "primary_key": true
            },
            {
                "name": "message",
                "field_type": "string",
                "nullable": false
            }
        ]
    });

    ctx.insert("module", "mod_hello");
    ctx.insert("entity", "test_entity");
    ctx.insert("schema", &schema);
    ctx.insert("primary_key", "id");
    ctx.insert("record", &json!({}));

    match state.tera.render("apanel/crud_edit.html", &ctx) {
        Ok(_) => println!("Render successful!"),
        Err(e) => println!("Render failed: {}", e),
    }
}
