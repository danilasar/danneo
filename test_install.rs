use danneo_core::state::AppState;
use sea_orm::Database;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    // But we need the real database to see the error.
}
