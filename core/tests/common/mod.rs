use danneo_core::state::AppState;
use sea_orm::Database;
use std::sync::Arc;

pub async fn create_test_state() -> Arc<AppState> {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    
    // Set environment variables for AppState::new
    unsafe {
        std::env::set_var("JWT_SECRET", "test_secret");
    }

    let state = AppState::new(db).await.expect("Failed to create AppState");
    Arc::new(state)
}
