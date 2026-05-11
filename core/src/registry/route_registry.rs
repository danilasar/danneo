use async_trait::async_trait;
pub use danneo_sdk::registry::RouteDescriptor;
use mlua::{UserData, UserDataMethods};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct RouteRegistry {
    pub frontend_routes: Arc<RwLock<Vec<(String, RouteDescriptor)>>>,
    pub admin_routes: Arc<RwLock<Vec<(String, RouteDescriptor)>>>,
}

#[async_trait]
impl danneo_sdk::registry::IRouteRegistry for RouteRegistry {
    async fn register_frontend(&self, module_code: &str, descriptor: RouteDescriptor) {
        self.frontend_routes
            .write()
            .await
            .push((module_code.to_string(), descriptor));
    }
    async fn register_admin(&self, module_code: &str, descriptor: RouteDescriptor) {
        self.admin_routes
            .write()
            .await
            .push((module_code.to_string(), descriptor));
    }
    async fn clear_routes(&self) {
        self.frontend_routes.write().await.clear();
        self.admin_routes.write().await.clear();
    }
    async fn get_frontend_routes(&self) -> Vec<(String, RouteDescriptor)> {
        self.frontend_routes.read().await.clone()
    }
    async fn get_admin_routes(&self) -> Vec<(String, RouteDescriptor)> {
        self.admin_routes.read().await.clone()
    }
}

impl RouteRegistry {
    pub fn new() -> Self {
        Self {
            frontend_routes: Arc::new(RwLock::new(Vec::new())),
            admin_routes: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register_frontend(&self, module_code: &str, descriptor: RouteDescriptor) {
        self.frontend_routes
            .write()
            .await
            .push((module_code.to_string(), descriptor));
    }

    pub async fn register_admin(&self, module_code: &str, descriptor: RouteDescriptor) {
        self.admin_routes
            .write()
            .await
            .push((module_code.to_string(), descriptor));
    }
}

// Lua-style Router bridge
#[derive(Clone, Default, Debug)]
pub struct LuaRouter {
    pub routes: Vec<RouteDescriptor>,
}

impl UserData for LuaRouter {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("get", |_, this, (path, handler): (String, String)| {
            this.routes.push(RouteDescriptor {
                name: format!["get_{}", path.replace("/", "_")],
                method: "GET".to_string(),
                path,
                handler,
                entity: None,
                template: None,
                middlewares: vec![],
            });
            Ok(())
        });

        methods.add_method_mut("post", |_, this, (path, handler): (String, String)| {
            this.routes.push(RouteDescriptor {
                name: format!["post_{}", path.replace("/", "_")],
                method: "POST".to_string(),
                path,
                handler,
                entity: None,
                template: None,
                middlewares: vec![],
            });
            Ok(())
        });
    }
}
