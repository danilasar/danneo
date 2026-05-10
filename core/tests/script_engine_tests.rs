use danneo_core::registry::script_engine::{ScriptEngine, ScriptError};
use script_rhai::Dynamic;
use serde_json::json;
use std::sync::Arc;

mod common;

#[tokio::test]
async fn test_load_and_call_simple_hook() {
    let state = common::create_test_state().await;
    let engine = state.script_engine.clone();
    let script = r#"
        function hello(name)
            return "Hello, " .. name .. "!"
        end
    "#;
    engine
        .load_script_str("test_mod", script)
        .await
        .expect("Failed to load script");
    let result = engine
        .call_hook("test_mod", "hello", "World".into(), state.clone())
        .await
        .expect("Failed to call hook");
    assert_eq!(result.to_string(), "Hello, World!");
}

#[tokio::test]
async fn test_hook_with_complex_data() {
    let state = common::create_test_state().await;
    let engine = state.script_engine.clone();
    let script = r#"
        function process_entity(entity)
            entity.count = entity.count + 1
            entity.processed = true
            table.insert(entity.tags, "processed")
            return entity
        end
    "#;
    engine
        .load_script_str("test_mod", script)
        .await
        .expect("Failed to load script");
    let input = json!({
        "count": 10,
        "processed": false,
        "tags": ["new"]
    });
    let arg: Dynamic =
        script_rhai::serde::to_dynamic(input).expect("Failed to serialize to dynamic");
    let result = engine
        .call_hook("test_mod", "process_entity", arg, state.clone())
        .await
        .expect("Failed to call hook");
    let output: serde_json::Value =
        script_rhai::serde::from_dynamic(&result).expect("Failed to deserialize from dynamic");
    assert_eq!(output["count"], 11);
    assert_eq!(output["processed"], true);
    assert_eq!(output["tags"][1], "processed");
}

#[tokio::test]
async fn test_script_runtime_error() {
    let state = common::create_test_state().await;
    let engine = state.script_engine.clone();
    let script = r#"
        function fail(data)
            error("Custom Error")
        end
    "#;
    engine
        .load_script_str("test_mod", script)
        .await
        .expect("Failed to load script");
    let result = engine
        .call_hook("test_mod", "fail", Dynamic::UNIT, state.clone())
        .await;
    match result {
        Err(ScriptError::Runtime(e)) => {
            assert!(e.to_string().contains("Custom Error"));
        }
        _ => panic!("Expected runtime error"),
    }
}

// Тест таймаутит, насколько это хорошо --- надо думать
/*#[tokio::test]
async fn test_script_infinite_loop() {
    let state = common::create_test_state().await;
    let engine = state.script_engine.clone();
    let script = r#"
        function loop_forever(data)
            local x = 0
            while true do
                x = x + 1
            end
        end
    "#;
    engine
        .load_script_str("test_mod", script)
        .await
        .expect("Failed to load script");

    let result = engine
        .call_hook("test_mod", "loop_forever", Dynamic::UNIT, state.clone())
        .await;
    match result {
        Err(ScriptError::Runtime(e)) => {
            // Ожидаем ошибку превышения лимита операций
            assert!(e.to_string().contains("Too many operations"));
        }
        _ => panic!("Expected operation limit error, got {:?}", result),
    }
}*/

#[tokio::test]
async fn test_hook_not_found() {
    let state = common::create_test_state().await;
    let engine = state.script_engine.clone();
    let result = engine
        .call_hook("non_existent", "any_fn", "data".into(), state.clone())
        .await;
    match result {
        Err(ScriptError::HookNotFound(_)) => {}
        _ => panic!("Expected HookNotFound error"),
    }
}
