use danneo_sdk::acl::service::AclService;
pub use danneo_sdk::models::settings::GlobalSettings;
use danneo_sdk::registry::{IModuleRegistry, IPackageRegistry};
pub use danneo_sdk::state::AppState;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tera::{Function, Result as TeraResult, Tera, Value};

pub async fn init_state(db: DatabaseConnection) -> Result<Arc<AppState>, String> {
    crate::module::init_native_modules();

    // 0. Core Migrations
    use sea_orm_migration::MigratorTrait;
    migration::Migrator::up(&db, None)
        .await
        .map_err(|e| format!("Migration failed: {}", e))?;

    let db_arc = Arc::new(db);
    let rpc_registry = Arc::new(crate::rpc::RpcRegistry::new());
    let function_registry = Arc::new(danneo_sdk::functions::FunctionRegistry::new());

    let script_engine = Arc::new(crate::registry::ScriptEngine::new(
        db_arc.clone(),
        rpc_registry.clone(),
    ));

    // 1. Initialize Registries
    let module_registry = Arc::new(crate::registry::ModuleRegistry::new(
        db_arc.clone(),
        rpc_registry.clone(),
    ));
    let routes = Arc::new(crate::registry::RouteRegistry::new());
    let block_registry = Arc::new(crate::registry::BlockRegistry::new(
        db_arc.clone(),
        script_engine.clone(),
    ));

    // 2. Discover and Register Native Modules via inventory
    {
        for registration in inventory::iter::<danneo_sdk::module::NativeModuleRegistration>() {
            let module = (registration.factory)(db_arc.clone());
            module_registry.register_native(module).await;
        }
    }

    // 3. System Defaults
    let settings = Arc::new(tokio::sync::RwLock::new(GlobalSettings::default()));
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "super_secret_key".to_string());

    // 4. Initialize ACL
    let model_paths = [
        "core/casbin_models/rbac_with_level.conf",
        "casbin_models/rbac_with_level.conf",
        "../../core/casbin_models/rbac_with_level.conf",
        "../core/casbin_models/rbac_with_level.conf",
    ];
    let mut model_path = "core/casbin_models/rbac_with_level.conf";
    for p in model_paths {
        if std::path::Path::new(p).exists() {
            model_path = p;
            break;
        }
    }
    let acl = Arc::new(AclService::new_db(db_arc.clone(), model_path).await);

    // 5. Initialize Templates (Tera)
    let template_paths = [
        "core/templates/**/*",
        "templates/**/*",
        "../../core/templates/**/*",
        "../../templates/**/*",
    ];
    let mut template_glob = "core/templates/**/*";
    for p in template_paths {
        let base = p.strip_suffix("/**/*").unwrap_or(p);
        if std::path::Path::new(base).exists() {
            template_glob = p;
            break;
        }
    }

    let mut tera = Tera::new(template_glob)
        .map_err(|e| format!("Failed to initialize Tera (glob {}): {}", template_glob, e))?;

    rust_i18n::set_locale("ru");
    struct I18nFunction;
    impl Function for I18nFunction {
        fn call(&self, args: &HashMap<String, Value>) -> TeraResult<Value> {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| tera::Error::msg("Missing 'key' argument"))?;
            Ok(Value::String(rust_i18n::t!(key).to_string()))
        }
    }
    tera.register_function("t", I18nFunction);

    let mut packages_dir = "lua";
    let packages_dirs = ["lua", "../lua", "../../lua", "core/lua", "../../core/lua"];
    for d in packages_dirs {
        if std::path::Path::new(d).exists() {
            packages_dir = d;
            break;
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
                        let name = format!("{}/{}/{}", module, theme, rest.to_string_lossy())
                            .replace('\\', "/");
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
        if a_is_base && !b_is_base {
            std::cmp::Ordering::Less
        } else if !a_is_base && b_is_base {
            std::cmp::Ordering::Greater
        } else {
            a.1.get(0..1).cmp(&b.1.get(0..1))
        }
    });

    for (path, name) in module_templates {
        if let Err(e) = tera.add_template_file(&path, Some(&name)) {
            tracing::warn!(
                "Failed to load module template {} ({}): {}",
                name,
                path.display(),
                e
            );
        }
    }
    let tera_arc = Arc::new(tera);

    let package_registry = Arc::new(crate::registry::PackageRegistry::new(packages_dir));
    package_registry.scan().await;

    // 6. Create Final State
    let state = Arc::new(AppState {
        db: db_arc.clone(),
        settings,
        tera: tera_arc,
        block_registry: block_registry.clone() as Arc<dyn danneo_sdk::registry::IBlockRegistry>,
        jwt_secret,
        acl,
        packages: package_registry.clone() as Arc<dyn danneo_sdk::registry::IPackageRegistry>,
        modules: module_registry.clone() as Arc<dyn danneo_sdk::registry::IModuleRegistry>,
        routes: routes.clone() as Arc<dyn danneo_sdk::registry::IRouteRegistry>,
        script_engine: script_engine.clone() as Arc<dyn danneo_sdk::registry::IScriptEngine>,
        rpc_registry: rpc_registry.clone() as Arc<dyn danneo_sdk::rpc::IRpcRegistry>,
        function_registry: function_registry.clone(),
    });

    // 7. Initialize Registries and scan modules
    {
        module_registry
            .init(
                script_engine.clone(),
                routes.clone(),
                PathBuf::from(packages_dir),
                state.clone(),
            )
            .await;
    }

    // 8. Automated Bootstrap (runs on_install for core modules if clean DB)
    let installer = crate::registry::PackageInstaller::new(
        db_arc.clone(),
        package_registry.clone(),
        module_registry.clone(),
        routes.clone(),
        script_engine.clone(),
        state.clone(),
    );
    let _ = installer.bootstrap().await;

    // 9. Final Module Initializations (Sync state with DB)
    {
        let native_modules = module_registry.get_native_modules().await;
        for (name, module) in native_modules.iter() {
            if let Err(e) = module.init(state.clone()).await {
                tracing::error!("Failed to initialize native module {}: {}", name, e);
            }
        }
    }

    let native_modules_map = module_registry.get_native_modules().await;
    block_registry.init(native_modules_map).await;

    Ok(state)
}
