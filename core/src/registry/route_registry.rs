use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDescriptor {
    pub name: String,
    pub method: String,
    pub path: String,
    pub handler: String,
    pub entity: Option<String>,
    pub template: Option<String>,
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
