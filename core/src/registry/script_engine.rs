use mlua::{Function, HookTriggers, Lua, LuaSerdeExt, Value, VmState};
use sea_orm::DatabaseConnection;
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::RwLock;

#[derive(Debug, thiserror::Error)]
pub enum ScriptError {
    #[error("Lua runtime error: {0}")]
    Runtime(String),
    #[error("Lua parse error: {0}")]
    Parse(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Hook not found: {0}")]
    HookNotFound(String),
}

const MAX_LUA_INSTRUCTIONS: usize = 100_000;

#[derive(Clone)]
struct DatabaseApi {
    db: Arc<DatabaseConnection>,
    module_code: String,
}

impl DatabaseApi {
    fn prefix_table(&self, table: &str) -> String {
        format!("mod_{}_{}", self.module_code, table)
    }

    fn lua_error(error: impl std::fmt::Display) -> mlua::Error {
        mlua::Error::external(error.to_string())
    }

    fn register(self, lua: &Lua) -> mlua::Result<()> {
        let db_table = lua.create_table()?;

        let api = self.clone();
        db_table.set(
            "insert",
            lua.create_async_function(move |lua, (table, data): (String, Value)| {
                let api = api.clone();
                async move {
                    let full_table = api.prefix_table(&table);
                    let data: serde_json::Value = lua.from_value(data)?;
                    crate::crud::insert_record(&api.db, &full_table, &data)
                        .await
                        .map_err(Self::lua_error)?;
                    Ok(Value::Nil)
                }
            })?,
        )?;

        let api = self.clone();
        db_table.set(
            "update",
            lua.create_async_function(
                move |lua, (table, pk_col, pk_val, data): (String, String, String, Value)| {
                    let api = api.clone();
                    async move {
                        let full_table = api.prefix_table(&table);
                        let data: serde_json::Value = lua.from_value(data)?;
                        crate::crud::update_record(&api.db, &full_table, &pk_col, &pk_val, &data)
                            .await
                            .map_err(Self::lua_error)?;
                        Ok(Value::Nil)
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
                        Ok(Value::Nil)
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
            lua.create_async_function(move |lua, schema: Value| {
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
                    Ok(Value::Nil)
                }
            })?,
        )?;

        let api = self;
        db_table.set(
            "drop_table",
            lua.create_async_function(move |_, table: String| {
                let api = api.clone();
                async move {
                    let full_table = api.prefix_table(&table);
                    crate::crud::drop_entity_table(&api.db, &full_table)
                        .await
                        .map_err(Self::lua_error)?;
                    Ok(Value::Nil)
                }
            })?,
        )?;

        lua.globals().set("db", db_table)
    }
}

pub struct ScriptEngine {
    scripts: Arc<RwLock<HashMap<String, String>>>,
    db: Arc<DatabaseConnection>,
}

impl ScriptEngine {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            scripts: Arc::new(RwLock::new(HashMap::new())),
            db,
        }
    }

    /// Загружает скрипты модуля из указанного пути
    pub async fn load_module_scripts(
        &self,
        module_code: &str,
        scripts_path: &Path,
    ) -> Result<(), ScriptError> {
        if !scripts_path.exists() {
            return Ok(());
        }

        if scripts_path.is_file() {
            let script = std::fs::read_to_string(scripts_path).map_err(ScriptError::Io)?;
            self.load_script_str(module_code, &script).await?;
        } else if scripts_path.is_dir() {
            let lua_hooks_path = scripts_path.join("hooks.lua");
            let rhai_hooks_path = scripts_path.join("hooks.rhai");
            let hooks_path = if lua_hooks_path.exists() {
                lua_hooks_path
            } else {
                rhai_hooks_path
            };

            if hooks_path.exists() {
                let script = std::fs::read_to_string(hooks_path).map_err(ScriptError::Io)?;
                self.load_script_str(module_code, &script).await?;
            }
        }

        Ok(())
    }

    /// Загружает скрипт из строки для конкретного модуля
    pub async fn load_script_str(
        &self,
        module_code: &str,
        script: &str,
    ) -> Result<(), ScriptError> {
        Lua::new()
            .load(script)
            .into_function()
            .map_err(|e| ScriptError::Parse(e.to_string()))?;

        let mut scripts = self.scripts.write().await;
        scripts.insert(module_code.to_string(), script.to_string());
        Ok(())
    }

    /// Вызывает функцию (хук) из скрипта модуля
    pub async fn call_hook(
        &self,
        module_code: &str,
        hook_name: &str,
        arg: script_rhai::Dynamic,
    ) -> Result<script_rhai::Dynamic, ScriptError> {
        let script = {
            let scripts = self.scripts.read().await;
            scripts.get(module_code).cloned().ok_or_else(|| {
                ScriptError::HookNotFound(format!("No scripts loaded for module: {}", module_code))
            })?
        };

        let lua = Lua::new();
        let instruction_count = Arc::new(AtomicUsize::new(0));
        let hook_count = instruction_count.clone();
        lua.set_global_hook(
            HookTriggers::new().every_nth_instruction(1_000),
            move |_, _| {
                let count = hook_count.fetch_add(1_000, Ordering::Relaxed) + 1_000;
                if count > MAX_LUA_INSTRUCTIONS {
                    Err(mlua::Error::RuntimeError(
                        "Too many operations in Lua script".to_string(),
                    ))
                } else {
                    Ok(VmState::Continue)
                }
            },
        )
        .map_err(|e| ScriptError::Runtime(e.to_string()))?;

        DatabaseApi {
            db: self.db.clone(),
            module_code: module_code.to_string(),
        }
        .register(&lua)
        .map_err(|e| ScriptError::Runtime(e.to_string()))?;

        lua.load(&script)
            .exec_async()
            .await
            .map_err(|e| ScriptError::Runtime(e.to_string()))?;

        let hook: Function = lua.globals().get(hook_name).map_err(|_| {
            ScriptError::HookNotFound(format!("Hook not found: {}::{}", module_code, hook_name))
        })?;

        let arg = if arg.is_unit() {
            Value::Nil
        } else {
            let json_arg = script_rhai::serde::from_dynamic::<serde_json::Value>(&arg)
                .map_err(|e| ScriptError::Runtime(e.to_string()))?;
            lua.to_value(&json_arg)
                .map_err(|e| ScriptError::Runtime(e.to_string()))?
        };

        let result: Value = hook
            .call_async(arg)
            .await
            .map_err(|e| ScriptError::Runtime(e.to_string()))?;

        let result = match result {
            Value::Nil => script_rhai::Dynamic::UNIT,
            value => {
                let json_value: serde_json::Value = lua
                    .from_value(value)
                    .map_err(|e| ScriptError::Runtime(e.to_string()))?;
                script_rhai::serde::to_dynamic(json_value)
                    .map_err(|e| ScriptError::Runtime(e.to_string()))?
            }
        };

        Ok(result)
    }
}
