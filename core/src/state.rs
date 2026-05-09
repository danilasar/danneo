use crate::acl::service::AclService;
use crate::models::core_settings;
use crate::module::DanneoModule;
use sea_orm::{DatabaseConnection, EntityTrait};
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
}

/// Глобальное состояние приложения (Ядра).
/// Доступно каждому роуту и каждому модулю.
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
    pub admin_menu: Arc<crate::module::admin_menu::AdminMenuModule>,
    pub rpc_registry: Arc<crate::rpc::registry::RpcRegistry>,
}

impl AppState {
    pub async fn new(db: DatabaseConnection) -> Result<Self, String> {
        let db_arc = Arc::new(db);
        let rpc_registry = Arc::new(crate::rpc::registry::RpcRegistry::new());

        let script_engine = Arc::new(crate::registry::ScriptEngine::new(
            db_arc.clone(),
            rpc_registry.clone(),
        ));
        // Пытаемся загрузить настройки из БД
        let settings_records = core_settings::Entity::find()
            .all(db_arc.as_ref())
            .await
            .map_err(|e| format!("Failed to load settings: {}", e))?;

        let mut settings = GlobalSettings::default();
        for record in settings_records {
            match record.key.as_str() {
                "site_name" => {
                    if let Some(val) = record.value.as_str() {
                        settings.site_name = val.to_string();
                    }
                }
                "admin_email" => {
                    if let Some(val) = record.value.as_str() {
                        settings.admin_email = val.to_string();
                    }
                }
                "site_url" => {
                    if let Some(val) = record.value.as_str() {
                        settings.site_url = val.to_string();
                    }
                }
                "site_temp" => {
                    if let Some(val) = record.value.as_str() {
                        settings.site_temp = val.to_string();
                    }
                }
                _ => {}
            }
        }
        let settings = Arc::new(tokio::sync::RwLock::new(settings));

        let jwt_secret =
            std::env::var("JWT_SECRET").unwrap_or_else(|_| "super_secret_key".to_string());

        // Removed BlockManager instantiation
        // Инициализируем ACL
        let model_path = if std::path::Path::new("core/casbin_models/rbac_with_level.conf").exists()
        {
            "core/casbin_models/rbac_with_level.conf"
        } else {
            "casbin_models/rbac_with_level.conf"
        };

        let acl = AclService::new_db(db_arc.clone(), model_path).await;
        let acl = Arc::new(acl);

        // Инициализируем Tera, загружая шаблоны из файловой системы
        let template_path = if std::path::Path::new("core/templates").exists() {
            "core/templates/**/*"
        } else {
            "templates/**/*"
        };

        let mut tera =
            Tera::new(template_path).map_err(|e| format!("Failed to initialize Tera: {}", e))?;

        rust_i18n::set_locale("ru");
        struct I18nFunction;
        impl Function for I18nFunction {
            fn call(&self, args: &HashMap<String, Value>) -> TeraResult<Value> {
                let key = args
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| tera::Error::msg("Missing 'key' argument"))?;
                // Используем функцию t из rust_i18n.
                // Если она конфликтует с макросом, можно попробовать вызвать через полное имя
                Ok(Value::String(rust_i18n::t!(key).to_string()))
            }
        }
        tera.register_function("t", I18nFunction);

        let packages_dir = if std::path::Path::new("modules").exists() {
            "modules"
        } else {
            "core/modules"
        };
        let blocks_dir = if std::path::Path::new("blocks").exists() {
            "blocks"
        } else {
            "core/blocks"
        };

        // Загружаем шаблоны модулей и их блоков
        let m_glob = format!("{}/*/templates/**/*", packages_dir);
        for entry in glob::glob(&m_glob).map_err(|e| e.to_string())? {
            if let Ok(path) = entry {
                if path.is_file() {
                    if let Ok(rel_path) = path.strip_prefix(packages_dir) {
                        let parts: Vec<_> = rel_path.components().collect();
                        if parts.len() >= 4 {
                            // parts: [Module, "templates", Theme, Template...]
                            let module = parts[0].as_os_str().to_string_lossy();
                            let theme = parts[2].as_os_str().to_string_lossy();
                            let rest: PathBuf = parts[3..].iter().collect();
                            let name = format!("{}/{}/{}", module, theme, rest.to_string_lossy())
                                .replace('\\', "/");
                            tera.add_template_file(&path, Some(&name)).map_err(|e| {
                                format!("Failed to add module template {}: {}", name, e)
                            })?;
                        }
                    }
                }
            }
        }

        let block_template_glob = format!("{}/*/blocks/*/templates/**/*", packages_dir);
        for entry in glob::glob(&block_template_glob).map_err(|e| e.to_string())? {
            if let Ok(path) = entry {
                if path.is_file() {
                    if let Ok(rel_path) = path.strip_prefix(packages_dir) {
                        let parts: Vec<_> = rel_path.components().collect();
                        if parts.len() >= 5 {
                            // parts: [Module, "blocks", Block, "templates", Template...]
                            let module = parts[0].as_os_str().to_string_lossy();
                            let block = parts[2].as_os_str().to_string_lossy();
                            let rest: PathBuf = parts[4..].iter().collect();
                            let name =
                                format!("{}/blocks/{}/{}", module, block, rest.to_string_lossy())
                                    .replace('\\', "/");
                            tera.add_template_file(&path, Some(&name)).map_err(|e| {
                                format!("Failed to add block template {}: {}", name, e)
                            })?;

                            if parts.len() >= 6 {
                                // parts: [Module, "blocks", Block, "templates", Theme, Template...]
                                let theme = parts[4].as_os_str().to_string_lossy();
                                let themed_rest: PathBuf = parts[5..].iter().collect();
                                let themed_name = format!(
                                    "{}/{}/blocks/{}/{}",
                                    module,
                                    theme,
                                    block,
                                    themed_rest.to_string_lossy()
                                )
                                .replace('\\', "/");
                                tera.add_template_file(&path, Some(&themed_name))
                                    .map_err(|e| {
                                        format!(
                                            "Failed to add themed block template {}: {}",
                                            themed_name, e
                                        )
                                    })?;
                            }
                        }
                    }
                }
            }
        }

        let standalone_block_template_glob = format!("{}/*/templates/**/*", blocks_dir);
        for entry in glob::glob(&standalone_block_template_glob).map_err(|e| e.to_string())? {
            if let Ok(path) = entry {
                if path.is_file() {
                    if let Ok(rel_path) = path.strip_prefix(blocks_dir) {
                        let parts: Vec<_> = rel_path.components().collect();
                        if parts.len() >= 3 {
                            // parts: [Block, "templates", Template...]
                            let block = parts[0].as_os_str().to_string_lossy();
                            let rest: PathBuf = parts[2..].iter().collect();
                            let names = [
                                format!("{}/{}", block, rest.to_string_lossy()).replace('\\', "/"),
                                format!("{}/templates/{}", block, rest.to_string_lossy())
                                    .replace('\\', "/"),
                            ];

                            for name in names {
                                tera.add_template_file(&path, Some(&name)).map_err(|e| {
                                    format!(
                                        "Failed to add standalone block template {}: {}",
                                        name, e
                                    )
                                })?;
                            }

                            if parts.len() >= 4 {
                                // parts: [Block, "templates", Theme, Template...]
                                let theme = parts[2].as_os_str().to_string_lossy();
                                let themed_rest: PathBuf = parts[3..].iter().collect();
                                let themed_name = format!(
                                    "{}/{}/{}",
                                    block,
                                    theme,
                                    themed_rest.to_string_lossy()
                                )
                                .replace('\\', "/");
                                tera.add_template_file(&path, Some(&themed_name))
                                    .map_err(|e| {
                                        format!(
                                            "Failed to add themed standalone block template {}: {}",
                                            themed_name, e
                                        )
                                    })?;
                            }
                        }
                    }
                }
            }
        }

        let mut package_registry = crate::registry::PackageRegistry::new(packages_dir);
        package_registry.scan();
        let packages = Arc::new(tokio::sync::RwLock::new(package_registry));

        let route_registry = crate::registry::RouteRegistry::new();
        let routes = Arc::new(tokio::sync::RwLock::new(route_registry));

        let rpc_registry = Arc::new(crate::rpc::registry::RpcRegistry::new());

        let script_engine = Arc::new(crate::registry::ScriptEngine::new(
            db_arc.clone(),
            rpc_registry.clone(),
        ));

        let admin_menu = Arc::new(crate::module::admin_menu::AdminMenuModule::new(
            db_arc.clone(),
        ));
        
        // Initialize AdminMenu (migrations + defaults)
        let state_arc_tmp = Arc::new(Self {
            db: db_arc.clone(),
            settings: settings.clone(),
            tera: Arc::new(tera.clone()),
            block_registry: Arc::new(crate::registry::BlockRegistry::new(db_arc.clone(), script_engine.clone())),
            jwt_secret: jwt_secret.clone(),
            acl: acl.clone(),
            packages: Arc::new(tokio::sync::RwLock::new(crate::registry::PackageRegistry::new(packages_dir))),
            modules: Arc::new(tokio::sync::RwLock::new(crate::registry::ModuleRegistry::new(db_arc.clone(), admin_menu.clone(), rpc_registry.clone()))),
            routes: routes.clone(),
            script_engine: script_engine.clone(),
            admin_menu: admin_menu.clone(),
            rpc_registry: rpc_registry.clone(),
        });
        
        admin_menu.init(state_arc_tmp.clone()).await.map_err(|e| format!("Failed to init admin_menu: {}", e))?;

        // Register admin_menu in RPC registry
        rpc_registry.register(
            "admin_menu", 
            Arc::new(crate::rpc::registry::NativeRpcHandler::new(admin_menu.clone())),
            admin_menu.rpc_methods()
        ).await;

        let module_registry = crate::registry::ModuleRegistry::new(
            db_arc.clone(),
            admin_menu.clone(),
            rpc_registry.clone(),
        );
        module_registry
            .init(
                script_engine.clone(),
                routes.clone(),
                std::path::PathBuf::from(packages_dir),
                state_arc_tmp,
            )
            .await;
        let modules = Arc::new(tokio::sync::RwLock::new(module_registry));

        let block_registry =
            crate::registry::BlockRegistry::new(db_arc.clone(), script_engine.clone());
        block_registry.init().await;
        let block_registry = Arc::new(block_registry);

        Ok(Self {
            db: db_arc,
            settings,
            tera: Arc::new(tera),
            block_registry,
            jwt_secret,
            acl,
            packages,
            modules,
            routes,
            script_engine,
            admin_menu,
            rpc_registry,
        })
    }
}
