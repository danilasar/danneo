use crate::state::AppState;
use std::path::PathBuf;
use std::sync::Arc;

pub struct TestRunner {
    pub module_name: String,
    pub base_path: PathBuf,
}

#[derive(Debug, PartialEq)]
pub enum TestStatus {
    Passed,
    Failed(String),
}

pub struct TestReport {
    pub name: String,
    pub status: TestStatus,
}

impl TestRunner {
    pub fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            base_path: PathBuf::from("modules").join(module_name),
        }
    }

    pub async fn run_all(&self, unit: bool, integration: bool) -> Result<(), String> {
        if !self.base_path.exists() {
            return Err(format!("Module directory not found: {:?}", self.base_path));
        }
        let mut reports = Vec::new();
        let run_all = !unit && !integration;

        if unit || run_all {
            let unit_tests = self.scan_unit_tests();
            if !unit_tests.is_empty() {
                println!("Running unit tests for module {}...", self.module_name);
                for file in unit_tests {
                    match Self::run_unit_test_file(&file) {
                        Ok(mut file_reports) => reports.append(&mut file_reports),
                        Err(e) => {
                            return Err(format!("Error running unit test file {:?}: {}", file, e));
                        }
                    }
                }
            }
        }

        if integration || run_all {
            let integration_tests = self.scan_integration_tests();
            if !integration_tests.is_empty() {
                println!(
                    "Running integration tests for module {}...",
                    self.module_name
                );
                for file in integration_tests {
                    let status =
                        Self::run_integration_test_file(file.clone(), self.module_name.clone());
                    reports.push(TestReport {
                        name: file.to_string_lossy().to_string(),
                        status,
                    });
                }
            }
        }

        if reports.is_empty() {
            println!("No tests found for module {}.", self.module_name);
            return Ok(());
        }

        let mut passed = 0;
        let mut failed = 0;

        for report in &reports {
            match &report.status {
                TestStatus::Passed => {
                    println!("  ✅ {} - OK", report.name);
                    passed += 1;
                }
                TestStatus::Failed(msg) => {
                    println!("  ❌ {} - FAILED", report.name);
                    println!("     Error: {}", msg);
                    failed += 1;
                }
            }
        }

        println!(
            "\nSummary: Total: {}, Passed: {}, Failed: {}",
            reports.len(),
            passed,
            failed
        );

        if failed > 0 {
            return Err(format!("{} tests failed", failed));
        }

        Ok(())
    }

    pub fn inject_assert(lua: &mlua::Lua) -> Result<(), mlua::Error> {
        let assert = lua.create_table()?;

        assert.set(
            "is_true",
            lua.create_function(|_, val: bool| {
                if !val {
                    return Err(mlua::Error::RuntimeError(
                        "Assertion failed: expected true, got false".into(),
                    ));
                }
                Ok(())
            })?,
        )?;

        assert.set(
            "is_false",
            lua.create_function(|_, val: bool| {
                if val {
                    return Err(mlua::Error::RuntimeError(
                        "Assertion failed: expected false, got true".into(),
                    ));
                }
                Ok(())
            })?,
        )?;

        assert.set(
            "equals",
            lua.create_function(|_, (a, b): (mlua::Value, mlua::Value)| {
                if a != b {
                    return Err(mlua::Error::RuntimeError(format!(
                        "Assertion failed: values are not equal. Left: {:?}, Right: {:?}",
                        a, b
                    )));
                }
                Ok(())
            })?,
        )?;

        assert.set(
            "not_equals",
            lua.create_function(|_, (a, b): (mlua::Value, mlua::Value)| {
                if a == b {
                    return Err(mlua::Error::RuntimeError(format!(
                        "Assertion failed: values are equal. Both are: {:?}",
                        a
                    )));
                }
                Ok(())
            })?,
        )?;

        assert.set(
            "is_nil",
            lua.create_function(|_, val: mlua::Value| {
                if !matches!(val, mlua::Value::Nil) {
                    return Err(mlua::Error::RuntimeError(
                        "Assertion failed: expected nil".into(),
                    ));
                }
                Ok(())
            })?,
        )?;

        assert.set(
            "is_not_nil",
            lua.create_function(|_, val: mlua::Value| {
                if matches!(val, mlua::Value::Nil) {
                    return Err(mlua::Error::RuntimeError(
                        "Assertion failed: expected not nil".into(),
                    ));
                }
                Ok(())
            })?,
        )?;

        lua.globals().set("assert", assert)?;
        Ok(())
    }

    pub fn run_unit_test_file(path: &std::path::Path) -> Result<Vec<TestReport>, String> {
        let lua = mlua::Lua::new();
        Self::inject_assert(&lua).map_err(|e| e.to_string())?;

        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        lua.load(&content).exec().map_err(|e| e.to_string())?;

        let mut reports = Vec::new();
        let globals = lua.globals();

        for pair in globals.pairs::<mlua::Value, mlua::Value>() {
            let (key, value) = pair.map_err(|e| e.to_string())?;
            if let mlua::Value::String(name) = key {
                let name_str = name.to_str().map_err(|e| e.to_string())?;
                if name_str.starts_with("test_") {
                    if let mlua::Value::Function(func) = value {
                        let status = match func.call::<()>(()) {
                            Ok(_) => TestStatus::Passed,
                            Err(e) => TestStatus::Failed(e.to_string()),
                        };
                        reports.push(TestReport {
                            name: name_str.to_string(),
                            status,
                        });
                    }
                }
            }
        }

        Ok(reports)
    }

    pub fn scan_unit_tests(&self) -> Vec<PathBuf> {
        let pattern = self.base_path.join("tests/unit/*.lua");
        glob::glob(pattern.to_str().unwrap_or(""))
            .unwrap()
            .filter_map(Result::ok)
            .collect()
    }

    pub fn scan_integration_tests(&self) -> Vec<PathBuf> {
        let pattern = self.base_path.join("tests/integration/*.lua");
        glob::glob(pattern.to_str().unwrap_or(""))
            .unwrap()
            .filter_map(Result::ok)
            .collect()
    }

    pub async fn boot_test_environment(module_name: &str) -> Result<Arc<AppState>, String> {
        let db = sea_orm::Database::connect("sqlite::memory:")
            .await
            .map_err(|e| e.to_string())?;

        // AppState::new already runs migrations and bootstrap
        let state = crate::state::init_state(db).await?;

        // Install the module under test if not already installed by bootstrap
        let installer = crate::registry::installer::PackageInstaller::new(
            state.db.clone(),
            state.packages.clone(),
            state.modules.clone(),
            state.routes.clone(),
            state.script_engine.clone(),
            state.clone(),
        );

        let module_path = PathBuf::from("modules").join(module_name);
        if module_path.exists() {
            installer
                .install_from_staging(module_name, &module_path)
                .await
                .map_err(|e| format!("Module installation failed: {}", e))?;
        }

        Ok(state)
    }

    pub fn run_integration_test_file(path: PathBuf, module_name: String) -> TestStatus {
        use nix::sys::wait::waitpid;
        use nix::unistd::{ForkResult, fork};

        match unsafe { fork() } {
            Ok(ForkResult::Parent { child }) => match waitpid(child, None) {
                Ok(nix::sys::wait::WaitStatus::Exited(_, code)) => {
                    if code == 0 {
                        TestStatus::Passed
                    } else {
                        TestStatus::Failed(format!("Child process exited with code {}", code))
                    }
                }
                Ok(s) => TestStatus::Failed(format!("Child process ended unexpectedly: {:?}", s)),
                Err(e) => TestStatus::Failed(format!("Waitpid error: {}", e)),
            },
            Ok(ForkResult::Child) => {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();

                let result = rt.block_on(async {
                    let _state = Self::boot_test_environment(&module_name).await?;
                    let lua = mlua::Lua::new();
                    Self::inject_assert(&lua).map_err(|e| e.to_string())?;

                    // Inject danneo API (mocked or partial)
                    // For now just basic injection

                    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
                    lua.load(&content).exec().map_err(|e| e.to_string())?;

                    let globals = lua.globals();
                    for pair in globals.pairs::<mlua::Value, mlua::Value>() {
                        let (key, value) = pair.map_err(|e| e.to_string())?;
                        if let mlua::Value::String(name) = key {
                            let name_str = name.to_str().map_err(|e| e.to_string())?;
                            if name_str.starts_with("test_") {
                                if let mlua::Value::Function(func) = value {
                                    func.call::<()>(()).map_err(|e| e.to_string())?;
                                }
                            }
                        }
                    }
                    Ok::<(), String>(())
                });

                match result {
                    Ok(_) => std::process::exit(0),
                    Err(e) => {
                        eprintln!("Integration test failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => TestStatus::Failed(format!("Fork failed: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_all_modules_lua_tests() {
        let mut modules_path = std::path::Path::new("../modules");
        if !modules_path.exists() {
            // If running from core/ directory
            modules_path = std::path::Path::new("modules");
            if !modules_path.exists() {
                return; // Skip if no modules dir found
            }
        }

        for entry in std::fs::read_dir(modules_path).unwrap() {
            let entry = entry.unwrap();
            if entry.path().is_dir() {
                let name = entry.file_name().into_string().unwrap();
                let runner = TestRunner::new(&name);
                // We only run unit tests here to avoid fork() complexity and DB setup in cargo test
                // Unless we want full integration.
                let unit_tests = runner.scan_unit_tests();
                for file in unit_tests {
                    let reports = TestRunner::run_unit_test_file(&file).unwrap();
                    for report in reports {
                        if let TestStatus::Failed(msg) = report.status {
                            panic!("Module {} unit test {} failed: {}", name, report.name, msg);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_scanner_finds_files() {
        let dir = tempdir().unwrap();
        let module_path = dir.path().join("modules").join("test_mod");
        fs::create_dir_all(module_path.join("tests/unit")).unwrap();
        fs::create_dir_all(module_path.join("tests/integration")).unwrap();

        fs::write(
            module_path.join("tests/unit/test_1.lua"),
            "function test_x() end",
        )
        .unwrap();
        fs::write(
            module_path.join("tests/integration/test_db.lua"),
            "function test_db() end",
        )
        .unwrap();

        let mut runner = TestRunner::new("test_mod");
        runner.base_path = module_path; // Override for test

        let unit_tests = runner.scan_unit_tests();
        assert_eq!(unit_tests.len(), 1);
        assert!(unit_tests[0].to_str().unwrap().contains("test_1.lua"));

        let integration_tests = runner.scan_integration_tests();
        assert_eq!(integration_tests.len(), 1);
        assert!(
            integration_tests[0]
                .to_str()
                .unwrap()
                .contains("test_db.lua")
        );
    }

    #[test]
    fn test_run_unit_tests_success() {
        let dir = tempdir().unwrap();
        let lua_file = dir.path().join("test_success.lua");
        fs::write(
            &lua_file,
            r#"
            function test_math()
                assert.equals(4, 2 + 2)
            end
            function test_logic()
                assert.is_true(true)
            end
        "#,
        )
        .unwrap();

        let reports = TestRunner::run_unit_test_file(&lua_file).unwrap();
        assert_eq!(reports.len(), 2);
        assert!(reports.iter().all(|r| r.status == TestStatus::Passed));
    }

    #[test]
    fn test_run_unit_tests_failure() {
        let dir = tempdir().unwrap();
        let lua_file = dir.path().join("test_fail.lua");
        fs::write(
            &lua_file,
            r#"
            function test_fail()
                assert.equals(5, 2 + 2)
            end
        "#,
        )
        .unwrap();

        let reports = TestRunner::run_unit_test_file(&lua_file).unwrap();
        assert_eq!(reports.len(), 1);
        match &reports[0].status {
            TestStatus::Failed(msg) => assert!(msg.contains("Assertion failed")),
            _ => panic!("Expected failure"),
        }
    }
}
