#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use danneo_core::registry::function_registry::FunctionRegistry;

    // 1. Test Native Function Registration (Inventory simulation)
    fn mock_native_fn(args: Value) -> Value {
        let val = args["val"].as_i64().unwrap_or(0);
        json!(val * 2)
    }

    #[tokio::test]
    async fn test_native_function_call() {
        let mut registry = FunctionRegistry::new();
        
        registry.register_native("test.double", mock_native_fn).await;

        let result = registry.call("test.double", json!({"val": 10})).await.unwrap();
        assert_eq!(result, json!(20));
    }

    // 2. Test Scripted/Dynamic Function Registration
    #[tokio::test]
    async fn test_scripted_function_call() {
        let registry = FunctionRegistry::new();
        
        registry.register_dynamic("lua.echo", |args| {
            Box::pin(async move {
                Ok(args)
            })
        }).await;

        let result = registry.call("lua.echo", json!({"hello": "world"})).await.unwrap();
        assert_eq!(result, json!({"hello": "world"}));
    }

    // 3. Test Function Not Found
    #[tokio::test]
    async fn test_function_not_found() {
        let registry = FunctionRegistry::new();
        let result = registry.call("non.existent", json!({})).await;
        assert!(result.is_err());
    }
}
