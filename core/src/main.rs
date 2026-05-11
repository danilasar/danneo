use clap::{Parser, Subcommand};
use danneo_core::cli::test_runner::TestRunner;

#[derive(Parser)]
#[command(name = "danneo")]
#[command(about = "Danneo CMS CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run tests for a module
    Test {
        /// Name of the module (optional, runs all if omitted)
        module_name: Option<String>,
        /// Run only unit tests
        #[arg(long)]
        unit: bool,
        /// Run only integration tests
        #[arg(long)]
        integration: bool,
    },
    /// Start the CMS server (default)
    Run,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Test {
            module_name,
            unit,
            integration,
        }) => {
            if let Some(name) = module_name {
                let runner = TestRunner::new(&name);
                if let Err(e) = runner.run_all(unit, integration).await {
                    eprintln!("Test runner error: {}", e);
                    std::process::exit(1);
                }
            } else {
                // Run for all modules
                let modules_path = std::path::Path::new("modules");
                if !modules_path.exists() {
                    eprintln!("'modules' directory not found.");
                    std::process::exit(1);
                }

                let mut any_failed = false;
                let mut modules_tested = 0;
                for entry in std::fs::read_dir(modules_path)? {
                    let entry = entry?;
                    if entry.path().is_dir() {
                        let name = entry.file_name().into_string().unwrap();
                        let runner = TestRunner::new(&name);

                        // Only print if there are tests
                        let unit_tests = runner.scan_unit_tests();
                        let integration_tests = runner.scan_integration_tests();

                        if !unit_tests.is_empty() || !integration_tests.is_empty() {
                            println!("\n--- Testing module: {} ---", name);
                            modules_tested += 1;
                            if let Err(e) = runner.run_all(unit, integration).await {
                                eprintln!("Test runner error for module {}: {}", name, e);
                                any_failed = true;
                            }
                        }
                    }
                }

                if modules_tested == 0 {
                    println!("No tests found in any module.");
                } else {
                    println!("\nFinished testing {} modules.", modules_tested);
                }

                if any_failed {
                    std::process::exit(1);
                }
            }
            Ok(())
        }
        Some(Commands::Run) | None => danneo_core::run().await,
    }
}
