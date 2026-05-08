use danneo_core::registry::script_engine::ScriptEngine;
use script_rhai::Dynamic;
use sea_orm::Database;
use std::sync::Arc;

#[tokio::test]
async fn test_module_db_api_isolation() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let db_arc = Arc::new(db);

    let engine = ScriptEngine::new(db_arc.clone());

    // Script for module "mod1"
    let script1 = r#"
        function setup(arg)
            db.create_table({
                table_name = "items",
                fields = {
                    { name = "id", field_type = "integer", primary_key = true },
                    { name = "val", field_type = "string" },
                },
            })
            db.insert("items", { id = 1, val = "from mod1" })
        end
        function get_items(arg)
            return db.select("items", { "id", "val" })
        end
    "#;

    // Script for module "mod2"
    let script2 = r#"
        function setup(arg)
            db.create_table({
                table_name = "items",
                fields = {
                    { name = "id", field_type = "integer", primary_key = true },
                    { name = "val", field_type = "string" },
                },
            })
            db.insert("items", { id = 1, val = "from mod2" })
        end
        function get_items(arg)
            return db.select("items", { "id", "val" })
        end
    "#;

    engine.load_script_str("mod1", script1).await.unwrap();
    engine.load_script_str("mod2", script2).await.unwrap();

    // Setup both
    let _ = engine
        .call_hook("mod1", "setup", Dynamic::UNIT)
        .await
        .unwrap();
    let _ = engine
        .call_hook("mod2", "setup", Dynamic::UNIT)
        .await
        .unwrap();

    // Verify mod1 sees its data
    let res1 = engine
        .call_hook("mod1", "get_items", Dynamic::UNIT)
        .await
        .unwrap();
    let items1: serde_json::Value = script_rhai::serde::from_dynamic(&res1).unwrap();
    assert_eq!(items1[0]["val"], "from mod1");

    // Verify mod2 sees its data
    let res2 = engine
        .call_hook("mod2", "get_items", Dynamic::UNIT)
        .await
        .unwrap();
    let items2: serde_json::Value = script_rhai::serde::from_dynamic(&res2).unwrap();
    assert_eq!(items2[0]["val"], "from mod2");

    // Verify physical tables exist with prefixes
    use sea_orm::ConnectionTrait;
    let backend = db_arc.get_database_backend();
    let stmt1 =
        sea_orm::Statement::from_string(backend, "SELECT val FROM mod_mod1_items WHERE id=1");
    let row1 = db_arc.query_one(stmt1).await.unwrap().unwrap();
    let v1: String = row1.try_get("", "val").unwrap();
    assert_eq!(v1, "from mod1");

    let stmt2 =
        sea_orm::Statement::from_string(backend, "SELECT val FROM mod_mod2_items WHERE id=1");
    let row2 = db_arc.query_one(stmt2).await.unwrap().unwrap();
    let v2: String = row2.try_get("", "val").unwrap();
    assert_eq!(v2, "from mod2");
}

#[tokio::test]
async fn test_db_update_delete() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let db_arc = Arc::new(db);
    let engine = ScriptEngine::new(db_arc.clone());

    let script = r#"
        function test(arg)
            db.create_table({
                table_name = "data",
                fields = {
                    { name = "id", field_type = "integer", primary_key = true },
                    { name = "name", field_type = "string" },
                },
            })
            db.insert("data", { id = 1, name = "initial" })
            db.update("data", "id", "1", { name = "updated" })
            local res = db.select("data", { "name" })
            db.delete("data", "id", "1")
            local res2 = db.select("data", { "name" })
            return { res[1].name, #res2 }
        end
    "#;

    engine.load_script_str("test_mod", script).await.unwrap();
    let res = engine
        .call_hook("test_mod", "test", Dynamic::UNIT)
        .await
        .unwrap();
    let arr = res.try_cast::<script_rhai::Array>().unwrap();
    assert_eq!(arr[0].to_string(), "updated");
    assert_eq!(arr[1].as_int().unwrap(), 0);
}
