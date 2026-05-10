use crate::module::DanneoModule;
use crate::state::AppState;
use async_trait::async_trait;
use axum::{Router, routing::{get, post}};
use std::sync::Arc;
use sea_orm::DatabaseConnection;

pub mod migrations;

pub struct SecurityModule;

crate::inventory::submit! {
    migration::ModuleMigrationRegistration { migration: &migrations::CreateAdminTable }
}

crate::inventory::submit! {
    migration::ModuleMigrationRegistration { migration: &migrations::UpdateCoreAdmins }
}

crate::inventory::submit! {
    migration::ModuleMigrationRegistration { migration: &migrations::AddAdminGroupsAndLevels }
}

impl SecurityModule {
    pub fn new(_db: Arc<DatabaseConnection>) -> Self {
        Self
    }
}

#[async_trait]
impl DanneoModule for SecurityModule {
    fn name(&self) -> &'static str {
        "security"
    }

    async fn init(&self, state: Arc<AppState>) -> Result<(), String> {
        // Register in Admin Menu
        state.rpc_registry.call(
            "admin_menu",
            "register_items",
            serde_json::json!({
                "module": "security",
                "items": [
                    {
                        "code": "admins",
                        "category": "security",
                        "label": "admin_amanage",
                        "link": "/admin/security/admins",
                        "weight": 10
                    },
                    {
                        "code": "groups",
                        "category": "security",
                        "label": "admin_agroups",
                        "link": "/admin/security/groups",
                        "weight": 20
                    }
                ]
            }),
            crate::rpc::RpcContext::default(),
            state.clone()
        ).await.ok();
        Ok(())
    }

    fn register_admin_routes(&self) -> Router<Arc<AppState>> {
        Router::new()
            .route("/admins", get(crate::apanel::amanage::list_admins))
            .route("/groups", get(crate::apanel::agroups::list_groups))
    }
}

crate::register_native_module!("security", |db| Arc::new(SecurityModule::new(db)));
