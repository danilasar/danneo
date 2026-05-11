use serde::{Deserialize, Serialize};
use mlua::{UserData, UserDataMethods};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDescriptor {
    pub name: String,
    pub method: String,
    pub path: String,
    pub handler: String,
    pub entity: Option<String>,
    pub template: Option<String>,
    pub middlewares: Vec<String>,
}

pub struct RouteRegistry {
    pub frontend_routes: Vec<(String, RouteDescriptor)>, // module_code, descriptor
    pub admin_routes: Vec<(String, RouteDescriptor)>,    // module_code, descriptor
}

impl RouteRegistry {
    pub fn new() -> Self {
        Self { 
            frontend_routes: Vec::new(),
            admin_routes: Vec::new(),
        }
    }

    pub fn register_frontend(&mut self, module_code: &str, descriptor: RouteDescriptor) {
        self.frontend_routes.push((module_code.to_string(), descriptor));
    }

    pub fn register_admin(&mut self, module_code: &str, descriptor: RouteDescriptor) {
        self.admin_routes.push((module_code.to_string(), descriptor));
    }
}

// Lua-style Router bridge
#[derive(Clone, Default, Debug)]
pub struct LuaRouter {
    pub routes: Vec<RouteDescriptor>,
    pub prefix: String,
}

impl mlua::FromLua for LuaRouter {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(ud.borrow::<LuaRouter>()?.clone()),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "LuaRouter".to_string(),
                message: Some("Expected LuaRouter UserData".to_string()),
            }),
        }
    }
}

impl UserData for LuaRouter {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("get", |_, this, (path, handler): (String, String)| {
            this.routes.push(RouteDescriptor {
                name: format!("get_{}", path.replace('/', "_").trim_matches('_')),
                method: "GET".to_string(),
                path,
                handler,
                entity: None,
                template: None,
                middlewares: vec![],
            });
            Ok(this.clone())
        });

        methods.add_method_mut("post", |_, this, (path, handler): (String, String)| {
            this.routes.push(RouteDescriptor {
                name: format!("post_{}", path.replace('/', "_").trim_matches('_')),
                method: "POST".to_string(),
                path,
                handler,
                entity: None,
                template: None,
                middlewares: vec![],
            });
            Ok(this.clone())
        });

        methods.add_method_mut("put", |_, this, (path, handler): (String, String)| {
            this.routes.push(RouteDescriptor {
                name: format!("put_{}", path.replace('/', "_").trim_matches('_')),
                method: "PUT".to_string(),
                path,
                handler,
                entity: None,
                template: None,
                middlewares: vec![],
            });
            Ok(this.clone())
        });

        methods.add_method_mut("delete", |_, this, (path, handler): (String, String)| {
            this.routes.push(RouteDescriptor {
                name: format!("delete_{}", path.replace('/', "_").trim_matches('_')),
                method: "DELETE".to_string(),
                path,
                handler,
                entity: None,
                template: None,
                middlewares: vec![],
            });
            Ok(this.clone())
        });

        methods.add_method_mut("middleware", |_, this, middleware: String| {
            if let Some(route) = this.routes.last_mut() {
                route.middlewares.push(middleware);
            }
            Ok(this.clone())
        });

        methods.add_method_mut("nest", |_, this, (path, other): (String, LuaRouter)| {
            for mut route in other.routes {
                let base = path.trim_matches('/');
                let sub = route.path.trim_matches('/');
                route.path = if sub.is_empty() {
                    format!("/{}", base)
                } else {
                    format!("/{}", [base, sub].join("/"))
                };
                this.routes.push(route);
            }
            Ok(this.clone())
        });
    }
}
