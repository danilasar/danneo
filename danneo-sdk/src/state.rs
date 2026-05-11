use crate::functions::FunctionRegistry;
use crate::models::core_modules;
use crate::models::settings::GlobalSettings;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::registry::{
    IBlockRegistry, IModuleRegistry, IPackageRegistry, IRouteRegistry, IScriptEngine,
};

pub struct AppState {
    pub db: Arc<DatabaseConnection>,
    pub settings: Arc<RwLock<GlobalSettings>>,
    pub tera: Arc<tera::Tera>,
    pub block_registry: Arc<dyn IBlockRegistry>,
    pub jwt_secret: String,
    pub acl: Arc<crate::acl::service::AclService>,
    pub packages: Arc<dyn IPackageRegistry>,
    pub modules: Arc<dyn IModuleRegistry>,
    pub routes: Arc<dyn IRouteRegistry>,
    pub script_engine: Arc<dyn IScriptEngine>,
    pub rpc_registry: Arc<dyn crate::rpc::IRpcRegistry>,
    pub function_registry: Arc<FunctionRegistry>,
}

impl AppState {
    pub async fn is_module_available(&self, module_code: &str) -> bool {
        core_modules::Entity::find()
            .filter(core_modules::Column::Code.eq(module_code))
            .filter(core_modules::Column::Enabled.eq(true))
            .one(self.db.as_ref())
            .await
            .unwrap_or(None)
            .is_some()
    }
}
