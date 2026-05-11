use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type NativeFunction = fn(Value) -> Value;
pub type DynamicFunction =
    Box<dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<Value, String>> + Send>> + Send + Sync>;

pub struct NativeFunctionDescriptor {
    pub name: &'static str,
    pub func: NativeFunction,
}

inventory::collect!(NativeFunctionDescriptor);

pub struct FunctionRegistry {
    native_functions: HashMap<String, NativeFunction>,
    dynamic_functions: Arc<RwLock<HashMap<String, DynamicFunction>>>,
}

impl FunctionRegistry {
    pub fn new() -> Self {
        let mut native_map = HashMap::new();
        for reg in inventory::iter::<NativeFunctionDescriptor> {
            native_map.insert(reg.name.to_string(), reg.func);
        }

        Self {
            native_functions: native_map,
            dynamic_functions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_native(&mut self, name: &str, func: NativeFunction) {
        self.native_functions.insert(name.to_string(), func);
    }

    pub async fn register_dynamic<F>(&self, name: &str, func: F)
    where
        F: Fn(Value) -> Pin<Box<dyn Future<Output = Result<Value, String>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        let mut guard = self.dynamic_functions.write().await;
        guard.insert(name.to_string(), Box::new(func));
    }

    pub async fn call(&self, name: &str, args: Value) -> Result<Value, String> {
        if let Some(func) = self.native_functions.get(name) {
            return Ok(func(args));
        }

        let guard = self.dynamic_functions.read().await;
        if let Some(func) = guard.get(name) {
            return func(args).await;
        }

        Err(format!("Function '{}' not found in registry", name))
    }
}
