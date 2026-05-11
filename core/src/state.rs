use crate::acl::service::AclService;
use crate::module::DanneoModule;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tera::{Function, Result as TeraResult, Tera, Value};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GlobalSettings {
    pub site_name: String,
    pub admin_email: String,
    pub site_url: String,
    pub site_temp: String,
    pub storage_endpoint: String,
    pub storage_access_key: String,
    pub storage_secret_key: String,
    pub storage_bucket: String,
    pub storage_region: String,
}

pub struct AppState {
    pub db: Arc<DatabaseConnection>,
    pub settings: Arc<tokio::sync::RwLock<GlobalSettings>>,
    pub tera: Arc<tera::Tera>,
    pub block_registry: Arc<crate::registry::BlockRegistry>,
    pub jwt_secret: String,
    pub acl: Arc<AclService>,
    pub packages: Arc<tokio::sync::RwLock<crate::registry::PackageRegistry>>,
    pub modules: Arc<tokio::sync::RwLock<crate::registry::ModuleRegistry>>,
    pub routes: Arc<tokio::sync::RwLock<crate::registry::RouteRegistry>>,
    pub script_engine: Arc<crate::registry::ScriptEngine>,
    pub rpc_registry: Arc<crate::rpc::registry::RpcRegistry>,
    pub function_registry: Arc<crate::registry::FunctionRegistry>,
}

impl AppState {
    pub async fn new(db: DatabaseConnection) -> Result<Self, String> {
        crate::module::init_native_modules();

        // 0. Core Migrations
        use sea_orm_migration::MigratorTrait;
        migration::Migrator::up(&db, None)
            .await
            .map_err(|e| format!("Migration failed: {}", e))?;

        let db_arc = Arc::new(db);
        let rpc_registry = Arc::new(crate::rpc::registry::RpcRegistry::new());
        let function_registry = Arc::new(crate::registry::FunctionRegistry::new());

        let script_engine = Arc::new(crate::registry::ScriptEngine::new(
            db_arc.clone(),
            rpc_registry.clone(),
        ));

        // 1. Initialize Registries
        let module_registry_inner = crate::registry::ModuleRegistry::new(db_arc.clone(), rpc_registry.clone());
        let modules = Arc::new(tokio::sync::RwLock::new(module_registry_inner));
        let routes = Arc::new(tokio::sync::RwLock::new(crate::registry::RouteRegistry::new()));
        let block_registry = Arc::new(crate::registry::BlockRegistry::new(db_arc.clone(), script_engine.clone()));

        // 2. Discover and Register Native Modules via inventory
        {
            let modules_guard = modules.write().await;
            for registration in inventory::iter::<crate::module::NativeModuleRegistration>() {
                let module = (registration.factory)(db_arc.clone());
                modules_guard.register_native(module).await;
            }
        }

        // 3. System Defaults
        let settings = Arc::new(tokio::sync::RwLock::new(GlobalSettings::default()));
        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "super_secret_key".to_string());

        // 4. Initialize ACL
        let model_path = if std::path::Path::new("core/casbin_models/rbac_with_level.conf").exists() {
            "core/casbin_models/rbac_with_level.conf"
        } else {
            "casbin_models/rbac_with_level.conf"
        };
        let acl = Arc::new(AclService::new_db(db_arc.clone(), model_path).await);

        // 5. Initialize Templates (Tera)
        let mut tera = Tera::new(if std::path::Path::new("core/templates").exists() { "core/templates/**/*" } else { "templates/**/*" })
            .map_err(|e| format!("Failed to initialize Tera: {}", e))?;

        rust_i18n::set_locale("ru");
        struct I18nFunction;
        impl Function for I18nFunction {
            fn call(&self, args: &HashMap<String, Value>) -> TeraResult<Value> {
                let key = args.get("key").and_then(|v| v.as_str()).ok_or_else(|| tera::Error::msg("Missing 'key' argument"))?;
                Ok(Value::String(rust_i18n::t!(key).to_string()))
            }
        }
        tera.register_function("t", I18nFunction);

        let mut packages_dir = "modules";
        if !std::path::Path::new(packages_dir).exists() {
            if std::path::Path::new("../modules").exists() {
                packages_dir = "../modules";
            } else {
                packages_dir = "core/modules";
            }
        }
        
        // Load module templates
        let m_glob = format!("{}/*/templates/**/*", packages_dir);
        let mut module_templates = Vec::new();

        for entry in glob::glob(&m_glob).map_err(|e| e.to_string())? {
            if let Ok(path) = entry {
                if path.is_file() {
                    if let Ok(rel_path) = path.strip_prefix(packages_dir) {
                        let parts: Vec<_> = rel_path.components().collect();
                        if parts.len() >= 4 {
                            let module = parts[0].as_os_str().to_string_lossy();
                            let theme = parts[2].as_os_str().to_string_lossy();
                            let rest: PathBuf = parts[3..].iter().collect();
                            let name = format!("{}/{}/{}", module, theme, rest.to_string_lossy()).replace('\\', "/");
                            module_templates.push((path.clone(), name));

                            if theme == "default" && rest.starts_with("apanel") {
                                let short_name = rest.to_string_lossy().replace('\\', "/");
                                module_templates.push((path, short_name));
                            }
                        }
                    }
                }
            }
        }

        module_templates.sort_by(|a, b| {
            let a_is_base = a.1 == "apanel/base.html";
            let b_is_base = b.1 == "apanel/base.html";
            if a_is_base && !b_is_base { std::cmp::Ordering::Less }
            else if !a_is_base && b_is_base { std::cmp::Ordering::Greater }
            else { a.1.get(0..1).cmp(&b.1.get(0..1)) }
        });

        for (path, name) in module_templates {
            if let Err(e) = tera.add_template_file(&path, Some(&name)) {
                tracing::warn!("Failed to load module template {} ({}): {}", name, path.display(), e);
            }
        }
        let tera_arc = Arc::new(tera);

        let mut package_registry = crate::registry::PackageRegistry::new(packages_dir);
        package_registry.scan();
        let packages = Arc::new(tokio::sync::RwLock::new(package_registry));

        // 6. Create Final State
        let state = Arc::new(Self {
            db: db_arc.clone(),
            settings,
            tera: tera_arc,
            block_registry: block_registry.clone(),
            jwt_secret,
            acl,
            packages,
            modules: modules.clone(),
            routes,
            script_engine: script_engine.clone(),
            rpc_registry: rpc_registry.clone(),
            function_registry: function_registry.clone(),
        });

        // 7. Initialize Registries and scan modules
        {
            let modules_guard = state.modules.read().await;
            modules_guard.init(script_engine.clone(), state.routes.clone(), PathBuf::from(packages_dir), state.clone()).await;
        }

        // 8. Automated Bootstrap (runs on_install for core modules if clean DB)
        let installer = crate::registry::PackageInstaller::new(
            db_arc.clone(),
            state.packages.clone(),
            state.modules.clone(),
            state.routes.clone(),
            state.script_engine.clone(),
            state.clone(),
        );
        let _ = installer.bootstrap().await;

        // 9. Final Module Initializations (Sync state with DB)
        {
            let modules_guard = state.modules.read().await;
            let native_modules = modules_guard.native_modules.read().await;
            for (name, module) in native_modules.iter() {
                if let Err(e) = module.init(state.clone()).await {
                    tracing::error!("Failed to initialize native module {}: {}", name, e);
                }
            }
        }

        let native_modules_map = {
            let modules_guard = state.modules.read().await;
            modules_guard.native_modules.read().await.clone()
        };
        block_registry.init(native_modules_map).await;

        Ok(Arc::try_unwrap(state).unwrap_or_else(|arc| (*arc).clone_dummy()))
    }

    fn clone_dummy(&self) -> Self {
        Self {
            db: self.db.clone(),
            settings: self.settings.clone(),
            tera: self.tera.clone(),
            block_registry: self.block_registry.clone(),
            jwt_secret: self.jwt_secret.clone(),
            acl: self.acl.clone(),
            packages: self.packages.clone(),
            modules: self.modules.clone(),
            routes: self.routes.clone(),
            script_engine: self.script_engine.clone(),
            rpc_registry: self.rpc_registry.clone(),
            function_registry: self.function_registry.clone(),
        }
    }

    pub async fn is_module_available(&self, module_code: &str) -> bool {
        use crate::models::core_modules;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
        core_modules::Entity::find()
            .filter(core_modules::Column::Code.eq(module_code))
            .filter(core_modules::Column::Enabled.eq(true))
            .one(self.db.as_ref())
            .await
            .unwrap_or(None)
            .is_some()
    }
}
