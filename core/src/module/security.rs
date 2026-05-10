use crate::module::DanneoModule;
use crate::state::AppState;
use async_trait::async_trait;
use axum::{Router, routing::{get, post}};
use std::sync::Arc;

pub struct SecurityModule;

crate::inventory::submit! {
    crate::module::NativeModuleRegistration {
        name: "security",
        factory: |_| Arc::new(SecurityModule),
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

    async fn on_uninstall(&self, state: Arc<AppState>) -> Result<(), String> {
        state.rpc_registry.call(
            "admin_menu",
            "unregister_module",
            serde_json::json!({ "module": "security" }),
            crate::rpc::RpcContext::default(),
            state.clone()
        ).await.ok();
        Ok(())
    }

    fn register_admin_routes(&self) -> Router<Arc<AppState>> {
        Router::new()
            .route("/admins", get(crate::apanel::amanage::list_admins))
            .route("/admins/edit", get(crate::apanel::amanage::edit_admin))
            .route("/admins/save", post(crate::apanel::amanage::save_admin))
            .route("/admins/delete", post(crate::apanel::amanage::delete_admin))
            .route("/groups", get(crate::apanel::agroups::list_groups))
            .route("/groups/edit/:id", get(crate::apanel::agroups::edit_group))
            .route("/groups/save", post(crate::apanel::agroups::save_group))
            .route("/groups/delete/:id", post(crate::apanel::agroups::delete_group))
    }
}
