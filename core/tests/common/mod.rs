use danneo_core::state::AppState;
use sea_orm::Database;
use std::sync::Arc;

pub async fn create_test_state() -> Arc<AppState> {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    use sea_orm_migration::MigratorTrait;
    migration::Migrator::up(&db, None).await.unwrap();

    // Set environment variables for AppState::new
    unsafe {
        std::env::set_var("JWT_SECRET", "test_secret");
    }

    // AppState::new expects some directories to exist.
    // If we are running from the project root, it should find them.
    // If not, it might fail.

    let state = AppState::new(db).await.expect("Failed to create AppState");
    Arc::new(state)
}
