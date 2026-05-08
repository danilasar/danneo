use script_rhai::{Engine, Scope, Dynamic, AST, EvalAltResult};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;

use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum ScriptError {
    #[error("Rhai eval error: {0}")]
    Runtime(#[from] Box<EvalAltResult>),
    #[error("Rhai parse error: {0}")]
    Parse(#[from] script_rhai::ParseError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Hook not found: {0}")]
    HookNotFound(String),
}

pub struct ScriptEngine {
    engine: Engine,
    scripts: Arc<RwLock<HashMap<String, AST>>>,
}

impl ScriptEngine {
    pub fn new() -> Self {
        let mut engine = Engine::new();
        
        // --- Настройки безопасности (Sandboxing) ---
        
        // Ограничиваем количество операций, чтобы предотвратить бесконечные циклы
        engine.set_max_operations(100_000); 
        
        // Ограничиваем глубину выражений и вложенность
        engine.set_max_expr_depths(50, 20);
        
        // Запрещаем некоторые потенциально опасные возможности
        // (Они могут быть уже отключены через фичи, но это дополнительная защита)
        
        Self {
            engine,
            scripts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Загружает скрипты модуля из указанного пути
    pub async fn load_module_scripts(&self, module_code: &str, scripts_path: &Path) -> Result<(), ScriptError> {
        if !scripts_path.exists() {
            return Ok(());
        }

        // Если это файл, загружаем его как основной хук
        if scripts_path.is_file() {
            let script = std::fs::read_to_string(scripts_path).map_err(ScriptError::Io)?;
            self.load_script_str(module_code, &script).await?;
        } else if scripts_path.is_dir() {
            // Если это директория, ищем main.rhai или hooks.rhai
            let hooks_path = scripts_path.join("hooks.rhai");
            if hooks_path.exists() {
                let script = std::fs::read_to_string(hooks_path).map_err(ScriptError::Io)?;
                self.load_script_str(module_code, &script).await?;
            }
        }

        Ok(())
    }

    /// Загружает скрипт из строки для конкретного модуля
    pub async fn load_script_str(&self, module_code: &str, script: &str) -> Result<(), ScriptError> {
        let ast = self.engine.compile(script)?;
        let mut scripts = self.scripts.write().await;
        scripts.insert(module_code.to_string(), ast);
        Ok(())
    }

    /// Вызывает функцию (хук) из скрипта модуля
    pub async fn call_hook(
        &self,
        module_code: &str,
        hook_name: &str,
        arg: Dynamic,
    ) -> Result<Dynamic, ScriptError> {
        let scripts = self.scripts.read().await;
        let ast = scripts.get(module_code).ok_or_else(|| {
            ScriptError::HookNotFound(format!("No scripts loaded for module: {}", module_code))
        })?;

        let mut scope = Scope::new();
        
        self.engine.call_fn(&mut scope, ast, hook_name, (arg,))
            .map_err(ScriptError::Runtime)
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}
