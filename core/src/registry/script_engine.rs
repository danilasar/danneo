use async_trait::async_trait;
use mlua::{HookTriggers, Lua, LuaSerdeExt, Value, VmState};
use sea_orm::DatabaseConnection;
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::RwLock;

const MAX_LUA_INSTRUCTIONS: usize = 100_000;

#[derive(Debug, thiserror::Error)]
pub enum ScriptError {
    #[error("Module script not found: {0}")]
    HookNotFound(String),
    #[error("Lua error: {0}")]
    Runtime(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl From<mlua::Error> for ScriptError {
    fn from(e: mlua::Error) -> Self {
        ScriptError::Runtime(e.to_string())
    }
}

pub struct ScriptEngine {
    scripts: Arc<RwLock<HashMap<String, String>>>,
    db: Arc<DatabaseConnection>,
    rpc_registry: Arc<dyn danneo_sdk::rpc::IRpcRegistry>,
}

#[async_trait]
impl danneo_sdk::registry::IScriptEngine for ScriptEngine {
    async fn call_hook(
        &self,
        module_code: &str,
        hook_name: &str,
        args: serde_json::Value,
        state: Arc<danneo_sdk::state::AppState>,
    ) -> Result<serde_json::Value, String> {
        self.call_hook_json(module_code, hook_name, args, state)
            .await
            .map_err(|e| e.to_string())
    }

    async fn load_module_scripts(
        &self,
        module_code: &str,
        scripts_path: &std::path::Path,
    ) -> Result<(), String> {
        self.load_module_scripts(module_code, scripts_path)
            .await
            .map_err(|e| e.to_string())
    }

    async fn load_script_str(&self, module_code: &str, script: &str) -> Result<(), String> {
        self.load_script_str(module_code, script)
            .await
            .map_err(|e| e.to_string())
    }
}

impl ScriptEngine {
    pub fn new(
        db: Arc<DatabaseConnection>,
        rpc_registry: Arc<dyn danneo_sdk::rpc::IRpcRegistry>,
    ) -> Self {
        Self {
            scripts: Arc::new(RwLock::new(HashMap::new())),
            db,
            rpc_registry,
        }
    }

    pub async fn load_module_scripts(
        &self,
        module_code: &str,
        scripts_path: &Path,
    ) -> Result<(), ScriptError> {
        let mut script_content = String::new();
        if scripts_path.exists() {
            if let Ok(entries) = std::fs::read_dir(scripts_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "lua") {
                        if let Ok(content) = std::fs::read_to_string(path) {
                            script_content.push_str(&content);
                            script_content.push('\n');
                        }
                    }
                }
            }
        }

        let mut scripts = self.scripts.write().await;
        scripts.insert(module_code.to_string(), script_content);
        Ok(())
    }

    pub async fn load_script_str(
        &self,
        module_code: &str,
        script: &str,
    ) -> Result<(), ScriptError> {
        let mut scripts = self.scripts.write().await;
        scripts.insert(module_code.to_string(), script.to_string());
        Ok(())
    }

    pub async fn call_hook_json(
        &self,
        module_code: &str,
        hook_name: &str,
        args: serde_json::Value,
        state: Arc<danneo_sdk::state::AppState>,
    ) -> Result<serde_json::Value, ScriptError> {
        let script = {
            let scripts = self.scripts.read().await;
            scripts
                .get(module_code)
                .cloned()
                .ok_or_else(|| ScriptError::HookNotFound(module_code.to_string()))?
        };

        let lua = Lua::new();

        // Safety hook
        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();
        lua.set_hook(
            HookTriggers {
                every_nth_instruction: Some(1000),
                ..Default::default()
            },
            move |_, _| {
                if count_clone.fetch_add(1000, Ordering::Relaxed) > MAX_LUA_INSTRUCTIONS {
                    return Err(mlua::Error::RuntimeError(
                        "Max Lua instructions reached".into(),
                    ));
                }
                Ok(VmState::Continue)
            },
        )
        .map_err(|e| ScriptError::Runtime(e.to_string()))?;

        // ---------------------------------------------------------
        // UNIFIED "danneo" API
        // ---------------------------------------------------------
        let danneo = lua.create_table()?;

        // 1. danneo.db
        let db_api = DatabaseApi {
            db: self.db.clone(),
            module_code: module_code.to_string(),
        };
        let db_table = lua.create_table()?;
        db_api.register_to_table(&db_table, &lua)?;
        danneo.set("db", db_table.clone())?;

        // 2. danneo.rpc
        let rpc_reg = self.rpc_registry.clone();
        let rpc_state = state.clone();
        let rpc_table = lua.create_table()?;
        rpc_table.set(
            "call",
            lua.create_async_function(
                move |lua, (target, method, payload): (String, String, mlua::Value)| {
                    let reg = rpc_reg.clone();
                    let st = rpc_state.clone();
                    async move {
                        let json_payload: serde_json::Value =
                            lua.from_value(payload).unwrap_or(serde_json::Value::Null);
                        let ctx = danneo_sdk::rpc::RpcContext::default();
                        match reg.call(&target, &method, json_payload, ctx, st).await {
                            Ok(res) => lua.to_value(&res),
                            Err(e) => Err(mlua::Error::RuntimeError(e.to_string())),
                        }
                    }
                },
            )?,
        )?;
        danneo.set("rpc", rpc_table.clone())?;

        // 3. danneo.system
        let system_table = lua.create_table()?;
        system_table.set("version", "2.0.0-alpha")?;
        let st_sys = state.clone();
        system_table.set(
            "is_available",
            lua.create_async_function(move |_, module_code: String| {
                let st = st_sys.clone();
                async move { Ok(st.is_module_available(&module_code).await) }
            })?,
        )?;
        danneo.set("system", system_table.clone())?;

        // 4. danneo.log
        let log_table = lua.create_table()?;
        log_table.set(
            "info",
            lua.create_function(|_, msg: String| {
                tracing::info!("Lua: {}", msg);
                Ok(())
            })?,
        )?;
        log_table.set(
            "warn",
            lua.create_function(|_, msg: String| {
                tracing::warn!("Lua: {}", msg);
                Ok(())
            })?,
        )?;
        log_table.set(
            "error",
            lua.create_function(|_, msg: String| {
                tracing::error!("Lua: {}", msg);
                Ok(())
            })?,
        )?;
        log_table.set(
            "debug",
            lua.create_function(|_, msg: String| {
                tracing::debug!("Lua: {}", msg);
                Ok(())
            })?,
        )?;
        danneo.set("log", log_table.clone())?;

        // 5. danneo.http (client)
        let http_table = lua.create_table()?;
        http_table.set(
            "get",
            lua.create_async_function(
                |lua, (url, params): (String, Option<mlua::Value>)| async move {
                    let mut builder = reqwest::Client::new().get(&url);
                    if let Some(p) = params {
                        let json_params: serde_json::Value =
                            lua.from_value(p).unwrap_or(serde_json::Value::Null);
                        builder = builder.query(&json_params);
                    }
                    let resp = builder
                        .send()
                        .await
                        .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
                    let status = resp.status().as_u16();
                    let body = resp.text().await.unwrap_or_default();
                    lua.to_value(&json!({"status": status, "body": body}))
                },
            )?,
        )?;
        danneo.set("http", http_table)?;

        lua.globals().set("danneo", danneo)?;

        // Backward compatibility aliases
        lua.globals().set("db", db_table)?;
        lua.globals().set("rpc", rpc_table)?;
        lua.globals().set("system", system_table)?;
        lua.globals().set("log", log_table)?;

        lua.load(&script).exec_async().await?;

        let globals = lua.globals();
        let hook: mlua::Function = match globals.get(hook_name) {
            Ok(f) => f,
            Err(_) => return Err(ScriptError::HookNotFound(hook_name.to_string())),
        };

        let lua_args = lua
            .to_value(&args)
            .map_err(|e| ScriptError::Runtime(e.to_string()))?;

        let res: Value = hook.call_async(lua_args).await?;

        let json_value: serde_json::Value = lua
            .from_value(res)
            .map_err(|e| ScriptError::Runtime(e.to_string()))?;

        Ok(json_value)
    }

    pub async fn call_lua_router_hook(
        &self,
        module_code: &str,
        hook_name: &str,
        _state: Arc<crate::state::AppState>,
    ) -> Result<crate::registry::LuaRouter, ScriptError> {
        let script = {
            let scripts = self.scripts.read().await;
            scripts
                .get(module_code)
                .cloned()
                .ok_or_else(|| ScriptError::HookNotFound(module_code.to_string()))?
        };

        let lua = Lua::new();
        let danneo = lua.create_table()?;

        // Register minimal API for router definition
        let router_table = lua.create_table()?;
        router_table.set(
            "new",
            lua.create_function(|_, _: ()| Ok(crate::registry::LuaRouter::default()))?,
        )?;
        danneo.set("Router", router_table)?;
        lua.globals().set("danneo", danneo)?;

        lua.load(&script).exec_async().await?;

        let globals = lua.globals();
        let hook: mlua::Function = match globals.get(hook_name) {
            Ok(f) => f,
            Err(_) => return Err(ScriptError::HookNotFound(hook_name.to_string())),
        };

        let res: mlua::AnyUserData = hook.call_async(()).await?;
        let router = res
            .borrow::<crate::registry::LuaRouter>()
            .map_err(|e| ScriptError::Runtime(e.to_string()))?
            .clone();
        Ok(router)
    }
}

#[derive(Clone)]
struct DatabaseApi {
    db: Arc<DatabaseConnection>,
    module_code: String,
}

impl DatabaseApi {
    fn prefix_table(&self, table: &str) -> String {
        if table.starts_with("core_") {
            table.to_string()
        } else {
            format!("{}_{}", self.module_code, table)
        }
    }

    fn lua_error<E: std::fmt::Display>(e: E) -> mlua::Error {
        mlua::Error::RuntimeError(e.to_string())
    }

    pub fn register_to_table(&self, db_table: &mlua::Table, lua: &Lua) -> mlua::Result<()> {
        let api = self.clone();
        db_table.set(
            "insert",
            lua.create_async_function(move |lua, (table, data): (String, mlua::Value)| {
                let api = api.clone();
                async move {
                    let full_table = api.prefix_table(&table);
                    let data: serde_json::Value = lua.from_value(data)?;
                    crate::crud::insert_record(&api.db, &full_table, &data)
                        .await
                        .map_err(Self::lua_error)?;
                    Ok(mlua::Value::Nil)
                }
            })?,
        )?;

        let api = self.clone();
        db_table.set(
            "update",
            lua.create_async_function(
                move |lua, (table, pk_col, pk_val, data): (String, String, String, mlua::Value)| {
                    let api = api.clone();
                    async move {
                        let full_table = api.prefix_table(&table);
                        let data: serde_json::Value = lua.from_value(data)?;
                        crate::crud::update_record(&api.db, &full_table, &pk_col, &pk_val, &data)
                            .await
                            .map_err(Self::lua_error)?;
                        Ok(mlua::Value::Nil)
                    }
                },
            )?,
        )?;

        let api = self.clone();
        db_table.set(
            "delete",
            lua.create_async_function(
                move |_, (table, pk_col, pk_val): (String, String, String)| {
                    let api = api.clone();
                    async move {
                        let full_table = api.prefix_table(&table);
                        crate::crud::delete_record(&api.db, &full_table, &pk_col, &pk_val)
                            .await
                            .map_err(Self::lua_error)?;
                        Ok(mlua::Value::Nil)
                    }
                },
            )?,
        )?;

        let api = self.clone();
        db_table.set(
            "select",
            lua.create_async_function(move |lua, (table, columns): (String, Vec<String>)| {
                let api = api.clone();
                async move {
                    let full_table = api.prefix_table(&table);
                    let rows = crate::crud::select_all(&api.db, &full_table, &columns)
                        .await
                        .map_err(Self::lua_error)?;
                    lua.to_value(&rows)
                }
            })?,
        )?;

        let api = self.clone();
        db_table.set(
            "create_table",
            lua.create_async_function(move |lua, schema: mlua::Value| {
                let api = api.clone();
                async move {
                    let mut schema_val: serde_json::Value = lua.from_value(schema)?;
                    if let Some(obj) = schema_val.as_object_mut() {
                        if let Some(table_name) = obj.get_mut("table_name") {
                            let name = table_name.as_str().unwrap_or_default();
                            *table_name = json!(api.prefix_table(name));
                        }
                    }

                    let schema: crate::crud::EntitySchema =
                        serde_json::from_value(schema_val).map_err(Self::lua_error)?;
                    crate::crud::create_entity_table(&api.db, &schema)
                        .await
                        .map_err(Self::lua_error)?;
                    Ok(mlua::Value::Nil)
                }
            })?,
        )?;

        let api = self.clone();
        db_table.set(
            "drop_table",
            lua.create_async_function(move |_, table: String| {
                let api = api.clone();
                async move {
                    let full_table = api.prefix_table(&table);
                    crate::crud::drop_entity_table(&api.db, &full_table)
                        .await
                        .map_err(Self::lua_error)?;
                    Ok(mlua::Value::Nil)
                }
            })?,
        )?;

        let api = self.clone();
        db_table.set(
            "query",
            lua.create_async_function(move |lua, (sql, params): (String, Option<mlua::Value>)| {
                let api = api.clone();
                async move {
                    let json_params = if let Some(p) = params {
                        lua.from_value(p).ok()
                    } else {
                        None
                    };
                    api.query(&lua, sql, json_params).await
                }
            })?,
        )?;

        Ok(())
    }

    async fn query<'lua>(
        &self,
        lua: &'lua Lua,
        sql: String,
        _params: Option<serde_json::Value>,
    ) -> mlua::Result<mlua::Value> {
        use sea_orm::{ConnectionTrait, Statement};
        let db = self.db.as_ref();
        let backend = db.get_database_backend();

        let stmt = Statement::from_string(backend, sql);
        let rows = db
            .query_all(stmt)
            .await
            .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let mut map = serde_json::Map::new();
            for col in row.column_names() {
                map.insert(col.to_string(), crate::crud::row_value(&row, &col));
            }
            results.push(serde_json::Value::Object(map));
        }

        lua.to_value(&results)
    }
}
