use crate::module::DanneoModule;
use crate::state::AppState;
use async_trait::async_trait;
use axum::{Router, routing::{get, post}};
use std::sync::Arc;
use sea_orm::DatabaseConnection;

pub mod migrations;
pub mod handlers;
pub mod groups;

pub struct SecurityModule;

fn match_level_fn(args: serde_json::Value) -> serde_json::Value {
    let a = &args[0];
    let b = &args[1];
    
    let r_level = a.as_i64().unwrap_or(0);
    let p_level = if b.is_i64() {
        b.as_i64().unwrap()
    } else {
        b.as_str().unwrap_or("0").parse::<i64>().unwrap_or(0)
    };
    
    serde_json::json!(r_level >= p_level)
}

crate::inventory::submit! {
    crate::registry::function_registry::NativeFunctionDescriptor {
        name: "casbin.matchLevel",
        func: match_level_fn,
    }
}

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
        // 1. Initialize Casbin through RPC (naked module)
        let model_path = if std::path::Path::new("core/casbin_models/rbac_with_level.conf").exists() {
            "core/casbin_models/rbac_with_level.conf"
        } else {
            "casbin_models/rbac_with_level.conf"
        };

        state.rpc_registry.call("casbin", "load_model", serde_json::json!({
            "path": model_path
        }), crate::rpc::RpcContext::default(), state.clone()).await.map_err(|e| e.to_string())?;

        // 2. Register Admin Menu items
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
            .route("/admins", get(handlers::list_admins))
            .route("/admins/edit", get(handlers::edit_admin))
            .route("/admins/save", post(handlers::save_admin))
            .route("/admins/delete", get(handlers::delete_admin))
            .route("/groups", get(groups::list_groups))
            .route("/groups/edit", get(groups::edit_group))
            .route("/groups/save", post(groups::save_group))
            .route("/groups/delete", get(groups::delete_group))
            .route("/login", get(handlers::show_login_page))
            .route("/api/login", post(handlers::admin_login))
    }

    fn register_admin_middleware(&self, router: Router<Arc<AppState>>, state: Arc<AppState>) -> Router<Arc<AppState>> {
        router.layer(axum::middleware::from_fn_with_state(
            state,
            crate::apanel::middleware::admin_acl_middleware
        ))
    }
}

crate::register_native_module!("security", |db| Arc::new(SecurityModule::new(db)));
