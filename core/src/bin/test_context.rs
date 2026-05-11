use sea_orm::Database;
use tera::Context;

#[tokio::main]
async fn main() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    use sea_orm_migration::MigratorTrait;
    migration::Migrator::up(&db, None).await.unwrap();
    let state = danneo_core::state::init_state(db).await.unwrap();

    let mut context = Context::new();
    danneo_core::apanel::prepare_admin_context(state.clone(), &mut context).await;

    println!("Context json: {:?}", context.into_json());
}
