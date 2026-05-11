use crate::models::core_modules;
use crate::registry::{AdminMenu, RouteRegistry, ScriptEngine};
use crate::module::DanneoModule;
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub struct ModuleRegistry {
    pub db: Arc<DatabaseConnection>,
    pub admin_menus: Arc<tokio::sync::RwLock<HashMap<String, AdminMenu>>>,
    pub rpc_registry: Arc<crate::rpc::registry::RpcRegistry>,
    pub native_modules:
        Arc<tokio::sync::RwLock<HashMap<String, Arc<dyn crate::module::DanneoModule>>>>,
}

impl ModuleRegistry {
    pub fn new(
        db: Arc<DatabaseConnection>,
        rpc_registry: Arc<crate::rpc::registry::RpcRegistry>,
    ) -> Self {
        Self {
            db,
            admin_menus: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            rpc_registry,
            native_modules: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_native(&self, module: Arc<dyn crate::module::DanneoModule>) {
        let name = module.name().to_string();

        // 1. Register in RPC
        self.rpc_registry
            .register(
                &name,
                Arc::new(crate::rpc::registry::NativeRpcHandler::new(module.clone())),
                module.rpc_methods(),
            )
            .await;

        // 2. Store instance
        self.native_modules.write().await.insert(name, module);
    }

    pub async fn init(
        &self,
        script_engine: Arc<ScriptEngine>,
        routes: Arc<tokio::sync::RwLock<RouteRegistry>>,
        packages_dir: PathBuf,
        state: Arc<crate::state::AppState>,
    ) {
        tracing::info!("Initializing ModuleRegistry");
        self.admin_menus.write().await.clear();

        match core_modules::Entity::find()
            .filter(core_modules::Column::Enabled.eq(true))
            .all(self.db.as_ref())
            .await
        {
            Ok(modules) => {
                tracing::info!("Loaded {} active modules", modules.len());
                for module in modules {
                    let module_path = packages_dir.join(&module.code);
                    let manifest_path = module_path.join("module.toml");

                    if manifest_path.exists() {
                        if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                            if let Ok(manifest) =
                                toml::from_str::<crate::registry::PackageManifest>(&content)
                            {
                                // 1. Load frontend routes
                                if let Some(descriptors) = manifest.frontend_routes {
                                    let mut routes_guard = routes.write().await;
                                    for desc in descriptors {
                                        routes_guard.register_frontend(&module.code, desc);
                                    }
                                }

                                // 2. Load admin routes
                                if let Some(descriptors) = manifest.admin_routes {
                                    let mut routes_guard = routes.write().await;
                                    for desc in descriptors {
                                        routes_guard.register_admin(&module.code, desc);
                                    }
                                }

                                if let Some(entry) = manifest.entrypoints {
                                    // 3. Load hooks
                                    if let Some(hooks_path) = entry.hooks {
                                        let full_path = module_path.join(hooks_path);
                                        if let Err(e) = script_engine
                                            .load_module_scripts(&module.code, &full_path)
                                            .await
                                        {
                                            tracing::error!(
                                                "Failed to load scripts for module {}: {}",
                                                module.code,
                                                e
                                            );
                                        }
                                    }

                                    // (JSON-based routing removed in favor of Axum-style code routing)

                                    // 6. Load admin menu
                                    if let Some(menu_path) = entry.admin_menu {
                                        let full_path = module_path.join(menu_path);
                                        if let Ok(content) = std::fs::read_to_string(&full_path) {
                                            if let Ok(menu_manifest) = serde_json::from_str::<
                                                crate::registry::AdminMenuManifest,
                                            >(
                                                &content
                                            ) {
                                                // Register via RPC
                                                if let Some(cats) = menu_manifest.categories {
                                                    for cat in cats {
                                                        let _ = self
                                                            .rpc_registry
                                                            .call(
                                                                "admin_menu",
                                                                "ensure_category",
                                                                serde_json::to_value(cat).unwrap(),
                                                                crate::rpc::RpcContext::default(),
                                                                state.clone(),
                                                            )
                                                            .await;
                                                    }
                                                }

                                                if let Some(items) = menu_manifest.items {
                                                    let _ = self
                                                        .rpc_registry
                                                        .call(
                                                            "admin_menu",
                                                            "register_items",
                                                            serde_json::json!({
                                                                "module": module.code,
                                                                "items": items
                                                            }),
                                                            crate::rpc::RpcContext::default(),
                                                            state.clone(),
                                                        )
                                                        .await;
                                                }
                                            }
                                        }
                                    }
                                }

                                // 7. Register RPC methods
                                if let Some(rpc) = manifest.rpc {
                                    let runtime_type = manifest
                                        .module
                                        .as_ref()
                                        .map(|m| m.runtime_type.as_str())
                                        .unwrap_or("lua");
                                    if runtime_type == "lua" {
                                        let handler = Arc::new(LuaRpcHandler {
                                            module_code: module.code.clone(),
                                            script_engine: script_engine.clone(),
                                        });
                                        self.rpc_registry
                                            .register(&rpc.namespace, handler, rpc.methods)
                                            .await;
                                    }
                                }

                                // 8. Register as DanneoModule adapter
                                let runtime_type = manifest
                                    .module
                                    .as_ref()
                                    .map(|m| m.runtime_type.as_str())
                                    .unwrap_or("lua");
                                if runtime_type == "lua" {
                                    let adapter = crate::module::lua_adapter::build_lua_module_adapter(
                                        module.code.clone(),
                                        script_engine.clone(),
                                        state.clone(),
                                    ).await;
                                    self.native_modules.write().await.insert(module.code.clone(), adapter as Arc<dyn DanneoModule>);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to load active modules: {}", e);
            }
        }

        // 2. Load active Native modules
        {
            let native_modules = self.native_modules.read().await;
            
            // Re-fetch enabled list to be sure
            let enabled_modules: std::collections::HashSet<String> = core_modules::Entity::find()
                .filter(core_modules::Column::Enabled.eq(true))
                .all(self.db.as_ref())
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|m| m.code)
                .collect();

            for (name, _module) in native_modules.iter() {
                if enabled_modules.contains(name) {
                    let _routes_guard = routes.write().await;
                    // (Removed legacy admin_routes and frontend_routes loops as we now use register_admin_routes/register_routes)
                }
            }
        }
    }

    pub async fn enable(&self, module_code: &str) -> Result<(), String> {
        let model_opt = core_modules::Entity::find()
            .filter(core_modules::Column::Code.eq(module_code))
            .one(self.db.as_ref())
            .await
            .map_err(|e| format!("DB Error: {}", e))?;

        if let Some(model) = model_opt {
            let mut active_model = model.into_active_model();
            active_model.enabled = Set(true);
            active_model
                .update(self.db.as_ref())
                .await
                .map_err(|e| format!("DB Error: {}", e))?;
            return Ok(());
        }

        use crate::models::core_block_definitions;
        let block_opt = core_block_definitions::Entity::find()
            .filter(core_block_definitions::Column::BlockCode.eq(module_code))
            .one(self.db.as_ref())
            .await
            .map_err(|e| format!("DB Error: {}", e))?;

        if let Some(block) = block_opt {
            let mut active_model = block.into_active_model();
            active_model.enabled = Set(true);
            active_model
                .update(self.db.as_ref())
                .await
                .map_err(|e| format!("DB Error: {}", e))?;
            return Ok(());
        }

        Err(format!("Package {} not found", module_code))
    }

    pub async fn disable(&self, module_code: &str) -> Result<(), String> {
        let model_opt = core_modules::Entity::find()
            .filter(core_modules::Column::Code.eq(module_code))
            .one(self.db.as_ref())
            .await
            .map_err(|e| format!("DB Error: {}", e))?;

        if let Some(model) = model_opt {
            let mut active_model = model.into_active_model();
            active_model.enabled = Set(false);
            active_model
                .update(self.db.as_ref())
                .await
                .map_err(|e| format!("DB Error: {}", e))?;
            return Ok(());
        }

        use crate::models::core_block_definitions;
        let block_opt = core_block_definitions::Entity::find()
            .filter(core_block_definitions::Column::BlockCode.eq(module_code))
            .one(self.db.as_ref())
            .await
            .map_err(|e| format!("DB Error: {}", e))?;

        if let Some(block) = block_opt {
            let mut active_model = block.into_active_model();
            active_model.enabled = Set(false);
            active_model
                .update(self.db.as_ref())
                .await
                .map_err(|e| format!("DB Error: {}", e))?;
            return Ok(());
        }

        Err(format!("Package {} not found", module_code))
    }
}

struct LuaRpcHandler {
    module_code: String,
    script_engine: Arc<crate::registry::ScriptEngine>,
}

#[async_trait]
impl crate::rpc::registry::RpcHandler for LuaRpcHandler {
    async fn call(
        &self,
        method: &str,
        payload: serde_json::Value,
        ctx: crate::rpc::RpcContext,
        state: Arc<crate::state::AppState>,
    ) -> Result<serde_json::Value, crate::rpc::RpcError> {
        let arg = serde_json::json!({
            "method": method,
            "payload": payload,
            "context": {
                "caller": ctx.caller,
                "trace_id": ctx.trace_id,
                "call_depth": ctx.call_depth,
            }
        });

        let dynamic_arg = script_rhai::serde::to_dynamic(arg).unwrap();
        match self
            .script_engine
            .call_hook(&self.module_code, "rpc_dispatch", dynamic_arg, state)
            .await
        {
            Ok(res) => script_rhai::serde::from_dynamic::<serde_json::Value>(&res)
                .map_err(|e| crate::rpc::RpcError::Runtime(e.to_string())),
            Err(e) => Err(crate::rpc::RpcError::Runtime(e.to_string())),
        }
    }
}
