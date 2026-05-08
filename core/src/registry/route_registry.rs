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
    pub routes: Vec<(String, RouteDescriptor)>, // module_code, descriptor
}

impl RouteRegistry {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    pub fn register(&mut self, module_code: &str, descriptor: RouteDescriptor) {
        self.routes.push((module_code.to_string(), descriptor));
    }
}
