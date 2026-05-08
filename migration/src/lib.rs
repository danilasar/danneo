pub use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260508_000001_create_core_tables::Migration),
            Box::new(m20260508_000002_create_block_tables::Migration),
            Box::new(m20260508_000003_create_menu_tables::Migration),
            Box::new(m20260508_000004_update_core_admins::Migration),
            Box::new(m20260508_000005_add_admin_groups_and_levels::Migration),
            Box::new(m20260508_000006_create_module_registry::Migration),
        ]
    }
}

pub mod m20260508_000001_create_core_tables;
pub mod m20260508_000002_create_block_tables;
pub mod m20260508_000003_create_menu_tables;
pub mod m20260508_000004_update_core_admins;
pub mod m20260508_000005_add_admin_groups_and_levels;
pub mod m20260508_000006_create_module_registry;
